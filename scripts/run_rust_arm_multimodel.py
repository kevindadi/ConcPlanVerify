#!/usr/bin/env python3
"""Same-model Rust-arm runner (G0/G1/G2) across the four models.

Runs the paper's real Rust arms from the raw requirement document, with the
unified audit layer, so each model can be compared to its own G3 on the same
tasks. One batch per invocation (per model-group); resumable via the persisted
budget.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.channels import AuditedClient, ChannelUnavailable, build_client  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    load_gen_tasks, run_rust_generation, rust_arm_oracle,
)
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, available_models, resolve_model  # noqa: E402

ARM_KEY = {"G0": "G0_direct", "G1": "G1_self_iter", "G2": "G2_tools_iter"}


def _sum_calls(events, cell_id):
    inp = out = tot = 0
    n = unknown = latency = 0
    for e in events:
        if e.get("cell_id") != cell_id or e.get("kind") != "model-call":
            continue
        n += 1
        latency += int(e.get("latency_ms") or 0)
        u = e.get("usage") or {}
        if u.get("input_tokens") is None and u.get("output_tokens") is None \
                and u.get("total_tokens") is None:
            unknown += 1
            continue
        inp += int(u.get("input_tokens") or 0)
        out += int(u.get("output_tokens") or 0)
        tot += int(u.get("total_tokens") or 0)
    return n, inp, out, tot, latency, unknown


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--models", default="")
    parser.add_argument("--arm", default="G2", choices=sorted(ARM_KEY))
    parser.add_argument("--all-tasks", action="store_true")
    parser.add_argument("--tasks", default="")
    parser.add_argument("--reps", type=int, default=1)
    parser.add_argument("--k", type=int, default=4)
    parser.add_argument("--max-requests", type=int, default=600)
    parser.add_argument("--out", default="experiments/multimodel-v1")
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    args = parser.parse_args()

    load_dotenv(REPO / ".env", override=True)
    env = dict(os.environ)
    binary = Path(args.binary)
    arm = ARM_KEY[args.arm]
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    wanted = sorted(tasks) if args.all_tasks else [t for t in args.tasks.split(",") if t]
    specs = available_models()
    if args.models:
        specs = [resolve_model(available_models(), m.strip()) for m in args.models.split(",")]

    slug = "-".join(s.display_name.lower().replace(" ", "") for s in specs)[:32]
    out = REPO / args.out
    batch = out / f"{args.arm.lower()}-{time.strftime('%Y%m%dT%H%M%S')}-{slug}"
    batch.mkdir(parents=True, exist_ok=True)
    audit = AuditLog(batch / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)
    protocol_hash = None
    if (out / "MANIFEST.json").is_file():
        protocol_hash = hashlib.sha256((out / "MANIFEST.json").read_bytes()).hexdigest()
    summary = {"batch": str(batch), "kind": "rust-arm", "run_id": batch.name,
               "protocol_hash": protocol_hash, "arm": arm,
               "models": [s.display_name for s in specs], "tasks": wanted,
               "reps": args.reps, "k": args.k, "max_requests": args.max_requests,
               "cells": []}

    for spec in specs:
        ch = CHANNELS[spec.channel]
        api_key = env.get(ch.api_key_env, "")
        if not api_key:
            print(f"skip {spec.display_name}: missing {ch.api_key_env}")
            continue
        for task_id in wanted:
            for rep in range(args.reps):
                task = tasks[task_id]
                cell_id = f"{spec.display_name}/{task_id}/rep{rep}"
                cell_dir = batch / spec.display_name / task_id.replace("/", "__") / f"rep{rep}"
                row = {"model": spec.display_name, "requested_model": spec.model_id,
                       "provider": spec.provider, "transport": spec.channel,
                       "arm": arm, "task": task_id, "tier": task.tier,
                       "replicate": rep, "status": None, "accepted": None,
                       "first_pass_round": None, "repairs": None,
                       "failure_stage": None, "failure_detail": None,
                       "calls": 0, "calls_usage_unknown": 0, "input_tokens": None,
                       "output_tokens": None, "total_tokens": None,
                       "latency_ms": None, "error": None}
                if budget.exhausted():
                    row["status"] = "budget_exhausted"
                    row["error"] = budget.exhausted()
                    summary["cells"].append(row); _flush(batch, summary); continue
                try:
                    inner = build_client(spec, budget=budget, evidence_dir=batch / "llm",
                                         api_key=api_key, timeout=120.0, max_tokens=4096)
                    client = AuditedClient(inner, audit=audit, run_id=batch.name,
                                           cell_id=cell_id, spec=spec, arm=arm,
                                           task_id=task_id, replicate=rep, stage="code")
                    run, record = run_rust_generation(client, arm, task, cell_dir,
                                                      k=args.k, miri_seeds=16)
                    if run.final_artifact_path:
                        oracle = rust_arm_oracle(Path(run.final_artifact_path), task,
                                                 cell_dir / "oracle", binary=binary)
                        record["oracle"] = oracle
                    (cell_dir / "CELL.json").write_text(
                        json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8")
                    accepted = bool(run.accepted)
                    row.update({
                        "status": "accepted" if accepted else (run.error or "not_accepted"),
                        "accepted": accepted,
                        "first_pass_round": run.accepted_round,
                        "repairs": None if run.accepted_round is None else run.accepted_round - 1,
                        "error": run.error,
                    })
                    if not accepted:
                        row["failure_stage"] = "code"
                        row["failure_detail"] = (run.rounds[-1].__dict__.get("status")
                                                 if run.rounds else "no_round")
                except ChannelUnavailable as exc:
                    row["status"] = "blocked"; row["error"] = str(exc)
                except Exception as exc:  # noqa: BLE001
                    row["status"] = "error"; row["error"] = f"{type(exc).__name__}: {exc}"
                events = read_events(batch / "REQUEST_EVENTS.jsonl")
                n, inp, outp, tot, latency, unknown = _sum_calls(events, cell_id)
                row.update({"calls": n, "calls_usage_unknown": unknown,
                            "input_tokens": inp, "output_tokens": outp,
                            "total_tokens": tot, "latency_ms": latency})
                summary["cells"].append(row); _flush(batch, summary)
                print(f"  {cell_id:52s} {str(row['status'])[:16]:16s} acc={row['accepted']} tok={tot}")
    summary["requests_used"] = budget.requests_used
    _flush(batch, summary)
    print(f"wrote {batch} ({budget.requests_used} requests)")
    return 0


def _flush(batch: Path, summary: dict) -> None:
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if summary["cells"]:
        with (batch / "CELL_RESULTS.csv").open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=list(summary["cells"][0].keys()))
            writer.writeheader(); writer.writerows(summary["cells"])


if __name__ == "__main__":
    raise SystemExit(main())
