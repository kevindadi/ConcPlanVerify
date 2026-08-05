"""Parametric CIR generators and the scaling sweep runner.

Three patterns, all statically valid by construction:

- ``lock_chain``       — safe: every worker takes ``locks`` mutexes in the
                         global order and releases them in reverse.
- ``lock_chain_buggy`` — same, but the last worker locks in reverse order,
                         creating a circular wait (Deadlock).
- ``branch_fan``       — safe: every worker writes its id to a shared int and
                         then walks ``branches`` chained two-way branches on
                         that value (both arms stay live across schedules).

The sweep runs ``cir2cvn --analyze`` for every (pattern, threads, locks/branches)
point and records CIR size, CVN size, state counts, and stage timings —
the raw data for locating the state-explosion knee.

Usage (from the repository root):

    PYTHONPATH=python python -m cir_workflow.scaling --out results/scaling.json
    PYTHONPATH=python python -m cir_workflow.scaling --emit-case lock_chain,4,3
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .metrics import cir_metrics
from .rust_cli import RustCli


def _worker_lock_chain(locks: list[str], reverse: bool) -> list[dict[str, Any]]:
    order = list(reversed(locks)) if reverse else list(locks)
    body: list[dict[str, Any]] = []
    sid = 0
    for lock in order:
        sid += 1
        body.append({"sid": f"s{sid}", "op": ["res_op", lock, "lock"], "transfer": ["next", f"s{sid + 1}"]})
    for lock in reversed(order):
        sid += 1
        body.append({"sid": f"s{sid}", "op": ["res_op", lock, "drop"], "transfer": ["next", f"s{sid + 1}"]})
    body.append({"sid": f"s{sid + 1}", "op": "return", "transfer": "return"})
    return body


def _main_body(workers: list[str]) -> list[dict[str, Any]]:
    body: list[dict[str, Any]] = []
    sid = 0
    for w in workers:
        sid += 1
        body.append({"sid": f"s{sid}", "op": ["spawn", w], "transfer": ["next", f"s{sid + 1}"]})
    for w in workers:
        sid += 1
        body.append({"sid": f"s{sid}", "op": ["join", w], "transfer": ["next", f"s{sid + 1}"]})
    body.append({"sid": f"s{sid + 1}", "op": "return", "transfer": "return"})
    return body


def lock_chain(threads: int, locks: int, *, buggy: bool = False) -> dict[str, Any]:
    """N workers each taking M mutexes; `buggy` reverses the last worker's order."""

    lock_names = [f"m{i + 1}" for i in range(locks)]
    workers = [f"w{i + 1}" for i in range(threads)]
    functions = [{"name": "main", "kind": "normal", "body": _main_body(workers)}]
    for index, w in enumerate(workers):
        reverse = buggy and index == threads - 1
        functions.append({
            "name": w,
            "kind": "closure",
            "body": _worker_lock_chain(lock_names, reverse),
        })
    return {
        "program": f"scale_lock_chain_{threads}x{locks}{'_buggy' if buggy else ''}",
        "resources": [
            {"name": name, "kind": "sync", "type": "Mutex", "mode": "Sync"}
            for name in lock_names
        ],
        "protection": [],
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
    }


