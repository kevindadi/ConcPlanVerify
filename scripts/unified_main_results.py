#!/usr/bin/env python3
"""Recompute the frozen G0–G3 tables from CELLS.jsonl. No model calls.

RF_all is the mean of requirement satisfaction over every planned cell.
A cell that was not accepted, or was accepted but not evaluable, contributes 0.
RF_acc is the mean over evaluable accepted cells only.

Paired intervals resample tasks, not cells. Seed 20260923 and 10000 draws match
the historical bootstrap. They are not the old published intervals.
"""

from __future__ import annotations

import csv
import hashlib
import json
import platform
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
EVIDENCE = REPO / "experiments/evidence-20260925/reeval"
ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir")
SEED = 20260923
ITERS = 10000


def _load(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _cell_rf_all(cell: dict) -> float:
    if cell.get("generation_accepted") and cell.get("evaluable"):
        return float(cell.get("rf") or 0.0)
    return 0.0


def _bootstrap(diffs: list[float]) -> dict:
    rng = random.Random(SEED)
    n = len(diffs)
    means = []
    for _ in range(ITERS):
        draw = [diffs[rng.randrange(n)] for _ in range(n)]
        means.append(sum(draw) / n)
    means.sort()
    lo = means[int(0.025 * ITERS)]
    hi = means[int(0.975 * ITERS) - 1]
    return {"mean": sum(diffs) / n, "ci95": [lo, hi], "n_tasks": n, "seed": SEED, "iters": ITERS}


def main() -> int:
    cells = _load(EVIDENCE / "CELLS.jsonl")
    keys = [(c["task"], c["arm"], c["rep"]) for c in cells]
    if len(keys) != len(set(keys)):
        print("duplicate task/arm/rep")
        return 2
    tasks = sorted({c["task"] for c in cells if c["arm"] in ARMS})
    reps = sorted({c["rep"] for c in cells if c["arm"] in ARMS})
    expected = {(task, arm, rep) for task in tasks for arm in ARMS for rep in reps}
    missing = expected - set(keys)
    if missing:
        print("missing", len(missing))
        return 2
    hash_mismatches = 0
    for cell in cells:
        raw_path = cell.get("rust_path")
        if not raw_path:
            continue
        path = Path(raw_path)
        if path.is_file() and _sha(path) != cell["rust_sha256"]:
            hash_mismatches += 1
    by_arm = {}
    for arm in ARMS:
        group = [c for c in cells if c["arm"] == arm]
        scored = [c for c in group if c.get("generation_accepted") and c.get("evaluable")]
        historical_unscored = [c for c in group if c.get("generation_accepted")
                               and c.get("historical_oracle_status") not in (None, "ok")]
        changed = [c for c in group if c.get("historical_oracle_status") == "ok"
                   and c.get("historical_rf") is not None
                   and abs(float(c["historical_rf"]) - float(c["rf"])) > 1e-9]
        by_arm[arm] = {
            "planned": len(group),
            "accepted": sum(bool(c.get("generation_accepted")) for c in group),
            "evaluable_accepted": len(scored),
            "rf_acc": sum(float(c["rf"]) for c in scored) / len(scored),
            "rf_all": sum(_cell_rf_all(c) for c in group) / len(group),
            "rc_mean_evaluable_accepted": sum(float(c["rc"]) for c in scored) / len(scored),
            "historical_accepted_not_ok": len(historical_unscored),
            "historical_ok_rf_changed": len(changed),
        }
    changed_cells = []
    for cell in cells:
        if cell["arm"] not in ARMS:
            continue
        if (cell.get("historical_oracle_status") == "ok" and cell.get("historical_rf") is not None
                and abs(float(cell["historical_rf"]) - float(cell["rf"])) > 1e-9):
            changed_cells.append({
                "task": cell["task"], "arm": cell["arm"], "rep": cell["rep"],
                "historical_rf": cell["historical_rf"], "rf": cell["rf"],
            })
    manifest = json.loads((REPO / "benchmarks/GENERATION_MANIFEST.json").read_text(encoding="utf-8"))
    tier_of = {task: tier for tier, names in manifest["tiers"].items() for task in names}
    tiers = {}
    for tier, names in manifest["tiers"].items():
        tiers[tier] = {}
        for arm in ARMS:
            group = [c for c in cells if c["arm"] == arm and c["task"] in names]
            scored = [c for c in group if c.get("generation_accepted") and c.get("evaluable")]
            tiers[tier][arm] = {
                "tasks": len(names),
                "planned": len(group),
                "accepted": sum(bool(c.get("generation_accepted")) for c in group),
                "evaluable_accepted": len(scored),
                "rf_all": sum(_cell_rf_all(c) for c in group) / len(group),
                "rf_acc": None if not scored else sum(float(c["rf"]) for c in scored) / len(scored),
                "rc_mean_evaluable_accepted": None if not scored else sum(float(c["rc"]) for c in scored) / len(scored),
            }
    families = defaultdict(lambda: {arm: [] for arm in ARMS})
    for cell in cells:
        if cell["arm"] not in ARMS:
            continue
        families[cell["task"].split("/")[0]][cell["arm"]].append(cell)
    family_rows = {}
    for family, arms in families.items():
        family_rows[family] = {}
        for arm, group in arms.items():
            scored = [c for c in group if c.get("generation_accepted") and c.get("evaluable")]
            family_rows[family][arm] = {
                "planned": len(group),
                "accepted": sum(bool(c.get("generation_accepted")) for c in group),
                "rf_all": sum(_cell_rf_all(c) for c in group) / len(group),
                "rf_acc": None if not scored else sum(float(c["rf"]) for c in scored) / len(scored),
            }
    # Task means: RF_all uses every rep, missing contribution 0.
    task_all = {arm: {} for arm in ARMS}
    task_acc = {arm: {} for arm in ARMS}
    for cell in cells:
        if cell["arm"] not in ARMS:
            continue
        task_all[cell["arm"]].setdefault(cell["task"], []).append(_cell_rf_all(cell))
        if cell.get("generation_accepted") and cell.get("evaluable"):
            task_acc[cell["arm"]].setdefault(cell["task"], []).append(float(cell["rf"]))
    paired = {}
    for left, right in (("G3_concir", "G0_direct"), ("G3_concir", "G1_self_iter"),
                        ("G3_concir", "G2_tools_iter")):
        diffs_all = []
        diffs_acc = []
        for task in tasks:
            a = sum(task_all[left][task]) / len(task_all[left][task])
            b = sum(task_all[right][task]) / len(task_all[right][task])
            diffs_all.append(a - b)
            if task in task_acc[left] and task in task_acc[right]:
                diffs_acc.append(
                    sum(task_acc[left][task]) / len(task_acc[left][task])
                    - sum(task_acc[right][task]) / len(task_acc[right][task]))
        paired[f"{left} minus {right}"] = {
            "rf_all": _bootstrap(diffs_all),
            "rf_acc_tasks_with_evaluable_accepted_on_both": _bootstrap(diffs_acc),
        }
    codegen = _load(EVIDENCE / "CODEGEN_ORACLE.jsonl")
    built = [c for c in codegen if c.get("built") and c.get("generation_accepted")]
    out = {
        "definitions": {
            "accepted": "generation_accepted is true. It is not requirement satisfaction.",
            "rf_all": "mean rf over all planned cells; not accepted or not evaluable contributes 0",
            "rf_acc": "mean rf over evaluable accepted cells",
            "rc": "requirement coverage on the recorded ri map; reported only for evaluable accepted cells",
            "paired": "bootstrap resamples the 24 tasks, seed 20260923, 10000 draws; not a reuse of published intervals",
            "translator": "G3_codegen uses 8 runs and codegen_project_not_reinstrumented; main arms are a different collection path. It is not in the main table.",
        },
        "integrity": {
            "cells": len(cells),
            "unique_task_arm_rep": len(set(keys)) == len(keys),
            "tasks": len(tasks),
            "reps": reps,
            "missing_cells": len(missing),
            "rust_sha256_mismatches": hash_mismatches,
            "input_sha256": {name: _sha(EVIDENCE / name) for name in
                             ("CELLS.jsonl", "INPUT_MANIFEST.jsonl", "CODEGEN_ORACLE.jsonl", "RF_SUMMARY.json", "TOOLS.json")},
        },
        "main_arms": by_arm,
        "historical_ok_rf_changed_cells": changed_cells,
        "tiers": tiers,
        "tier_task_count": {tier: len(names) for tier, names in manifest["tiers"].items()},
        "unassigned_tier_tasks": sorted({c["task"] for c in cells if c["arm"] in ARMS} - set(tier_of)),
        "families": family_rows,
        "paired_task_bootstrap": paired,
        "translator": {
            "rows": len(codegen),
            "built_accepted": len(built),
            "not_built": sum(1 for c in codegen if not c.get("built")),
            "n_runs": sorted({c.get("n_runs") for c in codegen if c.get("built")}),
            "oracle": "codegen_project_not_reinstrumented",
            "rf_acc_built": None if not built else sum(float(c["rf"]) for c in built) / len(built),
            "not_pooled_with_main_arms": True,
        },
        "runtime": {"python": sys.version.split()[0], "platform": platform.platform()},
    }
    dest = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v20")
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "UNIFIED_MAIN_RESULTS.json").write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    with (dest / "UNIFIED_MAIN_RESULTS.csv").open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=["arm", "planned", "accepted", "evaluable_accepted",
                                                    "rf_acc", "rf_all", "rc_mean_evaluable_accepted",
                                                    "historical_accepted_not_ok", "historical_ok_rf_changed"])
        writer.writeheader()
        for arm, row in by_arm.items():
            writer.writerow({"arm": arm, **row})
    print(json.dumps({arm: {"rf_acc": round(v["rf_acc"], 3), "rf_all": round(v["rf_all"], 3),
                            "accepted": v["accepted"]} for arm, v in by_arm.items()}, indent=2))
    print("hash_mismatches", hash_mismatches)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
