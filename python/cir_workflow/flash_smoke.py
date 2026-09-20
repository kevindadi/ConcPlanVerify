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
from .extract import (extraction_prompt, extraction_prompt_v4, parse_cir_only,
                      parse_extraction, schema_text, validate_extraction,
                      write_extraction_result)
from .instrument import instrument, labels_prompt_section
from .experiments_v2 import K, load_manifest
from .live import (
    ALLOWED_PROVIDER, DEFAULT_MAX_SECONDS, DeepSeekFlashClient, LiveBudget,
    RecordingLocalProvider, RecordingPatchProvider, RecordingProvider,
    assert_allowed_model,
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
    return _read("rust_to_cir_extract_v3.md")


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
        return write_extraction_result(
            work_dir, {"stage": "parse", "reason": f"{type(exc).__name__}: {exc}"})
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
        return write_extraction_result(
            work_dir, {"stage": "parse",
                       "reason": "reply not a {cir,rust} object"})
    try:
        return validate_extraction(parsed["cir"], str(parsed["rust"]), contract_path,
                                   work_dir, binary=binary)
    except Exception as exc:  # noqa: BLE001
        return write_extraction_result(
            work_dir, {"stage": "harness", "where": "validate_extraction",
                       "reason": f"{type(exc).__name__}: {exc}", "harness_error": True})


def _extract_labels_and_validate(api_key: str, budget: LiveBudget, batch: Path,
                                 system: str, rust_source: str, contract_path: Path,
                                 work_dir: Path, binary: Path | str,
                                 sdk_client: Any, timeout: float,
                                 max_tokens: int) -> dict[str, Any]:
    """v4 extraction: the tool instruments the Rust, the model writes only CIR."""

    llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                              evidence_dir=batch / "llm", timeout=timeout,
                              max_tokens=max_tokens, sdk_client=sdk_client)
    work_dir = Path(work_dir)
    try:
        inst = instrument(rust_source, work_dir / "instrument", binary=None)
    except Exception as exc:  # noqa: BLE001
        return write_extraction_result(
            work_dir, {"stage": "harness", "where": "instrument",
                       "reason": f"{type(exc).__name__}: {exc}", "harness_error": True})
    labels_md = labels_prompt_section(inst["labels"])
    try:
        outcome = llm.complete(
            system, extraction_prompt_v4(schema_text(binary), rust_source, labels_md))
    except Exception as exc:  # noqa: BLE001
        return write_extraction_result(
            work_dir, {"stage": "parse", "reason": f"{type(exc).__name__}: {exc}"})
    parsed = parse_cir_only(outcome.text)
    if not parsed:
        return write_extraction_result(
            work_dir, {"stage": "parse", "reason": "reply was not a CIR object"})
    try:
        return validate_extraction(parsed["cir"], inst["annotated"], contract_path,
                                   work_dir, binary=binary, lenient_unlock=True,
                                   attempt_events=True)
    except Exception as exc:  # noqa: BLE001
        return write_extraction_result(
            work_dir, {"stage": "harness", "where": "validate_extraction",
                       "reason": f"{type(exc).__name__}: {exc}", "harness_error": True})


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

