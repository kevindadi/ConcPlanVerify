"""A3_tiered arm (§5): local (<=2 rounds) -> whole (remaining rounds) on stall.

K=4 total: 2 local rounds; if not accepted (stalled / repeated explore_fail) the
run escalates once to the whole-artifact prompt carrying the last local
candidate. Appends the arm to the main batch's rep-N/<task>/ and SUMMARY.json.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.arms import cir_oracle  # noqa: E402
from cir_workflow.concir_client import ConcirClient  # noqa: E402
from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.flash_smoke import SMOKE_V3_TASKS  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    DeepSeekFlashClient, LiveBudget, RecordingLocalProvider, RecordingProvider)
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
MAX_REQUESTS = int(os.environ.get("TIERED_MAX_REQUESTS", "90"))


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    root = REPO / "benchmarks"
    budget = LiveBudget(budget_path=BATCH / "budget_tiered.json", max_requests=MAX_REQUESTS)
    client = ConcirClient(binary, workdir=BATCH / "tiered_calls", timeout=30.0)
    escalated = accepted = 0
    cells = 0
    for rep in summary["reps"]:
        rep_dir = BATCH / f"rep-{rep['rep']}"
        for task in rep["tasks"]:
            tid = task["task"]
            if tid not in SMOKE_V3_TASKS or budget.exhausted():
                continue
            t = tasks[tid]
            rt = json.loads((root / t.directory / "repair_task.json").read_text(encoding="utf-8"))
            contract_path = root / rt["contract"]
            contract = json.loads(contract_path.read_text(encoding="utf-8"))
            spec = (root / rt["requirements_file"]).read_text(encoding="utf-8")
            buggy = root / rt["input_cir"]
            arm_dir = rep_dir / tid.replace("/", "__") / "A3_tiered"
            calls = []
            decisions = {}
            consumed = {"http_requests": 0, "total_tokens": 0, "rounds": 0,
                        "llm_wall_ms": 0, "tool_wall_ms": 0}
            rounds = []

            def _phase(fmt: str, initial: Path, rounds_allowed: int, tag: str):
                llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                          evidence_dir=rep_dir / "llm", timeout=90.0,
                                          max_tokens=4096)
                if fmt == "local":
                    provider = RecordingLocalProvider(llm, buggy.read_text(encoding="utf-8"))
                else:
                    provider = RecordingProvider(llm)
                wf = WholeArtifactRevisionWorkflow(client, provider, out_dir=arm_dir / tag,
                                                   max_rounds=rounds_allowed,
                                                   reply_format=fmt)
                res = wf.run(spec, contract, task_id=tid, initial_program=initial)
                calls.extend(provider.calls)
                for v in res.versions:
                    decisions[v.decision] = decisions.get(v.decision, 0) + 1
                c = res.consumption.as_dict()
                for k in ("http_requests", "total_tokens", "rounds", "llm_wall_ms",
                          "tool_wall_ms"):
                    consumed[k] = consumed.get(k, 0) + (c.get(k) or 0)
                return res

            res = _phase("local", buggy, 2, "local")
            did_escalate = False
            if not res.accepted and not budget.exhausted():
                last = res.versions[-1].artifact_path if res.versions else None
                init = Path(last) if last and Path(last).is_file() else buggy
                res2 = _phase("whole", init, 2, "whole")
                did_escalate = True
                res = res2
            cells += 1
            escalated += 1 if did_escalate else 0
            accepted += 1 if res.accepted else 0
            arec = {"task": tid, "arm": "A3_tiered", "accepted": res.accepted,
                    "accepted_round": res.accepted_version, "status": res.status,
                    "escalated": did_escalate,
                    "decision_distribution": decisions, "consumption": consumed,
                    "llm_calls": calls}
            if res.versions and res.versions[-1].artifact_path:
                arec["final_cir"] = res.versions[-1].artifact_path
                arec["oracle"] = cir_oracle(client, res.versions[-1].artifact_path,
                                            contract_path)
            task["arms"]["A3_tiered"] = arec
    summary["requests_used"] = (summary.get("requests_used") or 0) + budget.requests_used
    summary["tiered"] = {"requests_used": budget.requests_used,
                         "cells": cells, "accepted": accepted,
                         "escalation_rate": round(escalated / cells, 3) if cells else None}
    (BATCH / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(summary["tiered"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
