#!/usr/bin/env python3
"""Recompute the evidence ledger for existing re-execution results.

Reads each ``result.json`` (which carries the recorded conform/monitor/binding
and the CIR property outcomes) and rewrites only its ``ledger`` field. No tool
runs, no trace changes.
"""
from __future__ import annotations
import argparse, json, sys
from pathlib import Path
REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow.evidence_v2 import evaluate_reexecution  # noqa: E402

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    args = ap.parse_args()
    n = 0
    for p in sorted(Path(args.root).glob("**/result.json")):
        r = json.loads(p.read_text())
        cir_path = r.get("cir_path"); contract_path = r.get("contract_path")
        if not contract_path or not Path(contract_path).is_file():
            continue
        contract = json.loads(Path(contract_path).read_text())
        led = evaluate_reexecution(r, contract, accepted=r.get("historical_acceptance",
                                r.get("ledger", {}).get("historical_acceptance", False)),
                                cir_props=r.get("cir_props") or {},
                                cir_complete=r.get("cir_complete"))
        r["ledger"] = led.to_dict()
        p.write_text(json.dumps(r, ensure_ascii=False, indent=2) + "\n")
        n += 1
    print(f"recomputed {n} ledgers")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
