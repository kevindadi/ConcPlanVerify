#!/usr/bin/env python3
"""strong-link-v25: offline re-eval of frozen Atomic/Condvar candidates with the
NEW value-observation toolchain. Original code/CIR/contract are read-only;
results are derived into a new directory."""

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
TASKS = {
    "atomic-data/atomic_lost_update": REPO / "benchmarks/families/atomic-data/atomic_lost_update",
    "condvar/bare_wait_no_predicate": REPO / "benchmarks/families/condvar/bare_wait_no_predicate",
}
BATCHES = [REPO / "experiments/strong-link-v23-thinking-2",
           REPO / "experiments/strong-link-v22-three-model"]
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def last_candidate(cell: Path, arm: str) -> Path | None:
    if arm == "G3_concir":
        rounds = sorted(cell.glob("code/round-*.rs"),
                        key=lambda p: int(p.stem.split("-")[1]))
    else:
        rounds = sorted(cell.glob("run-*/round-*/candidate.rs"),
                        key=lambda p: int(p.parent.name.split("-")[1]))
    return rounds[-1] if rounds else None


def var_eq(report: dict) -> dict:
    out = {}
    for p in report.get("properties", []):
        if p.get("kind") in ("reachability", "always_reachable", "reachable"):
            out[p.get("id")] = p.get("status")
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument"))
    args = ap.parse_args()
    binary, instrument = Path(args.binary), Path(args.instrument)
    records = []
    for task, tdir in TASKS.items():
        for batch in BATCHES:
            base = batch / task.replace("/", "__")
            if not base.is_dir():
                continue
            for model in sorted(p.name for p in base.iterdir() if p.is_dir()):
                for arm in ARMS:
                    cell = base / model / arm / "rep0"
                    if not cell.is_dir():
                        continue
                    cand = last_candidate(cell, arm)
                    if cand is None:
                        continue
                    cache = cell / "cache.json"
                    acc = json.loads(cache.read_text()).get("accepted") if cache.is_file() else None
                    try:
                        r = rust_oracle.evaluate(
                            cand.read_text(encoding="utf-8"), tdir / "contract.json",
                            tdir / "fixed.cir.json", OUT / "reeval-work" / task.replace("/", "__")
                            / batch.name / model / arm, n_native=32, miri_seeds=0,
                            run_timeout=10.0, binary=binary, instrument_binary=instrument)
                        rec = {"built": r.get("built"), "behavior_ok": r.get("behavior_ok"),
                               "hang": r.get("hang"), "var_eq": var_eq(r.get("monitor") or {}),
                               "coverage_statuses": (r.get("coverage") or {}).get("statuses"),
                               "limitations": r.get("limitations")}
                    except Exception as exc:  # noqa: BLE001 - one candidate must not stop
                        rec = {"error": f"{type(exc).__name__}: {exc}"[:200]}
                    records.append({
                        "task": task, "batch": batch.name, "model": model, "arm": arm,
                        "candidate": str(cand), "candidate_sha256": sha(cand),
                        "arm_accepted": acc, **rec})
    (OUT / "REEVAL.json").write_text(json.dumps({
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "contract_sha256": {t: sha(d / "contract.json") for t, d in TASKS.items()},
        "cir_sha256": {t: sha(d / "fixed.cir.json") for t, d in TASKS.items()},
        "records": records}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for rec in records:
        print(f"{rec['task'].split('/')[-1]:24} {rec['batch'][-12:]:12} {rec['model']:16} "
              f"{rec['arm']:14} built={rec.get('built')} var_eq={rec.get('var_eq') or rec.get('error')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
