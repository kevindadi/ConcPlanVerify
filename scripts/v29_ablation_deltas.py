#!/usr/bin/env python3
"""Paired, task-clustered bootstrap for the v29 ablation configs.

For each model, the per-task value is the independent requirement coverage of
the delivered candidate (RF_delivered_all contribution: rf if accepted, else 0).
Deltas are paired by task; the 95% CI resamples tasks (clusters), so cells of
one task stay together. Missing cells (external transport/budget) count as 0,
matching the pre-registered RF_delivered_all denominator.
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
CONFIGS = ["full", "without_explore", "without_goal_gate", "without_diagnostics"]
MODELS = ["gpt-6-luna", "glm-5.3-flash"]


def load(name: str) -> dict:
    d = json.loads((REPO / f"experiments/strong-link-v29-ablation/{name}/scored/"
                    "PHASE3_RESULTS.json").read_text(encoding="utf-8"))
    out = {}
    for r in d["records"]:
        rf = r.get("rf")
        value = float(rf) if (r.get("accepted") is True and rf is not None) else 0.0
        out[(r["model"], r["task"])] = value
    return out


def boot_delta(a: list[float], b: list[float], iters: int, seed: int = 7) -> dict:
    rng = random.Random(seed)
    n = len(a)
    diffs = [b[i] - a[i] for i in range(n)]
    mean = sum(diffs) / n
    samples = []
    for _ in range(iters):
        idx = [rng.randrange(n) for _ in range(n)]
        samples.append(sum(diffs[i] for i in idx) / n)
    samples.sort()
    lo = samples[int(0.025 * iters)]
    hi = samples[int(0.975 * iters)]
    same_sign = (lo > 0) or (hi < 0)
    return {"n_tasks": n, "mean_delta": round(mean, 4),
            "ci95": [round(lo, 4), round(hi, 4)], "excludes_zero": same_sign}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--iters", type=int, default=20000)
    ap.add_argument("--baseline", default="full")
    args = ap.parse_args()
    data = {c: load(c) for c in CONFIGS}
    result = {"baseline": args.baseline, "metric": "RF_delivered_all (per-task, clustered)",
              "comparisons": {}}
    for model in MODELS:
        for cfg in CONFIGS:
            if cfg == args.baseline:
                continue
            keys = sorted(k for k in data[args.baseline] if k[0] == model)
            a = [data[args.baseline][k] for k in keys]
            b = [data[cfg].get(k, 0.0) for k in keys]
            result["comparisons"][f"{model}/{cfg}_vs_{args.baseline}"] = boot_delta(
                a, b, args.iters)
    out = REPO / "experiments/strong-link-v29-ablation/ABLATION_DELTAS.json"
    out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
