#!/usr/bin/env python3
"""Offline rescore of the frozen v19 pilot. No model requests.

Reads notes/strong-link-v19/live and writes a derived jsonl under
notes/strong-link-v20. It does not modify the pilot directory.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import _extract_rust_body  # noqa: E402
from cir_workflow.task_score import score_for_task  # noqa: E402

SRC = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v19/live")
OUT = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v20")
WORK = Path("/tmp/cpv-v20-reeval")


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    rows = json.loads((SRC / "RESULTS.json").read_text(encoding="utf-8"))
    WORK.mkdir(exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    scorer_dir = REPO / "python/cir_workflow"
    scorer_files = (
        "condvar_requirements.py",
        "send_holding_requirements.py",
        "task_score.py",
    )
    lines = []
    for row in rows:
        dest = SRC / row["task"].replace("/", "__") / row["model_id"] / row["arm"]
        state = json.loads((dest / "state.json").read_text(encoding="utf-8"))
        result = json.loads((dest / "result.json").read_text(encoding="utf-8")) if (dest / "result.json").is_file() else {}
        rounds = []
        for req in state.get("requests") or []:
            text = req.get("response") or ""
            extracted = _extract_rust_body(text) if text else ""
            new = None
            if extracted and extracted.strip():
                work = WORK / f"{row['model_id']}-{row['arm']}-{row['task'].replace('/', '__')}-r{req.get('round')}"
                new = score_for_task(row["task"], extracted, work)
            original = next((item for item in (result.get("rounds") or []) if item.get("round") == req.get("round")), {})
            checks = {} if new is None else {
                key: value.get("status") for key, value in (new.get("checks") or {}).items()
            }
            rounds.append({
                "round": req.get("round"),
                "original_requirement_status": original.get("requirement_status"),
                "original_design_status": original.get("design_status"),
                "new_requirement_status": None if new is None else new.get("status"),
                "new_checks": checks,
                "new_nonpass": {key: value for key, value in checks.items() if value != "pass"},
                "empty_candidate": not bool((extracted or "").strip()),
                "finish_reason": (req.get("identity") or {}).get("finish_reason"),
                "response_chars": len(text),
                "status_changed": original.get("requirement_status") != (None if new is None else new.get("status")),
            })
        record = {
            "cell_key": row.get("cell_key"),
            "arm": row.get("arm"),
            "model_id": row.get("model_id"),
            "task": row.get("task"),
            "original_stop": row.get("stop"),
            "original_first_requirement_round": row.get("first_requirement_round"),
            "original_first_design_round": row.get("first_design_round"),
            "original_first_both_round": row.get("first_both_round"),
            "rounds": rounds,
            "real_requests": 0,
        }
        lines.append(record)
        print(row["task"], row["model_id"], row["arm"],
              [item["new_requirement_status"] for item in rounds],
              "was", row.get("stop"), flush=True)
    meta = {
        "real_requests": 0,
        "cells": len(lines),
        "scorer_sha256": {name: _sha(scorer_dir / name) for name in scorer_files},
        "source_results": str(SRC / "RESULTS.json"),
        "work_dir": str(WORK),
        "note": "Derived rescore. Original v19 scores stay in the pilot result.json files.",
    }
    (OUT / "V19_OFFLINE_REEVALUATION_META.json").write_text(
        json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    (OUT / "V19_OFFLINE_REEVALUATION.jsonl").write_text(
        "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in lines),
        encoding="utf-8")
    print("wrote", len(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
