#!/usr/bin/env python3
"""Build the interference-heavy CIR used by the repeated feedback ablation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "paper/rebuttal-experiments/diagnostic-repeated-fixtures"


def resource(name: str, resource_type: str, **extra: Any) -> dict[str, Any]:
    return {
        "name": name,
        "kind": "sync",
        "type": resource_type,
        "mode": "Sync",
        **extra,
    }


def statements(operations: list[Any], *, start: int = 1) -> list[dict[str, Any]]:
    body = []
    for offset, operation in enumerate(operations):
        sid = start + offset
        body.append(
            {
                "sid": f"s{sid}",
                "op": operation,
                "transfer": ["next", f"s{sid + 1}"],
            }
        )
    return_sid = start + len(operations)
    body.append({"sid": f"s{return_sid}", "op": "return", "transfer": "return"})
    return body


def function(name: str, operations: list[Any], *, kind: str = "closure") -> dict[str, Any]:
    return {"name": name, "kind": kind, "body": statements(operations)}


def lock_worker(
    name: str,
    locks: list[str],
    *,
    pre_call: str,
    inner_call: str,
    post_call: str,
) -> dict[str, Any]:
    operations: list[Any] = [["call", pre_call]]
    operations.extend(["res_op", lock, "lock"] for lock in locks)
    operations.append(["call", inner_call])
    operations.extend(["res_op", lock, "drop"] for lock in reversed(locks))
    operations.append(["call", post_call])
    return function(name, operations)


def semaphore_worker(name: str, semaphore: str, mutex: str, call: str) -> dict[str, Any]:
    return function(
        name,
        [
            ["call", "decode_envelope"],
            ["res_op", semaphore, "acquire"],
            ["res_op", mutex, "lock"],
            ["call", call],
            ["res_op", mutex, "drop"],
            ["res_op", semaphore, "release"],
            ["call", "record_metric"],
        ],
    )


def coordinator(name: str, children: list[str], calls: tuple[str, str]) -> dict[str, Any]:
    operations: list[Any] = [["call", calls[0]]]
    operations.extend(["spawn", child] for child in children)
    operations.append(["call", "record_metric"])
    operations.extend(["join", child] for child in reversed(children))
    operations.append(["call", calls[1]])
    return function(name, operations)


def staged_coordinator(
    name: str, children: list[str], calls: tuple[str, str]
) -> dict[str, Any]:
    operations: list[Any] = [["call", calls[0]]]
    for child in children:
        operations.extend((["spawn", child], ["join", child]))
        operations.append(["call", "record_metric"])
    operations.append(["call", calls[1]])
    return function(name, operations)


def build(*, fixed: bool) -> dict[str, Any]:
    resources = [resource(f"r{index:02d}", "Mutex") for index in range(18)]
    resources.extend(
        [
            resource("gate_a", "Semaphore", count=2),
            resource("gate_b", "Semaphore", count=3),
        ]
    )

    functions: list[dict[str, Any]] = []

    # Safe lock-order regions. Their names and shapes intentionally resemble the
    # defective region, so localization cannot rely on a conspicuous label.
    region_specs = {
        "route_a": [
            ("unit_a0", ["r00", "r01"]),
            ("unit_a1", ["r01", "r02"]),
            ("unit_a2", ["r02", "r03"]),
        ],
        "route_b": [
            ("unit_b0", ["r04", "r05"]),
            ("unit_b1", ["r05", "r06"]),
            ("unit_b2", ["r04", "r06"]),
        ],
        "route_c": [
            ("unit_c0", ["r07", "r08"]),
            ("unit_c1", ["r08", "r09"]),
            ("unit_c2", ["r07", "r09"]),
        ],
        "route_d": [
            ("unit_d0", ["r10", "r11"]),
            ("unit_d1", ["r11", "r12"]),
            # The only defect is the order below. The reference keeps the same
            # resources, SIDs, and operation multiset but orders r10 before r12.
            ("unit_d2", ["r10", "r12"] if fixed else ["r12", "r10"]),
        ],
    }
    call_sets = [
        ("decode_envelope", "normalize_record", "record_metric"),
        ("read_policy", "apply_policy", "record_metric"),
        ("decode_envelope", "validate_record", "flush_audit"),
    ]
    for route_index, (route, workers) in enumerate(region_specs.items()):
        names = []
        for worker_index, (name, locks) in enumerate(workers):
            calls = call_sets[(route_index + worker_index) % len(call_sets)]
            functions.append(
                lock_worker(
                    name,
                    locks,
                    pre_call=calls[0],
                    inner_call=calls[1],
                    post_call=calls[2],
                )
            )
            names.append(name)
        functions.append(coordinator(route, names, ("open_batch", "close_batch")))

    semaphore_names = ["unit_e0", "unit_e1", "unit_e2", "unit_e3"]
    for index, name in enumerate(semaphore_names):
        functions.append(
            semaphore_worker(
                name,
                "gate_a" if index < 2 else "gate_b",
                f"r{13 + index:02d}",
                "apply_policy" if index % 2 == 0 else "validate_record",
            )
        )
    functions.append(
        coordinator("route_e", semaphore_names, ("open_batch", "close_batch"))
    )

    # A second level in the spawn/join tree groups otherwise independent routes.
    functions.extend(
        [
            staged_coordinator(
                "dispatch_left", ["route_a", "route_b"], ("load_plan", "seal_plan")
            ),
            staged_coordinator(
                "dispatch_right", ["route_c", "route_d"], ("load_plan", "seal_plan")
            ),
        ]
    )

    main_operations: list[Any] = [
        ["call", "bootstrap_context"],
        ["spawn", "dispatch_left"],
        ["join", "dispatch_left"],
        ["call", "checkpoint_context"],
        ["spawn", "dispatch_right"],
        ["join", "dispatch_right"],
        ["call", "checkpoint_context"],
        ["spawn", "route_e"],
        ["join", "route_e"],
        ["call", "finalize_context"],
    ]
    main = function("main", main_operations, kind="normal")

    summaries = [
        {
            "name": "bootstrap_context",
            "reads": [],
            "writes": [],
            "callees": ["load_plan", "decode_envelope"],
            "has_concurrency": False,
        },
        {
            "name": "checkpoint_context",
            "reads": [],
            "writes": [],
            "callees": ["record_metric", "flush_audit"],
            "has_concurrency": False,
        },
        {
            "name": "finalize_context",
            "reads": [],
            "writes": [],
            "callees": ["seal_plan", "close_batch"],
            "has_concurrency": False,
        },
        {
            "name": "load_plan",
            "reads": [],
            "writes": [],
            "callees": ["read_policy"],
            "has_concurrency": False,
        },
        {
            "name": "seal_plan",
            "reads": [],
            "writes": [],
            "callees": ["flush_audit"],
            "has_concurrency": False,
        },
        {
            "name": "open_batch",
            "reads": [],
            "writes": [],
            "callees": ["decode_envelope", "normalize_record"],
            "has_concurrency": False,
        },
        {
            "name": "close_batch",
            "reads": [],
            "writes": [],
            "callees": ["record_metric"],
            "has_concurrency": False,
        },
    ]
    for name, callees in (
        ("decode_envelope", ["validate_record"]),
        ("normalize_record", ["validate_record"]),
        ("read_policy", []),
        ("apply_policy", ["read_policy"]),
        ("validate_record", []),
        ("record_metric", []),
        ("flush_audit", ["record_metric"]),
    ):
        summaries.append(
            {
                "name": name,
                "reads": [],
                "writes": [],
                "callees": callees,
                "has_concurrency": False,
            }
        )

    goals = [
        {
            "id": "g_dispatch_right_returns",
            "desc": "the right dispatch subtree must complete",
            "marking": {"dispatch_right.ret": 1},
        },
        {
            "id": "g_main_returns",
            "desc": "the complete processing pipeline must return",
            "marking": {"main.ret": 1},
        },
    ]
    return {
        "program": "interference_call_tree",
        "resources": resources,
        "protection": [],
        "functions": [main, *functions],
        "fn_summaries": summaries,
        "entry": "main",
        "goals": goals,
    }


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for variant, fixed in (("initial", False), ("reference", True)):
        path = OUT / f"interference_call_tree-{variant}.json"
        path.write_text(json.dumps(build(fixed=fixed), indent=2) + "\n", encoding="utf-8")
        print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()