# Observable terminal line per task (kept in sync with benchmarks/build_families).
TASK_TERMINAL = {
    "lock-order/abba_2lock": "DONE t1=1 t2=1",
    "lock-order/partial_deadlock_bystander": "DONE a=1 b=1",
    "condvar/bare_wait_no_predicate": "DONE ready=true",
    "condvar/lost_wakeup_notify_before_wait": "DONE ready=true",
    "semaphore/permit_leak": "DONE permits=1",
    "condvar/notify_one_multi_waiter_wrong_pick": "DONE waiters=0",
}


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
                            run_miri_many_seeds=False, miri_seed_count=16,
                            expected_terminal=TASK_TERMINAL.get(task_id))
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
                model_str = "unverified"
            else:
                model_str = "inconclusive"
            statuses = oracle.get("miri_statuses") or []
            miri_str = _status_counts(statuses) if statuses else str(oracle.get("miri_detected"))
            lines.append(
                f"| {task['task']} | {arm} | {run.get('accepted')} | "
                f"{run.get('accepted_round')} | {oracle.get('build_ok')} | "
                f"{oracle.get('behavior_status')} | {miri_str} | "
                f"{model_str} | {fa} |")
    # footnotes for long model reasons
    notes = []
    for task in summary["tasks"]:
        for arm, run in (task.get("arms") or {}).items():
            model = (run.get("oracle") or {}).get("model") or {}
            if model and model.get("extract_validated") is not True:
                notes.append(f"- {task['task']} / {arm}: {model.get('reason')}")
    if notes:
        lines += ["", "## oracle.model reasons", ""] + notes
    lines += ["", "`behavior_status`: terminated_ok / terminated_wrong_state / hang / "
                  "no_output / no_build. `oracle.model` is not truncated (see reasons "
                  "section above)."]
    return "\n".join(lines) + "\n"


def _status_counts(statuses: list[Any]) -> str:
    from collections import Counter

    return ", ".join(f"{k} {v}" for k, v in sorted(Counter(statuses).items()))


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
    "model. Emit `cir_trace::ev(\"<tag>\", \"<sid>\")` exactly as codegen does: "
    "after `mutex_lock`/`semaphore_acquire`/`condvar_wait`/`channel_send`/"
    "`channel_recv` return, and at `mutex_unlock`/`condvar_notify`/"
    "`condvar_notify_all`/`semaphore_release`/`spawn`/`scope`/`join`; use tag `t0` "
    "for main and `t<sid>_<i>` for the i-th member of a `scope`, with the sid from "
    "the CIR. You MUST call `cir_trace::finish()` once at the end of `main` (the "
    "runtime is already provided as `mod cir_trace`). Define `fn main`, use no "
    "external crate, and output only the Rust source in one ```rust fence."
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
        # reuse the generated runtime, replace main; drop any inline
        # `mod cir_trace { ... }` the model defined so the injected runtime
        # (which honours CIR_TRACE_OUT) is used.
        source = _strip_inline_cir_trace(source)
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


def _strip_inline_cir_trace(source: str) -> str:
    """Remove a top-level ``mod cir_trace { ... }`` block (balanced braces)."""

    marker = "mod cir_trace {"
    idx = source.find(marker)
    if idx == -1:
        return source
    depth = 0
    end = idx
    for i in range(idx, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    stripped = (source[:idx] + source[end:]).lstrip("\n")
    if "cir_trace::" in stripped and "mod cir_trace" not in stripped:
        stripped = "mod cir_trace;\n" + stripped
    return stripped


def _extract_rust(text: str) -> str:
    if "```" in text:
        parts = text.split("```")
        candidate = max(parts[1::2], key=len) if parts[1::2] else text
        lines = candidate.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n"


SMOKE_V3_TASKS = (
    "lock-order/partial_deadlock_bystander",
    "lock-order/cross_module_cycle",
    "lock-order/cycle_3lock",
    "structure/nested_scope_lock_order",
    "condvar/notify_one_multi_waiter_wrong_pick",
    "channel/bounded_backpressure_lock_held",
    "channel/send_while_holding_mutex",
    "semaphore/acquire_twice_no_release",
)
SMOKE_V3_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml",
                 "A3_local", "A3_whole")
ARC = {"A0_direct": "direct", "A1_self_iter": "self", "A2_tools_iter_ml": "tools"}


