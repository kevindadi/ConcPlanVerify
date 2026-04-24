#!/usr/bin/env python3
"""Reconstruct a partial RQ2 results JSON from the human-readable run log.

The run in rq2_run_new.log was interrupted before the final JSON was saved.
This helper parses that log, re-runs the cheap local CVN analysis for CVN
metrics, and writes an rq2 JSON that we can subsequently merge with fresh
re-runs of the interrupted cells.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "experiments"))

import run_experiment as rx  # noqa: E402


LOG_PATH = ROOT / "experiments/results/rq2_run_new.log"
OUT_PATH = ROOT / "experiments/results/rq2_partial_reconstructed.json"

PATTERN_RE = re.compile(
    r"^\s*\[(?P<pat>[a-z_]+)\] CVN: (?P<p>\d+)P/(?P<t>\d+)T, "
    r"(?P<s>\d+) states, (?P<tm>[\d.]+)ms, bug=(?P<bug>[A-Za-z_]+)\s*$"
)
MODEL_LINE_RE = re.compile(
    r"^\s*\[(?P<model>[a-z0-9\-.]+)\] repairing\.\.\.\s*"
    r"(?:fixed in (?P<rounds>\d+) rounds(?: \(\+(?P<regs>\d+) regressions\))?"
    r"|FAILED(?: \(\+(?P<fregs>\d+) regressions\))?)?"
)


def _parse_outcome(chunk: str) -> tuple[int, int, bool] | None:
    if "fixed in" in chunk:
        mm = re.search(r"fixed in (\d+) rounds(?: \(\+(\d+) regressions\))?",
                       chunk)
        if not mm:
            return None
        return int(mm.group(1)), int(mm.group(2) or 0), True
    if "FAILED" in chunk:
        mm = re.search(r"FAILED(?: \(\+(\d+) regressions\))?", chunk)
        return -1, int(mm.group(1) or 0) if mm else 0, False
    return None


def main() -> None:
    text = LOG_PATH.read_text()
    lines = text.splitlines()
    rows: list[dict] = []

    cur_pat: dict | None = None
    i = 0
    while i < len(lines):
        line = lines[i]
        m = PATTERN_RE.match(line)
        if m:
            cur_pat = {
                "pattern": m.group("pat"),
                "places": int(m.group("p")),
                "transitions": int(m.group("t")),
                "states": int(m.group("s")),
                "analysis_time_ms": float(m.group("tm")),
                "bug_detected": m.group("bug"),
            }
            i += 1
            continue

        if cur_pat is None:
            i += 1
            continue

        model_m = re.search(r"\[([a-z0-9\-.]+)\] repairing", line)
        if not model_m:
            i += 1
            continue

        model = model_m.group(1)
        # Accumulate until we hit an outcome or the next model line / next pattern
        chunk = line
        j = i + 1
        while j < len(lines):
            nxt = lines[j]
            if (re.search(r"\[([a-z0-9\-.]+)\] repairing", nxt)
                    or PATTERN_RE.match(nxt)):
                break
            chunk += "\n" + nxt
            if "fixed in" in nxt or "FAILED" in nxt:
                j += 1
                break
            j += 1

        outcome = _parse_outcome(chunk)
        if outcome is None:
            i = j
            continue

        rounds, regs, success = outcome
        rows.append({
            "model": model,
            **cur_pat,
            "repair_rounds": rounds,
            "regressions": regs,
            "success": success,
        })
        i = j

    # Deduplicate in case of weirdly wrapped lines
    seen = set()
    deduped = []
    for r in rows:
        key = (r["pattern"], r["model"])
        if key in seen:
            continue
        seen.add(key)
        deduped.append(r)

    OUT_PATH.write_text(json.dumps(deduped, indent=2))
    print(f"Reconstructed {len(deduped)} rows -> {OUT_PATH}")
    for r in deduped:
        status = "OK " if r["success"] else "FAIL"
        print(f"  {status} {r['pattern']:<20} {r['model']:<20} "
              f"rounds={r['repair_rounds']} regs={r['regressions']}")


if __name__ == "__main__":
    main()
