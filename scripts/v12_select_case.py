#!/usr/bin/env python3
"""Read the v6 matrix and preflight the one fixed generated defect.

``--out`` is required. This process writes the pool, toolchain evidence, and
the final selected list. Nothing else should edit that list afterwards.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
import sys
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.send_holding_requirements import evaluate_requirements  # noqa: E402

BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")
TARGET = {
    "model": "deepseekflash",
    "task": "channel/send_while_holding_mutex",
    "candidate_kind": "final",
    "round_no": 3,
}


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--matrix", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--derived-control", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    rows = [json.loads(line) for line in Path(args.matrix).read_text(encoding="utf-8").splitlines() if line.strip()]
    manifest_rows = [json.loads(line) for line in Path(args.manifest).read_text(encoding="utf-8").splitlines() if line.strip()]
    pool = []
    chosen = None
    for row in rows:
        item = {
            "model": row.get("model"),
            "task": row.get("task"),
            "rep": row.get("rep"),
            "candidate_kind": row.get("candidate_kind"),
            "round_no": row.get("round_no"),
            "accepted_round": row.get("accepted_round"),
            "final_round": row.get("final_round"),
            "historical_acceptance": row.get("historical_acceptance"),
            "origin_batch": row.get("origin_batch"),
            "source_path": row.get("source_path"),
            "cir_path": row.get("cir_path"),
            "contract_path": row.get("contract_path"),
            "source_sha256": row.get("source_sha256"),
            "cir_sha256": row.get("cir_sha256"),
            "contract_sha256": row.get("contract_sha256"),
            "origin_class": "historical_k3" if row.get("model") in {"kimi", "kimi-k3"} and "k3" in str(row.get("model")) else "generated_candidate",
        }
        if row.get("model") == "kimi":
            item["origin_class"] = "historical_k3_batch_label_not_kimi_2_7_code"
        match = all(row.get(key) == value for key, value in TARGET.items())
        item["decision"] = "preflight" if match else "not_the_predeclared_v12_case"
        if match:
            chosen = row
        pool.append(item)
    evidence = out / "preflight"
    evidence.mkdir(exist_ok=True)
    selected = []
    reason = "target row not in the matrix"
    if chosen:
        source = Path(chosen["source_path"])
        cir = Path(chosen["cir_path"])
        contract = Path(chosen["contract_path"])
        derived = Path(args.derived_control)
        for label, path in (("source", source), ("cir", cir), ("contract", contract), ("derived_control", derived)):
            shutil.copy(path, evidence / f"{label}{path.suffix}")
        requirements = REPO / "benchmarks/families/channel/send_while_holding_mutex/generation_input/REQUIREMENTS.md"
        shutil.copy(requirements, evidence / "REQUIREMENTS.md")
        defect_req = evaluate_requirements(source.read_text(encoding="utf-8"), evidence / "defect-req")
        control_req = evaluate_requirements(derived.read_text(encoding="utf-8"), evidence / "derived-req")
        defect_tool = evaluate_candidate(
            source.read_text(encoding="utf-8"), cir, contract, evidence / "defect-tool",
            binary=BIN, instrument=INS, n_runs=2, run_timeout=6, cell_id="defect")
        control_tool = evaluate_candidate(
            derived.read_text(encoding="utf-8"), cir, contract, evidence / "derived-tool",
            binary=BIN, instrument=INS, n_runs=2, run_timeout=6, cell_id="derived")
        defect_ok = (defect_req["status"] == "fail"
                     and defect_req["checks"]["R2"]["status"] == "fail"
                     and (defect_tool["ledger"].get("current_evaluation") == "explicit_failure"))
        control_ok = (control_req["status"] == "bounded_covered_satisfied"
                      and control_tool["ledger"].get("delivery_status") == "deliver_bounded")
        record = {
            "task": chosen["task"],
            "model": chosen["model"],
            "requested_model_id": "deepseek-flash",
            "candidate_kind": chosen["candidate_kind"],
            "round_no": chosen["round_no"],
            "historical_acceptance": chosen["historical_acceptance"],
            "origin_class": "natural_generated_defect",
            "derived_control_class": "human_derived_control",
            "source_path": str(source),
            "cir_path": str(cir),
            "contract_path": str(contract),
            "requirements_path": str(requirements),
            "derived_control_path": str(derived),
            "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "cir_sha256": hashlib.sha256(cir.read_bytes()).hexdigest(),
            "hashes_match_matrix": hashlib.sha256(source.read_bytes()).hexdigest() == chosen.get("source_sha256"),
            "defect_requirement": defect_req["status"],
            "defect_r2": defect_req["checks"]["R2"]["status"],
            "derived_requirement": control_req["status"],
            "defect_evaluation": defect_tool["ledger"].get("current_evaluation"),
            "defect_binding": (defect_tool["ledger"].get("layers") or {}).get("binding"),
            "derived_delivery": control_tool["ledger"].get("delivery_status"),
            "evidence": {
                "defect_tool": str(evidence / "defect-tool" / "result.json"),
                "derived_tool": str(evidence / "derived-tool" / "result.json"),
            },
        }
        (evidence / "DECISION.json").write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
        if defect_ok and control_ok and record["hashes_match_matrix"]:
            selected.append(record)
            reason = "preflight matched the reviewed generated defect and the human-derived control"
        else:
            reason = "preflight did not reproduce the required defect/control split"
            record["decision"] = "rejected_by_preflight"
    for item in pool:
        if item["decision"] == "preflight":
            item["decision"] = "selected" if selected else "rejected_by_preflight"
            item["reason"] = reason
    report = {"selected": [row["task"] for row in selected], "reason": reason,
              "manifest_rows": len(manifest_rows), "matrix_rows": len(rows),
              "selected_records": selected, "pool": pool}
    (out / "CANDIDATE_POOL.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"selected": report["selected"], "reason": reason}, ensure_ascii=False))
    return 0 if selected else 2


if __name__ == "__main__":
    raise SystemExit(main())
