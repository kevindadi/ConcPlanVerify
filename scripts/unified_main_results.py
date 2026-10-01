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


MAIN_ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir")
TRANSLATOR_ARM = "G3_codegen"
PROTOCOL_REPS = (0, 1, 2)


def _in_unit(value) -> bool:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return False
    return 0.0 <= number <= 1.0


def integrity_errors(input_dir: Path) -> tuple[list[str], dict]:
    """Expected cells come from the benchmark protocol and the frozen manifest.

    A missing task, a missing repeat, a declared source that is absent, or a
    hash conflict is an error. A null path on a cell that was not accepted is
    a legal missing artifact.
    """

    errors: list[str] = []
    info: dict = {}
    required = ("CELLS.jsonl", "INPUT_MANIFEST.jsonl", "CODEGEN_ORACLE.jsonl",
                "RF_SUMMARY.json", "TOOLS.json")
    for name in required:
        if not (input_dir / name).is_file():
            errors.append(f"missing input file {name}")
    if errors:
        return errors, info
    cells = _load(input_dir / "CELLS.jsonl")
    manifest_rows = _load(input_dir / "INPUT_MANIFEST.jsonl")
    protocol = json.loads((REPO / "benchmarks/GENERATION_MANIFEST.json").read_text(encoding="utf-8"))
    tasks = [item["task"] for item in protocol["tasks"]]
    info["protocol_tasks"] = len(tasks)
    info["protocol_manifest_sha256"] = _sha(REPO / "benchmarks/GENERATION_MANIFEST.json")
    if len(tasks) != 24 or len(set(tasks)) != 24:
        errors.append(f"benchmark protocol lists {len(set(tasks))} tasks; the main matrix requires 24")
    expected_main = {(task, arm, rep) for task in tasks for arm in MAIN_ARMS for rep in PROTOCOL_REPS}
    expected_translator = {(task, TRANSLATOR_ARM, 0) for task in tasks}
    if len(expected_main) != 288:
        errors.append(f"main protocol matrix has {len(expected_main)} cells; expected 288")
    manifest_main = [(row.get("task"), row.get("arm"), row.get("rep"))
                     for row in manifest_rows if row.get("arm") in MAIN_ARMS]
    manifest_translator = [(row.get("task"), row.get("arm"), row.get("rep"))
                           for row in manifest_rows if row.get("arm") == TRANSLATOR_ARM]
    cell_keys = [(cell.get("task"), cell.get("arm"), cell.get("rep")) for cell in cells]
    if len(cell_keys) != len(set(cell_keys)):
        errors.append("duplicate task/arm/rep in CELLS")
    if len(manifest_main) != len(set(manifest_main)):
        errors.append("duplicate task/arm/rep in INPUT_MANIFEST main arms")
    missing_main = expected_main - set(cell_keys)
    extra_main = {key for key in cell_keys if key[1] in MAIN_ARMS} - expected_main
    if missing_main:
        missing_tasks = sorted({key[0] for key in missing_main})
        errors.append(
            f"CELLS is missing {len(missing_main)} main-arm cells across tasks {missing_tasks}")
    if extra_main:
        errors.append(f"CELLS has {len(extra_main)} main-arm cells outside the protocol matrix")
    if set(manifest_main) != expected_main:
        errors.append("INPUT_MANIFEST main-arm matrix does not match the 24x4x3 protocol")
    if set(manifest_translator) != expected_translator:
        errors.append("INPUT_MANIFEST translator matrix is not the 24 tasks at rep 0")
    translator_cells = {key for key in cell_keys if key[1] == TRANSLATOR_ARM}
    if translator_cells != expected_translator:
        errors.append("CELLS translator rows are not the 24 planned G3_codegen rep-0 cells")
    manifest_by = {(row.get("task"), row.get("arm"), row.get("rep")): row for row in manifest_rows}
    for cell in cells:
        key = (cell.get("task"), cell.get("arm"), cell.get("rep"))
        if cell.get("arm") in MAIN_ARMS and cell.get("rep") not in PROTOCOL_REPS:
            errors.append(f"rep out of range at {key}")
        for field in ("rf", "rc"):
            if cell.get(field) is not None and not _in_unit(cell.get(field)):
                errors.append(f"{field} out of range at {key}")
        if cell.get("historical_rf") is not None and not _in_unit(cell.get("historical_rf")):
            errors.append(f"historical_rf out of range at {key}")
        other = manifest_by.get(key)
        if other is None:
            errors.append(f"CELLS key {key} is not in INPUT_MANIFEST")
            continue
        if bool(other.get("generation_accepted")) != bool(cell.get("generation_accepted")):
            errors.append(f"generation_accepted conflicts between CELLS and INPUT_MANIFEST at {key}")
        rust_path = cell.get("rust_path")
        if rust_path:
            path = Path(rust_path)
            if not path.is_file():
                errors.append(f"declared source is missing at {key}: {rust_path}")
            elif cell.get("rust_sha256") and _sha(path) != cell["rust_sha256"]:
                errors.append(f"source hash conflict at {key}")
        elif cell.get("generation_accepted"):
            errors.append(f"accepted cell has no source artifact at {key}")
        for path_field, hash_field in (("contract_path", "contract_sha256"), ("cir_path", "cir_sha256")):
            raw = other.get(path_field) or cell.get(path_field)
            digest = other.get(hash_field) or cell.get(hash_field)
            if not raw:
                continue
            artifact = Path(raw)
            if not artifact.is_file():
                errors.append(f"declared {path_field} is missing at {key}: {raw}")
            elif digest and _sha(artifact) != digest:
                errors.append(f"{hash_field} conflict at {key}")
    for item in protocol["tasks"]:
        req = REPO / "benchmarks/families" / item["task"] / "generation_input/REQUIREMENTS.md"
        recorded = item.get("requirements_sha256")
        if not req.is_file():
            errors.append(f"requirements file missing for {item['task']}")
        elif recorded and _sha(req) != recorded:
            errors.append(f"requirements hash conflict for {item['task']}")
        contract = REPO / "benchmarks/families" / item["task"] / "contract.json"
        recorded_contract = item.get("contract_sha256")
        if not contract.is_file():
            errors.append(f"contract file missing for {item['task']}")
        elif recorded_contract and _sha(contract) != recorded_contract:
            errors.append(f"benchmark contract hash conflict for {item['task']}")
    tools = json.loads((input_dir / "TOOLS.json").read_text(encoding="utf-8"))
    frozen_backend = tools.get("backend_sha256")
    frozen_instrument = tools.get("instrument_sha256")
    current_backend = None
    current_instrument = None
    backend_path = Path(str(tools.get("backend") or ""))
    instrument_path = Path(str(tools.get("instrument") or ""))
    if backend_path.is_file():
        current_backend = _sha(backend_path)
    if instrument_path.is_file():
        current_instrument = _sha(instrument_path)
    info["tool_identity"] = {
        "frozen_backend_sha256": frozen_backend,
        "frozen_instrument_sha256": frozen_instrument,
        "current_binary_matches_frozen_backend": current_backend == frozen_backend,
        "current_binary_matches_frozen_instrument": current_instrument == frozen_instrument,
        "status": "frozen_record",
        "note": "A rebuilt binary is not treated as the historical tool identity.",
    }
    codegen = _load(input_dir / "CODEGEN_ORACLE.jsonl")
    oracle_tasks = {row.get("task") for row in codegen}
    absent = [task for task in tasks if task not in oracle_tasks]
    absent_rows = []
    for task in absent:
        match = next((cell for cell in cells
                      if cell.get("task") == task and cell.get("arm") == TRANSLATOR_ARM and cell.get("rep") == 0), None)
        if match is None:
            errors.append(f"translator task {task} is absent from CODEGEN_ORACLE and from CELLS")
            continue
        if match.get("generation_accepted") or match.get("failure_class") != "no_artifact":
            errors.append(
                f"translator task {task} is absent from CODEGEN_ORACLE but is not an unaccepted no_artifact cell")
        absent_rows.append({"task": task, "generation_accepted": match.get("generation_accepted"),
                            "failure_class": match.get("failure_class")})
    info["translator_oracle_rows"] = len(codegen)
    info["translator_absent_unaccepted"] = absent_rows
    info["main_cells_expected"] = 288
    info["translator_cells_expected"] = 24
    return errors, info


