#!/usr/bin/env python3
"""§3 live: re-run the G3 code stage (fixed harness) and the semaphore-family
G0/G1/G2 baselines (which used the broken `mod concir_sync;` instruction).

The G0/G1/G2 cells outside the semaphore family are carried from freeze-5. A
single run id; no partial top-ups.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    load_gen_tasks, run_llmcode_from_cir, run_rust_generation, rust_arm_oracle,
)
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

SRC = REPO / "experiments/flash-gen-main-v3-code/run-20260923T182939"
SEMAPHORE_TASKS = {"semaphore/acquire_twice_no_release", "semaphore/permit_leak",
                   "semaphore/throttle_n_permits"}
CARRY_ARMS = {"G0_direct", "G1_self_iter", "G2_tools_iter", "G3_codegen"}


def _cell_from_record(rep, task_id, tier, arm, record, elapsed_ms):
    pt = ct = 0
    for rnd in record.get("rounds", []):
        pt += int(rnd.get("prompt_tokens") or 0)
        ct += int(rnd.get("completion_tokens") or 0)
    cov = record.get("coverage") or {}
    return {"rep": rep, "task": task_id, "tier": tier, "arm": arm,
            "accepted": record.get("accepted"),
            "accepted_with_proof": record.get("accepted_with_proof"),
            "rounds": len(record.get("rounds", [])),
            "prompt_tokens": pt, "completion_tokens": ct,
            "rc": cov.get("rc"), "rf": cov.get("rf"), "hang": None,
            "monitor_fail": record.get("monitor_fail"),
            "status": record.get("status"), "verify_ms": elapsed_ms,
            "record": record}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=280)
    parser.add_argument("--k-code", type=int, default=3)
    parser.add_argument("--out", default="experiments/flash-gen-main-v4-code")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "/Users/kevin/local-repos/ConcIR/target/release/concir-backend"))
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("DEEPSEEK_API_KEY missing", file=sys.stderr)
        return 2
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    binary = Path(args.binary)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    src = json.loads((SRC / "SUMMARY.json").read_text(encoding="utf-8"))
    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=8 * 3600)

    new_cells = []
    for cell in src["cells"]:
        arm, task_id, rep = cell["arm"], cell["task"], cell["rep"]
        rec = cell.get("record") or {}
        rerun_g3 = arm == "G3_concir"
        rerun_base = (arm in CARRY_ARMS and arm != "G3_codegen"
                      and task_id in SEMAPHORE_TASKS)
        if not (rerun_g3 or rerun_base):
            new_cells.append(cell)
            continue
        reason = budget.exhausted()
        if reason:
            new_cells.append({**cell, "status": "not_run", "reason": reason})
            continue
        task = tasks[task_id]
        cell_dir = batch / f"rep{rep}" / task_id.replace("/", "__") / arm.replace("_", "-")
        cell_dir.mkdir(parents=True, exist_ok=True)
        started = time.monotonic()
        try:
            client = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                         evidence_dir=batch / "llm", timeout=90.0,
                                         max_tokens=4096)
            if rerun_g3:
                cir = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
                if not cir or not Path(cir).is_file():
                    new_cells.append(cell)
                    continue
                record = run_llmcode_from_cir(client, binary, task, Path(cir),
                                              cell_dir / "code", k_code=args.k_code)
                arm_out = "G3_concir"
            else:
                run, record = run_rust_generation(client, arm, task, cell_dir, k=4,
                                                  miri_seeds=16)
                if run.final_artifact_path:
                    oracle = rust_arm_oracle(Path(run.final_artifact_path), task,
                                             cell_dir / "oracle", binary=binary)
                    record["oracle"] = oracle
                    record["coverage"] = oracle.get("coverage")
                    record["monitor_fail"] = oracle.get("monitor_fail")
                    record["hang"] = oracle.get("hang")
                arm_out = arm
        except Exception as exc:  # noqa: BLE001
            record = {"task": task_id, "status": "error",
                      "error": f"{type(exc).__name__}: {exc}", "rounds": []}
            arm_out = arm
        (cell_dir / "CELL.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        new_cells.append(_cell_from_record(rep, task_id, cell["tier"], arm_out, record,
                                           int((time.monotonic() - started) * 1000)))
        (batch / "SUMMARY.json").write_text(
            json.dumps({"cells": new_cells, "source": str(SRC)}, indent=2) + "\n",
            encoding="utf-8")
        print(f"[rep{rep}] {task_id} {arm_out} accepted={record.get('accepted')} "
              f"awp={record.get('accepted_with_proof')} reqs={budget.requests_used}",
              flush=True)

    (batch / "SUMMARY.json").write_text(
        json.dumps({"cells": new_cells, "source": str(SRC),
                    "requests_used": budget.requests_used}, indent=2) + "\n",
        encoding="utf-8")
    print(f"done: {budget.requests_used} requests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