def run_repair_smoke_v3(manifest_path: Path | str, out_dir: Path | str, *,
                        binary: Path | str, api_key: str, protocol_path: Path | str,
                        protocol_sha256: str, k: int = K, timeout: float = 90.0,
                        max_tokens: int = 4096, max_requests: int = 200,
                        max_seconds: float = 10800.0, sdk_client: Any | None = None,
                        tasks_filter: tuple[str, ...] | None = None) -> dict[str, Any]:
    from .arms import cir_oracle, rust_oracle
    from .revision_workflow import WholeArtifactRevisionWorkflow

    assert_protocol_confirmed(protocol_path, protocol_sha256)
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=max_requests,
                        max_seconds=max_seconds)
    tasks = {t.id: t for t in load_manifest(manifest_path)}
    wanted = tasks_filter or SMOKE_V3_TASKS
    summary: dict[str, Any] = {
        "batch_dir": str(batch), "protocol_sha256": protocol_sha256,
        "provider": ALLOWED_PROVIDER, "requested_model": "deepseek-flash",
        "binary_sha256": __import__("hashlib").sha256(Path(binary).read_bytes()).hexdigest(),
        "k": k, "max_requests": max_requests, "tasks": [], "stop_reason": None,
    }
    stop_reason = None
    for task_id in wanted:
        task = tasks.get(task_id)
        if task is None:
            summary["tasks"].append({"task": task_id, "skipped": "not_in_manifest"})
            continue
        if budget.exhausted():
            stop_reason = budget.exhausted()
            break
        task_dir = batch / task_id.replace("/", "__")
        rt = json.loads((root / task.directory / "repair_task.json").read_text(encoding="utf-8"))
        contract_path = root / rt["contract"]
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        spec = (root / rt["requirements_file"]).read_text(encoding="utf-8")
        buggy_cir = root / rt["input_cir"]
        input_rust = ((root / rt["input_rust"]).read_text(encoding="utf-8")
                      if rt.get("input_rust") else None)
        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        record: dict[str, Any] = {"task": task_id, "contract_sha256": __import__("hashlib")
                                  .sha256(contract_path.read_bytes()).hexdigest(), "arms": {}}
        try:
            if input_rust is not None:
                for arm in ("A0_direct", "A1_self_iter", "A2_tools_iter_ml"):
                    if budget.exhausted():
                        break
                    llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                              evidence_dir=batch / "llm", timeout=timeout,
                                              max_tokens=max_tokens, sdk_client=sdk_client)
                    provider = RustLiveProvider(llm, ARC[arm])
                    run = run_rust_arm(provider, arm=arm, task=task_id, spec=spec,
                                       contract=contract, out_dir=task_dir / arm, k=k,
                                       initial_source=input_rust, tool_timeout_s=8.0,
                                       run_miri_many_seeds=False)
                    arec = run.as_dict()
                    arec["llm_calls"] = provider.calls
                    arec["oracle"] = (rust_oracle(
                        run.final_artifact_path, run_miri=True, miri_seed_count=16,
                        run_miri_many_seeds=False,
                        expected_terminal=TASK_TERMINAL.get(task_id))
                        if run.final_artifact_path else {})
                    record["arms"][arm] = arec
            for arm, fmt in (("A3_local", "local"), ("A3_whole", "whole")):
                if budget.exhausted():
                    break
                llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                          evidence_dir=batch / "llm", timeout=timeout,
                                          max_tokens=max_tokens, sdk_client=sdk_client)
                if fmt == "local":
                    provider = RecordingLocalProvider(llm, buggy_cir.read_text(encoding="utf-8"))
                else:
                    provider = RecordingProvider(llm)
                workflow = WholeArtifactRevisionWorkflow(
                    client, provider, out_dir=task_dir / arm, max_rounds=k,
                    reply_format=fmt)
                res = workflow.run(spec, contract, task_id=task_id,
                                   initial_program=buggy_cir)
                arec = {"accepted": res.accepted, "accepted_round": res.accepted_version,
                        "status": res.status, "decision_distribution": _decision_dist(res),
                        "consumption": res.consumption.as_dict(),
                        "llm_calls": provider.calls}
                if res.versions and res.versions[-1].artifact_path:
                    arec["final_cir"] = res.versions[-1].artifact_path
                    arec["oracle"] = cir_oracle(client, res.versions[-1].artifact_path,
                                                contract_path)
                record["arms"][arm] = arec
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
    (batch / "SUMMARY.md").write_text(render_smoke_v3_md(summary), encoding="utf-8")
    return summary


