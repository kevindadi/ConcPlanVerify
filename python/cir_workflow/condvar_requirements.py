"""Finite-coverage checks for condvar/notify_one_multi_waiter_wrong_pick.

R8 and R9 stay uncovered. A pass means the covered clauses hold in this
finite check. It does not mean every interleaving is correct. The scorer does
not read CIR conformance.
"""

from __future__ import annotations

import re
from pathlib import Path

from .pilot_oracle import OracleUnknown, strip_rust
from .send_holding_requirements import (
    _body, _params, _reachable, call_snapshots, execute_source,
)

EXPECTED_STDOUT = "DONE waiters=0"
TASK_ID = "condvar/notify_one_multi_waiter_wrong_pick"
COVERED = ("R1", "R2", "R3", "R4", "R5", "R6", "R7", "R10")
UNCOVERED = (
    "R8 every schedule terminates",
    "R9 every waiter completes in every schedule",
)


def _unsupported(body: str) -> bool:
    return re.search(r"\b(if|match|for|loop|async)\b", body) is not None


def _check(status: str, evidence: str, **detail) -> dict:
    return {"status": status, "evidence_kind": evidence, "detail": detail}


def _sem_initial(root: str) -> int | None:
    match = re.match(r"sem:(-?\d+):", root or "")
    if not match:
        return None
    return int(match.group(1))


def _first(body: str, pattern: str) -> int:
    match = re.search(pattern, body)
    return -1 if match is None else match.start()


def _shadowed(body: str, name: str, limit: int) -> bool:
    return re.search(rf"\blet\s+(?:mut\s+)?{re.escape(name)}\s*=", body[:max(limit, 0)]) is not None


def _acquire_counts(region: str, name: str) -> list[int]:
    counts = []
    pattern = rf"\b{re.escape(name)}\s*\.\s*acquire_count\s*\(\s*(-?\d+)\s*\)"
    for match in re.finditer(pattern, region):
        counts.append(int(match.group(1)))
    plain = rf"\b{re.escape(name)}\s*\.\s*acquire\s*\("
    counts.extend(1 for _ in re.finditer(plain, region))
    return counts


def _release_counts(region: str, name: str) -> list[int]:
    pattern = rf"\b{re.escape(name)}\s*\.\s*release_count\s*\(\s*(-?\d+)\s*\)"
    return [int(match.group(1)) for match in re.finditer(pattern, region)]


def _shared_roles(main: str) -> tuple[str, list[list[str]] | None]:
    snaps = call_snapshots(main, {"w1", "w2", "notifier"})
    if snaps is None:
        return "unknown", None
    rows = []
    for role in ("w1", "w2", "notifier"):
        calls = snaps.get(role) or []
        spawned = re.search(rf"thread::spawn\s*\([^;]*\b{role}\s*\(", main, flags=re.S)
        if len(calls) != 1 or spawned is None:
            return ("fail" if not calls or spawned is None else "unknown"), None
        if len(calls[0]) != 4:
            return "unknown", None
        rows.append(calls[0])
    for column in range(4):
        values = [row[column] for row in rows]
        if any(item in {"unknown", "absent"} for item in values):
            return "unknown", None
        if len(set(values)) != 1:
            return "fail", None
    joins = len(re.findall(r"\.join\s*\(", main))
    if joins < 3:
        return "fail", None
    return "pass", rows


def structural_checks(source: str) -> dict:
    try:
        strip_rust(source)
        main = _body(source, "main")
        w1 = _body(source, "w1")
        w2 = _body(source, "w2")
        notifier = _body(source, "notifier")
    except OracleUnknown as exc:
        status = "fail" if "not found" in exc.reason else "unknown"
        return {clause: _check(status, "static_subset", reason=exc.reason) for clause in COVERED if clause != "R10"}
    stripped = strip_rust(source)
    main_r, main_flow = _reachable(main)
    if main_flow == "unknown" or _unsupported(main_r):
        role_status, rows = "unknown", None
    else:
        role_status, rows = _shared_roles(main_r)
    params = {name: _params(stripped, name) for name in ("w1", "w2", "notifier")}
    g12 = rows[0][2] if rows else None
    checks = {
        "R1": _check(role_status, "static_subset", roots=rows),
        "R2": _wait_clause(w1, w2, params, rows),
        "R3": _notify_clause(notifier),
        "R4": _lock_at_wait(w1, w2, params),
        "R5": _notifier_lock(notifier, params),
        "R6": _readiness(w1, w2, notifier, params, g12),
        "R7": _notify_clause(notifier),
    }
    return checks