def branch_fan(threads: int, branches: int) -> dict[str, Any]:
    """N workers writing their id to a shared int, then B chained branches on it."""

    workers = [f"w{i + 1}" for i in range(threads)]
    functions = [{"name": "main", "kind": "normal", "body": _main_body(workers)}]
    for index, w in enumerate(workers):
        wid = index + 1
        body: list[dict[str, Any]] = [
            {"sid": "s1", "op": ["res_op", "x", "write", str(wid)], "transfer": ["next", "s2"]},
        ]
        sid = 2
        for level in range(branches):
            read, true_t, false_t, nxt = sid, sid + 1, sid + 2, sid + 3
            body.extend([
                {"sid": f"s{read}", "op": ["res_op", "x", "read"],
                 "transfer": ["branch", f"x == {wid}", f"s{true_t}", f"s{false_t}"]},
                {"sid": f"s{true_t}", "op": "nop", "transfer": ["next", f"s{nxt}"]},
                {"sid": f"s{false_t}", "op": "nop", "transfer": ["next", f"s{nxt}"]},
            ])
            sid = nxt
        body.append({"sid": f"s{sid}", "op": "return", "transfer": "return"})
        functions.append({"name": w, "kind": "closure", "body": body})
    return {
        "program": f"scale_branch_fan_{threads}x{branches}",
        "resources": [
            {"name": "x", "kind": "var", "type": "Var", "base": "Int", "init": 0},
        ],
        "protection": [],
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
    }


def lock_chain_deep(threads: int, locks: int, *, buggy: bool = True) -> dict[str, Any]:
    """Deeply buried lock-order bug for the repair/judge experiments.

    Every worker writes its id to a shared flag, branches on it, and takes
    all mutexes in the global order on BOTH arms — except (when ``buggy``)
    one worker's second arm swaps the first two locks. The defect is a
    single adjacent transposition inside one branch arm of one of many
    near-identical workers, reachable only in schedules that take that arm
    while another worker holds m1.
    """

    lock_names = [f"m{i + 1}" for i in range(locks)]
    workers = [f"w{i + 1}" for i in range(threads)]
    culprit = threads // 2  # a middle worker, not first or last
    functions = [{"name": "main", "kind": "normal", "body": _main_body(workers)}]
    for index, w in enumerate(workers):
        wid = index + 1
        arm_a = list(lock_names)
        arm_b = list(lock_names)
        if buggy and index == culprit:
            # Arm A skips m2 entirely; arm B hoists m2 to the front. The two
            # arms share no conflicting pair (passes intra-function E505),
            # but arm B's m2-before-m1 conflicts with every other worker's
            # global m1-before-m2 order: a cross-function circular wait.
            arm_a = [lock_names[0]] + lock_names[2:]
            arm_b = [lock_names[1], lock_names[0]] + lock_names[2:]

        body: list[dict[str, Any]] = [
            {"sid": "s1", "op": ["res_op", "flag", "write", str(wid)], "transfer": ["next", "s2"]},
        ]
        sid = 3
        arm_starts: list[str] = []
        arm_stmts: list[dict[str, Any]] = []
        ret_sid = f"s{3 + 2 * (len(arm_a) + len(arm_b))}"
        for order in (arm_a, arm_b):
            arm_starts.append(f"s{sid}")
            seq = [(name, "lock") for name in order] + [
                (name, "drop") for name in reversed(order)
            ]
            for pos, (name, action) in enumerate(seq):
                nxt = f"s{sid + 1}" if pos < len(seq) - 1 else ret_sid
                arm_stmts.append({
                    "sid": f"s{sid}",
                    "op": ["res_op", name, action],
                    "transfer": ["next", nxt],
                })
                sid += 1
        body.append({
            "sid": "s2",
            "op": ["res_op", "flag", "read"],
            "transfer": ["branch", f"flag == {wid}", arm_starts[0], arm_starts[1]],
        })
        body.extend(arm_stmts)
        body.append({"sid": ret_sid, "op": "return", "transfer": "return"})
        functions.append({"name": w, "kind": "closure", "body": body})

    return {
        "program": f"deep_lock_chain_{threads}x{locks}{'' if buggy else '_safe'}",
        "resources": [
            {"name": name, "kind": "sync", "type": "Mutex", "mode": "Sync"}
            for name in lock_names
        ] + [
            {"name": "flag", "kind": "var", "type": "Var", "base": "Int", "init": 0},
        ],
        "protection": [],
        "functions": functions,
        "fn_summaries": [],
        "entry": "main",
    }