MAIN_V1_ARMS = SMOKE_V3_ARMS
# Priority order within reps>0 when the budget is tight (PROTOCOL §3.2).
MAIN_V1_ARM_ORDER = ("A0_direct", "A2_tools_iter_ml", "A3_local",
                     "A1_self_iter", "A3_whole")


def _run_task_arms(task_id: str, root: Path, task, contract_path: Path,
                   contract: dict[str, Any], spec: str, buggy_cir: Path,
                   input_rust: str | None, arm_dir: Path, llm_dir: Path,
                   binary: Path | str, api_key: str, budget: LiveBudget, k: int,
                   timeout: float, max_tokens: int, sdk_client: Any,
                   arm_order: tuple[str, ...]) -> dict[str, Any]:
    from .arms import cir_oracle, rust_oracle

    client = ConcirClient(binary, workdir=arm_dir / "calls", timeout=30.0)
    record: dict[str, Any] = {
        "task": task_id,
        "contract_sha256": __import__("hashlib").sha256(contract_path.read_bytes()).hexdigest(),
        "arms": {},
    }
    for arm in arm_order:
        if budget.exhausted():
            record["arms"][arm] = {"task": task_id, "arm": arm,
                                   "not_run": budget.exhausted()}
            continue
        if arm in ("A0_direct", "A1_self_iter", "A2_tools_iter_ml"):
            if input_rust is None:
                record["arms"][arm] = {"task": task_id, "arm": arm,
                                       "skipped": "no_rust_reference"}
                continue
            llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                      evidence_dir=llm_dir, timeout=timeout,
                                      max_tokens=max_tokens, sdk_client=sdk_client)
            provider = RustLiveProvider(llm, ARC[arm])
            run = run_rust_arm(provider, arm=arm, task=task_id, spec=spec,
                               contract=contract, out_dir=arm_dir / arm, k=k,
                               initial_source=input_rust, tool_timeout_s=8.0,
                               run_miri_many_seeds=False)
            arec = run.as_dict()
            arec["llm_calls"] = provider.calls
            arec["oracle"] = (rust_oracle(
                run.final_artifact_path, run_miri=True, miri_seed_count=16,
                run_miri_many_seeds=False,
                expected_terminal=TASK_TERMINAL.get(task_id))
                if run.final_artifact_path else {})
            record["arms"][arm] = arec
        else:
            fmt = "local" if arm == "A3_local" else "whole"
            llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                      evidence_dir=llm_dir, timeout=timeout,
                                      max_tokens=max_tokens, sdk_client=sdk_client)
            if fmt == "local":
                provider = RecordingLocalProvider(llm, buggy_cir.read_text(encoding="utf-8"))
            else:
                provider = RecordingProvider(llm)
            workflow = WholeArtifactRevisionWorkflow(
                client, provider, out_dir=arm_dir / arm, max_rounds=k, reply_format=fmt)
            res = workflow.run(spec, contract, task_id=task_id,
                               initial_program=buggy_cir)
            arec = {"accepted": res.accepted, "accepted_round": res.accepted_version,
                    "status": res.status, "decision_distribution": _decision_dist(res),
                    "consumption": res.consumption.as_dict(),
                    "llm_calls": provider.calls}
            if res.versions and res.versions[-1].artifact_path:
                arec["final_cir"] = res.versions[-1].artifact_path
                arec["oracle"] = cir_oracle(client, res.versions[-1].artifact_path,
                                            contract_path)
            record["arms"][arm] = arec
    return record


