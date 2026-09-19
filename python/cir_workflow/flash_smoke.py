"""Flash smoke batch: validate the multi-arm chain on a few frozen tasks.

This is the only live entry point for the v2 harness. It reuses
:mod:`cir_workflow.live` for the pinned DeepSeek Flash client, the shared
persisted budget, the model-identity check and the sanitized evidence log, and
reuses the offline arm runners for the actual per-arm loops. It refuses to run
unless the frozen protocol's sha256 matches the confirmation credential.

It is a chain-validation smoke run, not an effect-size experiment: 3 tasks, 6
arms, K=4, <= 48 HTTP requests, <= 60 minutes.
"""

from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path
from typing import Any

from .arms import run_cir_arm, run_rust_arm
from .concir_client import ConcirClient
from .extract import extraction_prompt, parse_extraction, schema_text, validate_extraction
from .experiments_v2 import K, load_manifest
from .live import (
    ALLOWED_PROVIDER, DEFAULT_MAX_SECONDS, DeepSeekFlashClient, LiveBudget,
    RecordingPatchProvider, RecordingProvider, assert_allowed_model,
)
from .offline_workflow import _exclusive_run_dir
from .patch_repair import ExternalPatchRepairWorkflow
from .providers import CandidateRequest, CandidateResponse
from .experiments_v2 import (
    ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER, ARM_OURS_REVISION, ARM_OURS_PATCH,
)

PROMPTS = Path(__file__).resolve().parents[2] / "prompts"
SMOKE_HTTP_CAP = 48
SMOKE_WALL_CAP_S = 3600.0
SMOKE_TASKS = ("lock-order/abba_2lock",
               "condvar/lost_wakeup_notify_before_wait",
               "lock-order/partial_deadlock_bystander")
SMOKE_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter", "A3_ours_revision",
              "A3p_ours_patch", "A3_tool_repair")


def _read(name: str) -> str:
    return (PROMPTS / name).read_text(encoding="utf-8")


def spec_extract() -> str:
    return _read("rust_to_cir_extract_v2.md")


def _extract_and_validate(api_key: str, budget: LiveBudget, batch: Path, system: str,
                          rust_source: str, contract_path: Path, work_dir: Path,
                          binary: Path | str, sdk_client: Any, timeout: float,
                          max_tokens: int) -> dict[str, Any]:
    """One extraction request + conformance validation (returns a model verdict)."""

    llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                              evidence_dir=batch / "llm", timeout=timeout,
                              max_tokens=max_tokens, sdk_client=sdk_client)
    try:
        schema = schema_text(binary)
        outcome = llm.complete(system, extraction_prompt(schema, rust_source))
    except Exception as exc:  # noqa: BLE001
        return {"extract_validated": False, "reason": f"{type(exc).__name__}: {exc}",
                "model_verdict": None}
    parsed = parse_extraction(outcome.text)
    if not parsed or "cir" not in parsed or "rust" not in parsed:
        # One format-feedback retry (counts against the budget).
        if not budget.exhausted():
            try:
                retry = llm.complete(
                    system,
                    extraction_prompt(schema_text(binary), rust_source)
                    + "\n\nYour previous reply was not a JSON object with keys `cir` "
                      "and `rust`, or its CIR had malformed statements. Reply again "
                      "with only that JSON object; every CIR statement must carry a "
                      "`\"sid\"` string like `s1`, `s2`, ... and the annotated Rust "
                      "must call `cir_trace::finish()`.")
                parsed = parse_extraction(retry.text)
            except Exception:  # noqa: BLE001
                parsed = None
    if not parsed or "cir" not in parsed or "rust" not in parsed:
        return {"extract_validated": False, "reason": "reply not a {cir,rust} object",
                "model_verdict": None}
    try:
        return validate_extraction(parsed["cir"], str(parsed["rust"]), contract_path,
                                   work_dir, binary=binary)
    except Exception as exc:  # noqa: BLE001
        return {"extract_validated": False, "reason": f"{type(exc).__name__}: {exc}",
                "model_verdict": None}


