#!/usr/bin/env python3
"""Offline evidence-ledger re-evaluation of frozen multi-model batches.

Reads saved CIR/Rust/trace/monitor artifacts (no model calls) and re-scores each
accepted cell into sufficient / insufficient / explicit_failure, with a per
property ledger. Emits a JSONL ledger and a historical-vs-new diff table.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.evidence import evaluate_cell  # noqa: E402


def _cell_dir(batch: Path, cell: dict) -> Path:
    return (batch / cell["model"] / cell["task"].replace("/", "__")
            / f"rep{cell.get('replicate', 0)}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch", action="append", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    ledger = []
    diff_rows = []
    per_batch = {}
    for bpath in args.batch:
        batch = Path(bpath)
        if not batch.is_absolute():
            batch = REPO / batch
        summary = json.loads((batch / "SUMMARY.json").read_text(encoding="utf-8"))
        cells = [c for c in summary.get("cells", []) if c.get("arm") == "G3"]
        hist_acc = sum(1 for c in cells if c.get("accepted") is True)
        verdicts = Counter()
        for c in cells:
            cdir = _cell_dir(batch, c)
            row = dict(c)
            row["cell"] = f"{c['model']}/{c['task']}/rep{c.get('replicate',0)}"
            if not (cdir / "code").is_dir():
                # CIR-only or failed before code: no code evidence.
                verdict = "cir_only" if c.get("cir_accepted") else "no_cir"
                verdicts[verdict] += 1
                if c.get("accepted"):
                    diff_rows.append({"batch": batch.name, "cell": row["cell"],
                                      "historical": "accepted", "new": verdict,
                                      "note": "accepted without a code directory"})
                continue
            ev = evaluate_cell(cdir, row)
            verdicts[ev.verdict] += 1
            ledger.append({"batch": batch.name, **ev.to_dict()})
            if c.get("accepted"):
                diff_rows.append({"batch": batch.name, "cell": row["cell"],
                                  "historical": "accepted", "new": ev.verdict,
                                  "correspondence": ev.correspondence,
                                  "observed_events": ev.observed_events,
                                  "observable_ops": ev.observable_ops,
                                  "reasons": ev.reasons})
        per_batch[batch.name] = {
            "cells": len(cells), "historical_accepted": hist_acc,
            "verdicts": dict(verdicts)}

    (out / "EVIDENCE_LEDGER.jsonl").write_text(
        "\n".join(json.dumps(x, ensure_ascii=False) for x in ledger) + "\n",
        encoding="utf-8")
    (out / "EVIDENCE_DIFF.json").write_text(
        json.dumps({"per_batch": per_batch, "diff": diff_rows}, ensure_ascii=False,
                   indent=2) + "\n", encoding="utf-8")

    L = ["# 证据账本离线重评（历史 accepted → 新判定）", "",
         "只读重评：使用已保存的 CIR/Rust/trace/monitor，未调用模型。", ""]
    for name, s in per_batch.items():
        L.append(f"## {name}")
        L.append("")
        L.append(f"- G3 cells: {s['cells']}；历史 accepted: {s['historical_accepted']}")
        L.append(f"- 新判定: {s['verdicts']}")
        L.append("")
    L.append("## 历史 accepted 的逐格重评")
    L.append("")
    L.append("| 批次 | cell | 新判定 | 对应证据 | 观测事件 | CIR 可观测op | 原因 |")
    L.append("| --- | --- | --- | --- | --- | --- | --- |")
    for r in diff_rows:
        L.append(f"| {r['batch']} | {r['cell']} | {r['new']} | "
                 f"{r.get('correspondence','—')} | {r.get('observed_events','—')} | "
                 f"{r.get('observable_ops','—')} | {'; '.join(r.get('reasons',[])) or r.get('note','')} |")
    (out / "EVIDENCE_REVIEW.md").write_text("\n".join(L) + "\n", encoding="utf-8")
    print(f"wrote {out}: ledger={len(ledger)} diff={len(diff_rows)}")
    for name, s in per_batch.items():
        print(f"  {name}: {s['verdicts']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
