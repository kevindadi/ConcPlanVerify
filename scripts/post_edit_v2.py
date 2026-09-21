"""post-edit-conform-v2 (§3): developer edits to the 23 v2 programs, strict
operation-bound conform."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.conformance import collect_traces, conform_all  # noqa: E402
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402

A3 = REPO / "experiments/a3-to-rust-v2"
OUT = REPO / "experiments/post-edit-conform-v2"
BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
MAX_REQUESTS = int(os.environ.get("POSTEDIT_MAX_REQUESTS", "69"))
TRACE_RE = re.compile(r"cir_trace::")
SID_RE = re.compile(r'"(s\d+|L\d+)"')

EDITS = {
    "E1": "Add progress logging: at each step of each thread, print a short line "
          "to stderr (eprintln!) so the run can be followed.",
    "E2": "Refactor for readability: extract each thread closure body into a "
          "separate top-level function, keeping behaviour identical.",
    "E3": "Optimise without changing functionality: reduce lock hold time and "
          "improve throughput.",
}


def _programs():
    summary = json.loads((A3 / "SUMMARY.json").read_text(encoding="utf-8"))
    run = sorted(A3.glob("run-*"))[-1]
    main = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    cir = {}
    for rep in main["reps"]:
        for t in rep["tasks"]:
            for arm in ("A3_local", "A3_whole", "A3_tiered"):
                rec = (t.get("arms") or {}).get(arm) or {}
                if rec.get("accepted") and rec.get("final_cir"):
                    cir[hashlib.sha256(Path(rec["final_cir"]).read_bytes()).hexdigest()] = rec["final_cir"]
    progs = []
    for sha, r in summary["results"].items():
        key = f"{r['task'].replace('/', '__')}__{r['arm']}__{sha[:12]}"
        skel = run / key / "skeleton"
        if (skel / "src/main.rs").is_file() and sha in cir:
            progs.append({"sha": sha, "task": r["task"], "arm": r["arm"], "skel": skel, "cir": cir[sha]})
    return progs


def _extract(text):
    if "```" in text:
        cands = text.split("```")[1::2]
        best = max(cands, key=len)
        lines = best.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n"


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    progs = _programs()
    if len(sys.argv) > 1:
        progs = [p for p in progs if sys.argv[1] in p["task"]]
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    budget = LiveBudget(budget_path=OUT / "budget.json", max_requests=MAX_REQUESTS)
    system = (REPO / "prompts/post_edit_v1.md").read_text(encoding="utf-8")
    cells = []
    for prog in progs:
        original = (prog["skel"] / "src/main.rs").read_text(encoding="utf-8")
        orig_calls = len(TRACE_RE.findall(original))
        orig_sids = set(SID_RE.findall(original))
        for eid, task in EDITS.items():
            if budget.exhausted():
                break
            llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                      evidence_dir=OUT / "llm", timeout=90.0, max_tokens=4096)
            try:
                outcome = llm.complete(system, f"Task: {task}\n\nProgram:\n```rust\n{original}\n```")
                edited = _extract(outcome.text)
            except Exception as exc:  # noqa: BLE001
                cells.append({"sha": prog["sha"], "task": prog["task"], "arm": prog["arm"],
                              "edit": eid, "build_ok": False, "reason": str(exc)[:80]})
                continue
            cell_dir = OUT / "cells" / f"{prog['sha'][:12]}" / eid
            (cell_dir / "src").mkdir(parents=True, exist_ok=True)
            (cell_dir / "src/main.rs").write_text(edited, encoding="utf-8")
            shutil.copy(prog["skel"] / "Cargo.toml", cell_dir / "Cargo.toml")
            shutil.copy(prog["skel"] / "src/cir_trace.rs", cell_dir / "src/cir_trace.rs")
            edited_sids = set(SID_RE.findall(edited))
            rec = {"sha": prog["sha"], "task": prog["task"], "arm": prog["arm"], "edit": eid,
                   "orig_calls": orig_calls, "edited_calls": len(TRACE_RE.findall(edited)),
                   "sid_dropped": sorted(orig_sids - edited_sids),
                   "reinstrumented": False}
            build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=cell_dir,
                                   capture_output=True, text=True, timeout=300)
            rec["build_ok"] = build.returncode == 0
            if not rec["build_ok"]:
                rec["reason"] = "build"
                cells.append(rec)
                continue
            try:
                proc = subprocess.run([str(cell_dir / "target/debug/cir_generated")],
                                      capture_output=True, text=True, timeout=8)
                rec["behavior"] = "terminated_ok" if proc.returncode == 0 else "crash"
            except subprocess.TimeoutExpired:
                rec["behavior"] = "hang"
            tries = collect_traces(cell_dir, native_runs=8, miri_seeds=16, timeout_s=8.0,
                                   calls_dir=cell_dir / "traces", run_miri=True)
            native = [t for t in tries if t.kind == "native"]
            agg = conform_all(Path(prog["cir"]), native, binary=BIN,
                              lenient_unlock=False, attempt_events=False)
            rec["conform"] = "PASS" if agg["violation"] == 0 and agg["conformant"] == len(native) else "FAIL"
            bad = next((d for d in agg["details"] if d.get("status") != "conformant"), None)
            rec["conform_reason"] = None if rec["conform"] == "PASS" else str((bad or {}).get("status"))
            miri = [t for t in tries if t.kind == "miri"]
            rec["miri_detected"] = sum(1 for t in miri if t.hang)
            rec["drift_caught_only_by_conform"] = bool(
                rec["conform"] == "FAIL" and rec["behavior"] == "terminated_ok"
                and rec["miri_detected"] == 0)
            cells.append(rec)
    payload = {"binary_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(),
               "requests_used": budget.requests_used, "cells": cells}
    (OUT / "CELLS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    by = {}
    for eid in EDITS:
        rs = [c for c in cells if c["edit"] == eid]
        if not rs:
            continue
        by[eid] = {"cells": len(rs), "build_ok": sum(1 for r in rs if r.get("build_ok")),
                   "conform_pass": sum(1 for r in rs if r.get("conform") == "PASS"),
                   "miri_detected": sum(1 for r in rs if r.get("miri_detected")),
                   "behavior_hang": sum(1 for r in rs if r.get("behavior") == "hang"),
                   "sid_dropped": sum(1 for r in rs if r.get("sid_dropped")),
                   "drift_caught_only_by_conform": sum(1 for r in rs if r.get("drift_caught_only_by_conform"))}
    (OUT / "SUMMARY.json").write_text(json.dumps({"ops": by}, indent=1) + "\n", encoding="utf-8")
    lines = ["# post-edit-conform-v2 — SUMMARY", "",
             f"- requests: {budget.requests_used}/{MAX_REQUESTS}",
             f"- binary sha256: `{payload['binary_sha256']}`", "",
             "| edit | cells | build_ok | conform PASS | miri | hang | sid_dropped | drift-only-conform |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for eid, s in by.items():
        lines.append(f"| {eid} | {s['cells']} | {s['build_ok']} | {s['conform_pass']} | "
                     f"{s['miri_detected']} | {s['behavior_hang']} | {s['sid_dropped']} | "
                     f"{s['drift_caught_only_by_conform']} |")
    lines += ["", "## drift_caught_only_by_conform", "",
              "| task | arm | edit | reason | sid_dropped |", "| --- | --- | --- | --- | --- |"]
    for c in cells:
        if c.get("drift_caught_only_by_conform"):
            lines.append(f"| {c['task']} | {c['arm']} | {c['edit']} | {c.get('conform_reason')} | "
                         f"{c.get('sid_dropped')} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps(by, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
