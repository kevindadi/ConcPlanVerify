#!/usr/bin/env python3
"""Generate the frozen CIR fixtures for the hard diagnostic ablation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "paper/rebuttal-experiments/diagnostic-hard-fixtures"


def statement(sid: int, op: Any, next_sid: int) -> dict[str, Any]:
    return {"sid": f"s{sid}", "op": op, "transfer": ["next", f"s{next_sid}"]}


def return_statement(sid: int) -> dict[str, Any]:
    return {"sid": f"s{sid}", "op": "return", "transfer": "return"}


def resource(name: str, kind: str, **extra: Any) -> dict[str, Any]:
    return {"name": name, "kind": "sync", "type": kind, "mode": "Sync", **extra}


def function(name: str, operations: list[Any]) -> dict[str, Any]:
    body = [statement(index, op, index + 1) for index, op in enumerate(operations, 1)]
    body.append(return_statement(len(operations) + 1))
    return {"name": name, "kind": "closure", "body": body}


def lock_worker(name: str, order: list[str]) -> dict[str, Any]:
    operations = [["res_op", lock, "lock"] for lock in order]
    operations.extend(["res_op", lock, "drop"] for lock in reversed(order))
    return function(name, operations)


def channel_workers(prefix: str, *, fixed: bool) -> list[dict[str, Any]]:
    mutex = f"{prefix}_m"
    channel = f"{prefix}_ch"
    sender = function(
        f"{prefix}_tx",
        [
            ["res_op", mutex, "lock"],
            ["res_op", channel, "send", "7"],
            ["res_op", mutex, "drop"],
        ],
    )
    receiver_ops = [
        ["res_op", mutex, "lock"],
        ["res_op", channel, "recv"],
        ["res_op", mutex, "drop"],
    ]
    if fixed:
        receiver_ops = [receiver_ops[1], receiver_ops[0], receiver_ops[2]]
    return [sender, function(f"{prefix}_rx", receiver_ops)]


def semaphore_workers(prefix: str, *, fixed: bool) -> list[dict[str, Any]]:
    lock_a = f"{prefix}_a"
    lock_b = f"{prefix}_b"
    sem_a = f"{prefix}_sa"
    sem_b = f"{prefix}_sb"
    worker_a = [
        ["res_op", lock_a, "lock"],
        ["res_op", sem_a, "release"],
        ["res_op", sem_b, "acquire"],
        ["res_op", lock_b, "lock"],
        ["res_op", lock_b, "drop"],
        ["res_op", lock_a, "drop"],
    ]
    worker_b = [
        ["res_op", lock_b, "lock"],
        ["res_op", sem_b, "release"],
        ["res_op", sem_a, "acquire"],
        ["res_op", lock_a, "lock"],
        ["res_op", lock_a, "drop"],
        ["res_op", lock_b, "drop"],
    ]
    if fixed:
        worker_a = [worker_a[1], worker_a[2], worker_a[0], worker_a[3], worker_a[4], worker_a[5]]
        worker_b = [worker_b[1], worker_b[2], worker_b[3], worker_b[0], worker_b[5], worker_b[4]]
    return [function(f"{prefix}_wa", worker_a), function(f"{prefix}_wb", worker_b)]


def main_function(stages: list[list[str]]) -> dict[str, Any]:
    operations: list[Any] = []
    for stage in stages:
        operations.extend(["spawn", name] for name in stage)
        operations.extend(["join", name] for name in stage)
    body = [statement(index, op, index + 1) for index, op in enumerate(operations, 1)]
    body.append(return_statement(len(operations) + 1))
    return {"name": "main", "kind": "normal", "body": body}


def program(
    name: str,
    resources: list[dict[str, Any]],
    functions: list[dict[str, Any]],
    stages: list[list[str]],
    goals: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    return {
        "program": name,
        "resources": resources,
        "protection": [],
        "functions": [main_function(stages), *functions],
        "fn_summaries": [],
        "entry": "main",
        "goals": goals or [],
    }


def dense_lock_graph(fixed: bool) -> dict[str, Any]:
    orders = [
        ["dlg_m0", "dlg_m1", "dlg_m2"],
        ["dlg_m1", "dlg_m2", "dlg_m3"],
        ["dlg_m2", "dlg_m3", "dlg_m4"],
        ["dlg_m3", "dlg_m4", "dlg_m0"],
    ]
    if fixed:
        orders[-1] = ["dlg_m0", "dlg_m3", "dlg_m4"]
    names = [f"dlg_w{index}" for index in range(4)]
    return program(
        "diagnostic_dense_lock_graph",
        [resource(f"dlg_m{index}", "Mutex") for index in range(5)],
        [lock_worker(name, order) for name, order in zip(names, orders)],
        [names],
    )


def partial_semaphore_goals(fixed: bool) -> dict[str, Any]:
    prefix = "psg"
    names = [f"{prefix}_wa", f"{prefix}_wb"]
    goals = [
        {
            "id": f"g_{name}_returns",
            "desc": f"{name} must reach its return place",
            "marking": {f"{name}.ret": 1},
        }
        for name in names
    ]
    return program(
        "diagnostic_partial_semaphore_goals",
        [
            resource(f"{prefix}_a", "Mutex"),
            resource(f"{prefix}_b", "Mutex"),
            resource(f"{prefix}_sa", "Semaphore", count=0),
            resource(f"{prefix}_sb", "Semaphore", count=0),
        ],
        semaphore_workers(prefix, fixed=fixed),
        [names],
        goals,
    )


def staged_lock_cycles(fixed: bool) -> dict[str, Any]:
    resources: list[dict[str, Any]] = []
    functions: list[dict[str, Any]] = []
    stages: list[list[str]] = []
    for stage in range(3):
        locks = [f"slc_{stage}_m{index}" for index in range(3)]
        names = [f"slc_{stage}_w{index}" for index in range(3)]
        orders = [[locks[0], locks[1]], [locks[1], locks[2]], [locks[2], locks[0]]]
        if fixed:
            orders[-1] = [locks[0], locks[2]]
        resources.extend(resource(lock, "Mutex") for lock in locks)
        functions.extend(lock_worker(name, order) for name, order in zip(names, orders))
        stages.append(names)
    return program("diagnostic_staged_lock_cycles", resources, functions, stages)


def mixed_three_stage(fixed: bool) -> dict[str, Any]:
    channel_names = ["mix_ch_tx", "mix_ch_rx"]
    lock_names = [f"mix_l_w{index}" for index in range(3)]
    sem_names = ["mix_s_wa", "mix_s_wb"]
    lock_orders = [
        ["mix_l_m0", "mix_l_m1"],
        ["mix_l_m1", "mix_l_m2"],
        ["mix_l_m2", "mix_l_m0"],
    ]
    if fixed:
        lock_orders[-1] = ["mix_l_m0", "mix_l_m2"]
    functions = channel_workers("mix_ch", fixed=fixed)
    functions.extend(lock_worker(name, order) for name, order in zip(lock_names, lock_orders))
    functions.extend(semaphore_workers("mix_s", fixed=fixed))
    goals = [
        {
            "id": f"g_{name}_returns",
            "desc": f"{name} must reach its return place",
            "marking": {f"{name}.ret": 1},
        }
        for name in sem_names
    ]
    resources = [
        resource("mix_ch_m", "Mutex"),
        resource("mix_ch_ch", "Channel", base="Int"),
        *[resource(f"mix_l_m{index}", "Mutex") for index in range(3)],
        resource("mix_s_a", "Mutex"),
        resource("mix_s_b", "Mutex"),
        resource("mix_s_sa", "Semaphore", count=0),
        resource("mix_s_sb", "Semaphore", count=0),
    ]
    return program(
        "diagnostic_mixed_three_stage",
        resources,
        functions,
        [channel_names, lock_names, sem_names],
        goals,
    )


FIXTURES = {
    "dense_lock_graph": dense_lock_graph,
    "partial_semaphore_goals": partial_semaphore_goals,
    "staged_lock_cycles": staged_lock_cycles,
    "mixed_three_stage": mixed_three_stage,
}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, builder in FIXTURES.items():
        for variant, fixed in (("initial", False), ("reference", True)):
            path = OUT / f"{name}-{variant}.json"
            path.write_text(json.dumps(builder(fixed), indent=2) + "\n", encoding="utf-8")
            print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()