def run_repair_main_v1(manifest_path: Path | str, out_dir: Path | str, *,
                       binary: Path | str, api_key: str, protocol_path: Path | str,
                       protocol_sha256: str, reps: int = 3, k: int = K,
                       timeout: float = 90.0, max_tokens: int = 4096,
                       max_requests: int = 300, max_seconds: float = 10800.0,
                       sdk_client: Any | None = None,
                       tasks_filter: tuple[str, ...] | None = None) -> dict[str, Any]:
    """Main batch: 8 tasks x 5 arms x `reps`, one shared budget, rep-outer order."""

    assert_protocol_confirmed(protocol_path, protocol_sha256)
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=max_requests,
                        max_seconds=max_seconds)
    tasks = {t.id: t for t in load_manifest(manifest_path)}
    wanted = tasks_filter or SMOKE_V3_TASKS
    summary: dict[str, Any] = {
        "batch_dir": str(batch), "protocol_sha256": protocol_sha256,
        "provider": ALLOWED_PROVIDER, "requested_model": "deepseek-flash",
        "binary_sha256": __import__("hashlib").sha256(Path(binary).read_bytes()).hexdigest(),
        "k": k, "max_requests": max_requests, "reps": [], "stop_reason": None,
    }
    stop_reason = None
    for rep in range(reps):
        rep_dir = batch / f"rep-{rep}"
        rep_summary: dict[str, Any] = {"rep": rep, "tasks": []}
        arm_order = MAIN_V1_ARMS if rep == 0 else MAIN_V1_ARM_ORDER
        for task_id in wanted:
            if budget.exhausted():
                stop_reason = budget.exhausted()
                break
            task = tasks.get(task_id)
            if task is None:
                rep_summary["tasks"].append({"task": task_id, "skipped": "not_in_manifest"})
                continue
            rt = json.loads((root / task.directory / "repair_task.json").read_text(
                encoding="utf-8"))
            contract_path = root / rt["contract"]
            contract = json.loads(contract_path.read_text(encoding="utf-8"))
            spec = (root / rt["requirements_file"]).read_text(encoding="utf-8")
            buggy_cir = root / rt["input_cir"]
            input_rust = ((root / rt["input_rust"]).read_text(encoding="utf-8")
                          if rt.get("input_rust") else None)
            try:
                record = _run_task_arms(
                    task_id, root, task, contract_path, contract, spec, buggy_cir,
                    input_rust, rep_dir / task_id.replace("/", "__"), rep_dir / "llm",
                    binary, api_key, budget, k, timeout, max_tokens, sdk_client,
                    arm_order)
                record["rep"] = rep
            except Exception as exc:  # noqa: BLE001
                stop_reason = f"{type(exc).__name__}: {exc}"
                record = {"task": task_id, "rep": rep, "error": stop_reason, "arms": {}}
            rep_summary["tasks"].append(record)
            if stop_reason:
                break
        summary["reps"].append(rep_summary)
        if stop_reason:
            break
    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    summary["stop_reason"] = stop_reason
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (batch / "SUMMARY.md").write_text(render_main_v1_md(summary), encoding="utf-8")
    return summary


