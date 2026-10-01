#!/usr/bin/env python3
"""Re-score the fixed condvar control and the readiness-order negative. No model requests."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
import sys
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.condvar_requirements import evaluate_condvar  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
OUT = NOTES / "strong-link-v16" / "second-task"
HUMAN = NOTES / "strong-link-v15/human_derived_control.rs"
ORIGINAL = (NOTES / "strong-link-v5/reexec-gpt6luna/GPT 6 Luna"
            / "condvar__notify_one_multi_waiter_wrong_pick/rep0/source.rs")
CIR = (REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-gpt6luna/GPT 6 Luna"
       / "condvar__notify_one_multi_waiter_wrong_pick/rep0/cir/revision-1.cir.json")
CONTRACT = REPO / "benchmarks/families/condvar/notify_one_multi_waiter_wrong_pick/contract.json"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")


def _negative(source: str) -> str:
    return source.replace(
        """    g12.acquire_count(2).unwrap();
    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);""",
        """    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);
    g12.acquire_count(2).unwrap();""",
    )


def _slim_design(result: dict) -> dict:
    ledger = result.get("ledger") or {}
    trace = ledger.get("trace") or {}
    violations = trace.get("violations") or []
    return {
        "current_evaluation": ledger.get("current_evaluation"),
        "delivery_status": ledger.get("delivery_status"),
        "trace_state": trace.get("state"),
        "conform_statuses": (result.get("conform") or {}).get("statuses"),
        "first_violation_kind": (violations[0].get("kind") if violations else None),
        "first_violation_detail": (violations[0].get("detail") if violations else None),
        "attributes": [
            {"resource": item.get("resource_id"), "status": item.get("status"),
             "observed": item.get("observed"), "evidence": item.get("evidence"),
             "reason": item.get("reason")}
            for item in (result.get("binding") or {}).get("attributes") or []
            if isinstance(item, dict) and "sem" in str(item.get("expected"))
        ],
        "design_correspondence": ledger.get("design_correspondence"),
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    human = HUMAN.read_text(encoding="utf-8")
    negative = _negative(human)
    (OUT / "readiness_order_negative.rs").write_text(negative, encoding="utf-8")
    rows = {}
    for name, source, design in (
        ("human_control", human, True),
        ("readiness_order_negative", negative, False),
        ("original_gpt", ORIGINAL.read_text(encoding="utf-8"), False),
    ):
        work = OUT / name
        if work.exists():
            shutil.rmtree(work)
        requirement = evaluate_condvar(source, work / "requirement", repeats=1)
        record = {
            "requirement_status": requirement["status"],
            "checks": {key: value.get("status") for key, value in requirement["checks"].items()},
            "uncovered": requirement["uncovered"],
            "r6": requirement["checks"]["R6"],
            "runs": [
                {"kind": run.get("kind"), "returncode": run.get("returncode"),
                 "timed_out": run.get("timed_out"), "stdout": run.get("stdout")}
                for run in requirement.get("runs") or []
            ],
        }
        if design:
            evaluated = evaluate_candidate(
                source, CIR, CONTRACT, work / "design", binary=BIN, instrument=INS,
                n_runs=1, run_timeout=8, cell_id="v16-human-control")
            record["design"] = _slim_design(evaluated)
            shutil.rmtree(work / "design" / "proj" / "target", ignore_errors=True)
        shutil.rmtree(work / "requirement" / "target", ignore_errors=True)
        rows[name] = record
        print(name, record["requirement_status"], record.get("design", {}).get("trace_state"), flush=True)
    (OUT / "SECOND_TASK_EVAL.json").write_text(
        json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
