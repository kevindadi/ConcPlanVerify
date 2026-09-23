#!/usr/bin/env python3
"""Run the §2 generation main batch (flash-gen-main-v1).

Usage:
  PYTHONPATH=python python3 scripts/run_gen_main.py --reps 3

Checkpoints `SUMMARY.json` after every cell so a long run can be polled.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    load_gen_tasks, run_g3, run_g3_v2, run_rust_generation, rust_arm_oracle,
)
from cir_workflow.flash_smoke import assert_protocol_confirmed  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir", "G3_codegen")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _tokens(record: dict) -> tuple[int, int]:
    pt = ct = 0
    for rnd in record.get("rounds", []):
        pt += int(rnd.get("prompt_tokens") or 0)
        ct += int(rnd.get("completion_tokens") or 0)
    return pt, ct


def _cell_summary(record: dict, elapsed_ms: int) -> dict:
    pt, ct = _tokens(record)
    cov = record.get("coverage") or {}
    return {
        "arm": record.get("arm"),
        "accepted": record.get("accepted"),
        "rounds": len(record.get("rounds", [])),
        "prompt_tokens": pt, "completion_tokens": ct,
        "llm_ms": (record.get("consumption") or {}).get("llm_wall_ms"),
        "tool_ms": (record.get("consumption") or {}).get("tool_wall_ms"),
        "verify_ms": elapsed_ms,
        "rc": cov.get("rc"), "rf": cov.get("rf"),
        "failed_reqs": cov.get("failed"),
        "monitor_fail": record.get("monitor_fail") or
                        [p.get("id") for p in (record.get("oracle", {}).get("monitor", {})
                                               .get("properties", []))
                         if p.get("status") == "FAIL"],
        "hang": (record.get("oracle") or {}).get("hang"),
        "model_outcome": (record.get("model") or {}).get("outcome"),
        "accepted_with_proof": record.get("accepted_with_proof"),
        "status": record.get("status"),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="experiments/flash-gen-main-v1")
    parser.add_argument("--reps", type=int, default=3)
    parser.add_argument("--tasks", default="")
    parser.add_argument("--arms", default="")
    parser.add_argument("--max-requests", type=int, default=1100)
    parser.add_argument("--k", type=int, default=4)
    parser.add_argument("--protocol", default="experiments/flash-gen-main-v1/PROTOCOL.md")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "concir/target/release/concir-backend"))
    args = parser.parse_args()

    load_dotenv(REPO / ".env")
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("DEEPSEEK_API_KEY missing", file=sys.stderr)
        return 2
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    protocol = REPO / args.protocol
    assert_protocol_confirmed(protocol, sha256(protocol))

    binary = Path(args.binary)
    tasks = load_gen_tasks(REPO)
    if args.tasks:
        wanted = set(args.tasks.split(","))
        tasks = [t for t in tasks if t.id in wanted]
    arms = tuple(args.arms.split(",")) if args.arms else ARMS

    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    (batch / "PROTOCOL.md").write_bytes(protocol.read_bytes())
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=12 * 3600)
    summary: dict = {
        "batch_dir": str(batch), "provider": ALLOWED_PROVIDER,
        "requested_model": "deepseek-flash", "protocol_sha256": sha256(protocol),
        "k": args.k, "reps": args.reps, "max_requests": args.max_requests,
        "arms": list(arms), "cells": [], "stop_reason": None,
    }
    stop = None
    for rep in range(args.reps):
        rep_arms = arms if rep == 0 else tuple(a for a in arms if a != "G3_codegen")
        for task in tasks:
            for arm in rep_arms:
                reason = budget.exhausted()
                if reason:
                    summary["cells"].append({"rep": rep, "task": task.id, "arm": arm,
                                             "status": "not_run", "reason": reason})
                    stop = reason
                    break
                cell_dir = batch / f"rep{rep}" / task.id.replace("/", "__") / arm
                cell_dir.mkdir(parents=True, exist_ok=True)
                started = time.monotonic()
                record: dict = {"arm": arm, "task": task.id}
                try:
                    client = DeepSeekFlashClient(
                        api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                        timeout=90.0, max_tokens=4096)
                    if arm == "G3_concir":
                        record = run_g3_v2(client, binary, task, cell_dir,
                                           k_cir=args.k, k_code=3)
                    elif arm == "G3_codegen":
                        record = run_g3(client, binary, task, cell_dir, k=args.k)
                        record["arm"] = "G3_codegen"
                    else:
                        run, record = run_rust_generation(client, arm, task, cell_dir,
                                                          k=args.k, miri_seeds=16)
                        artifact = run.final_artifact_path
                        if artifact:
                            oracle = rust_arm_oracle(Path(artifact), task,
                                                     cell_dir / "oracle", binary=binary)
                            record["oracle"] = oracle
                            record["coverage"] = oracle.get("coverage")
                            record["monitor_fail"] = oracle.get("monitor_fail")
                except Exception as exc:  # noqa: BLE001
                    record = {"arm": arm, "task": task.id, "status": "error",
                              "error": f"{type(exc).__name__}: {exc}"}
                elapsed_ms = int((time.monotonic() - started) * 1000)
                cell = {"rep": rep, "task": task.id, "tier": task.tier,
                        **_cell_summary(record, elapsed_ms), "record": record}
                summary["cells"].append(cell)
                (cell_dir / "CELL.json").write_text(
                    json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8")
                (batch / "SUMMARY.json").write_text(
                    json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8")
                print(f"[rep{rep}] {task.id} {arm} accepted={cell['accepted']} "
                      f"rf={cell['rf']} reqs={budget.requests_used}", flush=True)
                if stop:
                    break
            if stop:
                break
        if stop:
            summary["stop_reason"] = stop
            break

    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"done: {budget.requests_used} requests, stop={summary['stop_reason']}",
          flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