def _rust_system(mode: str) -> str:
    if mode == "self":
        return _read("rust_self_review_v1.md")
    if mode == "tools":
        return _read("rust_tool_feedback_v1.md")
    return _read("rust_generation_v1.md")


class RustLiveProvider:
    """Adapt the pinned Flash client to the Rust arm provider protocol."""

    name = "llm"

    def __init__(self, client: DeepSeekFlashClient, mode: str) -> None:
        self.client = client
        self.mode = mode
        self.calls: list[dict[str, Any]] = []

    def propose(self, request: CandidateRequest) -> CandidateResponse:
        system = _rust_system(self.mode)
        has_program = bool(request.previous_candidate)
        if request.feedback is None and not has_program:
            user = (f"Specification:\n{request.requirements}\n\n"
                    "Output the complete Rust program.")
        elif self.mode == "tools":
            diagnostics = request.feedback or "(no tool diagnostics yet)"
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    f"Tool diagnostics:\n{diagnostics}\n\n"
                    "Output the complete corrected Rust program.")
        else:
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    "Fix any concurrency defect and output the complete corrected Rust "
                    "program, or if there is none reply exactly NO_ISSUES.")
        outcome = self.client.complete(system, user)
        self.calls.append({
            "attempt": request.attempt, "request_id": outcome.request_id,
            "response_model": outcome.response_model,
            "usage": outcome.usage, "wall_ms": outcome.wall_ms,
            "transport_attempt": outcome.transport_attempt,
            "prompt_sha256": outcome.prompt_sha256,
        })
        response = CandidateResponse(
            text=outcome.text, source="llm", provider=ALLOWED_PROVIDER,
            model_id=outcome.response_model or "deepseek-flash", usage=outcome.usage)
        response.wall_ms = outcome.wall_ms
        return response


def assert_protocol_confirmed(protocol_path: Path | str, expected_sha256: str) -> None:
    import hashlib

    actual = hashlib.sha256(Path(protocol_path).read_bytes()).hexdigest()
    if actual != expected_sha256:
        raise RuntimeError("live smoke refused: protocol hash mismatch "
                           f"({actual} != {expected_sha256})")


