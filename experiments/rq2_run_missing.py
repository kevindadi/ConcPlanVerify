#!/usr/bin/env python3
"""Run only the RQ2 (model, pattern) combinations not present in a prior
partial result file.

Key behaviours:
  * Iterates by **model first** so we can easily quarantine a slow/broken
    model (run one model at a time via --model).
  * Each round prints its own timing ("r1 llm=3200ms") so we can tell
    which network/API is slow without waiting for an entire pattern to
    complete.
  * Writes results incrementally into an intermediate file after every
    completed combination — so a kill/interrupt never loses finished
    rows.
  * Final output is a single timestamped rq2_<stamp>.json with the merged
    prior partial + newly-completed rows.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import asdict
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "experiments"))

import run_experiment as re  # noqa: E402


def _load_partial(path: Path) -> list[dict]:
    if not path.exists():
        return []
    return json.loads(path.read_text())


def _save_incremental(path: Path, rows: list[dict]) -> None:
    path.write_text(json.dumps(rows, indent=2))


def _run_one(
    config, model, pattern: str, buggy_json: str, analysis: dict,
    goal_trigger: bool, unmet_initial,
) -> dict:
    print(f"    [{model.name}/{pattern}] start", flush=True)
    t0 = time.perf_counter()
    row = re._run_rq2_repair(
        config, model, pattern, buggy_json, analysis,
        initial_unmet_goals=unmet_initial if goal_trigger else None,
        synthetic_bug_kind="GoalUnreachable" if goal_trigger else None,
    )
    elapsed = time.perf_counter() - t0
    status = (
        f"fixed in {row.repair_rounds} rounds"
        if row.success else "FAILED"
    )
    reg = f" (+{row.regressions} regressions)" if row.regressions else ""
    print(
        f"    [{model.name}/{pattern}] -> {status}{reg} "
        f"(wall={elapsed:.1f}s)",
        flush=True,
    )
    return asdict(row)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--partial",
        default="experiments/results/rq2_partial_reconstructed.json",
        help="existing RQ2 JSON whose rows should be kept as-is",
    )
    ap.add_argument(
        "--model",
        default=None,
        help="only run this model (name from config.toml)",
    )
    ap.add_argument(
        "--intermediate",
        default="experiments/results/rq2_intermediate.json",
        help=(
            "file to incrementally write finished rows to; survives "
            "process kills"
        ),
    )
    ap.add_argument(
        "--out-stem",
        default=None,
        help="override final-output stem (default: rq2_<timestamp>)",
    )
    args = ap.parse_args()

    cfg_path = ROOT / "experiments" / "config.toml"
    config = re.load_config(cfg_path)

    partial_path = ROOT / args.partial
    existing = _load_partial(partial_path)

    # Load prior intermediate if present so a previous invocation's finished
    # rows are preserved.
    inter_path = ROOT / args.intermediate
    inter_rows = _load_partial(inter_path)
    have = {(r["model"], r["pattern"]) for r in (existing + inter_rows)}

    all_models = [m for m in config.models]
    if args.model:
        all_models = [m for m in all_models if m.name == args.model]
        if not all_models:
            sys.exit(f"unknown model: {args.model}")

    patterns = list(config.buggy_cirs.keys())

    todo: list[tuple[str, str]] = [
        (m.name, p) for m in all_models for p in patterns
        if (m.name, p) not in have
    ]
    if not todo:
        print("Nothing to run.", flush=True)
    else:
        print(
            f"Running {len(todo)} missing combinations "
            f"(model filter: {args.model or 'all'}):",
            flush=True,
        )
        for m, p in todo:
            print(f"  - {m} / {p}", flush=True)

    # Pre-compute per-pattern analysis so we don't re-run cir2cvn per model.
    pattern_cache: dict[str, dict] = {}
    for pattern in patterns:
        if not any(p == pattern for _, p in todo):
            continue
        cir_path = ROOT / config.buggy_cirs[pattern]
        if not cir_path.exists():
            print(
                f"  [{pattern}] SKIP - buggy CIR not found: {cir_path}",
                flush=True,
            )
            continue
        buggy_json = cir_path.read_text()
        analysis = re.translate_and_analyze_with_timing(buggy_json)
        if "error" in analysis:
            print(
                f"  [{pattern}] analysis error: {analysis['error']}",
                flush=True,
            )
            continue
        bugs = analysis.get("bugs", [])
        goal_report = re.check_goal_reachability(buggy_json)
        unmet_initial = (
            goal_report.get("unmet", [])
            if isinstance(goal_report, dict) else []
        )
        goal_trigger = (not bugs) and bool(unmet_initial)
        bug_kind = (
            bugs[0]["kind"] if bugs
            else ("GoalUnreachable" if goal_trigger else "none")
        )
        print(
            f"\n  [{pattern}] CVN: {analysis.get('places',0)}P/"
            f"{analysis.get('transitions',0)}T, "
            f"{analysis.get('states',0)} states, bug={bug_kind}",
            flush=True,
        )
        pattern_cache[pattern] = {
            "buggy_json": buggy_json,
            "analysis": analysis,
            "bugs": bugs,
            "goal_trigger": goal_trigger,
            "unmet_initial": unmet_initial,
        }

    # Iterate by model first so a slow model can be quarantined; within a
    # model, run each pattern sequentially.
    for model in all_models:
        print(
            f"\n=== model: {model.name} "
            f"(base={model.base_url}) ===",
            flush=True,
        )
        for pattern in patterns:
            if (model.name, pattern) not in todo:
                continue
            if pattern not in pattern_cache:
                continue
            entry = pattern_cache[pattern]
            if not entry["bugs"] and not entry["goal_trigger"]:
                row = asdict(re.RQ2Result(
                    model=model.name, pattern=pattern,
                    places=entry["analysis"].get("places", 0),
                    transitions=entry["analysis"].get("transitions", 0),
                    states=entry["analysis"].get("states", 0),
                    analysis_time_ms=entry["analysis"].get(
                        "analysis_time_ms", 0
                    ),
                    bug_detected="none", repair_rounds=0,
                    regressions=0, success=True,
                ))
                print(
                    f"    [{model.name}/{pattern}] -> no bugs (auto-pass)",
                    flush=True,
                )
            else:
                row = _run_one(
                    config, model, pattern,
                    entry["buggy_json"], entry["analysis"],
                    entry["goal_trigger"], entry["unmet_initial"],
                )
            inter_rows.append(row)
            _save_incremental(inter_path, inter_rows)

    merged = existing + inter_rows
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    stem = args.out_stem or f"rq2_{ts}"
    out_dir = ROOT / "experiments" / "results"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_json = out_dir / f"{stem}.json"
    out_json.write_text(json.dumps(merged, indent=2))
    print(
        f"\nWrote {out_json} ({len(merged)} total rows, "
        f"{len(inter_rows)} new this session).",
        flush=True,
    )


if __name__ == "__main__":
    main()
