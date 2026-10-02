#!/usr/bin/env python3
"""strong-link-v25: offline value-observation controls and frozen re-eval.

Uses the NEW toolchain (explicitly selected), no model requests. Writes one
machine-readable record per case: source/binary hashes, the var_eq property
status, the per-requirement statuses, and the guarantee scope.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import rust_oracle  # noqa: E402

OUT = REPO / "experiments/strong-link-v25-value"
CTRL = OUT / "controls"
ATOMIC = REPO / "benchmarks/families/atomic-data/atomic_lost_update"
BARE = REPO / "benchmarks/families/condvar/bare_wait_no_predicate"
REAL_ATOMIC = REPO / ("experiments/strong-link-v23-thinking-2/atomic-data__atomic_lost_update"
                      "/gpt-6-luna/G3_concir/rep0/code/round-1.rs")
REAL_BARE = REPO / ("experiments/strong-link-v23-thinking-2/condvar__bare_wait_no_predicate"
                    "/gpt-6-luna/G3_concir/rep0/code/round-1.rs")

CASES = [
    ("atomic_pos", REAL_ATOMIC, ATOMIC, "positive", "PASS_bounded"),
    ("atomic_neg_print", CTRL / "atomic_neg_print.rs", ATOMIC, "negative", "not_observed"),
    ("atomic_unknown_field", CTRL / "atomic_unknown_field.rs", ATOMIC, "unknown", "unsupported"),
    ("bare_pos", REAL_BARE, BARE, "positive", "PASS_bounded"),
    ("bare_neg_print", CTRL / "bare_neg_print.rs", BARE, "negative", "not_observed"),
    ("bare_shadow_other_mutex", CTRL / "bare_shadow_other_mutex.rs", BARE, "shadow", "not_observed"),
]


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def var_eq_status(report: dict) -> dict:
    out = {}
    for p in report.get("properties", []):
        if p.get("kind") in ("reachability", "always_reachable", "reachable"):
            out[p["id"]] = {"status": p.get("status"), "detail": p.get("detail")}
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument"))
    args = ap.parse_args()
    binary, instrument = Path(args.binary), Path(args.instrument)
    OUT.mkdir(parents=True, exist_ok=True)
    records = []
    for name, src, task_dir, category, expect in CASES:
        source = Path(src).read_text(encoding="utf-8")
        r = rust_oracle.evaluate(
            source, task_dir / "contract.json", task_dir / "fixed.cir.json",
            OUT / "work" / name, n_native=32, miri_seeds=0, run_timeout=10.0,
            binary=binary, instrument_binary=instrument)
        monitor = r.get("monitor") or {}
        veq = var_eq_status(monitor)
        # find the target var_eq property (the one whose goal is a var_eq)
        target_status = None
        for p in monitor.get("properties", []):
            if p.get("id") in ("both-increments", "ready-set", "main::c == 2",
                               "main::ready == True"):
                target_status = p.get("status")
                break
        records.append({
            "case": name, "category": category, "expected_var_eq": expect,
            "observed_var_eq": target_status,
            "match": target_status == expect,
            "built": r.get("built"), "behavior_ok": r.get("behavior_ok"),
            "hang": r.get("hang"),
            "limitations": r.get("limitations"),
            "source_sha256": sha(Path(src)),
            "contract_sha256": sha(task_dir / "contract.json"),
            "cir_sha256": sha(task_dir / "fixed.cir.json"),
            "var_eq_properties": veq,
            "coverage_statuses": (r.get("coverage") or {}).get("statuses"),
            "guarantee": ("bounded witness over 32 native runs; not a proof over all schedules"),
        })
    (OUT / "CONTROLS.json").write_text(json.dumps({
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "evaluator": "rust_oracle.evaluate n_native=32 miri_seeds=0",
        "cases": records}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for rec in records:
        print(f"{rec['case']:28} expect={rec['expected_var_eq']:14} "
              f"observed={rec['observed_var_eq']:14} match={rec['match']} built={rec['built']}")
    return 0 if all(r["match"] for r in records) else 2


if __name__ == "__main__":
    raise SystemExit(main())