def render_main_v1_md(summary: dict[str, Any]) -> str:
    lines = ["# flash-repair-main-v1 — SUMMARY", "",
             f"- batch: `{summary['batch_dir']}`",
             f"- protocol sha256: `{summary['protocol_sha256']}`",
             f"- binary sha256: `{summary.get('binary_sha256')}`",
             f"- requests used: {summary.get('requests_used')} / {summary.get('max_requests')}",
             f"- stop reason: {summary.get('stop_reason')}", ""]
    for rep in summary.get("reps", []):
        lines += [f"## rep-{rep['rep']}", "",
                  "| task | arm | accepted | round | tokens | behavior | false_accept |",
                  "| --- | --- | --- | --- | --- | --- | --- |"]
        for t in rep["tasks"]:
            for arm in MAIN_V1_ARMS:
                r = (t.get("arms") or {}).get(arm)
                if not r:
                    continue
                o = r.get("oracle") or {}
                c = r.get("consumption") or {}
                fa = bool(r.get("accepted") and _bug_sources(r))
                lines.append(f"| {t['task']} | {arm} | {r.get('accepted')} | "
                             f"{r.get('accepted_round')} | {c.get('total_tokens')} | "
                             f"{o.get('behavior_status')} | {fa} |")
        lines.append("")
    lines += ["## Merged (k/n over reps)", "",
              "| task | arm | accepted | round | tokens | false_accept |",
              "| --- | --- | --- | --- | --- | --- |"]
    buckets: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for rep in summary.get("reps", []):
        for t in rep["tasks"]:
            for arm, rec in (t.get("arms") or {}).items():
                buckets.setdefault((t["task"], arm), []).append(rec)
    for (task, arm), recs in buckets.items():
        acc = sum(1 for r in recs if r.get("accepted"))
        fa = sum(1 for r in recs if r.get("accepted") and _bug_sources(r))
        rounds = [r.get("accepted_round") for r in recs if r.get("accepted_round")]
        toks = [(r.get("consumption") or {}).get("total_tokens") or 0 for r in recs]
        lines.append(f"| {task} | {arm} | {acc}/{len(recs)} | "
                     f"{(sum(rounds) / len(rounds)) if rounds else 0:.2f} | "
                     f"{(sum(toks) / len(toks)) if toks else 0:.0f} | {fa}/{len(recs)} |")
    lines += ["", "`false_accept` = accepted AND bug_present, where bug_present follows "
                  "the §2.1 rule (behavior=hang, model FAIL, expert=yes, or "
                  "by_construction). `not_run` cells appear in the per-rep tables' JSON."]
    return "\n".join(lines) + "\n"


def _bug_sources(rec: dict[str, Any]) -> list[str]:
    """Sources of a `bug_present=True` verdict for one arm cell (§2.1 rule)."""

    oracle = rec.get("oracle") or {}
    sources = []
    if oracle.get("behavior_status") == "hang":
        sources.append("behavior")
    if oracle.get("verify_pass") is False:
        sources.append("model")
    if oracle.get("extract") == "FAIL":
        sources.append("extract")
    if oracle.get("expert") == "yes":
        sources.append("expert")
    if (rec.get("notes") or {}).get("bug_present_reason") == "by_construction":
        sources.append("by_construction")
    return sources


def _decision_dist(res) -> dict[str, int]:
    from collections import Counter

    # The decisions live in the per-run result.json; recompute from the workflow
    # result object is not available here, so readers use the run dir. Placeholder
    # is replaced by _decision_dist_from_runs at render time.
    return {}


def _decision_dist_from_runs(summary: dict[str, Any]) -> dict[str, dict[str, int]]:
    from collections import Counter

    out: dict[str, dict[str, int]] = {}
    for t in summary["tasks"]:
        for arm in ("A3_local", "A3_whole"):
            rec = (t.get("arms") or {}).get(arm) or {}
            path = rec.get("final_cir")
            if not path:
                continue
            result_path = Path(path).parent / "result.json"
            if not result_path.is_file():
                continue
            data = json.loads(result_path.read_text(encoding="utf-8"))
            counter = Counter(v.get("decision") for v in data.get("versions", []))
            out[f"{t['task']}|{arm}"] = dict(counter)
    return out


