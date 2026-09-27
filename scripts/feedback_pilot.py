#!/usr/bin/env python3
"""Feedback-ablation pilot: frozen config + request budget + dry-run.

``--dry-run`` computes the request upper bound and lists the matrix WITHOUT
calling any model. The real run (not performed here) would start both arms from
the same initial candidate and differ only in the feedback content.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", required=True)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--out", default=None)
    args = ap.parse_args()
    cfg = json.loads(Path(args.config).read_text())

    cells = (len(cfg["models"]) * len(cfg["tasks"]) * len(cfg["arms"]) * cfg["reps"])
    # Upper bound: initial + (k_code - 1) repair attempts per cell.
    calls_upper = cells * cfg["k_code"]
    plan = {
        "config_version": cfg["version"],
        "question": cfg["question"],
        "matrix": {"models": cfg["models"], "tasks": cfg["tasks"], "arms": cfg["arms"],
                   "reps": cfg["reps"], "k_code": cfg["k_code"]},
        "cells": cells,
        "requests_upper_bound": calls_upper,
        "unknown_token_policy": "record null, never 0",
        "arms_differ_only_in": cfg["only_difference"],
        "task_set": cfg["task_set_note"],
        "dry_run": bool(args.dry_run),
    }
    if args.out:
        Path(args.out).write_text(json.dumps(plan, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(plan, ensure_ascii=False, indent=2))
    if args.dry_run:
        print(f"DRY RUN: no model calls; {cells} cells, <= {calls_upper} requests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
