"""§4 K-4a: add abba_2lock and bare_wait_no_predicate to flash-repair-main-v1.

6 arms x 3 reps, same protocol and BIN_MAIN, appended to the existing run's
SUMMARY.json.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.arms import cir_oracle  # noqa: E402
from cir_workflow.concir_client import ConcirClient  # noqa: E402
from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.flash_smoke import _run_task_arms  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    DeepSeekFlashClient, LiveBudget, RecordingLocalProvider, RecordingProvider)
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
EXTRA = ("lock-order/abba_2lock", "condvar/bare_wait_no_predicate")
ARMS5 = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml", "A3_local", "A3_whole")
MAX_REQUESTS = int(os.environ.get("EXTRA_MAX_REQUESTS", "70"))


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
    budget = LiveBudget(budget_path=BATCH / "budget_extra.json", max_requests=MAX_REQUESTS)
    client = ConcirClient(binary, workdir=BATCH / "extra_calls", timeout=30.0)
    for rep in summary["reps"]:
        rep_dir = BATCH / f"rep-{rep['rep']}"
        existing = {t["task"] for t in rep["tasks"]}
        for tid in EXTRA:
            if tid in existing or budget.exhausted():
                continue
            task = tasks[tid]
            rt = json.loads((root / task.directory / "repair_task.json").read_text())
            contract_path = root / rt["contract"]
            contract = json.loads(contract_path.read_text())
            spec = (root / rt["requirements_file"]).read_text()
            buggy = root / rt["input_cir"]
            input_rust = (root / rt["input_rust"]).read_text()
            arm_dir = rep_dir / tid.replace("/", "__")
            record = _run_task_arms(tid, root, task, contract_path, contract, spec,
                                    buggy, input_rust, arm_dir, rep_dir / "llm", binary,
                                    api_key, budget, 4, 90.0, 4096, None, ARMS5)
            # A3_tiered (local -> whole)
            calls, decisions, consumed = [], {}, {"http_requests": 0, "total_tokens": 0, "rounds": 0}
            escalated = False

            def phase(fmt, initial, n, tag):
                llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                          evidence_dir=rep_dir / "llm", timeout=90.0,
                                          max_tokens=4096)
                provider = (RecordingLocalProvider(llm, buggy.read_text(encoding="utf-8"))
                            if fmt == "local" else RecordingProvider(llm))
                wf = WholeArtifactRevisionWorkflow(client, provider,
                                                   out_dir=arm_dir / "A3_tiered" / tag,
                                                   max_rounds=n, reply_format=fmt)
                r = wf.run(spec, contract, task_id=tid, initial_program=initial)
                calls.extend(provider.calls)
                for v in r.versions:
                    decisions[v.decision] = decisions.get(v.decision, 0) + 1
                c = r.consumption.as_dict()
                for k in ("http_requests", "total_tokens", "rounds"):
                    consumed[k] += c.get(k) or 0
                return r

            r = phase("local", buggy, 2, "local")
            if not r.accepted and not budget.exhausted():
                last = r.versions[-1].artifact_path if r.versions else None
                init = Path(last) if last and Path(last).is_file() else buggy
                r = phase("whole", init, 2, "whole")
                escalated = True
            arec = {"task": tid, "arm": "A3_tiered", "accepted": r.accepted,
                    "accepted_round": r.accepted_version, "status": r.status,
                    "escalated": escalated, "decision_distribution": decisions,
                    "consumption": consumed, "llm_calls": calls}
            if r.versions and r.versions[-1].artifact_path:
                arec["final_cir"] = r.versions[-1].artifact_path
                arec["oracle"] = cir_oracle(client, r.versions[-1].artifact_path, contract_path)
            record["arms"]["A3_tiered"] = arec
            record["rep"] = rep["rep"]
            rep["tasks"].append(record)
    summary["requests_used"] = (summary.get("requests_used") or 0) + budget.requests_used
    summary["extra"] = {"tasks": list(EXTRA), "requests_used": budget.requests_used}
    (BATCH / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({"requests_used": budget.requests_used,
                      "tasks": list(EXTRA)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
