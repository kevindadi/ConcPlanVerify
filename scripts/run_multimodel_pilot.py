#!/usr/bin/env python3
"""Bounded multi-model G3 pilot over a fixed task subset.

This is the *pilot*, not the main matrix: it runs the full G3 flow (verified CIR
then model-written Rust) from the raw requirement document, for every available
model, on a small fixed task subset with one replicate. It records a unified
audit event per model call and a cell row per (model, task).

The main comparison (all 24 tasks, paper replicates) is a separate run; this
pilot validates the transport/audit layer end to end and gives a first,
honestly-labelled signal.
"""

from __future__ import annotations

import argparse
import csv
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
from cir_workflow.generation import load_gen_tasks, run_g3_v2  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, available_models, resolve_model  # noqa: E402

DEFAULT_TASKS = ["channel/rendezvous_both_send", "lock-order/cross_module_cycle"]


def _first_round(rounds, stage, accept_decision="accepted"):
    for rnd in rounds:
        if rnd.get("stage") == stage and rnd.get("decision") == accept_decision:
            return rnd.get("round")
    return None


def _sum_calls(events, cell_id):
    """Sum known server usage; count calls whose usage is unknown separately.

    A missing usage is never coerced to zero: it is counted as unknown so the
    reported total is not presented as a complete cost.
    """

    inp = out = tot = 0
    n = unknown = 0
    latency = 0
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


