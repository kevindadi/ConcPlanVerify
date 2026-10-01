#!/usr/bin/env python3
"""Score the fixed condvar controls and the three reproduced negatives. No model requests."""

from __future__ import annotations

import hashlib
import json
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.condvar_requirements import evaluate_condvar, structural_checks  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
OUT = NOTES / "strong-link-v17" / "second-task"
HUMAN = (NOTES / "strong-link-v15/human_derived_control.rs").read_text(encoding="utf-8")
ORIGINAL = (NOTES / "strong-link-v5/reexec-gpt6luna/GPT 6 Luna"
            / "condvar__notify_one_multi_waiter_wrong_pick/rep0/source.rs").read_text(encoding="utf-8")


def _variants() -> dict[str, str]:
    return {
        "human_control": HUMAN,
        "two_acquire_count_1": HUMAN.replace(
            "g12.acquire_count(2).unwrap();",
            "g12.acquire_count(1).unwrap();\n    g12.acquire_count(1).unwrap();"),
        "original_gpt": ORIGINAL,
        "raii_reuses_one_permit": HUMAN.replace(
            "g12.acquire_count(2).unwrap();",
            "let permit = g12.acquire();\n    permit.release();\n"
            "    let permit = g12.acquire();\n    permit.release();"),
        "notifier_self_supplies": HUMAN.replace(
            "g12.acquire_count(2).unwrap();",
            "g12.release_count(2).unwrap();\n    g12.acquire_count(2).unwrap();"),
        "unreachable_waits": HUMAN.replace("while !*proceed {", "while false {"),
        "renamed_guard": HUMAN.replace("proceed", "flag"),
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rows = {}
    for name, source in _variants().items():
        (OUT / f"{name}.rs").write_text(source, encoding="utf-8")
        work = OUT / name
        if work.exists():
            shutil.rmtree(work)
        structural = structural_checks(source)
        result = evaluate_condvar(source, work, repeats=1)
        shutil.rmtree(work / "target", ignore_errors=True)
        rows[name] = {
            "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
            "in_natural_defect_denominator": False,
            "structural": {key: value["status"] for key, value in structural.items()},
            "status": result["status"],
            "checks": {key: value["status"] for key, value in result["checks"].items()},
            "r6_reason": (result["checks"]["R6"].get("detail") or {}).get("reason"),
            "uncovered": result["uncovered"],
            "runs": [{k: run.get(k) for k in ("kind", "returncode", "timed_out", "stdout", "timeout_s")}
                     for run in result.get("runs") or []],
        }
        print(name, rows[name]["status"], rows[name]["checks"], flush=True)
    (OUT / "SECOND_TASK_EVAL.json").write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
