"""Structural checks on generated modular CIR.

These checks inspect the *structure* of the candidate (which functions lock
which resources, in what order, across which modules). They are deliberately
independent of the Rust verifier's verdict: they judge whether the model
faithfully represented the requested concurrency pattern, so a model that quietly
"fixes" an ABBA into a same-order program is flagged even though the tool would
return PASS.
"""

from __future__ import annotations

from typing import Any

MUTEX_LOCK = "mutex_lock"
MUTEX_UNLOCK = "mutex_unlock"


def _functions(program: dict[str, Any]):
    for module in program.get("modules", []) or []:
        module_name = module.get("name")
        for function in module.get("functions", []) or []:
            yield module_name, function


def _function_trace(function: dict[str, Any]) -> list[tuple[str, str]]:
    trace = []
    for stmt in function.get("body", []) or []:
        if not isinstance(stmt, dict):
            continue
        kind = stmt.get("kind")
        if kind in (MUTEX_LOCK, MUTEX_UNLOCK):
            trace.append((kind, str(stmt.get("resource"))))
    return trace


def mutex_trace(program: dict[str, Any]) -> dict[str, list[tuple[str, str]]]:
    out: dict[str, list[tuple[str, str]]] = {}
    for module_name, function in _functions(program):
        out[f"{module_name}::{function.get('name')}"] = _function_trace(function)
    return out


def lock_order(program: dict[str, Any]) -> dict[str, list[str]]:
    out: dict[str, list[str]] = {}
    for fqn, trace in mutex_trace(program).items():
        out[fqn] = [res for kind, res in trace if kind == MUTEX_LOCK]
    return out


def locks_balanced(program: dict[str, Any]) -> tuple[bool, list[str]]:
    problems = []
    for fqn, trace in mutex_trace(program).items():
        counts: dict[str, int] = {}
        for kind, res in trace:
            counts[res] = counts.get(res, 0) + (1 if kind == MUTEX_LOCK else -1)
        for res, balance in counts.items():
            if balance != 0:
                problems.append(f"{fqn}: resource {res!r} lock/unlock balance {balance}")
    return (not problems), problems


def inversions(program: dict[str, Any]) -> list[dict[str, Any]]:
    """All (pair, f1, f2) where f1 locks x then y and f2 locks y then x."""
    orders = {fqn: [r for r in seq] for fqn, seq in lock_order(program).items()}
    found = []
    fqns = sorted(orders)
    for i, f1 in enumerate(fqns):
        for f2 in fqns:
            if f1 == f2:
                continue
            a = orders[f1]
            b = orders[f2]
            for x in a:
                for y in a:
                    if x == y:
                        continue
                    xi, yi = _index(a, x), _index(a, y)
                    if xi is None or yi is None or xi > yi:
                        continue
                    xj, yj = _index(b, y), _index(b, x)
                    if xj is None or yj is None or xj > yj:
                        continue
                    found.append({"pair": [x, y], "first": f1, "second": f2})
    return found


def _index(seq: list[str], value: str):
    try:
        return seq.index(value)
    except ValueError:
        return None


def check_same_order_pair(program: dict[str, Any]) -> dict[str, Any]:
    orders = lock_order(program)
    balance_ok, balance_problems = locks_balanced(program)
    inv = inversions(program)
    shared = _shared_pairs(orders)
    ok = balance_ok and not inv and bool(shared)
    reason = None
    if not shared:
        reason = "no two functions lock a common resource pair"
    elif inv:
        reason = f"lock-order inversion present: {inv[:3]}"
    elif not balance_ok:
        reason = f"unbalanced locks: {balance_problems[:3]}"
    return {"ok": ok, "reason": reason, "shared_pairs": sorted(shared),
            "inversions": inv, "locks_balanced": balance_ok}


def check_abba_inversion(program: dict[str, Any]) -> dict[str, Any]:
    balance_ok, balance_problems = locks_balanced(program)
    inv = inversions(program)
    ok = balance_ok and bool(inv)
    reason = None
    if not inv:
        reason = "no two functions acquire a common resource pair in opposite order"
    elif not balance_ok:
        reason = f"unbalanced locks: {balance_problems[:3]}"
    return {"ok": ok, "reason": reason, "inversions": inv, "locks_balanced": balance_ok}


def check_cross_module_abba(program: dict[str, Any]) -> dict[str, Any]:
    base = check_abba_inversion(program)
    modules = [m.get("name") for m in program.get("modules", []) or []]
    cross_inv = [i for i in base["inversions"]
                 if i["first"].split("::")[0] != i["second"].split("::")[0]]
    cross_refs = _cross_module_resource_refs(program)
    ok = base["ok"] and len(modules) >= 2 and bool(cross_inv) and bool(cross_refs)
    reason = base["reason"]
    if base["ok"] and not cross_inv:
        reason = "inversion is not across two modules"
    if base["ok"] and not cross_refs:
        reason = "no cross-module resource reference (FQN in requires.resources)"
    return {"ok": ok, "reason": reason, "modules": modules,
            "cross_module_inversions": cross_inv, "cross_resource_refs": cross_refs,
            "base": base}


def _shared_pairs(orders: dict[str, list[str]]) -> set[tuple[str, str]]:
    by_pair: dict[tuple[str, str], set[str]] = {}
    for fqn, seq in orders.items():
        for i, x in enumerate(seq):
            for y in seq[i + 1:]:
                if x != y:
                    by_pair.setdefault(tuple(sorted((x, y))), set()).add(fqn)
    return {pair for pair, users in by_pair.items() if len(users) >= 2}


def _cross_module_resource_refs(program: dict[str, Any]) -> list[dict[str, str]]:
    refs = []
    for module in program.get("modules", []) or []:
        module_name = module.get("name")
        requires = (module.get("requires") or {}).get("resources", []) or []
        for res in requires:
            if "::" in str(res):
                refs.append({"module": module_name, "resource": str(res)})
    return refs


CHECKS = {
    "same_order_pair": check_same_order_pair,
    "abba_inversion": check_abba_inversion,
    "cross_module_abba": check_cross_module_abba,
}


def run_structural_check(name: str, program: dict[str, Any]) -> dict[str, Any]:
    check = CHECKS.get(name)
    if check is None:
        return {"ok": None, "reason": f"unknown structural check {name!r}"}
    return check(program)
