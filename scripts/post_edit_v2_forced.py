"""§A2: mark no_edit cells, then one forced-edit retry for the E2/E3 no-op cells
and re-run conform v2."""

from __future__ import annotations

import difflib
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from collections import Counter
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
MAX_REQUESTS = int(os.environ.get("FORCED_MAX_REQUESTS", "60"))
SYNC_RE = re.compile(r'cir_trace::(\w+)\([^;]*?"(s\d+|L\d+)"')

FORCED = {
    "E2": ("Refactor for readability. You MUST add at least one new top-level "
           "`fn` and call it from a thread body. Before answering, check that "
           "the file differs from the input. Output the complete program."),
    "E3": ("Optimise. You MUST change at least one line related to a lock, "
           "condvar, channel or semaphore (e.g. shorten a critical section, "
           "merge/split it, change a notification), keeping the program output "
           "identical. Before answering, check that the file differs from the "
           "input. Output the complete program."),
}


def _changed(orig, edited):
    return len([l for l in difflib.unified_diff(orig.splitlines(), edited.splitlines(),
                                                lineterm="")
                if l.startswith(("+", "-")) and not l.startswith(("+++", "---"))])


def _sync_sig(src):
    return [(m.group(1), m.group(2)) for m in SYNC_RE.finditer(src)]


def _extract(text):
    if "```" in text:
        cands = text.split("```")[1::2]
        best = max(cands, key=len)
        lines = best.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        return "\n".join(lines).strip() + "\n"
    return text.strip() + "\n"


