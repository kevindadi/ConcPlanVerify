#!/usr/bin/env python3
"""strong-link-v23 phase 2: score the completed thinking batch with the shared
independent requirement oracle, and reconcile tokens/requests.

Reuses the phase-1 candidate selection and oracle (same contract, same
instrumentation, 32 native runs, no Miri). Offline: no model call.
"""

from __future__ import annotations

import argparse
import collections
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO / "scripts"))

import v23_phase1_eval as p1  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402

BATCH = REPO / "experiments/strong-link-v23-thinking-2"
OUT = REPO / "experiments/strong-link-v23-phase2"
MODELS = ["qwen3.8-flash", "gpt-6-luna", "glm-5.3-flash"]
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]


def cells() -> list[dict]:
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    out = []
    for c in summary["cells"]:
        cell = c["cell"]
        cdir = (BATCH / cell["task"].replace("/", "__") / cell["model_id"]
                / cell["arm"] / f"rep{cell['rep']}")
        out.append({"source": "v22",  # reuse the v22-layout candidate resolver
                    "model_id": cell["model_id"], "task": cell["task"],
                    "arm": cell["arm"], "rep": cell["rep"], "cell_dir": cdir,
                    "accepted": bool(c.get("accepted")),
                    "record_status": c.get("record_status"),
                    "cached": c.get("cached"), "calls": c.get("calls"),
                    "run_status": c["status"]})
    return out


def tokens() -> dict:
    out = {}
    for model in MODELS:
        p = BATCH / "transport" / model / "requests.jsonl"
        if not p.is_file():
            continue
        rows = [json.loads(l) for l in p.read_text().splitlines() if l.strip()]
        ok = [r for r in rows if r.get("status") == "ok"]
        usage = [r.get("usage") or {} for r in ok]
        def g(u, *keys):
            for k in keys:
                if isinstance(u, dict) and u.get(k) is not None:
                    return int(u[k])
            return None
        inp = [g(u, "input_tokens", "prompt_tokens") for u in usage]
        outp = [g(u, "output_tokens", "completion_tokens") for u in usage]
        reason = [g(u, "reasoning_tokens") for u in usage]
        reason = [g((u.get("completion_tokens_details") or u.get("output_tokens_details") or {}),
                    "reasoning_tokens") if r is None else r
                  for u, r in zip(usage, reason)]
        walls = [r.get("wall_ms") for r in ok]
        out[model] = {
            "attempts": len(rows), "ok": len(ok),
            "errors": collections.Counter(r.get("error_type") for r in rows
                                          if r.get("status") != "ok"),
            "input_tokens_sum": sum(x for x in inp if x is not None),
            "output_tokens_sum": sum(x for x in outp if x is not None),
            "reasoning_tokens_sum": sum(x for x in reason if x is not None),
            "input_missing": sum(1 for x in inp if x is None),
            "output_missing": sum(1 for x in outp if x is None),
            "reasoning_missing": sum(1 for x in reason if x is None),
            "wall_ms_sum": sum(x for x in walls if x is not None),
        }
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(OUT))
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    args = ap.parse_args()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=True)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    cache = p1.EvalCache(out, Path(args.binary), Path(args.instrument), tasks)
    rows = [p1.evaluate_cell(c, cache) for c in cells()]
    with (out / "PER_CELL.jsonl").open("w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")
    agg = p1.aggregate(rows, MODELS, "v22")
    report = {"model_arm": agg, "tokens": tokens(), "cells": len(rows)}
    (out / "PHASE2_RESULTS.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"cells": len(rows), "out": str(out)}, indent=2))
    for m in MODELS:
        for a in ARMS:
            x = agg.get(m, {}).get(a)
            if x:
                print(f"{m:16} {a:14} exec {x['executed']:2} acc {x['accepted']} "
                      f"eval {x['delivered_evaluable']} RC {x['rc_all']} RF_all {x['rf_all']} RF_acc {x['rf_acc']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
