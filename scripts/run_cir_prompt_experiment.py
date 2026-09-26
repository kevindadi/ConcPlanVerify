#!/usr/bin/env python3
"""Phase B: controlled CIR-prompt experiment on a frozen dev set (one model).

Configs:
- P0: v3 prompt (current) + current feedback
- P1: v4 prompt (complete interface) + current feedback
(P2 named feedback and P3 no-progress handling are on by default in the
current code; P0/P1 isolate the prompt-version effect.)

CIR only (no code stage): this stage measures the language interface, not the
Rust toolchain.
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
from cir_workflow.channels import AuditedClient, build_client  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks, run_g3  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, resolve_model, available_models  # noqa: E402

DEV_TASKS = [
    "condvar/bare_wait_no_predicate",
    "condvar/lost_wakeup_notify_before_wait",
    "lock-order/cross_module_cycle",
    "atomic-data/bounded_counter_invariant",
    "atomic-data/counter_overflow_safety",
    "channel/send_while_holding_mutex",
]
CONFIG_PROMPT = {"P0": "v3", "P1": "v4"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="DeepSeek Flash")
    parser.add_argument("--config", required=True, choices=sorted(CONFIG_PROMPT))
    parser.add_argument("--tasks", default=",".join(DEV_TASKS))
    parser.add_argument("--reps", type=int, default=1)
    parser.add_argument("--k-cir", type=int, default=4)
    parser.add_argument("--max-requests", type=int, default=60)
    parser.add_argument("--out", default="experiments/g3-rootcause-v1")
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    args = parser.parse_args()

    os.environ["CIR_PROMPT_VERSION"] = CONFIG_PROMPT[args.config]
    load_dotenv(REPO / ".env", override=True)
    env = dict(os.environ)
    binary = Path(args.binary)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    wanted = [t for t in args.tasks.split(",") if t]
    spec = resolve_model(available_models(), args.model)

    out = REPO / args.out
    batch = out / f"phaseb-{args.config.lower()}-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    audit = AuditLog(batch / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=3600.0)
    protocol_hash = hashlib.sha256(
        (REPO / "prompts" / f"concir_generation_{CONFIG_PROMPT[args.config]}.md").read_bytes()
    ).hexdigest()
    summary = {"batch": str(batch), "kind": "phase-b", "run_id": batch.name,
               "config": args.config, "prompt_version": CONFIG_PROMPT[args.config],
               "prompt_sha256": protocol_hash, "model": spec.display_name,
               "requested_model": spec.model_id, "tasks": wanted, "k_cir": args.k_cir,
               "cells": []}

    ch = CHANNELS[spec.channel]
    api_key = env.get(ch.api_key_env, "")
    for task_id in wanted:
        for rep in range(args.reps):
            task = tasks[task_id]
            cell_id = f"{args.config}/{task_id}/rep{rep}"
            cell_dir = batch / task_id.replace("/", "__") / f"rep{rep}"
            row = {"config": args.config, "prompt_version": CONFIG_PROMPT[args.config],
                   "model": spec.display_name, "task": task_id, "replicate": rep,
                   "cir_accepted": None, "first_cir_pass_round": None,
                   "cir_repairs": None, "rounds": 0,
                   "distinct_candidate_hashes": 0, "repeated_candidate": False,
                   "repeated_diagnostic": False, "calls": 0,
                   "input_tokens": None, "output_tokens": None, "total_tokens": None,
                   "calls_usage_unknown": 0, "error": None}
            try:
                inner = build_client(spec, budget=budget, evidence_dir=batch / "llm",
                                     api_key=api_key, timeout=120.0, max_tokens=4096)
                client = AuditedClient(inner, audit=audit, run_id=batch.name,
                                       cell_id=cell_id, spec=spec, arm="G3", task_id=task_id,
                                       replicate=rep, stage="cir")
                rec = run_g3(client, binary, task, cell_dir, k=args.k_cir,
                             with_codegen=False)
                rounds = rec.get("rounds", [])
                hashes = [r.get("program_sha256") for r in rounds if r.get("program_sha256")]
                pass_round = next((r.get("round") for r in rounds
                                   if r.get("decision") == "accepted"), None)
                row.update({
                    "cir_accepted": bool(rec.get("accepted")),
                    "first_cir_pass_round": pass_round,
                    "cir_repairs": None if pass_round is None else pass_round - 1,
                    "rounds": len(rounds),
                    "distinct_candidate_hashes": len(set(hashes)),
                    "repeated_candidate": any(r.get("same_program_as_previous") for r in rounds),
                    "repeated_diagnostic": any(r.get("repeated_explore_signature") for r in rounds),
                    "error": rec.get("error"),
                })
            except Exception as exc:  # noqa: BLE001
                row["error"] = f"{type(exc).__name__}: {exc}"
            events = read_events(batch / "REQUEST_EVENTS.jsonl")
            inp = out_t = tot = unknown = 0
            for e in events:
                if e.get("cell_id") != cell_id or e.get("kind") != "model-call":
                    continue
                u = e.get("usage") or {}
                if u.get("input_tokens") is None and u.get("total_tokens") is None:
                    unknown += 1
                    continue
                inp += int(u.get("input_tokens") or 0)
                out_t += int(u.get("output_tokens") or 0)
                tot += int(u.get("total_tokens") or 0)
            row.update({"calls": sum(1 for e in events if e.get("cell_id") == cell_id
                                     and e.get("kind") == "model-call"),
                        "input_tokens": inp, "output_tokens": out_t, "total_tokens": tot,
                        "calls_usage_unknown": unknown})
            summary["cells"].append(row)
            (batch / "SUMMARY.json").write_text(
                json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print(f"  {cell_id:46s} cir={row['cir_accepted']} pass_round={pass_round} "
                  f"rounds={row['rounds']} distinct={row['distinct_candidate_hashes']} tok={tot}")
    summary["requests_used"] = budget.requests_used
    (batch / "SUMMARY.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
                                        encoding="utf-8")
    print(f"wrote {batch} ({budget.requests_used} requests)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