def run_flash_smoke(manifest_path: Path | str, out_dir: Path | str, *, binary: Path | str,
                    api_key: str, protocol_path: Path | str, protocol_sha256: str,
                    k: int = K, timeout: float = 90.0, max_tokens: int = 4096,
                    sdk_client: Any | None = None,
                    tasks_filter: tuple[str, ...] | None = None) -> dict[str, Any]:
    assert_protocol_confirmed(protocol_path, protocol_sha256)
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=SMOKE_HTTP_CAP,
                        max_seconds=min(SMOKE_WALL_CAP_S, DEFAULT_MAX_SECONDS))
    wanted = tasks_filter or SMOKE_TASKS
    tasks = {t.id: t for t in load_manifest(manifest_path)}

    summary: dict[str, Any] = {
        "batch_dir": str(batch),
        "provider": ALLOWED_PROVIDER, "requested_model": "deepseek-flash",
        "protocol_sha256": protocol_sha256, "k": k,
        "max_requests": budget.max_requests, "max_seconds": budget.max_seconds,
        "tasks": [], "stop_reason": None,
    }
    stop_reason = None
    for task_id in wanted:
        task = tasks.get(task_id)
        if task is None:
            summary["tasks"].append({"task": task_id, "skipped": "not_in_manifest"})
            continue
        reason = budget.exhausted()
        if reason:
            summary["tasks"].append({"task": task_id, "skipped": reason})
            stop_reason = reason
            break
        task_dir = batch / task_id.replace("/", "__")
        contract_path = root / task.contract
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        spec = (root / task.spec).read_text(encoding="utf-8") if task.spec else task_id
        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        record: dict[str, Any] = {"task": task_id, "arms": {}}
        try:
            # A0 / A1 / A2 (Rust), then A3 (CIR), A3p, tool-repair
            for arm in (ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER):
                mode = {"A0_direct": "direct", "A1_self_iter": "self",
                        "A2_tools_iter": "tools"}[arm]
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                provider = RustLiveProvider(client_llm, mode)
                run = run_rust_arm(provider, arm=arm, task=task_id, spec=spec,
                                   contract=contract, out_dir=task_dir / arm,
                                   k=k, tool_timeout_s=60.0)
                record["arms"][arm] = run.as_dict()
                record["arms"][arm]["llm_calls"] = provider.calls
                if budget.exhausted():
                    stop_reason = budget.exhausted()
                    break
            if not stop_reason:
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                cir_provider = RecordingProvider(client_llm)
                a3 = run_cir_arm(client, cir_provider, arm=ARM_OURS_REVISION, task=task_id,
                                 spec=spec, contract=contract, out_dir=task_dir / "A3",
                                 k=k)
                record["arms"][ARM_OURS_REVISION] = a3.as_dict()
                record["arms"][ARM_OURS_REVISION]["llm_calls"] = cir_provider.calls
            if not stop_reason and task.buggy_cir:
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                patch_provider = RecordingPatchProvider(client_llm)
                a3p = ExternalPatchRepairWorkflow(
                    client, patch_provider, out_dir=task_dir / ARM_OURS_PATCH,
                    max_rounds=k).run(root / task.buggy_cir, contract_path)
                payload = a3p.__dict__
                payload["rounds"] = [r.__dict__ for r in a3p.rounds]
                payload["llm_calls"] = patch_provider.calls
                record["arms"][ARM_OURS_PATCH] = payload
            if not stop_reason and task.buggy_cir:
                repair = client.repair(root / task.buggy_cir, contract_path, strategy="c")
                record["arms"]["A3_tool_repair"] = {
                    "status": repair.status, "outcome": repair.outcome,
                    "complete": repair.complete, "artifact_path": repair.artifact_path,
                    "wall_ms": repair.wall_ms,
                }
        except Exception as exc:  # noqa: BLE001 - stop the batch and record
            stop_reason = f"{type(exc).__name__}: {exc}"
            record["error"] = stop_reason
        summary["tasks"].append(record)
        if stop_reason:
            summary["stop_reason"] = stop_reason
            break

    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (batch / "SUMMARY.md").write_text(render_smoke_markdown(summary), encoding="utf-8")
    return summary


REPAIR_TASKS = ("lock-order/abba_2lock", "lock-order/partial_deadlock_bystander",
                "condvar/bare_wait_no_predicate")


