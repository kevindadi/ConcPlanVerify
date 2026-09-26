#!/usr/bin/env python3
"""§2 kimi-k3 G3 code stage (current harness).

Reuses the CIRs of `gen-model-probe-v2` (no CIR requests); re-runs only the G3
code stage with `kimi-k3`. Non-G3 cells and CIR-not-PASS cells are carried.
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
from cir_workflow.generation import load_gen_tasks, run_llmcode_from_cir  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.opencode_go import OpenCodeGoClient  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=72)
    parser.add_argument("--k-code", type=int, default=3)
    parser.add_argument("--model", default="kimi-k3")
    parser.add_argument("--out", default="experiments/gen-model-probe-v3-code")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "concir/target/release/concir-backend"))
    args = parser.parse_args()
    load_dotenv(REPO / ".env", override=True)
    api_key = os.environ.get("OPENCODE_API_KEY", "")
    if not api_key:
        print("OPENCODE_API_KEY missing", file=sys.stderr)
        return 2
    binary = Path(args.binary)
    if not binary.is_file():
        binary = Path(REPO.parent / "ConcIR/target/release/concir-backend")
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    src_dir = REPO / "experiments/gen-model-probe-v2"
    src_run = sorted(src_dir.glob("run-*"))[-1]
    src = json.loads((src_run / "SUMMARY.json").read_text(encoding="utf-8"))
    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)
    cells = []
    for cell in src["cells"]:
        arm, task_id, rep = cell["arm"], cell["task"], cell.get("rep", 0)
        rec = cell.get("record") or {}
        cir = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
        if arm != "G3_concir" or not cir or not Path(cir).is_file():
            cells.append(cell)
            continue
        reason = budget.exhausted()
        if reason:
            cells.append({**cell, "status": "not_run", "reason": reason})
            continue
        task = tasks[task_id]
        cell_dir = batch / f"rep{rep}" / task_id.replace("/", "__")
        cell_dir.mkdir(parents=True, exist_ok=True)
        try:
            client = OpenCodeGoClient(api_key=api_key, budget=budget,
                                      evidence_dir=batch / "llm", model=args.model,
                                      timeout=120.0, max_tokens=4096)
            record = run_llmcode_from_cir(client, binary, task, Path(cir),
                                          cell_dir / "code", k_code=args.k_code)
        except Exception as exc:  # noqa: BLE001
            record = {"task": task_id, "status": "error",
                      "error": f"{type(exc).__name__}: {exc}", "rounds": []}
        (cell_dir / "CELL.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        pt = ct = 0
        for rnd in record.get("rounds", []):
            pt += int(rnd.get("prompt_tokens") or 0)
            ct += int(rnd.get("completion_tokens") or 0)
        cov = record.get("coverage") or {}
        cells.append({**cell, "arm": "G3_concir", "accepted": record.get("accepted"),
                      "accepted_with_proof": record.get("accepted_with_proof"),
                      "rc": cov.get("rc"), "rf": cov.get("rf"),
                      "prompt_tokens": pt, "completion_tokens": ct,
                      "rounds": len(record.get("rounds", [])),
                      "status": record.get("status"), "record": record})
        (batch / "SUMMARY.json").write_text(
            json.dumps({"model": args.model, "cells": cells, "source": str(src_run)},
                       indent=2) + "\n", encoding="utf-8")
        print(f"[rep{rep}] {task_id} accepted={record.get('accepted')} "
              f"awp={record.get('accepted_with_proof')} reqs={budget.requests_used}",
              flush=True)
    (batch / "SUMMARY.json").write_text(
        json.dumps({"model": args.model, "cells": cells, "source": str(src_run),
                    "requests_used": budget.requests_used}, indent=2) + "\n",
        encoding="utf-8")
    print(f"done: {budget.requests_used} requests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