def render_smoke_v3_md(summary: dict[str, Any]) -> str:
    lines = ["# flash-repair-smoke-v3 — SUMMARY", "",
             f"- batch: `{summary['batch_dir']}`",
             f"- protocol sha256: `{summary['protocol_sha256']}`",
             f"- binary sha256: `{summary.get('binary_sha256')}`",
             f"- requests used: {summary.get('requests_used')} / {summary.get('max_requests')}",
             f"- stop reason: {summary.get('stop_reason')}", "",
             "## Main table", "",
             "| task | arm | accepted | round | tokens | llm_ms | tool_ms | oracle.build | "
             "oracle.behavior | oracle.miri | oracle.model | oracle.expert | oracle.extract | "
             "false_accept |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for t in summary["tasks"]:
        for arm in ("A0_direct", "A1_self_iter", "A2_tools_iter_ml", "A3_local", "A3_whole"):
            r = (t.get("arms") or {}).get(arm)
            if not r:
                continue
            o = r.get("oracle") or {}
            c = r.get("consumption") or {}
            if arm in ("A3_local", "A3_whole"):
                mv = o.get("verify_pass")
                fa = bool(r.get("accepted") and mv is False)
                lines.append(f"| {t['task']} | {arm} | {r.get('accepted')} | "
                             f"{r.get('accepted_round')} | {c.get('total_tokens')} | "
                             f"{c.get('llm_wall_ms')} | {c.get('tool_wall_ms')} | — | — | — | "
                             f"{'PASS' if mv else ('FAIL' if mv is False else 'n/a')} | {fa} |")
            else:
                statuses = o.get("miri_statuses") or []
                miri = _status_counts(statuses) if statuses else str(o.get("miri_detected"))
                model = o.get("model") or {}
                ms = (f"validated:{model.get('model_verdict')}"
                      if model.get("extract_validated") else "inconclusive")
                fa = bool(r.get("accepted") and _bug_sources(r))
                lines.append(f"| {t['task']} | {arm} | {r.get('accepted')} | "
                             f"{r.get('accepted_round')} | {c.get('total_tokens')} | "
                             f"{c.get('llm_wall_ms')} | {c.get('tool_wall_ms')} | "
                             f"{o.get('build_ok')} | {o.get('behavior_status')} | {miri} | "
                             f"{ms} | {o.get('expert', '—')} | {o.get('extract', '—')} | {fa} |")
    # per-arm aggregates
    lines += ["", "## Per-arm aggregates", "",
              "| arm | cells | accepted | accept_rate | false_accept | by source | "
              "inconclusive | mean_round | mean_tokens |",
              "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in SMOKE_V3_ARMS:
        cells = [ (t.get("arms") or {}).get(arm) for t in summary["tasks"] ]
        cells = [c for c in cells if c]
        if not cells:
            continue
        accepted = sum(1 for c in cells if c.get("accepted"))
        fa_cells = [c for c in cells if c.get("accepted") and _bug_sources(c)]
        by_source: dict[str, int] = {}
        for c in fa_cells:
            for source in set(_bug_sources(c)):
                by_source[source] = by_source.get(source, 0) + 1
        inc = sum(1 for c in cells if (c.get("oracle") or {}).get("model") is None
                  and arm not in ("A3_local", "A3_whole"))
        rounds = [c.get("accepted_round") for c in cells if c.get("accepted_round")]
        toks = [ (c.get("consumption") or {}).get("total_tokens") or 0 for c in cells ]
        lines.append(f"| {arm} | {len(cells)} | {accepted} | {accepted/len(cells):.2f} | "
                     f"{len(fa_cells)} | {by_source or '—'} | "
                     f"{inc} | {(sum(rounds)/len(rounds)) if rounds else 0:.2f} | "
                     f"{(sum(toks)/len(toks)) if toks else 0:.0f} |")
    lines += ["", "## A3 decision distribution", "",
              "| task | arm | decisions |", "| --- | --- | --- |"]
    for key, dist in sorted(_decision_dist_from_runs(summary).items()):
        task, arm = key.split("|")
        lines.append(f"| {task} | {arm} | {dist} |")
    lines += ["", "`oracle.behavior`: terminated_ok / terminated_wrong_state / hang / "
                  "no_output / no_build. `oracle.model` for Rust arms is `inconclusive` "
                  "in this batch (deviation D-19: the batch is not gated on Rust-arm "
                  "oracle completeness; extraction/expert labels are filled in later)."]
    return "\n".join(lines) + "\n"
