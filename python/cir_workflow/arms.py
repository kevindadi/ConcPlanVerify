"""Track G arm orchestration (offline scripted now; live gated).

This module is the only glue between the workflows and the benchmark manifest. It
runs one arm on one task and returns an :class:`ArmResult` with measured
consumption. It never fabricates a verdict: the terminal oracle is a separate
function (:func:`rust_oracle` / :func:`cir_oracle`).

Offline the provider is scripted; live the provider is the pinned DeepSeek Flash
client and the whole batch is gated by the frozen protocol hash. The live entry
point is intentionally unimplemented here until the user confirms the protocol;
``assert_protocol_confirmed`` fails closed instead of silently spending requests.
"""

from __future__ import annotations

import hashlib
import json
import re
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient, sha256_file
from .experiments_v2 import (
    ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER, ARM_OURS_REVISION, ARM_OURS_PATCH,
    ARM_TOOL_REPAIR, K, DIAGNOSTIC_TRUNCATION_BYTES, Consumption, RoundRecord,
    derived_flags, sha256_text,
)
from .models import normalize_token_usage
from .offline_workflow import _exclusive_run_dir
from .providers import CandidateProvider, CandidateRequest, ScriptedProvider
from .structural import run_structural_check

ARM_GEN_DIRECT = "G0_direct"
ARM_GEN_SELF = "G1_self_iter"
ARM_GEN_TOOLS = "G2_tools_iter"
GEN_ARMS = (ARM_GEN_DIRECT, ARM_GEN_SELF, ARM_GEN_TOOLS)

SELF_REPORT_TOKENS = ("no problems", "no_problems", "no concurrency")


def _direct(arm: str) -> bool:
    return arm in (ARM_DIRECT, ARM_GEN_DIRECT)


def _self_iter(arm: str) -> bool:
    return arm in (ARM_SELF_ITER, ARM_GEN_SELF)


def _tools(arm: str) -> bool:
    return arm.startswith("A2") or arm in (ARM_TOOLS_ITER, ARM_GEN_TOOLS)

# Word list for the prose branch of the three-way reply classifier. A reply with
# no code fence that matches any of these is a *claim of no defect*. Written to
# the batch PROTOCOL.md; keep in sync.
CLAIMS_NO_ISSUE_PATTERNS = (
    r"no issue", r"no defect", r"no bug", r"is correct", r"already correct",
    r"does not (have|contain) (a )?(bug|deadlock)",
    r"there is no (bug|deadlock|defect)", r"same order", r"no deadlock",
)


def _is_no_issues(text: str) -> bool:
    return "".join(ch for ch in text.strip().upper() if ch.isalpha()) == "NOISSUES"


def classify_reply(text: str) -> dict[str, Any]:
    """Three-way classification of an A0/A1 reply.

    ``program``: contains (exactly) one complete Rust program (fenced or raw).
    ``claims_no_issue``: the ``NO_ISSUES`` sentinel, or fence-free prose that
    matches the no-defect word list.
    ``other``: neither; one format retry is owed.
    """

    stripped = text.strip()
    if _is_no_issues(stripped):
        return {"kind": "claims_no_issue", "program": None, "matched": ["NO_ISSUES"]}
    program = _program_source(stripped)
    if program is not None and "fn main" in program:
        return {"kind": "program", "program": program, "matched": []}
    if "```" not in stripped:
        matched = [p for p in CLAIMS_NO_ISSUE_PATTERNS
                   if re.search(p, stripped, re.IGNORECASE)]
        if matched:
            return {"kind": "claims_no_issue", "program": None, "matched": matched}
    return {"kind": "other", "program": None, "matched": []}


def _program_source(text: str) -> str | None:
    """Return the Rust source from a fenced block or a raw reply, if any."""

    if "```" in text:
        candidates = text.split("```")[1::2]
        if not candidates:
            return None
        best = max(candidates, key=len)
        lines = best.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n" if text.strip() else None


