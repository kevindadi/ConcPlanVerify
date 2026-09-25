#!/usr/bin/env python3
"""Freeze independently judgeable [U] functional checks from requirement text.

Print-line clauses are behavioral oracles. Other [U] clauses are listed as
not automatically judgeable. This file is derived only from frozen requirements,
not from model output.
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
MANIFEST = REPO / "benchmarks/GENERATION_MANIFEST.json"
OUT = REPO / "experiments/evidence-20260925/u-tests"
PRINT_RE = re.compile(r"print exactly the line `([^`]+)`")
CLAUSE_RE = re.compile(r"^(R\d+)\.\s+(.*)$")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    tasks = json.loads(MANIFEST.read_text(encoding="utf-8"))["tasks"]
    rows = []
    for entry in tasks:
        path = REPO / "benchmarks/families" / entry["task"] / "generation_input/REQUIREMENTS.md"
        text = path.read_text(encoding="utf-8")
        for line in text.splitlines():
            if "[U]" not in line:
                continue
            m = CLAUSE_RE.match(line.strip())
            if not m:
                continue
            rid, body = m.group(1), m.group(2)
            printed = PRINT_RE.search(body)
            rows.append({
                "task": entry["task"],
                "clause": rid,
                "text": body,
                "judgeable": bool(printed),
                "kind": "terminal_line" if printed else "not_automatically_judgeable",
                "expected_line": printed.group(1) if printed else None,
                "source_sha256": hashlib.sha256(text.encode()).hexdigest(),
            })
    payload = {
        "rule": "A [U] clause is automatically judgeable only when it requires one exact terminal line. Naming differences do not fail a program that prints that line. Other [U] clauses stay not_automatically_judgeable.",
        "n_clauses": len(rows),
        "n_judgeable": sum(1 for r in rows if r["judgeable"]),
        "clauses": rows,
    }
    (OUT / "TESTS.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"judgeable {payload['n_judgeable']} / {payload['n_clauses']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