def main(argv: list[str] | None = None) -> int:
    import argparse
    parser = argparse.ArgumentParser(description="Recompute frozen G0-G3 tables. No model calls.")
    parser.add_argument("--input", type=Path, default=EVIDENCE)
    parser.add_argument("--output", type=Path, default=None)
    args = parser.parse_args(argv)
    source = args.input
    errors, integrity_info = integrity_errors(source)
    if errors:
        diagnostic = {"ok": False, "errors": errors, "info": integrity_info}
        print(json.dumps(diagnostic, indent=2))
        if args.output is not None:
            args.output.mkdir(parents=True, exist_ok=True)
            (args.output / "INTEGRITY_DIAGNOSTIC.json").write_text(
                json.dumps(diagnostic, indent=2) + "\n", encoding="utf-8")
            success = args.output / "UNIFIED_MAIN_RESULTS.json"
            if success.is_file():
                success.unlink()
        return 2
    cells = _load(source / "CELLS.jsonl")
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
    comparable_changes = []
    for cell in cells:
        if cell["arm"] not in ARMS:
            continue
        historical = cell.get("historical_rf")
        current = cell.get("rf")
        changed = (historical is not None and current is not None
                   and abs(float(historical) - float(current)) > 1e-9)
        if (cell.get("historical_oracle_status") == "ok" and changed):
            changed_cells.append({
                "task": cell["task"], "arm": cell["arm"], "rep": cell["rep"],
                "generation_accepted": bool(cell.get("generation_accepted")),
                "historical_rf": historical, "rf": current,
            })
        if changed:
            comparable_changes.append({
                "task": cell["task"], "arm": cell["arm"], "rep": cell["rep"],
                "generation_accepted": bool(cell.get("generation_accepted")),
                "historical_oracle_status": cell.get("historical_oracle_status"),
                "historical_rf": historical, "rf": current,
            })
    accepted_changes = [row for row in comparable_changes if row["generation_accepted"]]
    accepted_by_arm = {arm: sum(1 for row in accepted_changes if row["arm"] == arm) for arm in ARMS}
    score_changes = {
        "historical_status_ok": {
            "cells": len(changed_cells),
            "accepted": sum(1 for row in changed_cells if row["generation_accepted"]),
            "note": "Requires historical_oracle_status ok. G3 rows have null status, so this filter excludes them.",
        },
        "historical_rf_present": {
            "cells": len(comparable_changes),
            "accepted": len(accepted_changes),
            "unaccepted": len(comparable_changes) - len(accepted_changes),
            "accepted_by_arm": accepted_by_arm,
            "cells_detail": comparable_changes,
        },
        "build_failed_now_evaluable_accepted": sum(
            1 for cell in cells if cell["arm"] in ARMS and cell.get("generation_accepted")
            and cell.get("historical_oracle_status") == "build_failed"),
    }
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
    codegen = _load(source / "CODEGEN_ORACLE.jsonl")
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
            "input_sha256": {name: _sha(source / name) for name in
                             ("CELLS.jsonl", "INPUT_MANIFEST.jsonl", "CODEGEN_ORACLE.jsonl", "RF_SUMMARY.json", "TOOLS.json")},
            **integrity_info,
        },
        "main_arms": by_arm,
        "score_changes": score_changes,
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
    if args.output is None:
        print(json.dumps({arm: {"rf_acc": round(v["rf_acc"], 3), "rf_all": round(v["rf_all"], 3),
                                "accepted": v["accepted"]} for arm, v in by_arm.items()}, indent=2))
        print("score_changes", json.dumps({
            "ok_filter": score_changes["historical_status_ok"]["cells"],
            "comparable": score_changes["historical_rf_present"]["cells"],
            "accepted_comparable": score_changes["historical_rf_present"]["accepted"],
            "accepted_by_arm": score_changes["historical_rf_present"]["accepted_by_arm"],
        }))
        return 0
    dest = args.output
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
