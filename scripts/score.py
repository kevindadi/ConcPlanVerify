#!/usr/bin/env python3
"""Re-evaluate final Rust candidates from every arm using the same bounded oracle."""
import argparse
import hashlib
import json
import os
import sys
from collections import defaultdict
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from cir_workflow.generation import load_gen_tasks
from cir_workflow import rust_oracle


def read(p): return json.loads(Path(p).read_text())


def usage_for(cell):
    path = cell / "audit.jsonl"
    events = [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []
    events = [e for e in events if e.get("kind") == "model-call"]
    usage = {}
    for key in ("input_tokens", "output_tokens", "reasoning_tokens", "total_tokens"):
        values = [(e.get("usage") or {}).get(key) for e in events]
        known = [int(x) for x in values if x is not None]
        usage[key] = {"known_sum": sum(known) if known else None,
                      "calls_with_usage": len(known), "calls_missing_usage": len(values)-len(known)}
    return {"physical_attempts_observed": len(events), "usage": usage,
            "steps": [{"stage": e.get("stage"), "round": e.get("candidate_round"),
                       "status": e.get("status"), "requested_model": e.get("requested_model"),
                       "returned_model": e.get("returned_model"), "usage": e.get("usage")} for e in events]}


def candidate(cell, arm):
    paths = (list(cell.glob("code/round-*.rs")) if arm == "G3_concir" else
             list(cell.glob("run-*/round-*/candidate.rs")))
    def key(p):
        round_no = int(p.stem.split("-")[1]) if arm == "G3_concir" else int(p.parent.name.split("-")[1])
        return (round_no, str(p))
    return max(paths, key=key) if paths else None


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--batch", required=True); p.add_argument("--out", required=True)
    p.add_argument("--native-runs", type=int, default=32); a = p.parse_args()
    if a.native_runs < 1: p.error("--native-runs must be positive")
    batch = Path(a.batch).resolve(); out = Path(a.out).resolve()
    out.mkdir(parents=True, exist_ok=False)
    summary = read(batch / "SUMMARY.json")
    tasks = {t.id: t for t in load_gen_tasks(ROOT)}
    rows = []
    for item in summary["cells"]:
        c = item["cell"]; task = tasks[c["task"]]
        cell = batch / c["task"].replace("/", "__") / c["model_id"] / c["arm"] / f"rep{c['rep']}"
        source = candidate(cell, c["arm"])
        record = read(cell / "CELL_RECORD.json") if (cell / "CELL_RECORD.json").exists() else {}
        row = {"cell": c, "tier": task.tier, "accepted": item.get("accepted"),
               "execution_status": item["status"], "method_status": item.get("record_status"),
               "emitted_candidate": source is not None, "candidate_sha256":
               hashlib.sha256(source.read_bytes()).hexdigest() if source else None,
               "accepted_round": record.get("accepted_round"), "accounting": usage_for(cell),
               "coverage": None, "oracle_status": "no_candidate"}
        work = out / c["task"].replace("/", "__") / c["model_id"] / c["arm"] / f"rep{c['rep']}"
        if source:
            try:
                result = rust_oracle.evaluate(source.read_text(), task.contract_path, task.reference_cir_path,
                    work, n_native=a.native_runs, miri_seeds=0, run_timeout=10,
                    binary=os.environ["CONCIR_BACKEND"], instrument_binary=os.environ["CONCIR_INSTRUMENT"],
                    binding_binary=os.environ["CONCIR_BIND_CHECK"])
                row.update(coverage=result.get("coverage"), oracle_status=result.get("status"),
                           hang=result.get("hang"), behavior_ok=result.get("behavior_ok"))
                work.mkdir(parents=True, exist_ok=True)
                (work / "ORACLE.json").write_text(json.dumps(result, indent=2)+"\n")
            except Exception as exc:
                row.update(oracle_status="oracle_error", error=f"{type(exc).__name__}: {exc}")
        if c["arm"] == "G3_concir":
            row["cir_stage"] = record.get("cir_stage")
            row["code_rounds"] = [{"round": r.get("round"), "accepted": r.get("accepted"),
                                    "status": r.get("status")} for r in (record.get("code_stage") or {}).get("rounds", [])]
        rows.append(row)
        (out / "CELLS.json").write_text(json.dumps(rows, indent=2)+"\n")
    groups = defaultdict(list)
    for row in rows:
        c=row["cell"]; groups[(c["model_id"],c["arm"],row["tier"])].append(row)
    aggregate=[]
    for (model, arm, tier), cells in sorted(groups.items()):
        scored=[r for r in cells if r["coverage"] is not None]
        accepted=[r for r in scored if r["accepted"]]
        unknown=[r for r in cells if r["emitted_candidate"] and r["coverage"] is None
                 and r["oracle_status"] != "build_failed"]
        total=sum(r["coverage"]["rf"] for r in scored)
        aggregate.append({"model":model,"arm":arm,"tier":tier,"cells_present":len(cells),
            "accepted":sum(bool(r["accepted"]) for r in cells),"emitted_candidates":sum(r["emitted_candidate"] for r in cells),
            "scored_candidates":len(scored),"unknown_score_cells":len(unknown),
            "RF_all":total/len(cells) if not unknown else None,
            "RF_all_known_lower_bound":total/len(cells),
            "RF_accepted_scored":sum(r["coverage"]["rf"] for r in accepted)/len(accepted) if accepted else None})
    report={"planned_cells":summary.get("planned_cells"),"matrix_complete":summary.get("matrix_complete"),
            "cells":rows,"groups":aggregate,"notes":[
        "All arms use the same reference CIR, frozen contract, structural binding, and bounded native-run oracle.",
        "Method acceptance, emitted candidates, and requirement satisfaction are different quantities.",
        "A missing candidate or build failure contributes zero to RF_all; unknown oracle scores make RF_all null.",
        "RF_all_known_lower_bound is explicitly conservative; unknown scores are not labelled failed requirements.",
        "Incomplete matrices must not be treated as final results: check planned_cells and matrix_complete.",
        "Reasoning tokens can be included in output tokens; do not add them again. Missing usage is null."]}
    (out / "REPORT.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({"scored":len(rows),"output":str(out / "REPORT.json")}))


if __name__ == "__main__": main()
