#!/usr/bin/env python3
"""Offline rescore of the frozen v19 pilot. No model requests.

Design credit is taken from the saved same-round snapshot only when the
extracted candidate hash matches the stored candidate hash. A missing source
does not become a joint pass.
"""

from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import _extract_rust_body  # noqa: E402
from cir_workflow.task_score import score_for_task  # noqa: E402

SRC = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v19/live")
OUT = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v21")
WORK = Path("/tmp/cpv-v21-reeval")


def _sha_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def _sha_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _passed(status: str | None) -> bool:
    return status in {"pass", "bounded_covered_satisfied"}


def main() -> int:
    rows = json.loads((SRC / "RESULTS.json").read_text(encoding="utf-8"))
    WORK.mkdir(exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    scorer_dir = REPO / "python/cir_workflow"
    names = ("condvar_requirements.py", "send_holding_requirements.py", "task_score.py")
    lines = []
    for row in rows:
        dest = SRC / row["task"].replace("/", "__") / row["model_id"] / row["arm"]
        state = json.loads((dest / "state.json").read_text(encoding="utf-8"))
        result = json.loads((dest / "result.json").read_text(encoding="utf-8")) if (dest / "result.json").is_file() else {}
        rounds = []
        first_requirement = None
        first_design = None
        first_both = None
        for req in state.get("requests") or []:
            text = req.get("response") or ""
            extracted = _extract_rust_body(text) if text else ""
            extracted_sha = _sha_text(extracted) if extracted else None
            stored_sha = req.get("candidate_sha256")
            hash_matches = bool(extracted_sha and stored_sha and extracted_sha == stored_sha)
            new = None
            if extracted and extracted.strip():
                work = WORK / f"{row['model_id']}-{row['arm']}-{row['task'].replace('/', '__')}-r{req.get('round')}"
                new = score_for_task(row["task"], extracted, work)
            original = next((item for item in (result.get("rounds") or []) if item.get("round") == req.get("round")), {})
            new_status = None if new is None else new.get("status")
            design_status = original.get("design_status")
            design_credit = design_status if hash_matches else None
            requirement_pass = _passed(new_status)
            design_pass = design_credit == "pass"
            if requirement_pass and first_requirement is None:
                first_requirement = req.get("round")
            if design_pass and first_design is None:
                first_design = req.get("round")
            joint = requirement_pass and design_pass
            if joint and first_both is None:
                first_both = req.get("round")
            checks = {} if new is None else {
                key: value.get("status") for key, value in (new.get("checks") or {}).items()
            }
            rounds.append({
                "round": req.get("round"),
                "original_requirement_status": original.get("requirement_status"),
                "original_design_status": design_status,
                "new_requirement_status": new_status,
                "candidate_sha256": extracted_sha,
                "stored_candidate_sha256": stored_sha,
                "candidate_hash_matches_saved": hash_matches,
                "design_credit": design_credit,
                "joint_pass": joint,
                "new_nonpass": {key: value for key, value in checks.items() if value != "pass"},
                "empty_candidate": not bool((extracted or "").strip()),
                "finish_reason": (req.get("identity") or {}).get("finish_reason"),
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
            "new_first_requirement_round": first_requirement,
            "new_first_design_round": first_design,
            "new_first_both_round": first_both,
            "rounds": rounds,
            "real_requests": 0,
        }
        lines.append(record)
        print(row["model_id"], row["arm"], row["task"].split("/")[0],
              [item["new_requirement_status"] for item in rounds],
              "both", first_both, "was", row.get("stop"), flush=True)
    summary = {
        "cells": len(lines),
        "by_arm": {},
        "by_model": {},
        "note": "Observed first passing rounds of saved candidates. Not a new model run and not a theoretical minimum.",
    }
    for key, bucket in (("arm", "by_arm"), ("model_id", "by_model")):
        groups = {}
        for line in lines:
            groups.setdefault(line[key], []).append(line)
        for name, group in groups.items():
            summary[bucket][name] = {
                "cells": len(group),
                "requirement_pass": sum(1 for item in group if item["new_first_requirement_round"] is not None),
                "design_pass": sum(1 for item in group if item["new_first_design_round"] is not None),
                "same_round_both": sum(1 for item in group if item["new_first_both_round"] is not None),
                "original_stops": dict(Counter(item["original_stop"] for item in group)),
            }
    meta = {
        "real_requests": 0,
        "cells": len(lines),
        "scorer_sha256": {name: _sha_file(scorer_dir / name) for name in names},
        "source_results_sha256": _sha_file(SRC / "RESULTS.json"),
        "work_dir": str(WORK),
        "summary": summary,
    }
    (OUT / "V19_OFFLINE_REEVALUATION.jsonl").write_text(
        "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in lines), encoding="utf-8")
    (OUT / "V19_OFFLINE_REEVALUATION_META.json").write_text(
        json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    print("wrote", len(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