def _write_reply(round_dir: Path, round_no: int, text: str, classification: dict,
                 decision: str | None) -> None:
    round_dir.mkdir(parents=True, exist_ok=True)
    (round_dir / "reply.json").write_text(json.dumps({
        "round": round_no, "kind": classification["kind"],
        "matched": classification.get("matched", []),
        "decision": decision, "raw_text": text,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


@dataclass
class ArmRun:
    arm: str
    task: str
    rounds: list[RoundRecord] = field(default_factory=list)
    accepted: bool = False
    accepted_round: int | None = None
    consumption: Consumption = field(default_factory=Consumption)
    final_artifact_path: str | None = None
    final_artifact_sha256: str | None = None
    notes: dict[str, Any] = field(default_factory=dict)
    error: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return {
            "arm": self.arm, "task": self.task,
            "rounds": [r.as_dict() for r in self.rounds],
            "accepted": self.accepted, "accepted_round": self.accepted_round,
            "consumption": self.consumption.as_dict(),
            "final_artifact_path": self.final_artifact_path,
            "final_artifact_sha256": self.final_artifact_sha256,
            "notes": self.notes, "error": self.error,
        }


def _truncate(text: str) -> str:
    data = text.encode("utf-8")
    if len(data) <= DIAGNOSTIC_TRUNCATION_BYTES:
        return text
    return data[:DIAGNOSTIC_TRUNCATION_BYTES].decode("utf-8", "ignore") + "\n[truncated]"


def extract_rust(text: str) -> str:
    """Return the Rust source from a model response (fenced block or raw)."""

    if "```" in text:
        parts = text.split("```")
        # choose the longest fenced segment
        candidates = parts[1::2]
        if candidates:
            best = max(candidates, key=len)
            lines = best.splitlines()
            if lines and lines[0].strip().lower() in ("rust", "rs"):
                lines = lines[1:]
            return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n"


def _tokens(response) -> tuple[int | None, int | None]:
    if response.usage:
        return normalize_token_usage(response.usage)
    if response.input_tokens is not None or response.output_tokens is not None:
        return response.input_tokens, response.output_tokens
    return None, None


def _consume_response(run: ArmRun, response) -> None:
    run.consumption.rounds += 1
    if response.source == "llm":
        run.consumption.http_requests += 1
    run.consumption.llm_wall_ms += int(getattr(response, "wall_ms", 0) or 0)
    usage = response.usage
    if usage is None and response.input_tokens is not None:
        usage = {"prompt_tokens": response.input_tokens,
                 "completion_tokens": response.output_tokens or 0}
    run.consumption.add_usage(usage)


def _is_no_issues(text: str) -> bool:
    return "".join(ch for ch in text.strip().upper() if ch.isalpha()) == "NOISSUES"


def run_rust_arm(provider: CandidateProvider, *, arm: str, task: str, spec: str,
                 contract: dict[str, Any], out_dir: Path | str, k: int = K,
                 tool_timeout_s: float = 60.0, initial_source: str | None = None,
                 tools_tier: str = "ml", run_miri_many_seeds: bool = True,
                 miri_seed_count: int | None = None) -> ArmRun:
    """A0/A1/A2: LLM writes whole Rust programs; feedback differs by arm.

    Replies are classified three ways (see :func:`classify_reply`):

    - ``program``: a complete Rust program; tool analysis runs as before.
    - ``claims_no_issue``: the sentinel or fence-free no-defect prose. For A0
      this accepts the *buggy input* (``decision=claims_no_issue``,
      ``bug_present=True`` by construction, ``false_accept=True``). For A1 it
      accepts the most recent ``build_ok=True`` candidate, or records
      ``claims_no_issue_unbuilt`` (not accepted) if none was ever built.
    - ``other``: one format retry (counted); if still ``other`` the round is a
      ``format_error``.
    """

    from .rust_arm import RustArmProject

    run = ArmRun(arm=arm, task=task)
    base = _exclusive_run_dir(Path(out_dir))
    feedback: str | None = None
    previous: str | None = initial_source
    initial_path: Path | None = None
    if initial_source is not None:
        initial_path = base / "initial.rs"
        initial_path.write_text(initial_source, encoding="utf-8")
    last_path: Path | None = None
    wants_tools = _tools(arm)
    if tools_tier == "ml" and arm.endswith("_m"):
        tools_tier = "m"
    run.notes["tools_tier"] = tools_tier

    last_build_ok: tuple[int, str] | None = None
    last_build_ok_path: Path | None = None
    accepted_path: Path | None = None

    for round_no in range(1, k + 1):
        response = None
        classification: dict[str, Any] | None = None
        for attempt in (1, 2):
            response = provider.propose(CandidateRequest(
                requirements=spec, contract=contract, feedback=feedback,
                attempt=round_no, previous_candidate=previous))
            _consume_response(run, response)
            if response.error:
                run.error = response.error
                break
            classification = classify_reply(response.text)
            if classification["kind"] != "other":
                break
            if attempt == 1:
                feedback = ("Your reply was neither a complete Rust program nor a "
                            "clear 'no issues' statement. Output the full program "
                            "including `fn main` inside one ```rust fence, or reply "
                            "exactly NO_ISSUES.")
        if response is None or response.error:
            break

        round_dir = base / f"round-{round_no}"
        round_dir.mkdir(parents=True, exist_ok=True)
        rr = RoundRecord(
            round=round_no,
            request_sha256=sha256_text(json.dumps({"spec_sha256": sha256_text(spec),
                                                   "feedback": feedback,
                                                   "attempt": round_no},
                                                  sort_keys=True)),
            feedback_sha256=sha256_text(feedback) if feedback else None,
            llm_wall_ms=int(getattr(response, "wall_ms", 0) or 0),
        )
        run.rounds.append(rr)
        prompt_tokens, completion_tokens = _tokens(response)
        rr.prompt_tokens, rr.completion_tokens = prompt_tokens, completion_tokens
        kind = classification["kind"]

        if kind == "claims_no_issue":
            run.notes.setdefault("claims_no_issue_rounds", []).append(round_no)
            if _direct(arm):
                if initial_path is None:
                    # Generation: there is no input program to accept; a bare
                    # no-issue claim is not a program.
                    rr.decision = "claims_no_issue_no_program"
                    run.accepted = False
                    _write_reply(round_dir, round_no, response.text,
                                 classification, rr.decision)
                    break
                rr.decision = "claims_no_issue"
                run.accepted = True
                run.accepted_round = round_no
                run.notes["accepted_artifact"] = "input"
                run.notes["bug_present_reason"] = "by_construction"
                accepted_path = initial_path
                _write_reply(round_dir, round_no, response.text, classification, rr.decision)
                break
            if _self_iter(arm):
                if last_build_ok is not None:
                    built_round = last_build_ok[0]
                    rr.decision = "claims_no_issue"
                    run.accepted = True
                    run.accepted_round = built_round
                    run.notes["accepted_candidate_round"] = built_round
                    accepted_path = last_build_ok_path
                else:
                    rr.decision = "claims_no_issue_unbuilt"
                    run.accepted = False
                    feedback = ("You reported no issues but no candidate has built "
                                "successfully yet; output the complete program.")
                    rr.feedback_sha256 = sha256_text(feedback)
                _write_reply(round_dir, round_no, response.text, classification, rr.decision)
                break
            # A2: a no-issue claim is not a tool-green result; keep iterating.
            rr.decision = "claims_no_issue"
            feedback = ("You reported no issues, but the tool chain did not pass. "
                        "Output the complete corrected program.")
            rr.feedback_sha256 = sha256_text(feedback)
            _write_reply(round_dir, round_no, response.text, classification, rr.decision)
            continue

        if kind == "other":
            rr.decision = "format_error"
            feedback = ("Your reply was not a complete Rust program. Output the full "
                        "program including `fn main` and every function, inside one "
                        "```rust fence, or reply exactly NO_ISSUES.")
            rr.feedback_sha256 = sha256_text(feedback)
            _write_reply(round_dir, round_no, response.text, classification, rr.decision)
            if _direct(arm):
                break
            continue

        # kind == "program"
        source = classification["program"]
        candidate_path = round_dir / "candidate.rs"
        candidate_path.write_text(source, encoding="utf-8")
        last_path = candidate_path
        previous = source

        project = RustArmProject(round_dir, source, name="probe")
        record = project.analyze(run_miri=wants_tools, run_lockbud=wants_tools,
                                 timeout_s=tool_timeout_s,
                                 run_miri_many_seeds=run_miri_many_seeds,
                                 miri_seed_count=miri_seed_count)
        tool_wall = _tool_wall(record)
        run.consumption.tool_wall_ms += tool_wall
        run.notes.setdefault("tool_rounds", []).append(record)
        build_ok = record.get("build_ok") is True
        test_ok = record.get("behavior_test_ok")
        miri_runs = list(record.get("miri", []))
        extended = record.get("miri_extended")
        if isinstance(extended, dict):
            miri_runs.extend(extended.get("runs", []) if "runs" in extended else [extended])
        miri_detected = any(r.get("extra", {}).get("detected") for r in miri_runs)
        miri_green = bool(miri_runs) and all(
            r.get("extra", {}).get("status") == "clean" for r in miri_runs)
        lockbud = record.get("lockbud") or {}
        lockbud_status = lockbud.get("status") or lockbud.get("extra", {}).get("status")
        lockbud_detected = bool(lockbud.get("extra", {}).get("detected"))
        lockbud_green = (lockbud_status in ("clean", "lockbud_unavailable", "skipped")
                         and not lockbud_detected)
        rr.tool_wall_ms = tool_wall
        rr.tool_output_sha256 = sha256_text(json.dumps(record, sort_keys=True))
        run.notes.setdefault("round_columns", []).append({
            "round": round_no, "build_ok": build_ok, "test_ok": test_ok,
            "miri_green": miri_green, "miri_detected": miri_detected,
            "lockbud_green": lockbud_green, "lockbud_detected": lockbud_detected,
            "lockbud_status": lockbud_status,
        })
        run.notes.setdefault("build_ok_rounds", [])
        if build_ok:
            last_build_ok = (round_no, source)
            last_build_ok_path = candidate_path
            run.notes["build_ok_rounds"].append(round_no)

        if _direct(arm):
            run.accepted = build_ok
            rr.decision = "build_ok" if build_ok else "build_fail"
            accepted_path = candidate_path if build_ok else None
            _write_reply(round_dir, round_no, response.text, classification, rr.decision)
            break
        if _self_iter(arm):
            rr.decision = "continue"
            feedback = ("Review the program for concurrency defects and either fix it "
                        "(full program) or reply exactly NO_ISSUES.")
            rr.feedback_sha256 = sha256_text(feedback)
            _write_reply(round_dir, round_no, response.text, classification, rr.decision)
            continue
        # A2-m (build + miri) vs A2-ml (build + miri + lockbud).
        accepted_m = bool(build_ok and miri_green and not miri_detected)
        accepted_ml = bool(accepted_m and lockbud_green)
        tier_ok = accepted_m if tools_tier == "m" else accepted_ml
        run.accepted = tier_ok
        rr.decision = ("tools_green_m" if tier_ok and tools_tier == "m"
                       else "tools_green_ml" if tier_ok else "tools_dirty")
        _write_reply(round_dir, round_no, response.text, classification, rr.decision)
        if run.accepted:
            accepted_path = candidate_path
            break
        feedback = _truncate(json.dumps({
            "build_ok": build_ok,
            "miri_detected": miri_detected,
            "miri_statuses": [r.get("extra", {}).get("status") for r in miri_runs],
            "lockbud_status": lockbud_status,
            "lockbud_detected": lockbud_detected,
            "build": record.get("build"),
            "behavior_test": record.get("behavior_test"),
        }, sort_keys=True))
        rr.feedback_sha256 = sha256_text(feedback)

    if run.accepted and run.accepted_round is None:
        run.accepted_round = run.rounds[-1].round if run.rounds else None
    if run.accepted:
        run.consumption.first_correct_round = run.accepted_round
    if accepted_path is not None and Path(accepted_path).is_file():
        run.final_artifact_path = str(accepted_path)
        run.final_artifact_sha256 = sha256_file(accepted_path)
    elif last_path is not None and last_path.is_file():
        run.final_artifact_path = str(last_path)
        run.final_artifact_sha256 = sha256_file(last_path)
    elif initial_source is not None:
        run.notes["initial_source_sha256"] = sha256_text(initial_source)
    # Invariant (I-3): any accepted A1 cell must have a successfully built candidate.
    if _self_iter(arm) and run.accepted and last_build_ok is None:
        raise AssertionError("A1 accepted without a build_ok candidate")
    return run


def _tool_wall(record: dict[str, Any]) -> int:
    total = 0
    build = record.get("build") or {}
    total += int(build.get("wall_ms") or 0)
    test = record.get("behavior_test") or {}
    total += int(test.get("wall_ms") or 0)
    for run in record.get("miri", []) or []:
        total += int(run.get("wall_ms") or 0)
    lockbud = record.get("lockbud")
    if isinstance(lockbud, dict):
        total += int(lockbud.get("wall_ms") or 0)
    return total


def run_cir_arm(client: ConcirClient, provider: CandidateProvider, *, arm: str,
                task: str, spec: str, contract: dict[str, Any], out_dir: Path | str,
                k: int = K, fidelity_name: str | None = None,
                diagnostics: bool = True, check_fidelity: bool = True,
                initial_program: Path | str | None = None) -> ArmRun:
    """A3 family: whole-artifact CIR revision (offline scripted or live)."""

    from .revision_workflow import WholeArtifactRevisionWorkflow

    workflow = WholeArtifactRevisionWorkflow(
        client, provider, out_dir=out_dir, max_rounds=k,
        diagnostics=diagnostics, check_fidelity=check_fidelity,
        fidelity_name=fidelity_name)
    result = workflow.run(spec, contract, task_id=task,
                          initial_program=initial_program)
    run = ArmRun(arm=arm, task=task)
    run.accepted = result.accepted
    run.accepted_round = result.accepted_version
    run.consumption = result.consumption
    run.error = result.error
    run.notes["revision_status"] = result.status
    run.notes["fidelity"] = [v.fidelity for v in result.versions if v.fidelity]
    for version in result.versions:
        run.rounds.append(RoundRecord(
            round=version.version,
            request_sha256=version.request_sha256,
            feedback_sha256=version.feedback_sha256,
            prompt_tokens=version.prompt_tokens,
            completion_tokens=version.completion_tokens,
            llm_wall_ms=version.llm_wall_ms,
            tool_wall_ms=version.tool_wall_ms,
            tool_output_sha256=version.tool_output_sha256,
            decision=version.decision,
        ))
    if result.versions:
        last = result.versions[-1]
        run.final_artifact_path = last.artifact_path
        if last.program_sha256:
            run.final_artifact_sha256 = last.program_sha256
    return run


def rust_oracle(source_path: Path | str, *, run_miri: bool = True,
                timeout_s: float = 60.0, run_miri_many_seeds: bool = True,
                miri_seed_count: int | None = None,
                expected_terminal: str | None = None) -> dict[str, Any]:
    """Terminal Rust verdict: build / behavior test / miri / bug rule.

    ``bug_present`` stays ``None`` unless a task-specific rule is available; it is
    never inferred from "no detector fired".
    """

    from .rust_arm import RustArmProject

    source = Path(source_path).read_text(encoding="utf-8")
    behavior = None
    with tempfile.TemporaryDirectory() as tmp:
        project = RustArmProject(tmp, source, name="oracle")
        record = project.analyze(run_miri=run_miri, run_lockbud=False,
                                 timeout_s=timeout_s,
                                 run_miri_many_seeds=run_miri_many_seeds,
                                 miri_seed_count=miri_seed_count)
        if record.get("build_ok"):
            behavior = project.behavior_run(timeout_s=10.0)
    miri_detected = any(r.get("extra", {}).get("detected") for r in record.get("miri", []))
    miri_statuses = [r.get("extra", {}).get("status") for r in record.get("miri", [])]
    if behavior is None:
        behavior_ok = None
        behavior_status = "no_build"
    elif behavior.timed_out:
        behavior_ok = False
        behavior_status = "hang"
    elif behavior.exit_code != 0:
        behavior_ok = False
        behavior_status = "crash"
    elif expected_terminal is not None and expected_terminal not in behavior.stdout:
        behavior_ok = False
        behavior_status = ("no_output" if not behavior.stdout.strip()
                           else "terminated_wrong_state")
    else:
        behavior_ok = True
        behavior_status = "terminated_ok"
    # F-3: only non-termination is a defect signal here; a terminated program
    # that does not print the (newly required) terminal line is recorded as its
    # status but not counted as a bug, and model extraction is reported apart.
    bug_present = behavior_status == "hang"
    return {
        "build_ok": record.get("build_ok"),
        "behavior_status": behavior_status,
        "behavior_ok": behavior_ok,
        "lockbud_detected": None,
        "miri_detected": miri_detected,
        "miri_statuses": miri_statuses,
        "bug_present": bug_present,
        "evidence": {"source_sha256": record.get("source_sha256")},
    }


def cir_oracle(client: ConcirClient, program: Path | str, contract: Path | str,
               engine: str = "petri", *, fidelity_name: str | None = None) -> dict[str, Any]:
    """Terminal CIR verdict: Rust full verification (+ optional fidelity)."""

    explore = client.explore(program, contract, engine)
    verify_pass = explore.outcome == "PASS" and explore.complete is True
    fidelity_ok = None
    if fidelity_name:
        program_json = json.loads(Path(program).read_text(encoding="utf-8"))
        fidelity = run_structural_check(fidelity_name, program_json)
        fidelity_ok = fidelity.get("ok")
    return {
        "verify_pass": verify_pass,
        "preserved_ok": verify_pass,
        "fidelity_ok": fidelity_ok,
        "bug_present": None if verify_pass is None else (not verify_pass),
        "evidence": {
            "outcome": explore.outcome, "complete": explore.complete,
            "states_explored": (explore.payload or {}).get("states_explored"),
            "wall_ms": explore.wall_ms,
        },
    }


def assert_protocol_confirmed(protocol_path: Path | str, expected_sha256: str) -> None:
    """Fail closed unless the frozen protocol hash matches (live gate)."""

    actual = hashlib.sha256(Path(protocol_path).read_bytes()).hexdigest()
    if actual != expected_sha256:
        raise RuntimeError(
            "live batch refused: protocol hash mismatch "
            f"({actual} != {expected_sha256}); confirm the frozen protocol first")


def run_offline_batch(manifest_path: Path | str, out_dir: Path | str, *,
                      cir_scripts: dict[str, Any] | None = None,
                      rust_scripts: dict[str, Any] | None = None) -> dict[str, Any]:
    """Exercise the harness end to end with scripted providers (no network)."""

    from .experiments_v2 import load_manifest

    root = Path(manifest_path).resolve().parent
    tasks = load_manifest(manifest_path)
    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    summary: dict[str, Any] = {"schema_version": "flash-arms-offline-v1", "tasks": []}
    cir_scripts = cir_scripts or {}
    rust_scripts = rust_scripts or {}
    for task in tasks:
        if task.status != "ready" or not (task.buggy_cir and task.contract):
            summary["tasks"].append({"id": task.id, "status": task.status, "runs": []})
            continue
        contract = json.loads((root / task.contract).read_text(encoding="utf-8"))
        spec = (root / task.spec).read_text(encoding="utf-8") if task.spec else task.id
        work = out / task.id
        runs = []
        if task.id in cir_scripts:
            provider = ScriptedProvider(cir_scripts[task.id])
            from .concir_client import ConcirClient as _C

            binary = _cir_binary()
            client = _C(binary, workdir=work / "calls", timeout=60.0)
            runs.append(run_cir_arm(client, provider, arm=ARM_OURS_REVISION, task=task.id,
                                    spec=spec, contract=contract, out_dir=work).as_dict())
        if task.id in rust_scripts:
            provider = ScriptedProvider(rust_scripts[task.id])
            runs.append(run_rust_arm(provider, arm=ARM_DIRECT, task=task.id, spec=spec,
                                     contract=contract, out_dir=work).as_dict())
        summary["tasks"].append({"id": task.id, "status": "ready", "runs": runs})
    (out / "ARMS_OFFLINE.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return summary


def _cir_binary() -> Path:
    import os

    env = os.environ.get("CONCIR_BACKEND")
    candidates = [Path(env)] if env else []
    repo = Path(__file__).resolve().parents[2]
    candidates.append(repo.parent / "ConcIR/target/release/concir-backend")
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError("concir-backend not found")
