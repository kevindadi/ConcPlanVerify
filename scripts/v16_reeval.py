#!/usr/bin/env python3
"""Offline re-score of the fixed historical 17. No model requests."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
OUT = NOTES / "strong-link-v16"
PRIOR = NOTES / "strong-link-v15" / "V15_OFFLINE_REEVALUATION.jsonl"


def _load():
    spec = importlib.util.spec_from_file_location("v13_reeval", REPO / "scripts/v13_reeval.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    mod = _load()
    OUT.mkdir(parents=True, exist_ok=True)
    previous = {}
    for line in PRIOR.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            previous[(row.get("source_sha256"), row.get("model"), row.get("arm"), row.get("round"))] = row
    v14_path = NOTES / "strong-link-v14" / "V14_OFFLINE_REEVALUATION.jsonl"
    v14_rows = {}
    for line in v14_path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            v14_rows[(row.get("source_sha256"), row.get("model"), row.get("arm"), row.get("round"))] = row
    runtime = (REPO / "runtime/concir_sync/src/lib.rs").read_bytes()
    backend = REPO.parent / "ConcIR/target/release/concir-backend"
    work_root = OUT / "historical-work"
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
        old = mod.old_delivery(item["model"], item["arm"], item["round"]) if item["origin"] == "v12_model_candidate" else None
        v15 = previous.get((digest, item["model"], item["arm"], item["round"]))
        v14 = v14_rows.get((digest, item["model"], item["arm"], item["round"]))
        record = {
            "origin": item["origin"],
            "counts_in_natural_defect_denominator": item["origin"] == "natural_generated_defect",
            "model": item["model"], "arm": item["arm"], "round": item["round"],
            "source_path": str(item["path"]), "source_sha256": digest,
            "runtime_concir_sync_sha256": hashlib.sha256(runtime).hexdigest(),
            "concir_backend_sha256": hashlib.sha256(backend.read_bytes()).hexdigest(),
            "requirement": mod.slim_requirement(requirement),
            "identity": design.get("binding_layer"),
            "attributes": design.get("attributes_layer"),
            "trace": design.get("trace"),
            "uncovered_sync": design.get("uncovered_sync"),
            "design_correspondence": design.get("design_correspondence"),
            "new_evaluation": design.get("current_evaluation"),
            "new_delivery": design.get("delivery_status"),
            "v12_delivery": None if old is None else old.get("delivery_status"),
            "v13_delivery": None if v15 is None else v15.get("v13_delivery"),
            "v14_delivery": None if v14 is None else v14.get("new_delivery"),
            "v15_delivery": None if v15 is None else v15.get("new_delivery"),
            "delivery_changed_from_v15": None if v15 is None else v15.get("new_delivery") != design.get("delivery_status"),
            "real_model_requests": 0,
        }
        lines.append(record)
        print(item["model"], item["arm"], item["round"], record["new_delivery"],
              record["delivery_changed_from_v15"], flush=True)
    (OUT / "V16_OFFLINE_REEVALUATION.jsonl").write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in lines), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
