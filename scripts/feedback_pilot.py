#!/usr/bin/env python3
"""Freeze and dry-run the counterexample-feedback ablation. No model calls."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def _sha(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def freeze_inputs(cfg: dict, manifest_path: Path) -> list[dict]:
    """Pin round-1 sources. Missing round-1 stays listed and is not replaced."""

    dev = [json.loads(line) for line in manifest_path.read_text(encoding="utf-8").splitlines()
           if line.strip()]
    rows = []
    for cell in dev:
        task = cell["task"]
        if cfg["sampling_rule"]["exclude_substring"] in task:
            continue
        if task not in cfg["tasks"]:
            continue
        source = (REPO / cell["batch"] / cell["model"] / task.replace("/", "__")
                  / f"rep{cell['rep']}" / "code" / "round-1.rs")
        rows.append({
            "model": cell["model"],
            "task": task,
            "rep": cell["rep"],
            "origin_batch": cell["batch"],
            "initial_round": 1,
            "initial_source": str(source) if source.is_file() else None,
            "initial_sha256": _sha(source) if source.is_file() else None,
            "available": source.is_file(),
            "unavailable_reason": None if source.is_file() else "round-1 source missing; not replaced",
            "task_set": "development",
            "holdout": False,
        })
    return rows


def dry_run(cfg: dict, inputs: list[dict]) -> dict:
    arms = cfg["arms"]
    k_code = cfg["k_code"]
    cells = []
    for item in inputs:
        for arm in arms:
            for rep in range(cfg["reps"]):
                runnable = bool(item["available"])
                cells.append({
                    "model": item["model"], "task": item["task"], "arm": arm,
                    "rep": rep, "initial_sha256": item["initial_sha256"],
                    "initial_source": item["initial_source"],
                    "runnable": runnable,
                    "k_code": k_code,
                    "request_upper_bound": k_code if runnable else 0,
                    "prompt_tokens": None, "completion_tokens": None,
                    "tool_time_s": None, "model_time_s": None,
                    "tool_errors": None, "capability_gaps": None,
                    "first_correct_round": None,
                })
    runnable = [c for c in cells if c["runnable"]]
    return {
        "config_version": cfg["version"],
        "question": cfg["question"],
        "dry_run": True,
        "llm_calls": 0,
        "task_set": cfg["task_set_note"],
        "holdout": False,
        "arms_differ_only_in": cfg["only_difference"],
        "controlled": cfg["controlled"],
        "support_subset": cfg["supported_subset"],
        "sampling_rule": cfg["sampling_rule"],
        "truth": cfg["truth"],
        "metrics": cfg["metrics"],
        "planned_model_tasks": len(inputs),
        "unavailable_model_tasks": sum(1 for i in inputs if not i["available"]),
        "cells": len(cells),
        "runnable_cells": len(runnable),
        "requests_upper_bound": sum(c["request_upper_bound"] for c in cells),
        "unknown_token_policy": "record null, never 0",
        "future_G2_G3": cfg["future_G2_G3_note"],
        "matrix_cells": cells,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    parser.add_argument("--inputs", required=True)
    parser.add_argument("--dev-manifest",
                        default="notes-path-unused")
    parser.add_argument("--freeze-inputs", action="store_true")
    parser.add_argument("--dev-manifest-file", default=str(
        Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v5/INPUT_MANIFEST.jsonl")))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--out", default=None)
    args = parser.parse_args()
    cfg = json.loads(Path(args.config).read_text(encoding="utf-8"))
    inputs_path = Path(args.inputs)
    if args.freeze_inputs:
        inputs = freeze_inputs(cfg, Path(args.dev_manifest_file))
        inputs_path.parent.mkdir(parents=True, exist_ok=True)
        inputs_path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in inputs),
                               encoding="utf-8")
    else:
        inputs = [json.loads(line) for line in inputs_path.read_text(encoding="utf-8").splitlines()
                  if line.strip()]
    if not args.dry_run and not args.freeze_inputs:
        print("refusing to call a model; pass --dry-run or --freeze-inputs")
        return 2
    plan = dry_run(cfg, inputs)
    text = json.dumps({k: v for k, v in plan.items() if k != "matrix_cells"},
                      ensure_ascii=False, indent=2)
    print(text)
    print(f"DRY RUN: llm_calls=0 runnable_cells={plan['runnable_cells']} "
          f"requests_upper_bound={plan['requests_upper_bound']}")
    if args.out:
        Path(args.out).write_text(json.dumps(plan, ensure_ascii=False, indent=2) + "\n",
                                  encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
