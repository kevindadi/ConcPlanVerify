"""Three-arm conditional repair for one verified generated defect.

A and B share ordinary build/run observations. B and C share the non-feedback
context, including the same CIR. Only C adds binding and conformance output.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .send_holding_requirements import evaluate_requirements

ARMS = ("A", "B", "C")


def ordinary_feedback(requirement: dict) -> str:
    runs = requirement.get("runs") or []
    lines = []
    if runs and runs[0].get("kind") == "candidate_build_failed":
        lines.append("cargo_build=failed")
        log = runs[0].get("log") or ""
        if log:
            lines.append("cargo_log=" + log.replace("\n", " ")[:500])
        return "\n".join(lines) + "\n"
    lines.append("cargo_build=ok")
    for index, run in enumerate(runs, start=1):
        stdout = run.get("stdout")
        shown = stdout.strip() if isinstance(stdout, str) else None
        lines.append(
            f"run{index}.kind={run.get('kind')} returncode={run.get('returncode')} "
            f"timed_out={bool(run.get('timed_out'))} stdout={shown!r}")
    return "\n".join(lines) + "\n"


def conformance_feedback(evaluation: dict) -> str:
    ledger = evaluation.get("ledger") or {}
    layers = ledger.get("layers") or {}
    trace = ledger.get("trace") or {}
    lines = [
        f"binding={(layers.get('binding') or (evaluation.get('stages') or {}).get('binding'))}",
        f"bounded_trace_deviation={trace.get('state')}",
    ]
    violation = (trace.get("violations") or [None])[0] or {}
    if trace.get("state") == "observed_violation" and violation:
        lines.append(
            f"conform.event_index={violation.get('event_index')} got={violation.get('got')} "
            f"resource={violation.get('resource')} checker_next={violation.get('expected')}")
    binding = evaluation.get("binding") or {}
    for item in binding.get("attributes") or []:
        if not isinstance(item, dict):
            continue
        if item.get("status") not in {"mismatch", "unknown"}:
            continue
        lines.append(
            f"attribute.resource={item.get('resource_id')} status={item.get('status')} "
            f"expected={item.get('expected')} observed={item.get('observed')} "
            f"site={item.get('construction_site')} evidence={item.get('evidence')} "
            f"reason={item.get('reason')}")
    for gap in binding.get("uncovered_sync") or []:
        if isinstance(gap, dict):
            lines.append(
                f"uncovered_sync.kind={gap.get('kind')} form={gap.get('form')} "
                f"site={gap.get('site')} function={gap.get('function')}")
    return "\n".join(lines) + "\n"


def render_prompt(*, arm: str, requirements: str, cir_text: str | None, previous: str,
                   feedback: str) -> str:
    parts = [
        "Repair the Rust program so the requirements are met.",
        "Return one complete Rust file inside a single ```rust fence.",
        "",
        "Requirements:",
        requirements.strip(),
        "",
    ]
    if arm in {"B", "C"}:
        parts += ["Verified CIR:", (cir_text or "").strip(), ""]
    parts += ["Previous Rust program:", previous.rstrip(), "", "Feedback:", feedback.rstrip()]
    return "\n".join(parts) + "\n"


def feedback_for(arm: str, requirement: dict, evaluation: dict | None) -> str:
    text = ordinary_feedback(requirement)
    if arm == "C" and evaluation is not None:
        text += conformance_feedback(evaluation)
    return text


SYSTEM = "Repair the Rust program so the requirements are met."


def _nonempty_without_state(path: Path) -> bool:
    if not path.exists():
        return False
    files = [item for item in path.rglob("*") if item.is_file()]
    if not files:
        return False
    return not (path / "state.json").is_file()


def plan_row(cell: dict, *, global_stop: str | None, model_stop: str | None,
             executed: dict | None) -> dict:
    """Every planned cell is a row, including ones that never send."""

    if executed is not None:
        row = {**cell, "status": "executed"}
        row.update(executed)
        return row
    if global_stop:
        return {**cell, "status": "not_started", "stop": global_stop}
    if model_stop:
        return {**cell, "status": "blocked", "stop": model_stop}
    return {**cell, "status": "not_started", "stop": "not_started"}


def run_repairs(case: dict, arm: str, client, out_dir: Path, *, max_repairs: int = 2,
                evaluate_candidate=None, score=None, audit_events=None,
                freeze_files: dict[str, str] | None = None) -> dict[str, Any]:
    """One cell on the shared repair state machine.

    Prompt and feedback differ by arm. Request identity, recovery, and budget
    handling are the ones already used by the two-arm runner.
    """

    from .audit import read_events
    from .feedback_runner import run_arm
    from .transport import build_registry, require_experiment_model

    out_dir = Path(out_dir)
    if _nonempty_without_state(out_dir):
        return {"arm": arm, "stop": "refuse_nonempty_without_state",
                "actual_model_requests_this_run": 0, "physical_attempts_this_run": 0,
                "first_requirement_round": None, "rounds": [], "send_status": "not_sent"}
    spec = getattr(client, "spec", None)
    if spec is None:
        spec = require_experiment_model(build_registry(), case.get("model_name") or "DeepSeek Flash")
    if audit_events is None:
        audit = getattr(client, "audit", None)
        if audit is not None and Path(getattr(audit, "path", "")).is_file():
            audit_events = read_events(audit.path)
    scorer = score or evaluate_requirements

    def adapted(case_obj, source, work):
        raw = scorer(source, work)
        status = raw["status"]
        mapped = "pass" if status == "bounded_covered_satisfied" else status
        return {"requirement": {"status": mapped, "raw_status": status},
                "design": {"status": "separate"},
                "requirement_error": status == "fail",
                "tool_error": False, "capability_gap": False,
                "support": {"supported": status != "unknown"}, "raw": raw}

    def builder(arm_name, requirement, evaluation):
        raw = requirement.get("raw") or requirement
        return feedback_for(arm_name, raw, evaluation if evaluate_candidate is not None else None)

    def toolchain(source, case_obj, work):
        if evaluate_candidate is None:
            return {"ledger": {}, "followup": {}, "binding": {}, "functional": {}}
        return evaluate_candidate(source, case_obj, work)

    cir_text = case.get("cir_text") or ""
    try:
        cir_doc = json.loads(cir_text) if cir_text else case.get("cir") or {}
    except json.JSONDecodeError:
        cir_doc = case.get("cir") or {}
    arm_case = {"id": case.get("id") or case.get("task") or "cell",
                "defect": case["defect"], "requirements": case.get("requirements") or "",
                "cir": cir_doc, "cir_text": cir_text,
                "cir_path": case.get("cir_path"), "contract_path": case.get("contract_path"),
                "tools": case.get("tools")}
    result = run_arm(
        arm_case, spec, arm, client, out_dir, max_repairs=max_repairs,
        evaluate=toolchain, score=adapted, audit_events=audit_events,
        prompt_renderer=render_prompt, feedback_builder=builder,
        stop_on_unknown=True, require_defect_signal=False,
        freeze_files=freeze_files, classify_responses=True, system_prompt=SYSTEM)
    if result.get("stop") == "requirement_pass":
        result["stop"] = "bounded_covered_satisfied"
    if result.get("stop") in {"global_request_budget_exhausted", "global_time_budget_exhausted"}:
        result["send_status"] = "not_sent" if not result.get("physical_attempts_this_run") else "sent"
    result["arm"] = arm
    (out_dir / "result.json").write_text(json.dumps({
        "arm": arm, "stop": result.get("stop"),
        "first_requirement_round": result.get("first_requirement_round"),
        "rounds": result.get("rounds"),
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return result


def arm_order(models: list[str], seed: int = 12) -> dict[str, list[str]]:
    import random
    rng = random.Random(seed)
    shift = rng.randrange(3)
    base = list(ARMS)
    orders = {}
    for index, model in enumerate(models):
        start = (shift + index) % 3
        orders[model] = base[start:] + base[:start]
    return orders
