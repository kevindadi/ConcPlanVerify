"""ConcIR size metrics for experiment reporting.

Computes the structural size of one ConcIR ``Program`` JSON document. The
fields follow the experiment design in ``todo.md`` so every analyze /
generate / repair round can attach a comparable size record.
"""

from __future__ import annotations

import json
from typing import Any


def cir_metrics(cir_json: str) -> dict[str, Any]:
    """Return the size metrics for one ConcIR JSON document.

    Raises ``ValueError`` when the document is not a JSON object; individual
    missing sections simply count as zero so the collector also works on
    partially malformed LLM candidates.
    """

    program = json.loads(cir_json)
    if not isinstance(program, dict):
        raise ValueError("ConcIR JSON root must be an object")

    resources = _as_list(program.get("resources"))
    functions = _as_list(program.get("functions"))

    resource_by_type: dict[str, int] = {}
    for resource in resources:
        if isinstance(resource, dict):
            res_type = str(resource.get("type", "unknown"))
            resource_by_type[res_type] = resource_by_type.get(res_type, 0) + 1

    function_by_kind: dict[str, int] = {}
    statements_per_function: list[int] = []
    op_counts = {"spawn": 0, "spawn_async": 0, "join": 0, "await": 0, "call": 0}
    branch_count = 0
    switch_count = 0
    statement_count = 0

    for function in functions:
        if not isinstance(function, dict):
            continue
        kind = str(function.get("kind", "unknown"))
        function_by_kind[kind] = function_by_kind.get(kind, 0) + 1

        body = _as_list(function.get("body"))
        statements_per_function.append(len(body))
        statement_count += len(body)

        for statement in body:
            if not isinstance(statement, dict):
                continue
            op = statement.get("op")
            if isinstance(op, list) and op and op[0] in op_counts:
                op_counts[str(op[0])] += 1
            transfer = statement.get("transfer")
            if isinstance(transfer, list) and transfer:
                if transfer[0] == "branch":
                    branch_count += 1
                elif transfer[0] == "switch":
                    switch_count += 1

    return {
        "resource_count": len(resources),
        "resource_by_type": resource_by_type,
        "function_count": len(functions),
        "function_by_kind": function_by_kind,
        "statement_count": statement_count,
        "statements_per_function": _distribution(statements_per_function),
        "spawn_count": op_counts["spawn"] + op_counts["spawn_async"],
        "join_count": op_counts["join"] + op_counts["await"],
        "call_count": op_counts["call"],
        "branch_count": branch_count,
        "switch_count": switch_count,
        "bodyless_function_count": sum(
            1 for f in functions if not _as_list(f.get("body"))
        ),
        "goal_count": len(_as_list(program.get("goals"))),
        "protection_count": len(_as_list(program.get("protection"))),
        "cir_json_bytes": len(cir_json.encode("utf-8")),
    }


def _as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def _distribution(values: list[int]) -> dict[str, float]:
    if not values:
        return {"min": 0, "max": 0, "avg": 0.0}
    return {
        "min": min(values),
        "max": max(values),
        "avg": round(sum(values) / len(values), 2),
    }
