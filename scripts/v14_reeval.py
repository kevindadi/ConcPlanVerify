#!/usr/bin/env python3
"""Offline re-score of the fixed v13 historical set. No model requests."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
OUT = NOTES / "strong-link-v14"
V13 = NOTES / "strong-link-v13" / "V12_OFFLINE_REEVALUATION.jsonl"


def _load():
    spec = importlib.util.spec_from_file_location("v13_reeval", REPO / "scripts/v13_reeval.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    mod = _load()
    OUT.mkdir(parents=True, exist_ok=True)
    previous = {}
    for line in V13.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            previous[(row.get("source_sha256"), row.get("model"), row.get("arm"), row.get("round"))] = row
    work_root = OUT / "work"
    if work_root.exists():
        shutil.rmtree(work_root)
    lines = []
    for index, item in enumerate(mod.candidates()):
        source = item["path"].read_text(encoding="utf-8")
        digest = hashlib.sha256(item["path"].read_bytes()).hexdigest()
        work = work_root / f"{index:02d}"
        requirement = mod.evaluate_requirements(source, work / "requirement")
        shutil.rmtree(work / "requirement" / "target", ignore_errors=True)
        design = mod.eval_design(source, work / "design")
        old = None
        if item["origin"] == "v12_model_candidate":
            old = mod.old_delivery(item["model"], item["arm"], item["round"])
        v13 = previous.get((digest, item["model"], item["arm"], item["round"]))
        record = {
            "origin": item["origin"],
            "counts_in_natural_defect_denominator": item["origin"] == "natural_generated_defect",
            "model": item["model"],
            "arm": item["arm"],
            "round": item["round"],
            "source_path": str(item["path"]),
            "source_sha256": digest,
            "highlight_same_task_round": item["highlight"],
            "requirement": mod.slim_requirement(requirement),
            "identity": design.get("binding_layer"),
            "attributes": design.get("attributes_layer"),
            "attribute_rows": design.get("attributes"),
            "trace": design.get("trace"),
            "uncovered_sync": design.get("uncovered_sync"),
            "design_correspondence": design.get("design_correspondence"),
            "new_evaluation": design.get("current_evaluation"),
            "new_delivery": design.get("delivery_status"),
            "v12_delivery": None if old is None else old.get("delivery_status"),
            "v13_delivery": None if v13 is None else v13.get("new_delivery"),
            "v13_evaluation": None if v13 is None else v13.get("new_evaluation"),
            "delivery_changed_from_v13": None if v13 is None else v13.get("new_delivery") != design.get("delivery_status"),
            "reasons": design.get("reasons"),
            "evidence": design.get("evidence"),
            "real_model_requests": 0,
        }
        lines.append(record)
        print(item["origin"], item["model"], item["arm"], item["round"],
              record["requirement"]["status"], record["new_delivery"],
              record["delivery_changed_from_v13"], flush=True)
    (OUT / "V14_OFFLINE_REEVALUATION.jsonl").write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in lines), encoding="utf-8")
    print("rows", len(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
