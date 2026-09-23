#!/usr/bin/env python3
"""§3 live code stage: re-run only the G3 code stage on freeze-5's verified CIRs.

Reuses the model-PASS CIRs of `flash-gen-main-v2` (no CIR requests) with the
fixed prompt/tools; G0/G1/G2 (and the `G3_codegen` ablation) are carried over
from freeze-5. Writes a merged batch under `experiments/flash-gen-main-v3-code`.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks, run_llmcode_from_cir  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

SRC = REPO / "experiments/flash-gen-main-v2/run-20260923T005232"


def _cell_from_record(rep, task_id, tier, record, elapsed_ms):
    pt = ct = 0
    for rnd in record.get("rounds", []):
        pt += int(rnd.get("prompt_tokens") or 0)
        ct += int(rnd.get("completion_tokens") or 0)
    cov = record.get("coverage") or {}
    return {"rep": rep, "task": task_id, "tier": tier, "arm": "G3_concir",
            "accepted": record.get("accepted"),
            "accepted_with_proof": record.get("accepted_with_proof"),
            "rounds": len(record.get("rounds", [])),
            "prompt_tokens": pt, "completion_tokens": ct,
            "rc": cov.get("rc"), "rf": cov.get("rf"),
            "hang": None, "monitor_fail": record.get("monitor_fail"),
            "status": record.get("status"), "verify_ms": elapsed_ms,
            "record": record}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=200)
    parser.add_argument("--k-code", type=int, default=3)
    parser.add_argument("--out", default="experiments/flash-gen-main-v3-code")
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
    cells = src["cells"]

    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)

    new_cells = []
    for cell in cells:
        if cell.get("arm") != "G3_concir":
            new_cells.append(cell)
            continue
        rec = cell.get("record") or {}
        cir_path = (rec.get("cir_path")
                    or (rec.get("code_stage") or {}).get("cir_path"))
        if not cir_path or not Path(cir_path).is_file():
            new_cells.append(cell)  # CIR not PASS: carry over
            continue
        reason = budget.exhausted()
        if reason:
            new_cells.append({**cell, "status": "not_run", "reason": reason})
            continue
        task = tasks[cell["task"]]
        cell_dir = batch / f"rep{cell['rep']}" / cell["task"].replace("/", "__")
        cell_dir.mkdir(parents=True, exist_ok=True)
        started = time.monotonic()
        try:
            client = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                         evidence_dir=batch / "llm", timeout=90.0,
                                         max_tokens=4096)
            record = run_llmcode_from_cir(client, binary, task, Path(cir_path),
                                          cell_dir / "code", k_code=args.k_code)
        except Exception as exc:  # noqa: BLE001
            record = {"arm": "G3_concir_llmcode", "task": cell["task"],
                      "accepted": False, "status": "error",
                      "error": f"{type(exc).__name__}: {exc}", "rounds": []}
        (cell_dir / "CELL.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        new_cells.append(_cell_from_record(cell["rep"], cell["task"], cell["tier"],
                                           record, int((time.monotonic() - started) * 1000)))
        (batch / "SUMMARY.json").write_text(
            json.dumps({"cells": new_cells, "carried": True}, indent=2) + "\n",
            encoding="utf-8")
        print(f"[rep{cell['rep']}] {cell['task']} accepted={record.get('accepted')} "
              f"awp={record.get('accepted_with_proof')} reqs={budget.requests_used}",
              flush=True)

    (batch / "SUMMARY.json").write_text(
        json.dumps({"cells": new_cells, "carried": True,
                    "source": str(SRC), "requests_used": budget.requests_used},
                   indent=2) + "\n", encoding="utf-8")
    print(f"done: {budget.requests_used} requests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