def _wait_clause(w1: str, w2: str, params: dict, rows) -> dict:
    if rows is None:
        return _check("unknown", "static_subset", reason="shared roots were not established")
    for body, role in ((w1, "w1"), (w2, "w2")):
        if _unsupported(body):
            return _check("unknown", "static_subset", role=role)
        if len(params[role]) < 2:
            return _check("unknown", "static_subset", role=role)
        cv = params[role][1]
        if _first(body, rf"\b{re.escape(cv)}\s*\.\s*wait\s*\(") < 0:
            return _check("fail", "static_subset", role=role, reason="waiter does not wait on the passed condvar")
    return _check("pass", "static_subset")


def _notify_clause(notifier: str) -> dict:
    if _unsupported(notifier):
        return _check("unknown", "static_subset")
    has_all = _first(notifier, r"\bnotify_all\s*\(") >= 0
    has_one = _first(notifier, r"\bnotify_one\s*\(") >= 0
    if has_all and not has_one:
        return _check("pass", "static_subset", note="notify_all is present; this is not an all-interleaving proof")
    if has_one and not has_all:
        return _check("fail", "static_subset", reason="notify_one does not wake every blocked waiter")
    return _check("unknown" if has_all and has_one else "fail", "static_subset")


def _lock_at_wait(w1: str, w2: str, params: dict) -> dict:
    for body, role in ((w1, "w1"), (w2, "w2")):
        if _unsupported(body):
            return _check("unknown", "static_subset", role=role)
        if len(params[role]) < 2:
            return _check("unknown", "static_subset", role=role)
        lock_name, cv = params[role][0], params[role][1]
        lock_at = _first(body, rf"\b{re.escape(lock_name)}\s*\.\s*lock\s*\(")
        wait_at = _first(body, rf"\b{re.escape(cv)}\s*\.\s*wait\s*\(")
        drop_at = _first(body, r"\bdrop\s*\(")
        if min(lock_at, wait_at, drop_at) < 0:
            return _check("fail", "static_subset", role=role)
        if _shadowed(body, lock_name, wait_at) or _shadowed(body, cv, wait_at):
            return _check("unknown", "static_subset", role=role, reason="lock receiver is rebound")
        if not (lock_at < wait_at < drop_at):
            return _check("fail", "static_subset", role=role, reason="wait is not between lock and drop")
    return _check("pass", "static_subset")


def _notifier_lock(notifier: str, params: dict) -> dict:
    if _unsupported(notifier):
        return _check("unknown", "static_subset")
    if len(params["notifier"]) < 2:
        return _check("unknown", "static_subset")
    lock_name, cv = params["notifier"][0], params["notifier"][1]
    lock_at = _first(notifier, rf"\b{re.escape(lock_name)}\s*\.\s*lock\s*\(")
    notify_at = _first(notifier, rf"\b{re.escape(cv)}\s*\.\s*notify_all\s*\(")
    drop_at = _first(notifier, r"\bdrop\s*\(")
    if min(lock_at, notify_at, drop_at) < 0:
        return _check("fail", "static_subset")
    if _shadowed(notifier, lock_name, notify_at):
        return _check("unknown", "static_subset", reason="notifier lock is rebound")
    if not (lock_at < notify_at < drop_at):
        return _check("fail", "static_subset", reason="notify_all is not between lock and drop")
    return _check("pass", "static_subset")


