"""§3 A3_tiered addendum on partial_deadlock_bystander.

Config A (T2, K=4): 1 local round, escalate immediately if not accepted (the
round-1 diagnostic carries the holds_all hint), then 3 whole rounds.
Config B (original trigger, K=6): 2 local rounds then 4 whole rounds.
3 reps each, <=20 requests.
"""

from __future__ import annotations

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
from cir_workflow.live import (  # noqa: E402
    DeepSeekFlashClient, LiveBudget, RecordingLocalProvider, RecordingProvider)
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow  # noqa: E402

TASK = "lock-order/partial_deadlock_bystander"
OUT = REPO / "experiments/case-partial-deadlock-v1/tiered"
MAX_REQUESTS = int(os.environ.get("TIERED_PARTIAL_MAX", "20"))


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    root = REPO / "benchmarks"
    task = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}[TASK]
    rt = json.loads((root / task.directory / "repair_task.json").read_text())
    contract_path = root / rt["contract"]
    contract = json.loads(contract_path.read_text())
    spec = (root / rt["requirements_file"]).read_text()
    buggy = root / rt["input_cir"]
    OUT.mkdir(parents=True, exist_ok=True)
    run = OUT / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(budget_path=run / "budget.json", max_requests=MAX_REQUESTS)
    client = ConcirClient(binary, workdir=run / "calls", timeout=30.0)
    configs = {"A_T2_K4": (1, 3), "B_orig_K6": (2, 4)}
    out = {}
    for name, (local_rounds, whole_rounds) in configs.items():
        res_list = []
        for rep in range(3):
            if budget.exhausted():
                break
            rep_dir = run / name / f"rep-{rep}"
            calls = []
            decisions = {}
            consumed = {"http_requests": 0, "total_tokens": 0, "rounds": 0}

            def phase(fmt, initial, n, tag):
                llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                          evidence_dir=run / "llm", timeout=90.0, max_tokens=4096)
                provider = (RecordingLocalProvider(llm, buggy.read_text(encoding="utf-8"))
                            if fmt == "local" else RecordingProvider(llm))
                wf = WholeArtifactRevisionWorkflow(client, provider, out_dir=rep_dir / tag,
                                                   max_rounds=n, reply_format=fmt)
                r = wf.run(spec, contract, task_id=TASK, initial_program=initial)
                calls.extend(provider.calls)
                for v in r.versions:
                    decisions[v.decision] = decisions.get(v.decision, 0) + 1
                c = r.consumption.as_dict()
                for k in ("http_requests", "total_tokens", "rounds"):
                    consumed[k] += c.get(k) or 0
                return r

            r1 = phase("local", buggy, local_rounds, "local")
            # T2 trigger: round-1 diagnostic carries the holds_all hint.
            hint = any("holds all of" in json.dumps(c) for c in calls)
            r = r1
            if not r1.accepted:
                last = r1.versions[-1].artifact_path if r1.versions else None
                init = Path(last) if last and Path(last).is_file() else buggy
                r = phase("whole", init, whole_rounds, "whole")
            res_list.append({"rep": rep, "accepted": r.accepted,
                             "accepted_round": r.accepted_version, "status": r.status,
                             "holds_all_hint_round1": hint,
                             "escalated": not r1.accepted,
                             "local_rounds": local_rounds, "whole_rounds": whole_rounds,
                             "decision_distribution": decisions, "consumption": consumed})
        out[name] = {"local_rounds": local_rounds, "whole_rounds": whole_rounds,
                     "reps": res_list,
                     "accepted": sum(1 for x in res_list if x["accepted"])}
    payload = {"task": TASK, "binary_sha256": __import__("hashlib").sha256(
        binary.read_bytes()).hexdigest(), "requests_used": budget.requests_used,
        "configs": out}
    (run / "SUMMARY.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({k: {"accepted": v["accepted"], "reps": [x["accepted"] for x in v["reps"]]}
                      for k, v in out.items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
