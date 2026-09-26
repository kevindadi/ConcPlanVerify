#!/usr/bin/env python3
"""Common evaluation of G2 and G3 final artifacts under one monitor metric.

Both arms are scored by the same ``bounded_monitor.coverage`` over the frozen
contract. G2's coverage comes from its saved oracle; G3's is recomputed from the
accepted round's saved monitor.json. No model calls, no source edits.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402


def g2_rf(cell_dir: Path) -> dict | None:
    cj = cell_dir / "CELL.json"
    if not cj.is_file():
        return None
    rec = json.loads(cj.read_text())
    oracle = rec.get("oracle") or {}
    cov = oracle.get("coverage")
    return cov or None


def g3_rf(cell_dir: Path, row: dict, task, work_root: Path,
          instrument_binary: Path, binary: Path) -> dict | None:
    """Re-run the SAME oracle on the G3 final Rust (offline, no model calls)."""

    round_no = row.get("first_code_accept_round")
    if not round_no:
        return None
    rust = cell_dir / f"code/round-{round_no}.rs"
    if not rust.is_file():
        return None
    work = work_root / task.id.replace("/", "__") / f"rep{row.get('replicate',0)}"
    work.mkdir(parents=True, exist_ok=True)
    result = rust_oracle.evaluate(
        rust.read_text(encoding="utf-8"), task.contract_path, task.reference_cir_path,
        work, n_native=32, miri_seeds=0, run_timeout=10.0,
        binary=binary, instrument_binary=instrument_binary)
    return result.get("coverage")


def _agg(covs: list[dict | None], n_tasks: int) -> dict:
    rf_all = sum((c or {}).get("rf", 0.0) for c in covs) / n_tasks
    delivered = [c for c in covs if c]
    rf_acc = (sum(c["rf"] for c in delivered) / len(delivered)) if delivered else None
    return {"rf_all": round(rf_all, 4), "rf_acc": round(rf_acc, 4) if rf_acc else None,
            "delivered": len(delivered)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pairs", required=True,
                        help="JSON: {model: {g2: <batch>, g3: <batch>}}")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    pairs = json.loads(Path(args.pairs).read_text())
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    tasks_by_id = {t.id: t for t in load_gen_tasks(REPO)}
    binary = REPO.parent / "ConcIR/target/release/concir-backend"
    instrument_binary = REPO.parent / "ConcIR/target/release/concir-instrument"
    work_root = out / "g3-work"
    report = {}
    for model, b in pairs.items():
        g2dir = REPO / b["g2"]; g3dir = REPO / b["g3"]
        g2 = {c["task"]: c for c in json.loads((g2dir / "SUMMARY.json").read_text())["cells"]}
        g3 = {c["task"]: c for c in json.loads((g3dir / "SUMMARY.json").read_text())["cells"]}
        tasks = sorted(set(g2) & set(g3))
        g2c, g3c = [], []
        for t in tasks:
            d2 = g2dir / model / t.replace("/", "__") / f"rep{g2[t].get('replicate',0)}"
            d3 = g3dir / model / t.replace("/", "__") / f"rep{g3[t].get('replicate',0)}"
            g2c.append(g2_rf(d2))
            g3c.append(g3_rf(d3, g3[t], tasks_by_id[t], work_root,
                             instrument_binary, binary))
        report[model] = {"n": len(tasks),
                         "g2": _agg(g2c, len(tasks)), "g3": _agg(g3c, len(tasks))}
        r = report[model]
        print(f"{model:14s} n={r['n']} | G2 RF_all={r['g2']['rf_all']} RF_acc={r['g2']['rf_acc']} "
              f"del={r['g2']['delivered']} | G3 RF_all={r['g3']['rf_all']} "
              f"RF_acc={r['g3']['rf_acc']} del={r['g3']['delivered']}")
    (out / "COMMON_EVAL.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n",
                                          encoding="utf-8")
    print(f"wrote {out}/COMMON_EVAL.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
