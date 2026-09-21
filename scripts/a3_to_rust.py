"""a3-to-rust-v1 (§3): codegen + holes + strict conform + behavior + Miri 16
for every accepted A3 CIR of flash-repair-main-v1, deduplicated by CIR sha256.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.conformance import (  # noqa: E402
    codegen, collect_traces, conform_all, fill_holes, fill_prompt, holes_of,
    lint_filled, parse_fill)
from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
OUT = Path(os.environ.get("A3_OUT", str(REPO / "experiments/a3-to-rust-v1")))
MAX_REQUESTS = int(os.environ.get("A3_MAX_REQUESTS", "110"))


def _run_behavior(binary: Path, timeout_s: float = 10.0) -> dict:
    started = time.monotonic()
    try:
        proc = subprocess.run([str(binary)], capture_output=True, text=True,
                              timeout=timeout_s)
        code, out = proc.returncode, proc.stdout
        hang = False
    except subprocess.TimeoutExpired:
        code, out, hang = None, "", True
    wall = int((time.monotonic() - started) * 1000)
    if hang:
        status = "hang"
    elif code != 0:
        status = "crash"
    else:
        status = "terminated_ok"
    return {"status": status, "exit_code": code, "wall_ms": wall,
            "stdout_sha256": hashlib.sha256(out.encode()).hexdigest()}


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    run = OUT / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(budget_path=run / "budget.json", max_requests=MAX_REQUESTS)

    # Collect accepted A3 cells and dedup by CIR sha.
    unique: dict[str, dict] = {}
    cells: list[tuple[int, str, str, str]] = []
    for rep in summary["reps"]:
        for task in rep["tasks"]:
            for arm in ("A3_local", "A3_whole", "A3_tiered"):
                rec = (task.get("arms") or {}).get(arm) or {}
                if not rec.get("accepted"):
                    continue
                cir = rec.get("final_cir")
                if not cir or not Path(cir).is_file():
                    continue
                sha = hashlib.sha256(Path(cir).read_bytes()).hexdigest()
                unique.setdefault(sha, {"cir": cir, "task": task["task"], "arm": arm})
                cells.append((rep["rep"], task["task"], arm, sha))

    results: dict[str, dict] = {}
    for sha, info in unique.items():
        task_id = info["task"]
        spec = ""
        task = tasks.get(task_id)
        if task:
            rt = json.loads((REPO / "benchmarks" / task.directory /
                             "repair_task.json").read_text(encoding="utf-8"))
            spec = (REPO / "benchmarks" / rt["requirements_file"]).read_text(encoding="utf-8")
        cell = run / f"{task_id.replace('/', '__')}__{info['arm']}__{sha[:12]}"
        skeleton = cell / "skeleton"
        entry: dict = {"cir_sha256": sha, "task": task_id, "arm": info["arm"]}
        try:
            codegen(Path(info["cir"]), skeleton, binary=binary)
        except Exception as exc:  # noqa: BLE001
            entry.update({"conform": "codegen", "reason": str(exc)[:160]})
            results[sha] = entry
            continue
        holes = holes_of(skeleton)
        entry["holes"] = [h["id"] for h in holes]
        fill_rounds = 0
        if holes:
            system = "You fill only the marked holes in a Rust skeleton."
            for attempt in (1, 2):
                if budget.exhausted() or not api_key:
                    break
                llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                          evidence_dir=run / "llm", timeout=90.0,
                                          max_tokens=4096)
                try:
                    outcome = llm.complete(system, fill_prompt(skeleton, spec))
                except Exception:  # noqa: BLE001
                    break
                fills = parse_fill(outcome.text)
                filled = cell / f"filled-{attempt}"
                fill_holes(skeleton, filled, fills)
                lint = lint_filled(skeleton, filled)
                fill_rounds = attempt
                if lint["ok"]:
                    skeleton = filled
                    break
        entry["fill_rounds"] = fill_rounds
        # build
        build = subprocess.run(["cargo", "build", "--offline", "--quiet"],
                               cwd=skeleton, capture_output=True, text=True, timeout=300)
        (cell / "build.stderr.txt").write_text(build.stderr, encoding="utf-8")
        if build.returncode != 0:
            entry.update({"conform": "no_build", "build_ok": False,
                          "reason": build.stderr.strip().splitlines()[0][:160] if build.stderr.strip() else "build failed"})
            results[sha] = entry
            continue
        entry["build_ok"] = True
        # Behavior must run before tracing: collect_traces `cargo clean`s per
        # Miri seed, which removes target/debug.
        entry["behavior"] = _run_behavior(skeleton / "target/debug/cir_generated")
        tries = collect_traces(skeleton, native_runs=20, miri_seeds=16,
                               timeout_s=8.0, calls_dir=cell / "traces", run_miri=True)
        agg = conform_all(Path(info["cir"]), tries, binary=binary,
                          lenient_unlock=False, attempt_events=False)
        expected = 20 + 16
        conform_pass = agg["conformant"] == expected and agg["violation"] == 0
        bad = next((d for d in agg["details"] if d.get("status") != "conformant"), None)
        entry.update({
            "conform": "PASS" if conform_pass else "FAIL",
            "conform_traces": agg["traces_total"], "conform_conformant": agg["conformant"],
            "conform_violation": agg["violation"], "conform_timeout": agg["timeout"],
            "reason": None if conform_pass else str((bad or {}).get("status")),
        })
        miri_runs = [t for t in tries if t.kind == "miri"]
        entry["miri"] = {
            "seeds": len(miri_runs),
            "detected": any(t.hang for t in miri_runs),
            "hang": sum(1 for t in miri_runs if t.hang),
        }
        results[sha] = entry

    # Write back into the main batch cells' oracle.*
    for rep in summary["reps"]:
        for task in rep["tasks"]:
            for arm in ("A3_local", "A3_whole", "A3_tiered"):
                rec = (task.get("arms") or {}).get(arm) or {}
                if not rec.get("accepted"):
                    continue
                cir = rec.get("final_cir")
                if not cir or not Path(cir).is_file():
                    continue
                sha = hashlib.sha256(Path(cir).read_bytes()).hexdigest()
                r = results.get(sha)
                if not r:
                    continue
                oracle = rec.setdefault("oracle", {})
                oracle["conform"] = r.get("conform")
                oracle["build_ok"] = r.get("build_ok")
                oracle["behavior_status"] = (r.get("behavior") or {}).get("status")
                oracle["miri_detected"] = (r.get("miri") or {}).get("detected")
                oracle["miri_seeds"] = (r.get("miri") or {}).get("seeds")
                oracle["a3_to_rust"] = {"cir_sha256": sha,
                                        "conform_traces": r.get("conform_traces"),
                                        "dedup_of": None}
    (BATCH / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")

    payload = {"batch": str(BATCH.relative_to(REPO)),
               "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
               "requests_used": budget.requests_used, "max_requests": MAX_REQUESTS,
               "unique_cirs": len(unique), "cells": len(cells),
               "results": results}
    (OUT / "SUMMARY.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    lines = ["# a3-to-rust-v1 — SUMMARY", "",
             f"- binary sha256: `{payload['binary_sha256']}`",
             f"- accepted A3 cells: {len(cells)}; unique CIRs: {len(unique)}",
             f"- requests: {budget.requests_used}/{MAX_REQUESTS}", "",
             "| cir sha | task | arm | holes | conform | traces | behavior | miri |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for sha, r in results.items():
        lines.append(f"| `{sha[:12]}` | {r['task']} | {r['arm']} | {len(r.get('holes') or [])} | "
                     f"{r.get('conform')} | {r.get('conform_traces')} | "
                     f"{(r.get('behavior') or {}).get('status')} | "
                     f"{(r.get('miri') or {}).get('seeds')} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"unique": len(unique), "cells": len(cells),
                      "requests_used": budget.requests_used,
                      "conform": {k: r.get("conform") for k, r in results.items()}},
                     indent=2)[:1500])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
