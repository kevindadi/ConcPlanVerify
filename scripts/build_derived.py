#!/usr/bin/env python3
"""Build the derived SUMMARY and per-task table from the per-cell ledgers.

Raw execution evidence (result.json) is immutable; the SUMMARY is a versioned
derived view rebuilt from it, so report and summary never diverge. The previous
SUMMARY, if any, is kept as ``SUMMARY.raw.json``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

LEDGER_VERSION = "strong-link-v6"


def _sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    args = ap.parse_args()
    root = Path(args.root)
    results = sorted(root.glob("**/result.json"))
    cells = []
    fingerprints = {}
    verdicts = Counter()
    for p in results:
        r = json.loads(p.read_text())
        led = r.get("ledger") or {}
        cells.append({
            "cell": r.get("cell"), "candidate_kind": r.get("candidate_kind"),
            "round_no": r.get("round_no"),
            "current_evaluation": led.get("current_evaluation"),
            "historical_acceptance": led.get("historical_acceptance"),
            "trace_state": (led.get("trace") or {}).get("state"),
            "run_state": (led.get("run") or {}).get("state"),
            "model_verified": (led.get("model") or {}).get("verified"),
            "binding_source": (r.get("binding") or {}).get("source"),
            "evidence_path": str(p),
        })
        verdicts[led.get("current_evaluation")] += 1
        fingerprints[str(p.relative_to(root))] = _sha(p)

    derived = {"ledger_version": LEDGER_VERSION, "root": str(root),
               "cells_total": len(cells), "verdict_totals": dict(verdicts),
               "cells": cells, "fingerprints": fingerprints}

    summary = root / "SUMMARY.json"
    if summary.is_file() and not (root / "SUMMARY.raw.json").is_file():
        (root / "SUMMARY.raw.json").write_text(summary.read_text())
    summary.write_text(json.dumps(derived, ensure_ascii=False, indent=2) + "\n")

    lines = [f"# Derived summary ({LEDGER_VERSION})", "",
             f"cells: {len(cells)}; verdicts: {dict(verdicts)}", "",
             "| cell | kind | verdict | trace | run | model_verified |",
             "| --- | --- | --- | --- | --- | --- |"]
    for c in sorted(cells, key=lambda x: x["cell"] or ""):
        lines.append(f"| {c['cell']} | {c['candidate_kind']} | {c['current_evaluation']} | "
                     f"{c['trace_state']} | {c['run_state']} | {c['model_verified']} |")
    (root / "DERIVED_TABLE.md").write_text("\n".join(lines) + "\n")
    print(f"built {summary} ({len(cells)} cells): {dict(verdicts)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