def _run(cell_dir, cir):
    build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=cell_dir,
                           capture_output=True, text=True, timeout=300)
    rec = {"build_ok": build.returncode == 0}
    if not rec["build_ok"]:
        rec["reason"] = "build"
        return rec
    try:
        proc = subprocess.run([str(cell_dir / "target/debug/cir_generated")],
                              capture_output=True, text=True, timeout=8)
        rec["behavior"] = "terminated_ok" if proc.returncode == 0 else "crash"
    except subprocess.TimeoutExpired:
        rec["behavior"] = "hang"
    tries = collect_traces(cell_dir, native_runs=8, miri_seeds=16, timeout_s=8.0,
                           calls_dir=cell_dir / "traces", run_miri=True)
    native = [t for t in tries if t.kind == "native"]
    agg = conform_all(Path(cir), native, binary=BIN, lenient_unlock=False,
                      attempt_events=False)
    rec["conform"] = "PASS" if agg["violation"] == 0 and agg["conformant"] == len(native) else "FAIL"
    bad = next((d for d in agg["details"] if d.get("status") != "conformant"), None)
    rec["conform_reason"] = None if rec["conform"] == "PASS" else str((bad or {}).get("status"))
    miri = [t for t in tries if t.kind == "miri"]
    rec["miri_detected"] = sum(1 for t in miri if t.hang)
    rec["drift_caught_only_by_conform"] = bool(
        rec["conform"] == "FAIL" and rec["behavior"] == "terminated_ok"
        and rec["miri_detected"] == 0)
    return rec


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    payload = json.loads((OUT / "CELLS.json").read_text())
    main_b = json.loads((BATCH / "SUMMARY.json").read_text())
    cir_by_task = {}
    for rep in main_b["reps"]:
        for t in rep["tasks"]:
            for arm in ("A3_local", "A3_whole", "A3_tiered"):
                r = (t["arms"] or {}).get(arm) or {}
                if r.get("accepted") and r.get("final_cir"):
                    cir_by_task[hashlib.sha256(Path(r["final_cir"]).read_bytes()).hexdigest()] = r["final_cir"]
    run = sorted(A3.glob("run-*"))[-1]
    # mark no_edit and collect no-op E2/E3
    budget = LiveBudget(budget_path=OUT / "budget_forced.json", max_requests=MAX_REQUESTS)
    system = (REPO / "prompts/post_edit_v2.md").read_text(encoding="utf-8")
    retried = 0
    for cell in payload["cells"]:
        if not cell.get("build_ok"):
            continue
        skel = run / f"{cell['task'].replace('/', '__')}__{cell['arm']}__{cell['sha'][:12]}/skeleton/src/main.rs"
        if not skel.is_file():
            continue
        orig = skel.read_text(encoding="utf-8")
        edited_path = OUT / "cells" / cell["sha"][:12] / cell["edit"] / "src/main.rs"
        edited = edited_path.read_text(encoding="utf-8")
        cell["changed_lines"] = _changed(orig, edited)
        cell["no_edit"] = cell["changed_lines"] == 0
        cell["sync_calls_moved"] = sum(1 for a, b in zip(_sync_sig(orig), _sync_sig(edited)) if a != b)
        if not (cell["no_edit"] and cell["edit"] in ("E2", "E3")):
            continue
        if budget.exhausted():
            cell["no_edit_after_retry"] = True
            continue
        llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                  evidence_dir=OUT / "llm", timeout=90.0, max_tokens=4096)
        try:
            outcome = llm.complete(system, f"Task: {FORCED[cell['edit']]}\n\nProgram:\n```rust\n{orig}\n```")
            new = _extract(outcome.text)
        except Exception as exc:  # noqa: BLE001
            cell["no_edit_after_retry"] = True
            cell["retry_error"] = str(exc)[:80]
            continue
        rdir = OUT / "forced" / cell["sha"][:12] / cell["edit"]
        (rdir / "src").mkdir(parents=True, exist_ok=True)
        (rdir / "src/main.rs").write_text(new, encoding="utf-8")
        shutil.copy(skel.parent.parent / "Cargo.toml", rdir / "Cargo.toml")
        shutil.copy(skel.parent / "cir_trace.rs", rdir / "src/cir_trace.rs")
        cir = cir_by_task.get(cell["sha"])
        rec = _run(rdir, cir) if cir else {"build_ok": False, "reason": "no_cir"}
        cell.update({"retry": rec, "retry_changed_lines": _changed(orig, new),
                     "retry_sync_calls_moved": sum(1 for a, b in zip(_sync_sig(orig), _sync_sig(new)) if a != b),
                     "no_edit_after_retry": _changed(orig, new) == 0})
        retried += 1
    (OUT / "CELLS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    # summary with no_edit denominators
    by = {}
    for eid in ("E1", "E2", "E3"):
        rs = [c for c in payload["cells"] if c["edit"] == eid]
        real = [c for c in rs if not c.get("no_edit")]
        rr = [c for c in rs if c.get("retry")]
        by[eid] = {
            "cells": len(rs), "no_edit": sum(1 for c in rs if c.get("no_edit")),
            "real_edits": len(real),
            "conform_pass_real": sum(1 for c in real if c.get("conform") == "PASS"),
            "drift_only_conform_real": sum(1 for c in real if c.get("drift_caught_only_by_conform")),
            "retried": len(rr),
            "retry_real": sum(1 for c in rr if not c.get("no_edit_after_retry")),
            "retry_conform_pass": sum(1 for c in rr if (c.get("retry") or {}).get("conform") == "PASS"),
            "retry_drift_only_conform": sum(1 for c in rr if (c.get("retry") or {}).get("drift_caught_only_by_conform")),
        }
    (OUT / "SUMMARY.json").write_text(json.dumps({"ops": by, "binary_sha256": payload["binary_sha256"]},
                                                 indent=1) + "\n", encoding="utf-8")
    lines = ["# post-edit-conform-v2 — SUMMARY", "",
             f"- binary sha256: `{payload['binary_sha256']}`",
             "- `no_edit` = 0 changed lines; excluded from the conform/drift denominator.", "",
             "| edit | cells | no_edit | real edits | conform PASS (real) | drift-only (real) | retried | retry real | retry conform PASS | retry drift-only |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for eid, s in by.items():
        lines.append(f"| {eid} | {s['cells']} | {s['no_edit']} | {s['real_edits']} | "
                     f"{s['conform_pass_real']} | {s['drift_only_conform_real']} | "
                     f"{s['retried']} | {s['retry_real']} | {s['retry_conform_pass']} | "
                     f"{s['retry_drift_only_conform']} |")
    lines += ["", "## Forced-edit retry (E2/E3 no-op cells)", "",
              "E2 retry must add and call a `fn`; E3 retry must change sync-related "
              "code while keeping output identical.", ""]
    for eid in ("E2", "E3"):
        rr = [c for c in payload["cells"] if c["edit"] == eid and c.get("retry")]
        lines.append(f"### {eid} ({len(rr)} retried)")
        lines.append("")
        lines.append("| task | arm | changed_lines | sync_calls_moved | conform | reason |")
        lines.append("| --- | --- | --- | --- | --- | --- |")
        for c in rr:
            r = c.get("retry") or {}
            lines.append(f"| {c['task']} | {c['arm']} | {c.get('retry_changed_lines')} | "
                         f"{c.get('retry_sync_calls_moved')} | {r.get('conform')} | "
                         f"{r.get('conform_reason')} |")
        lines.append("")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"retried": retried, "by": by}, indent=2)[:1200])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
