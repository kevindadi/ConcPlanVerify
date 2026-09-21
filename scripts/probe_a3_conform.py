"""§A1: run a3-to-rust-v2 (codegen -> strict conform v2) on the accepted A3 CIRs
of the model probe. 0 LLM requests."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.conformance import codegen, collect_traces, conform_all  # noqa: E402

OUT = REPO / "experiments/model-probe-v2"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")


def main() -> int:
    run = sorted(OUT.glob("run-*"))[-1]
    d = json.loads((run / "SUMMARY.json").read_text())
    main_b = json.loads((BATCH / "SUMMARY.json").read_text())
    contract_by_task = {}
    for rep in main_b["reps"]:
        for t in rep["tasks"]:
            contract_by_task[t["task"]] = t.get("contract_sha256")
    root = REPO / "benchmarks"
    from cir_workflow.experiments_v2 import load_manifest
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    results = {}
    work = OUT / f"a3-conform-{run.name}"
    work.mkdir(exist_ok=True)
    seen = set()
    for model, v in d["models"].items():
        for c in v["cells"]:
            if c.get("arm") != "A3_local" or not c.get("accepted"):
                continue
            cir = c.get("final_cir")
            if not cir or not Path(cir).is_file():
                continue
            sha = hashlib.sha256(Path(cir).read_bytes()).hexdigest()
            if sha in seen:
                results[sha]["cells"].append(f"{model}:{c['task']}")
                continue
            seen.add(sha)
            task = tasks[c["task"]]
            contract = task.resolve(root, task.contract)
            skel = work / f"{c['task'].replace('/', '__')}__{sha[:12]}" / "skeleton"
            entry = {"task": c["task"], "cells": [f"{model}:{c['task']}"]}
            try:
                codegen(Path(cir), skel, binary=BIN)
            except Exception as exc:  # noqa: BLE001
                entry.update({"conform": "codegen", "reason": str(exc)[:120]})
                results[sha] = entry
                continue
            build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=skel,
                                   capture_output=True, text=True, timeout=300)
            if build.returncode != 0:
                entry.update({"conform": "no_build"})
                results[sha] = entry
                continue
            tries = collect_traces(skel, native_runs=20, miri_seeds=16, timeout_s=8.0,
                                   calls_dir=skel.parent / "traces", run_miri=True)
            native = [t for t in tries if t.kind == "native"]
            agg = conform_all(Path(cir), native, binary=BIN, lenient_unlock=False,
                              attempt_events=False)
            ok = agg["violation"] == 0 and agg["conformant"] == len(native)
            entry.update({"conform": "PASS" if ok else "FAIL",
                          "conform_traces": len(tries)})
            results[sha] = entry
    payload = {"binary_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(),
               "results": results}
    (OUT / "A3_CONFORM.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    from collections import Counter
    print(Counter(r.get("conform") for r in results.values()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
