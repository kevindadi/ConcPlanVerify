#!/usr/bin/env python3
"""Build a structural-correspondence audit from existing human records.

A 'no bug' verdict does not verify every mapping field. Fields are verified
only when the written review explicitly supports them. Everything else is
evidence_missing. Machine-proposed alignments are labeled separately and are
not owner verdicts.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
QUEUE = REPO / "experiments/HUMAN_REVIEW_QUEUE_GEN.md"
OUT = REPO / "experiments/evidence-20260925/structural-audit"

FIELDS = [
    "thread_spawn_join", "resource_identity", "lock_guard_drop", "wait_notify",
    "channel_semaphore", "branch_loop_guard", "shared_data_atomics",
    "required_behavior",
]


def parse_queue(text: str) -> list[dict]:
    rows = []
    for line in text.splitlines():
        if not line.startswith("| G"):
            continue
        cols = [c.strip() for c in line.strip("|").split("|")]
        if len(cols) < 7:
            continue
        rows.append({
            "arm": cols[0], "task": cols[1], "rep": int(cols[2]),
            "rust_sha12": cols[3].strip("`"),
            "agent_bug_present": cols[4],
            "owner_bug_present": cols[5],
            "owner_note": cols[6],
            "provenance": "owner confirmation recorded 2026-09-25; verdict text retained",
            "reviewer": "owner",
        })
    return rows


def field_status(note: str, field: str) -> str:
    n = note.lower()
    if field == "lock_guard_drop" and ("lock" in n and ("order" in n or "drop" in n or "mutex" in n)):
        return "verified_partial"
    if field == "required_behavior" and ("prints" in n or "print" in n or "[u]" in n):
        return "verified_partial"
    if field == "channel_semaphore" and ("channel" in n or "semaphore" in n or "sync_channel" in n):
        return "verified_partial"
    if field == "wait_notify" and ("wait" in n or "notify" in n or "predicate" in n):
        return "verified_partial"
    if field == "resource_identity" and ("lock" in n or "mutex" in n or "channel" in n):
        return "verified_partial"
    return "evidence_missing"


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rows = parse_queue(QUEUE.read_text(encoding="utf-8"))
    audited = []
    for row in rows:
        fields = {name: field_status(row["owner_note"], name) for name in FIELDS}
        audited.append({
            **row,
            "fields": fields,
            "structural_equivalence": "not_claimed",
            "note": "owner bug_present=no is not a structural-equivalence audit",
        })
    payload = {
        "schema": ["rust_sha", "task", "arm", "rep", *FIELDS, "reviewer", "provenance", "verdict"],
        "n_rows": len(audited),
        "rows": audited,
        "machine_proposed": "not generated as owner verdicts; see per-cell reeval resources for tool mappings",
    }
    (OUT / "AUDIT.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"audit rows {len(audited)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
