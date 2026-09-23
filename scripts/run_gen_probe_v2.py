#!/usr/bin/env python3
"""§4 gen-model-probe-v2: kimi-k3, arms G3_concir (v2) and G0_direct.

G3 is prioritised (all tasks first), then G0 with the remaining budget
(<= 150 OpenCode Go requests); unfinished G0 cells are `not_run`.
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    load_gen_tasks, run_g3_v2, run_rust_generation, rust_arm_oracle,
)
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.opencode_go import OpenCodeGoClient, list_models  # noqa: E402

ARMS = ("G3_concir", "G0_direct")  # priority order


def resolve_model(api_key: str) -> str:
    try:
        ids = [m.get("id", "") for m in list_models(api_key).get("data", [])]
    except Exception:  # noqa: BLE001
        ids = []
    for name in ("kimi-k3", "kimi-k2.7-code"):
        if name in ids:
            return name
    return next((m for m in ids if "kimi" in m), "kimi-k2.7-code")


def _mean(values):
    values = [v for v in values if v is not None]
    return round(statistics.mean(values), 3) if values else None


def _cell(record: dict) -> dict:
    pt = ct = 0
    for rnd in record.get("rounds", []):
        pt += int(rnd.get("prompt_tokens") or 0)
        ct += int(rnd.get("completion_tokens") or 0)
    cov = record.get("coverage") or {}
    return {"accepted": record.get("accepted"),
            "accepted_with_proof": record.get("accepted_with_proof"),
            "rc": cov.get("rc"), "rf": cov.get("rf"),
            "prompt_tokens": pt, "completion_tokens": ct,
            "rounds": len(record.get("rounds", [])), "record": record}


def render(model, cells, temperature) -> str:
    lines = ["# gen-model-probe-v2 — frontier generation under v3.1", "",
             f"Model `{model}`, 1 rep, G3 v2 (K_cir=4, K_code=3) then G0. "
             f"temperature {temperature}.", "",
             "| arm | cells | not_run | accepted | accept rate | RC | RF_all | RF_acc | awp | tokens |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in ARMS:
        cs = [c for c in cells if c.get("arm") == arm and c.get("status") != "not_run"]
        acc = [c for c in cs if c.get("accepted")]
        lines.append(f"| {arm} | {len(cs)} | {sum(1 for c in cells if c.get('arm')==arm and c.get('status')=='not_run')} | "
                     f"{len(acc)} | {round(len(acc)/len(cs),3) if cs else None} | "
                     f"{_mean([c.get('rc') for c in cs])} | {_mean([c.get('rf') for c in cs])} | "
                     f"{_mean([c.get('rf') for c in acc])} | "
                     f"{sum(1 for c in cs if c.get('accepted_with_proof'))} | "
                     f"{sum((c.get('prompt_tokens') or 0)+(c.get('completion_tokens') or 0) for c in cs)} |")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=150)
    parser.add_argument("--out", default="experiments/gen-model-probe-v2")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "concir/target/release/concir-backend"))
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    api_key = os.environ.get("OPENCODE_API_KEY", "")
    if not api_key:
        print("OPENCODE_API_KEY missing", file=sys.stderr)
        return 2
    model = resolve_model(api_key)
    binary = Path(args.binary)
    tasks = load_gen_tasks(REPO)
    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)
    cells = []
    temperature = None
    stop = None
    for arm in ARMS:
        for task in tasks:
            reason = budget.exhausted()
            if reason:
                cells.append({"arm": arm, "task": task.id, "status": "not_run",
                              "reason": reason})
                stop = reason
                continue
            cell_dir = batch / task.id.replace("/", "__") / arm
            cell_dir.mkdir(parents=True, exist_ok=True)
            try:
                client = OpenCodeGoClient(api_key=api_key, budget=budget,
                                          evidence_dir=batch / "llm", model=model,
                                          timeout=120.0, max_tokens=4096)
                if arm == "G3_concir":
                    record = run_g3_v2(client, binary, task, cell_dir, k_cir=4, k_code=3)
                else:
                    run, record = run_rust_generation(client, arm, task, cell_dir,
                                                      k=4, miri_seeds=16)
                    if run.final_artifact_path:
                        oracle = rust_arm_oracle(Path(run.final_artifact_path), task,
                                                 cell_dir / "oracle", binary=binary)
                        record["oracle"] = oracle
                        record["coverage"] = oracle.get("coverage")
                temperature = client.temperature
            except Exception as exc:  # noqa: BLE001
                record = {"task": task.id, "status": "error",
                          "error": f"{type(exc).__name__}: {exc}", "rounds": []}
            cell = {"arm": arm, "task": task.id, **_cell(record)}
            cells.append(cell)
            (cell_dir / "CELL.json").write_text(
                json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            (batch / "SUMMARY.json").write_text(
                json.dumps({"model": model, "cells": cells, "stop_reason": stop},
                           indent=2) + "\n", encoding="utf-8")
            print(f"{arm} {task.id} accepted={cell['accepted']} rf={cell['rf']} "
                  f"reqs={budget.requests_used}", flush=True)
    md = render(model, cells, temperature)
    (batch / "SUMMARY.md").write_text(md, encoding="utf-8")
    out.mkdir(parents=True, exist_ok=True)
    (out / "SUMMARY.md").write_text(md, encoding="utf-8")
    print(f"done: {budget.requests_used} requests, stop={stop}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