def build(pattern: str, threads: int, size: int) -> dict[str, Any]:
    if pattern == "lock_chain":
        return lock_chain(threads, size)
    if pattern == "lock_chain_buggy":
        return lock_chain(threads, size, buggy=True)
    if pattern == "branch_fan":
        return branch_fan(threads, size)
    if pattern == "lock_chain_deep":
        return lock_chain_deep(threads, size)
    if pattern == "lock_chain_deep_safe":
        return lock_chain_deep(threads, size, buggy=False)
    raise ValueError(f"unknown pattern: {pattern}")


DEFAULT_SWEEP: list[tuple[str, list[int], list[int]]] = [
    # (pattern, thread counts, size counts) — size = locks or branch levels
    ("lock_chain", [2, 3, 4, 5, 6], [1, 2, 3]),
    ("lock_chain_buggy", [2, 3, 4, 5], [2, 3]),
    ("branch_fan", [2, 3, 4, 5], [1, 2, 3]),
]


def run_sweep(rust_cli: RustCli, sweep=DEFAULT_SWEEP) -> list[dict[str, Any]]:
    points: list[dict[str, Any]] = []
    for pattern, thread_counts, sizes in sweep:
        for threads in thread_counts:
            for size in sizes:
                program = build(pattern, threads, size)
                cir_json = json.dumps(program, ensure_ascii=False, indent=2)
                started = time.perf_counter()
                result = rust_cli.analyze(cir_json)
                wall_ms = (time.perf_counter() - started) * 1000
                payload = result.payload or {}
                bugs = payload.get("bugs") or []
                point = {
                    "pattern": pattern,
                    "threads": threads,
                    "size": size,
                    "status": result.status,
                    "bug_kinds": sorted({
                        next(iter(b.get("kind")), None) if isinstance(b.get("kind"), dict) else b.get("kind")
                        for b in bugs
                    } - {None}) if bugs else [],
                    "cir_metrics": cir_metrics(cir_json),
                    "places": payload.get("places"),
                    "transitions": payload.get("transitions"),
                    "input_arcs": payload.get("input_arcs"),
                    "output_arcs": payload.get("output_arcs"),
                    "state_count": payload.get("state_count"),
                    "analysis_complete": payload.get("analysis_complete"),
                    "max_states": payload.get("max_states"),
                    "timings": payload.get("timings"),
                    "wall_ms": wall_ms,
                }
                points.append(point)
                print(
                    f"[scaling] {pattern} threads={threads} size={size} -> "
                    f"{result.status} states={payload.get('state_count')} "
                    f"({wall_ms:.0f} ms)",
                    file=sys.stderr,
                )
    return points


def main() -> int:
    parser = argparse.ArgumentParser(description="CIR scaling sweep")
    parser.add_argument("--out", help="sweep output JSON path")
    parser.add_argument(
        "--emit-case",
        help="write one generated CIR to benchmarks/cir/: pattern,threads,size",
    )
    parser.add_argument("--binary", help="path to the cir2cvn binary")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]

    if args.emit_case:
        pattern, threads, size = args.emit_case.split(",")
        program = build(pattern.strip(), int(threads), int(size))
        name = program["program"]
        out_path = repo_root / "benchmarks" / "cir" / name / "buggy.json"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(
            json.dumps(program, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"[scaling] wrote {out_path}", file=sys.stderr)
        return 0

    rust_cli = RustCli(repo_root=repo_root, binary=args.binary)
    points = run_sweep(rust_cli)
    output = {
        "meta": {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "sweep": [
                {"pattern": p, "threads": t, "sizes": s} for p, t, s in DEFAULT_SWEEP
            ],
        },
        "points": points,
    }
    text = json.dumps(output, ensure_ascii=False, indent=2)
    if args.out:
        out_path = (repo_root / args.out).resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(text, encoding="utf-8")
        print(f"[scaling] wrote {out_path}", file=sys.stderr)
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
