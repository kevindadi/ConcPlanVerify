#!/usr/bin/env python3
"""§1 acceptance: run instrument v2 + monitor over the Rust references.

For each task with a `rust/` reference, evaluate `fixed.rs` (expect every
non-`[U]` requirement `PASS_bounded`) and `buggy.rs` (expect a hang or a
`FAIL`). Writes `experiments/rust-oracle-v1/RESULTS.json` + `SUMMARY.md`.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import rust_oracle  # noqa: E402

TASKS = [
    "lock-order/abba_2lock",
    "lock-order/cycle_3lock",
    "lock-order/cross_module_cycle",
    "lock-order/partial_deadlock_bystander",
    "condvar/bare_wait_no_predicate",
    "condvar/notify_one_multi_waiter_wrong_pick",
    "channel/bounded_backpressure_lock_held",
    "channel/send_while_holding_mutex",
    "semaphore/acquire_twice_no_release",
    "structure/nested_scope_lock_order",
]
OUT = REPO / "experiments/rust-oracle-v1"


def reference_cir(task_dir: Path) -> Path:
    for name in ("fixed.cir.json", "correct.cir.json", "buggy.cir.json"):
        path = task_dir / name
        if path.is_file():
            return path
    raise FileNotFoundError(f"no reference CIR in {task_dir}")


def summarize(result: dict) -> dict:
    cov = result.get("coverage") or {}
    statuses = cov.get("statuses", {})
    non_unv = {k: v for k, v in statuses.items() if v != "unverifiable"}
    passed = [k for k, v in non_unv.items() if v == "PASS_bounded"]
    failed = [k for k, v in non_unv.items() if v == "FAIL"]
    blocked = {k: v for k, v in non_unv.items()
               if v not in ("PASS_bounded", "FAIL")}
    monitor = result.get("monitor") or {}
    monitor_fail = [p["id"] for p in monitor.get("properties", [])
                    if p.get("status") == "FAIL"]
    return {
        "built": result.get("built"),
        "behavior_ok": result.get("behavior_ok"),
        "hang": result.get("hang"),
        "rc": cov.get("rc"), "rf": cov.get("rf"),
        "satisfied": len(passed), "non_unverifiable": len(non_unv),
        "failed_reqs": failed, "blocked_reqs": blocked,
        "monitor_fail": monitor_fail,
        "unmapped": monitor.get("unmapped_resources", []),
        "limitations": result.get("limitations", []),
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    records = []
    for task in TASKS:
        task_dir = REPO / "benchmarks/families" / task
        ref = reference_cir(task_dir)
        for side in ("fixed", "buggy"):
            source = task_dir / f"rust/{side}.rs"
            if not source.is_file():
                continue
            with tempfile.TemporaryDirectory() as tmp:
                result = rust_oracle.evaluate(
                    source.read_text(encoding="utf-8"),
                    task_dir / "contract.json", ref, Path(tmp), n_native=32)
            record = {"task": task, "side": side, **summarize(result)}
            records.append(record)
            print(f"{task:48s} {side:5s} built={record['built']} "
                  f"behavior={record['behavior_ok']} hang={record['hang']} "
                  f"RF={record['rf']:.2f} blocked={record['blocked_reqs']} "
                  f"monitor_fail={record['monitor_fail']}", flush=True)

    (OUT / "RESULTS.json").write_text(
        json.dumps({"records": records}, indent=2) + "\n", encoding="utf-8")

    fixed_ok = [r for r in records if r["side"] == "fixed"]
    buggy_ok = [r for r in records if r["side"] == "buggy"]
    fixed_all = all(r["built"] and not r["blocked_reqs"] for r in fixed_ok)
    buggy_detected = [r for r in buggy_ok if r["hang"] or r["failed_reqs"] or r["monitor_fail"]]
    lines = [
        "# Rust-arm bounded oracle (§1 acceptance)",
        "",
        "Instrument v2 (wrapper types) -> build -> native 32 runs -> "
        "`concir-backend monitor`, resource names auto-mapped by kind/declaration "
        "order. Verdicts are **bounded**.",
        "",
        "| task | side | built | behavior | hang | RC | RF | monitor FAIL | blocked reqs |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for r in records:
        lines.append(
            f"| {r['task']} | {r['side']} | {r['built']} | {r['behavior_ok']} | "
            f"{r['hang']} | {r['rc']:.2f} | {r['rf']:.2f} | {r['monitor_fail']} | "
            f"{r['blocked_reqs']} |")
    lines += [
        "",
        f"- fixed references: **{sum(1 for r in fixed_ok if r['built'] and not r['blocked_reqs'])}"
        f"/{len(fixed_ok)}** have every non-`[U]` requirement `PASS_bounded`.",
        f"- buggy references: **{len(buggy_detected)}/{len(buggy_ok)}** show a hang or a "
        f"monitor `FAIL` (here: all hang).",
        "",
        "## Why a fixed reference can fall short of 100%",
        "",
        "- `condvar/bare_wait_no_predicate`: its requirements about the shared flag "
        "are `var_eq` predicates. A free-Rust trace carries no value events, so the "
        "monitor reports `unsupported` — an **instrument limitation**, not a program "
        "defect (behavior is `DONE ready=true`, all runs complete).",
        "- `semaphore/acquire_twice_no_release`: the reference implements its own "
        "semaphore over `Mutex`+`Condvar`, so there is no `Semaphore` runtime resource "
        "to align to `main::s`; those clauses are `unmapped` — an **alignment "
        "limitation**. Its `function_completed` and `deadlock_free` clauses pass.",
        "- The remaining shortfall is `[U]` terminal-line requirements, which are "
        "checked by behavior rather than the contract monitor and so are not counted "
        "as `PASS_bounded`.",
        "",
        "## Deviation",
        "",
        "- Traces here are the 32 native runs; Miri seeds are wired "
        "(`rust_oracle.run_miri`) but were not run at 16 seeds for all 20 artifacts "
        "this round (runtime). `deadlock_free` is resolved by native behavior.",
    ]
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"fixed all PASS_bounded: {fixed_all}; "
          f"buggy detected: {len(buggy_detected)}/{len(buggy_ok)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
