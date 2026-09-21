"""§A1: complete model-probe-v2 to 10 tasks x 3 arms for kimi/glm, reusing the
cells already produced by run-20260921T172737."""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.arms import cir_oracle, run_rust_arm, rust_oracle  # noqa: E402
from cir_workflow.concir_client import ConcirClient  # noqa: E402
from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.flash_smoke import TASK_TERMINAL, RustLiveProvider  # noqa: E402
from cir_workflow.live import LiveBudget, RecordingLocalProvider  # noqa: E402
from cir_workflow.opencode_go import OpenCodeGoClient  # noqa: E402
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow  # noqa: E402

OUT = REPO / "experiments/model-probe-v2"
REUSE = Path(os.environ.get("PROBE_REUSE", str(OUT / "run-20260921T172737")))
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
ARMS = ("A0_direct", "A2_tools_iter_ml", "A3_local")
MODELS = ("kimi-k2.7-code", "glm-5.3-flash")
MAIN_TASKS = (
    "lock-order/partial_deadlock_bystander", "lock-order/cross_module_cycle",
    "lock-order/cycle_3lock", "structure/nested_scope_lock_order",
    "condvar/notify_one_multi_waiter_wrong_pick",
    "channel/bounded_backpressure_lock_held", "channel/send_while_holding_mutex",
    "semaphore/acquire_twice_no_release",
    "lock-order/abba_2lock", "condvar/bare_wait_no_predicate",
)
MAX_REQUESTS = int(os.environ.get("PROBE_MAX_REQUESTS", "120"))


def _reuse():
    out = {}
    if not (REUSE / "SUMMARY.json").is_file():
        return out
    d = json.loads((REUSE / "SUMMARY.json").read_text())
    for model, v in d.get("models", {}).items():
        for c in v.get("cells", []):
            if c.get("error") or c.get("decision") == "not_run":
                continue
            out[(model, c["task"], c["arm"])] = c
    return out


def main() -> int:
    api_key = os.environ.get("OPENCODE_API_KEY", "")
    if not api_key:
        print("missing OPENCODE_API_KEY")
        return 2
    root = REPO / "benchmarks"
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    reuse = _reuse()
    run = OUT / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    summary = {"models": {}, "binary_sha256": __import__("hashlib").sha256(
        BIN.read_bytes()).hexdigest(), "reused_from": REUSE.name}
    for model in MODELS:
        budget = LiveBudget(budget_path=run / f"budget_{model}.json",
                            max_requests=MAX_REQUESTS, max_seconds=7200.0)
        client = ConcirClient(BIN, workdir=run / model / "calls", timeout=30.0)
        cells = []
        for tid in MAIN_TASKS:
            task = tasks[tid]
            rt = json.loads((root / task.directory / "repair_task.json").read_text())
            contract_path = root / rt["contract"]
            contract = json.loads(contract_path.read_text())
            spec = (root / rt["requirements_file"]).read_text()
            buggy = root / rt["input_cir"]
            input_rust = (root / rt["input_rust"]).read_text()
            tdir = run / model / tid.replace("/", "__")
            for arm in ARMS:
                key = (model, tid, arm)
                if key in reuse:
                    c = dict(reuse[key])
                    c["reused_from"] = REUSE.name
                    cells.append(c)
                    continue
                if budget.exhausted():
                    cells.append({"task": tid, "arm": arm, "decision": "not_run",
                                  "reason": "budget"})
                    continue
                try:
                    if arm in ("A0_direct", "A2_tools_iter_ml"):
                        llm = OpenCodeGoClient(api_key=api_key, budget=budget,
                                               evidence_dir=run / model / "llm", model=model)
                        provider = RustLiveProvider(llm, "direct" if arm == "A0_direct" else "tools")
                        r = run_rust_arm(provider, arm=arm, task=tid, spec=spec,
                                         contract=contract, out_dir=tdir / arm, k=4,
                                         initial_source=input_rust, tool_timeout_s=8.0,
                                         run_miri_many_seeds=False)
                        arec = r.as_dict()
                        oracle = (rust_oracle(r.final_artifact_path, run_miri=True,
                                              miri_seed_count=16, run_miri_many_seeds=False,
                                              expected_terminal=TASK_TERMINAL.get(tid))
                                  if r.final_artifact_path else {})
                        cells.append({"task": tid, "arm": arm, "accepted": r.accepted,
                                      "accepted_round": r.accepted_round,
                                      "consumption": r.consumption.as_dict(),
                                      "final_artifact_path": r.final_artifact_path,
                                      "oracle": oracle})
                    else:
                        llm = OpenCodeGoClient(api_key=api_key, budget=budget,
                                               evidence_dir=run / model / "llm", model=model)
                        provider = RecordingLocalProvider(llm, buggy.read_text(encoding="utf-8"))
                        wf = WholeArtifactRevisionWorkflow(client, provider,
                                                           out_dir=tdir / "A3_local",
                                                           max_rounds=4, reply_format="local")
                        res = wf.run(spec, contract, task_id=tid, initial_program=buggy)
                        arec = {"task": tid, "arm": "A3_local", "accepted": res.accepted,
                                "accepted_round": res.accepted_version, "status": res.status,
                                "consumption": res.consumption.as_dict()}
                        if res.versions and res.versions[-1].artifact_path:
                            arec["final_cir"] = res.versions[-1].artifact_path
                            arec["oracle"] = cir_oracle(client, res.versions[-1].artifact_path,
                                                        contract_path)
                        cells.append(arec)
                except Exception as exc:  # noqa: BLE001
                    cells.append({"task": tid, "arm": arm, "decision": "not_run",
                                  "reason": f"{type(exc).__name__}: {str(exc)[:100]}"})
        summary["models"][model] = {"cells": cells, "requests_used": budget.requests_used,
                                    "stop_reason": budget.exhausted()}
    (run / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({m: {"cells": len(v["cells"]), "requests": v["requests_used"]}
                      for m, v in summary["models"].items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
