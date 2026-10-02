#!/usr/bin/env python3
"""strong-link-v27: read-only audit of the four v26 attempts.

Counts physical reservations, retries, per-cell attempt distribution and duplicate
reservations. Does not refund, reset, or rewrite any historical ledger.
"""

from __future__ import annotations

import argparse
import collections
import json
import sqlite3
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUT = REPO / "experiments/strong-link-v27"
ATTEMPTS = ["experiments/strong-link-v26-batch", "experiments/strong-link-v26-batch-2",
            "experiments/strong-link-v26-batch-3", "experiments/strong-link-v26-batch-4"]


def read_attempts(db: Path) -> list[dict]:
    if not db.is_file():
        return []
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        cols = [r[1] for r in conn.execute("pragma table_info(attempt)")]
        return [dict(zip(cols, row)) for row in conn.execute("select * from attempt")]
    finally:
        conn.close()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(OUT))
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    report = {"attempts": [], "note": "read-only; no refund, reset or rewrite"}
    for name in ATTEMPTS:
        d = REPO / name
        budget = json.loads((d / "budget.json").read_text()) if (d / "budget.json").is_file() else {}
        rows = read_attempts(d / "budget.sqlite")
        statuses = collections.Counter(r.get("status") for r in rows)
        retries = collections.Counter(r.get("transport_retry") for r in rows)
        # duplicate reservation = same (cell_id, logical_attempt, transport_retry)
        keys = collections.Counter((r.get("cell_id"), r.get("logical_attempt"),
                                    r.get("transport_retry")) for r in rows if r.get("cell_id"))
        dup = {str(k): n for k, n in keys.items() if n > 1}
        cells = collections.Counter(r.get("cell_id") for r in rows if r.get("cell_id"))
        report["attempts"].append({
            "dir": name,
            "requests_used": budget.get("requests_used"),
            "attempt_rows": len(rows),
            "statuses": dict(statuses),
            "transport_retry_histogram": {str(k): v for k, v in sorted(retries.items(), key=lambda x: str(x[0]))},
            "unique_cells": len(cells),
            "max_attempts_on_one_cell": max(cells.values()) if cells else 0,
            "duplicate_reservations": dup,
        })
    (out / "AUDIT_V26.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