def _readiness(w1: str, w2: str, notifier: str, params: dict, g12_root: str | None) -> dict:
    bodies = {"w1": w1, "w2": w2, "notifier": notifier}
    if any(_unsupported(body) for body in bodies.values()):
        return _check("unknown", "static_subset")
    if g12_root is None or any(len(params[role]) < 3 for role in bodies):
        return _check("unknown", "static_subset", reason="g12 root was not resolved at the call")
    initial = _sem_initial(g12_root)
    if initial is None:
        return _check("unknown", "static_subset", reason="g12 initial count is not a literal at the used construction")
    waiter_release = True
    for role in ("w1", "w2"):
        body = bodies[role]
        g12 = params[role][2]
        cv = params[role][1]
        wait_at = _first(body, rf"\b{re.escape(cv)}\s*\.\s*wait\s*\(")
        if wait_at < 0 or _shadowed(body, g12, wait_at):
            return _check("unknown", "static_subset", role=role)
        region = body[:wait_at]
        if _acquire_counts(region, g12) or _release_counts(region, g12) != [1]:
            waiter_release = False
    note = params["notifier"][2]
    cv = params["notifier"][1]
    notify_at = _first(notifier, rf"\b{re.escape(cv)}\s*\.\s*notify_all\s*\(")
    if notify_at < 0 or _shadowed(notifier, note, notify_at):
        return _check("unknown", "static_subset")
    acquires = _acquire_counts(notifier[:notify_at], note)
    acquire_at = _first(notifier, rf"\b{re.escape(note)}\s*\.\s*acquire(?:_count)?\s*\(")
    if acquire_at < 0 or notify_at < acquire_at:
        return _check("fail", "static_subset", reason="notifier wakes the waiters before the readiness acquire")
    if initial > 0 and acquires and sum(acquires) <= initial:
        return _check(
            "fail", "static_subset",
            reason="notifier acquires can finish from the initial count, before both waiters release",
            initial=initial, acquires=acquires)
    if initial == 0 and waiter_release and sum(acquires) == 2:
        return _check("pass", "static_subset", initial=0, acquires=acquires,
                      note="the counting order is visible in the source; R8 and R9 stay uncovered")
    return _check("unknown", "static_subset", initial=initial, acquires=acquires,
                  waiter_release=waiter_release)


def execution_checks(runs: list[dict]) -> dict:
    if not runs:
        return {"R10": _check("unknown", "finite_run", reason="no runs")}
    rows = []
    for run in runs:
        stdout = run.get("stdout")
        rows.append({"stdout": stdout.strip() if isinstance(stdout, str) else None,
                     "returncode": run.get("returncode"), "timed_out": bool(run.get("timed_out")),
                     "kind": run.get("kind")})
    printed = all(item["stdout"] == EXPECTED_STDOUT and item["returncode"] == 0
                  and item["kind"] == "completed" and not item["timed_out"] for item in rows)
    return {"R10": _check("pass" if printed else "fail", "finite_run", expected=EXPECTED_STDOUT, runs=rows)}


def combine(structural: dict, execution: dict) -> dict:
    checks = {**structural, **execution}
    statuses = [checks[clause]["status"] for clause in COVERED]
    if any(status == "fail" for status in statuses):
        overall = "fail"
    elif any(status == "unknown" for status in statuses):
        overall = "unknown"
    elif all(status == "pass" for status in statuses):
        overall = "bounded_covered_satisfied"
    else:
        overall = "unknown"
    return {
        "status": overall,
        "label": ("已覆盖需求在有限检查中满足" if overall == "bounded_covered_satisfied" else overall),
        "checks": checks,
        "uncovered": list(UNCOVERED),
        "not_a_complete_proof": True,
        "task_id": TASK_ID,
    }


def evaluate_condvar(source: str, work: Path, repeats: int = 3) -> dict:
    structural = structural_checks(source)
    runs = execute_source(source, work, repeats=repeats)
    if runs and runs[0].get("kind") == "candidate_build_failed":
        execution = {"R10": _check("fail", "finite_run", reason="build_failed", runs=runs)}
    elif runs and runs[0].get("kind") == "tool_unavailable":
        execution = {"R10": _check("unknown", "finite_run", reason="tool_unavailable", runs=runs)}
    else:
        execution = execution_checks(runs)
    result = combine(structural, execution)
    result["runs"] = runs
    return result