def run_repair_smoke(manifest_path: Path | str, out_dir: Path | str, *, binary: Path | str,
                     api_key: str, protocol_path: Path | str, protocol_sha256: str,
                     k: int = K, timeout: float = 90.0, max_tokens: int = 4096,
                     sdk_client: Any | None = None,
                     tasks_filter: tuple[str, ...] | None = None) -> dict[str, Any]:
    """Repair-type arms: the defective input is guaranteed, acceptance is
    measured against an independent terminal oracle (three columns)."""

    from .arms import rust_oracle

    assert_protocol_confirmed(protocol_path, protocol_sha256)
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=SMOKE_HTTP_CAP,
                        max_seconds=SMOKE_WALL_CAP_S)
    wanted = tasks_filter or REPAIR_TASKS
    tasks = {t.id: t for t in load_manifest(manifest_path)}
    summary: dict[str, Any] = {
        "batch_dir": str(batch), "provider": ALLOWED_PROVIDER,
        "requested_model": "deepseek-flash", "protocol_sha256": protocol_sha256,
        "k": k, "max_requests": budget.max_requests, "tasks": [], "stop_reason": None,
    }
    stop_reason = None
    for task_id in wanted:
        task = tasks.get(task_id)
        if task is None or not task.buggy_cir:
            summary["tasks"].append({"task": task_id, "skipped": "no_buggy_cir"})
            continue
        if budget.exhausted():
            stop_reason = budget.exhausted()
            break
        task_dir = batch / task_id.replace("/", "__")
        # Prefer the de-leaked repair_input declared by repair_task.json.
        repair_path = root / task.directory / "repair_task.json"
        if repair_path.is_file():
            rt = json.loads(repair_path.read_text(encoding="utf-8"))
            contract_path = root / rt["contract"]
            spec = (root / rt["requirements_file"]).read_text(encoding="utf-8")
            buggy_cir = root / rt["input_cir"]
            input_rust = ((root / rt["input_rust"]).read_text(encoding="utf-8")
                          if rt.get("input_rust") else None)
        else:
            contract_path = root / task.contract
            spec = (root / task.spec).read_text(encoding="utf-8") if task.spec else task_id
            buggy_cir = root / task.buggy_cir
            input_rust = ((root / task.buggy_rs).read_text(encoding="utf-8")
                          if task.buggy_rs else None)
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        record: dict[str, Any] = {"task": task_id, "arms": {}}
        try:
            if input_rust is not None:
                for arm, tier in ((ARM_DIRECT, "ml"), ("A1_self_iter", "ml"),
                                  ("A2_tools_iter_m", "m"), ("A2_tools_iter_ml", "ml")):
                    if budget.exhausted():
                        break
                    mode = {"A0_direct": "direct", "A1_self_iter": "self"}.get(arm, "tools")
                    llm = DeepSeekFlashClient(
                        api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                        timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                    provider = RustLiveProvider(llm, mode)
                    run = run_rust_arm(provider, arm=arm, task=task_id, spec=spec,
                                       contract=contract, out_dir=task_dir / arm,
                                       k=k, initial_source=input_rust,
                                       tool_timeout_s=8.0, run_miri_many_seeds=False)
                    arm_record = run.as_dict()
                    arm_record["llm_calls"] = provider.calls
                    if run.final_artifact_path:
                        arm_record["oracle"] = rust_oracle(
                            run.final_artifact_path, run_miri=True, timeout_s=8.0,
                            run_miri_many_seeds=False, miri_seed_count=16)
                        # Extraction oracle (<=2 requests per candidate).
                        if (not budget.exhausted()
                                and arm_record["oracle"].get("build_ok")):
                            source = Path(run.final_artifact_path).read_text(encoding="utf-8")
                            arm_record["oracle"]["model"] = _extract_and_validate(
                                api_key, budget, batch, spec_extract(), source,
                                contract_path, task_dir / arm / "extract",
                                binary, sdk_client, timeout, max_tokens)
                    else:
                        arm_record["oracle"] = {"build_ok": None, "miri_detected": None,
                                                "behavior_ok": None, "bug_present": None}
                    model = (arm_record["oracle"].get("model") or {})
                    arm_record["oracle"]["bug_present"] = bool(
                        arm_record["oracle"].get("behavior_ok") is False
                        or (model.get("extract_validated") is True
                            and model.get("model_verdict") == "FAIL"))
                    record["arms"][arm] = arm_record
            # A3: revise the buggy CIR.
            if not budget.exhausted():
                llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                cir_provider = RecordingProvider(llm)
                a3 = run_cir_arm(client, cir_provider, arm=ARM_OURS_REVISION,
                                 task=task_id, spec=spec, contract=contract,
                                 out_dir=task_dir / ARM_OURS_REVISION, k=k,
                                 initial_program=buggy_cir)
                a3_record = a3.as_dict()
                a3_record["llm_calls"] = cir_provider.calls
                if a3.final_artifact_path:
                    from .arms import cir_oracle
                    co = cir_oracle(client, a3.final_artifact_path, contract_path)
                    passed = co.get("verify_pass") is True
                    a3_record["oracle"] = {
                        "build_ok": None, "behavior_ok": None, "miri_detected": None,
                        "model": {"extract_validated": True,
                                  "model_verdict": "PASS" if passed else "FAIL",
                                  "verify_pass": co.get("verify_pass"),
                                  "evidence": co.get("evidence")},
                        "bug_present": not passed,
                    }
                record["arms"][ARM_OURS_REVISION] = a3_record
        except Exception as exc:  # noqa: BLE001
            stop_reason = f"{type(exc).__name__}: {exc}"
            record["error"] = stop_reason
        summary["tasks"].append(record)
        if stop_reason:
            summary["stop_reason"] = stop_reason
            break
    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (batch / "SUMMARY.md").write_text(render_repair_md(summary), encoding="utf-8")
    return summary


def render_repair_md(summary: dict[str, Any]) -> str:
    lines = ["# Flash repair smoke — SUMMARY", "",
             f"- batch dir: `{summary['batch_dir']}`",
             f"- requests used: {summary.get('requests_used')}",
             f"- stop reason: {summary.get('stop_reason')}", "",
             "| task | arm | accepted | round | oracle.build | oracle.behavior | oracle.miri | oracle.model | false_accept |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for task in summary["tasks"]:
        for arm, run in (task.get("arms") or {}).items():
            oracle = run.get("oracle") or {}
            bug = oracle.get("bug_present")
            fa = (run.get("accepted") and bug is True) if bug is not None else None
            model = oracle.get("model") or {}
            if model.get("extract_validated") is True:
                model_str = f"validated:{model.get('model_verdict')}"
            elif model:
                model_str = f"unverified:{model.get('reason', '')}"[:60]
            else:
                model_str = "inconclusive"
            lines.append(
                f"| {task['task']} | {arm} | {run.get('accepted')} | "
                f"{run.get('accepted_round')} | {oracle.get('build_ok')} | "
                f"{oracle.get('behavior_status')} | {oracle.get('miri_detected')} | "
                f"{model_str} | {fa} |")
    return "\n".join(lines) + "\n"


def render_smoke_markdown(summary: dict[str, Any]) -> str:
    lines = ["# Flash smoke batch — SUMMARY", "",
             f"- batch dir: `{summary['batch_dir']}`",
             f"- protocol sha256: `{summary['protocol_sha256']}`",
             f"- requests used: {summary.get('requests_used')}",
             f"- stop reason: {summary.get('stop_reason')}", "",
             "| task | arm | accepted/status | round | http | tokens | llm_ms | tool_ms | notes |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for task in summary["tasks"]:
        if "arms" not in task:
            lines.append(f"| {task['task']} | — | — | — | — | — | — | — | {task.get('skipped', '')} |")
            continue
        for arm, run in task["arms"].items():
            if arm == ARM_OURS_PATCH:
                # ExternalPatchRepairWorkflow payload: status/rounds, no consumption
                lines.append(
                    f"| {task['task']} | {arm} | {run.get('status')} | "
                    f"{len(run.get('rounds') or [])} | — | — | — | — | "
                    f"{run.get('error') or ''} |")
                continue
            if arm == "A3_tool_repair":
                lines.append(
                    f"| {task['task']} | {arm} | {run.get('status')} | — | — | — | — "
                    f"| {run.get('wall_ms')} | {run.get('outcome')} |")
                continue
            consumption = run.get("consumption", {}) if isinstance(run, dict) else {}
            lines.append(
                f"| {task['task']} | {arm} | {run.get('accepted')} | "
                f"{run.get('accepted_round')} | {consumption.get('http_requests')} | "
                f"{consumption.get('total_tokens')} | {consumption.get('llm_wall_ms')} | "
                f"{consumption.get('tool_wall_ms')} | |")
    return "\n".join(lines) + "\n"


FILL_SYSTEM = (
    "You fill only the marked holes in a Rust skeleton. Replace ONLY the text of a "
    "`/* HOLE(id) expected: T */ Default::default()` placeholder; do not touch any "
    "other line, statement, or sid comment. Hole code is sequential local "
    "computation only: no synchronization, thread, channel, atomic, unsafe, or "
    "`cir_trace` construct, and no new `use`. Reply with a JSON object mapping each "
    "hole id to the Rust expression or statement block to substitute."
)

FREE_SYSTEM = (
    "You write a complete standard-library-only Rust program for the given ConcIR "
    "model. Insert `cir_trace::ev(\"<tag>\", \"<sid>\")` immediately before every "
    "concurrency operation, using tag `t0` for main and `t<sid>_<i>` for the i-th "
    "member of a `scope`, and the sid from the CIR. The program must define `fn "
    "main` and must not use any external crate. Output only the Rust source in one "
    "```rust fence."
)


def run_fill_smoke(program: Path | str, contract: Path | str, out_dir: Path | str, *,
                   binary: Path | str, api_key: str, protocol_path: Path | str,
                   protocol_sha256: str, timeout: float = 90.0,
                   max_tokens: int = 4096, native_runs: int = 50,
                   miri_seeds: int = 16, sdk_client: Any | None = None) -> dict[str, Any]:
    """Fill the codegen holes with Flash, validate by conformance; also run the
    `A3_free` ablation (LLM writes the whole annotated Rust)."""

    from . import conformance

    assert_protocol_confirmed(protocol_path, protocol_sha256)
    program = Path(program).resolve()
    contract = Path(contract).resolve()
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=6, max_seconds=1800.0)
    llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                              evidence_dir=batch / "llm", timeout=timeout,
                              max_tokens=max_tokens, sdk_client=sdk_client)
    summary: dict[str, Any] = {"batch_dir": str(batch), "program": str(program),
                               "arms": {}, "stop_reason": None}
    spec = (program.parent / "spec.md").read_text(encoding="utf-8")

    # ---- skeleton arm (fill holes) ----
    skeleton = batch / "skeleton"
    conformance.codegen(program, skeleton, binary=binary)
    holes = conformance.holes_of(skeleton)
    summary["holes"] = [h["id"] for h in holes]
    try:
        resp = llm.complete(FILL_SYSTEM, conformance.fill_prompt(skeleton, spec))
        fills = conformance.parse_fill(resp.text)
        filled = batch / "filled"
        applied = conformance.fill_holes(skeleton, filled, fills)
        lint = conformance.lint_filled(skeleton, filled)
        traces = conformance.collect_traces(filled, native_runs=native_runs,
                                            miri_seeds=miri_seeds,
                                            calls_dir=batch / "skeleton_calls")
        agg = conformance.conform_all(program, traces, binary=binary)
        summary["arms"]["skeleton_fill"] = {
            "applied": applied, "lint": lint, "conformance": agg,
            "requests": 1,
        }
    except Exception as exc:  # noqa: BLE001
        summary["arms"]["skeleton_fill"] = {"error": f"{type(exc).__name__}: {exc}"}

    # ---- A3_free: LLM writes the whole annotated Rust ----
    free = batch / "free"
    free.mkdir(parents=True, exist_ok=True)
    try:
        cir_text = program.read_text(encoding="utf-8")
        resp = llm.complete(FREE_SYSTEM,
                            "ConcIR model:\n```json\n" + cir_text + "\n```\n\n"
                            "Output the annotated Rust program.")
        source = _extract_rust(resp.text)
        # reuse the generated runtime, replace main
        conformance.codegen(program, free, binary=binary)
        (free / "src/main.rs").write_text(source, encoding="utf-8")
        build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=free,
                               capture_output=True, text=True, timeout=300)
        if build.returncode != 0:
            summary["arms"]["A3_free"] = {"build_ok": False,
                                          "build_stderr": build.stderr[-300:]}
        else:
            traces = conformance.collect_traces(free, native_runs=native_runs,
                                                miri_seeds=miri_seeds,
                                                calls_dir=batch / "free_calls")
            agg = conformance.conform_all(program, traces, binary=binary)
            summary["arms"]["A3_free"] = {"build_ok": True, "conformance": agg,
                                          "requests": 1}
    except Exception as exc:  # noqa: BLE001
        summary["arms"]["A3_free"] = {"error": f"{type(exc).__name__}: {exc}"}

    summary["requests_used"] = budget.requests_used
    (batch / "FILL_SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return summary


def _extract_rust(text: str) -> str:
    if "```" in text:
        parts = text.split("```")
        candidate = max(parts[1::2], key=len) if parts[1::2] else text
        lines = candidate.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n"