def _failure_stage(rec):
    """Classify a non-accepted cell by the stage it actually stopped at."""

    if rec.get("accepted"):
        return "accepted", None
    cir = rec.get("cir_stage") or {}
    if not cir.get("accepted"):
        status = cir.get("status") or rec.get("status") or "cir_failed"
        return "cir", status
    code = rec.get("code_stage") or {}
    if not code:
        return "code", "code_not_run"
    rounds = code.get("rounds") or []
    last = rounds[-1] if rounds else {}
    decision = last.get("decision") or code.get("status") or "code_failed"
    return "code", decision


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--models", default="")
    parser.add_argument("--tasks", default=",".join(DEFAULT_TASKS))
    parser.add_argument("--all-tasks", action="store_true")
    parser.add_argument("--reps", type=int, default=1)
    parser.add_argument("--k-cir", type=int, default=4)
    parser.add_argument("--k-code", type=int, default=3)
    parser.add_argument("--max-requests", type=int, default=100)
    parser.add_argument("--out", default="experiments/multimodel-v1")
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    parser.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    args = parser.parse_args()

    load_dotenv(REPO / ".env", override=True)
    env = dict(os.environ)
    binary = Path(args.binary)
    instrument = Path(args.instrument)
    if not binary.is_file():
        print(f"backend not found: {binary}", file=sys.stderr)
        return 2

    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    if args.all_tasks:
        wanted = sorted(tasks)
    else:
        wanted = [t for t in args.tasks.split(",") if t]
    specs = available_models()
    if args.models:
        specs = [resolve_model(available_models(), m.strip()) for m in args.models.split(",")]

    slug = "-".join(s.display_name.lower().replace(" ", "") for s in specs)[:40]
    out = REPO / args.out
    batch = out / f"pilot-{time.strftime('%Y%m%dT%H%M%S')}-{slug}"
    batch.mkdir(parents=True, exist_ok=True)
    audit = AuditLog(batch / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)
    manifest_path = out / "MANIFEST.json"
    protocol_hash = None
    if manifest_path.is_file():
        import hashlib
        protocol_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    summary = {"batch": str(batch), "kind": "g3-pilot", "run_id": batch.name,
               "protocol_hash": protocol_hash,
               "started_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
               "tasks": wanted,
               "models": [s.display_name for s in specs], "reps": args.reps,
               "k_cir": args.k_cir, "k_code": args.k_code,
               "max_requests": args.max_requests, "cells": []}

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
                       "arm": "G3", "task": task_id, "tier": task.tier,
                       "replicate": rep, "status": None, "cir_accepted": None,
                       "code_accepted": None, "accepted": None,
                       "accepted_with_proof": None,
                       "first_cir_pass_round": None, "first_code_accept_round": None,
                       "cir_repairs": None, "code_repairs": None,
                       "first_accept_total_candidate_rounds": None,
                       "failure_stage": None, "failure_detail": None,
                       "calls": 0, "calls_usage_unknown": 0,
                       "input_tokens": None, "output_tokens": None,
                       "total_tokens": None, "latency_ms": None, "error": None}
                reason = budget.exhausted()
                if reason:
                    row["status"] = "budget_exhausted"
                    row["error"] = reason
                    summary["cells"].append(row)
                    _flush(batch, summary)
                    continue
                try:
                    inner = build_client(spec, budget=budget,
                                         evidence_dir=batch / "llm",
                                         api_key=api_key, timeout=120.0,
                                         max_tokens=4096)
                    client = AuditedClient(inner, audit=audit, run_id=batch.name,
                                           cell_id=cell_id, spec=spec, arm="G3",
                                           task_id=task_id, replicate=rep,
                                           stage="cir")
                    rec = run_g3_v2(client, binary, task, cell_dir,
                                    k_cir=args.k_cir, k_code=args.k_code,
                                    instrument_binary=instrument)
                    rounds = rec.get("rounds", [])
                    cir_pass = _first_round(rounds, "cir")
                    code_pass = _first_round(rounds, "code")
                    accepted = bool(rec.get("accepted"))
                    fail_stage, fail_detail = _failure_stage(rec)
                    row.update({
                        "status": rec.get("status"),
                        "cir_accepted": bool((rec.get("cir_stage") or {}).get("accepted")),
                        "code_accepted": bool((rec.get("code_stage") or {}).get("accepted")),
                        "accepted": accepted,
                        "accepted_with_proof": bool(rec.get("accepted_with_proof")),
                        "first_cir_pass_round": cir_pass,
                        "first_code_accept_round": code_pass,
                        "cir_repairs": None if cir_pass is None else cir_pass - 1,
                        "code_repairs": None if code_pass is None else code_pass - 1,
                        # End-to-end first-accept total is only defined when the
                        # cell was actually accepted; a CIR pass with a code
                        # failure is null, never cir_pass.
                        "first_accept_total_candidate_rounds":
                            (cir_pass + (code_pass or 0))
                            if accepted and cir_pass is not None and code_pass is not None
                            else None,
                        "failure_stage": None if accepted else fail_stage,
                        "failure_detail": None if accepted else fail_detail,
                        "error": rec.get("error"),
                    })
                except ChannelUnavailable as exc:
                    row["status"] = "blocked"
                    row["error"] = str(exc)
                except Exception as exc:  # noqa: BLE001
                    row["status"] = "error"
                    row["error"] = f"{type(exc).__name__}: {exc}"
                events = read_events(batch / "REQUEST_EVENTS.jsonl")
                n, inp, outp, tot, latency, unknown = _sum_calls(events, cell_id)
                row.update({"calls": n, "calls_usage_unknown": unknown,
                            "input_tokens": inp, "output_tokens": outp,
                            "total_tokens": tot, "latency_ms": latency})
                summary["cells"].append(row)
                _flush(batch, summary)
                print(f"  {cell_id:48s} {row['status']:14s} "
                      f"accepted={row['accepted']} cir={row['first_cir_pass_round']} "
                      f"code={row['first_code_accept_round']} tok={tot}")

    summary["requests_used"] = budget.requests_used
    _flush(batch, summary)
    print(f"wrote {batch} ({budget.requests_used} requests)")
    return 0


def _flush(batch: Path, summary: dict) -> None:
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if summary["cells"]:
        fields = list(summary["cells"][0].keys())
        with (batch / "CELL_RESULTS.csv").open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields)
            writer.writeheader()
            writer.writerows(summary["cells"])


if __name__ == "__main__":
    raise SystemExit(main())
