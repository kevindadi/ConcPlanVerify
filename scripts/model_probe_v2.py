"""model-probe-v2 (§5): second/third model on A0_direct, A2_tools_iter_ml and
A3_local over the 10 main tasks, 1 rep, K=4, OpenCode Go endpoint."""

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
from cir_workflow.flash_smoke import (  # noqa: E402
    SMOKE_V3_TASKS, TASK_TERMINAL, RustLiveProvider)
from cir_workflow.live import LiveBudget, RecordingLocalProvider  # noqa: E402
from cir_workflow.opencode_go import OpenCodeGoClient, list_models  # noqa: E402
from cir_workflow.revision_workflow import WholeArtifactRevisionWorkflow  # noqa: E402

OUT = REPO / "experiments/model-probe-v2"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
PRIORITY = ["kimi-k2.7-code", "kimi-k3", "glm-5.3-flash", "glm-5.2", "mimo-v2.5"]
COST_CAP = 8.0
MAX_REQUESTS = int(os.environ.get("PROBE_MAX_REQUESTS", "70"))


def _family(model: str) -> str:
    return model.split("-")[0]


def _pick(models: list[str]) -> list[str]:
    chosen = []
    for name in PRIORITY:
        if name in models and _family(name) not in {_family(c) for c in chosen}:
            chosen.append(name)
        if len(chosen) == 2:
            break
    return chosen


def _run_task(tid, model, api_key, budget, client, tasks, root, run, cells):
    task = tasks[tid]
    rt = json.loads((root / task.directory / "repair_task.json").read_text())
    contract_path = root / rt["contract"]
    contract = json.loads(contract_path.read_text())
    spec = (root / rt["requirements_file"]).read_text()
    buggy = root / rt["input_cir"]
    input_rust = (root / rt["input_rust"]).read_text()
    tdir = run / model / tid.replace("/", "__")
    for arm, mode in (("A0_direct", "direct"), ("A2_tools_iter_ml", "tools")):
        if budget.exhausted():
            return
        try:
            llm = OpenCodeGoClient(api_key=api_key, budget=budget,
                                   evidence_dir=run / model / "llm", model=model)
            provider = RustLiveProvider(llm, mode)
            r = run_rust_arm(provider, arm=arm, task=tid, spec=spec, contract=contract,
                             out_dir=tdir / arm, k=4, initial_source=input_rust,
                             tool_timeout_s=8.0, run_miri_many_seeds=False)
            arec = r.as_dict()
            arec["oracle"] = (rust_oracle(r.final_artifact_path, run_miri=True,
                                          miri_seed_count=16, run_miri_many_seeds=False,
                                          expected_terminal=TASK_TERMINAL.get(tid))
                              if r.final_artifact_path else {})
            cells.append({"task": tid, "arm": arm,
                          **{k: arec[k] for k in ("accepted", "accepted_round", "oracle")}})
        except Exception as exc:  # noqa: BLE001
            cells.append({"task": tid, "arm": arm,
                          "error": f"{type(exc).__name__}: {str(exc)[:100]}"})
    if not budget.exhausted():
        try:
            llm = OpenCodeGoClient(api_key=api_key, budget=budget,
                                   evidence_dir=run / model / "llm", model=model)
            provider = RecordingLocalProvider(llm, buggy.read_text(encoding="utf-8"))
            wf = WholeArtifactRevisionWorkflow(client, provider, out_dir=tdir / "A3_local",
                                               max_rounds=4, reply_format="local")
            res = wf.run(spec, contract, task_id=tid, initial_program=buggy)
            arec = {"task": tid, "arm": "A3_local", "accepted": res.accepted,
                    "accepted_round": res.accepted_version, "status": res.status,
                    "consumption": res.consumption.as_dict()}
            if res.versions and res.versions[-1].artifact_path:
                arec["final_cir"] = res.versions[-1].artifact_path
                arec["oracle"] = cir_oracle(client, res.versions[-1].artifact_path, contract_path)
            cells.append(arec)
        except Exception as exc:  # noqa: BLE001
            cells.append({"task": tid, "arm": "A3_local",
                          "error": f"{type(exc).__name__}: {str(exc)[:100]}"})


def main() -> int:
    api_key = os.environ.get("OPENCODE_API_KEY", "")
    if not api_key:
        print("missing OPENCODE_API_KEY")
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    models_payload = list_models(api_key)
    (OUT / "MODELS.json").write_text(json.dumps(models_payload, indent=1) + "\n",
                                     encoding="utf-8")
    ids = [m.get("id") for m in models_payload.get("data", [])]
    chosen = _pick(ids)
    print(json.dumps({"available": len(ids), "chosen": chosen}, indent=2))
    if len(sys.argv) > 1:
        chosen = [sys.argv[1]]
    root = REPO / "benchmarks"
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    run = OUT / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    summary = {"models": {}, "binary_sha256": __import__("hashlib").sha256(
        BIN.read_bytes()).hexdigest()}
    for model in chosen:
        budget = LiveBudget(budget_path=run / f"budget_{model}.json",
                            max_requests=MAX_REQUESTS, max_seconds=7200.0)
        client = ConcirClient(BIN, workdir=run / model / "calls", timeout=30.0)
        cells = []
        cost_total = 0.0
        stop_reason = None
        for tid in SMOKE_V3_TASKS:
            if budget.exhausted() or cost_total > COST_CAP:
                stop_reason = "budget" if budget.exhausted() else "cost_cap"
                break
            try:
                _run_task(tid, model, api_key, budget, client, tasks, root, run, cells)
            except Exception as exc:  # noqa: BLE001
                cells.append({"task": tid, "arm": "error",
                              "error": f"{type(exc).__name__}: {str(exc)[:120]}"})
                continue
        summary["models"][model] = {"cells": cells, "requests_used": budget.requests_used,
                                    "stop_reason": stop_reason}
    (run / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({m: {"requests": v["requests_used"], "cells": len(v["cells"])}
                      for m, v in summary["models"].items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
