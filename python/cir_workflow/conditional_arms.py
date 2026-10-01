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


def run_repairs(case: dict, arm: str, client, out_dir: Path, *, max_repairs: int = 2,
                evaluate_candidate=None, score=None) -> dict[str, Any]:
    """One cell. Round 0 is the original defect and does not call the model."""

    from .generation import _extract_rust_body
    out_dir.mkdir(parents=True, exist_ok=True)
    score = score or evaluate_requirements
    current = case["defect"]
    rounds = []
    prompts = []
    stop = None
    first_requirement = None
    for repair in range(0, max_repairs + 1):
        requirement = score(current, out_dir / f"round-{repair}")
        evaluation = None
        if evaluate_candidate is not None:
            evaluation = evaluate_candidate(current, case, out_dir / f"eval-{repair}")
        rounds.append({"round": repair, "requirement_status": requirement["status"],
                       "design_status": (evaluation or {}).get("ledger", {}).get("trace", {}).get("state"),
                       "delivery_status": (evaluation or {}).get("ledger", {}).get("delivery_status")})
        if requirement["status"] == "bounded_covered_satisfied" and first_requirement is None:
            first_requirement = repair
            stop = "bounded_covered_satisfied"
            break
        if requirement["status"] == "unknown":
            stop = "unknown"
            break
        if repair == max_repairs:
            stop = "cell_repair_budget_exhausted"
            break
        feedback = feedback_for(arm, requirement, evaluation)
        prompt = render_prompt(arm=arm, requirements=case["requirements"],
                               cir_text=case.get("cir_text"), previous=current, feedback=feedback)
        (out_dir / f"prompt-{repair + 1}.txt").write_text(prompt, encoding="utf-8")
        prompts.append(prompt)
        try:
            outcome = client.complete(
                "Repair the Rust program so the requirements are met.", prompt)
        except Exception as exc:  # noqa: BLE001
            from .live import BudgetExhausted
            from .transport import ModelIdentityError
            if isinstance(exc, BudgetExhausted):
                stop = "global_request_budget_exhausted" if "deadline" not in str(exc) else "global_time_budget_exhausted"
                return {"arm": arm, "stop": stop, "first_requirement_round": first_requirement,
                        "rounds": rounds, "prompts": prompts, "final_source": current,
                        "physical_attempts": getattr(exc, "physical_attempts", 0),
                        "send_status": "not_sent" if not getattr(exc, "physical_attempts", 0) else "sent"}
            if isinstance(exc, ModelIdentityError):
                return {"arm": arm, "stop": "identity_mismatch", "first_requirement_round": first_requirement,
                        "rounds": rounds, "prompts": prompts, "final_source": current,
                        "error": str(exc)}
            raise
        extracted = _extract_rust_body(getattr(outcome, "text", "") or "")
        if not extracted:
            current = current
            continue
        current = extracted
        (out_dir / f"source-{repair + 1}.rs").write_text(current, encoding="utf-8")
    (out_dir / "result.json").write_text(json.dumps({
        "arm": arm, "stop": stop, "first_requirement_round": first_requirement, "rounds": rounds,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return {"arm": arm, "stop": stop, "first_requirement_round": first_requirement,
            "rounds": rounds, "prompts": prompts, "final_source": current}


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
