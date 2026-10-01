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


_SYNC_WORD = re.compile(
    r"\b(?:lock|wait|notify_all|notify_one|acquire_count|release_count|acquire|release|drop)\b")


def _classify_stmt(stmt: str) -> dict | None:
    """One supported statement, or None when it does not touch the protocol."""

    patterns = (
        (r"^let\s+(?:mut\s+)?(\w+)\s*=\s*(\w+)\s*\.\s*lock\s*\(",
         lambda m: {"op": "lock", "guard": m.group(1), "mutex": m.group(2)}),
        (r"^(\w+)\s*\.\s*release_count\s*\(\s*(-?\d+)\s*\)",
         lambda m: {"op": "release_count", "sem": m.group(1), "n": int(m.group(2))}),
        (r"^(\w+)\s*\.\s*acquire_count\s*\(\s*(-?\d+)\s*\)",
         lambda m: {"op": "acquire_count", "sem": m.group(1), "n": int(m.group(2))}),
        (r"^let\s+(?:mut\s+)?(\w+)\s*=\s*(\w+)\s*\.\s*acquire\s*\(",
         lambda m: {"op": "raii_acquire", "permit": m.group(1), "sem": m.group(2)}),
        (r"^(\w+)\s*\.\s*release\s*\(",
         lambda m: {"op": "permit_release", "permit": m.group(1)}),
        (r"^drop\s*\(\s*(\w+)\s*\)",
         lambda m: {"op": "drop", "name": m.group(1)}),
        (r"^\*\s*(\w+)\s*=\s*(true|false)\b",
         lambda m: {"op": "store_flag", "guard": m.group(1), "value": m.group(2) == "true"}),
        (r"^(\w+)\s*\.\s*notify_all\s*\(",
         lambda m: {"op": "notify_all", "cv": m.group(1)}),
        (r"^(\w+)\s*\.\s*notify_one\s*\(",
         lambda m: {"op": "notify_one", "cv": m.group(1)}),
        (r"^(\w+)\s*=\s*(\w+)\s*\.\s*wait\s*\(\s*(\w+)\s*\)",
         lambda m: {"op": "wait", "guard": m.group(1), "cv": m.group(2), "arg": m.group(3)}),
    )
    for pattern, build in patterns:
        match = re.match(pattern, stmt)
        if match:
            return build(match)
    return None


def _parse_sequence(text: str) -> dict:
    """Straight-line statements plus a supported ``while``.

    ``while false`` is recognized and its body is not reachable. Any other
    control form, or a sync statement outside this subset, is unknown.
    """

    if re.search(r"\b(if|match|for|loop|async)\b", text):
        return {"events": [], "unknown": True, "reason": "unsupported control flow"}
    events: list[dict] = []
    index = 0
    while index < len(text):
        while index < len(text) and text[index] in " \t\r\n":
            index += 1
        if index >= len(text):
            break
        if text.startswith("while", index) and (index == 0 or not (text[index - 1].isalnum() or text[index - 1] == "_")):
            brace = text.find("{", index)
            if brace < 0:
                return {"events": events, "unknown": True, "reason": "unclosed while"}
            condition = text[index + 5:brace].strip()
            inner, end = _brace_block(text, brace)
            if end is None:
                return {"events": events, "unknown": True, "reason": "unclosed while"}
            if condition == "false":
                events.append({"op": "while_false"})
            else:
                guard = re.fullmatch(r"!\s*\*\s*(\w+)", condition)
                if not guard:
                    return {"events": events, "unknown": True, "reason": "while condition is outside the subset"}
                inner_parsed = _parse_sequence(inner)
                if inner_parsed["unknown"]:
                    return inner_parsed
                events.append({"op": "while_not", "guard": guard.group(1), "body": inner_parsed["events"]})
            index = end
            continue
        end = _statement_end(text, index)
        if end is None:
            return {"events": events, "unknown": True, "reason": "unclosed statement"}
        stmt = text[index:end].strip()
        index = end + 1
        if not stmt:
            continue
        event = _classify_stmt(stmt)
        if event is None:
            if _SYNC_WORD.search(stmt):
                return {"events": events, "unknown": True, "reason": f"unrecognized sync statement: {stmt[:80]}"}
            continue
        events.append(event)
    return {"events": events, "unknown": False, "reason": None}


def _brace_block(text: str, open_at: int) -> tuple[str, int | None]:
    depth = 0
    cursor = open_at
    while cursor < len(text):
        if text[cursor] == "{":
            depth += 1
        elif text[cursor] == "}":
            depth -= 1
            if depth == 0:
                return text[open_at + 1:cursor], cursor + 1
        cursor += 1
    return "", None


def _statement_end(text: str, start: int) -> int | None:
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char in "({[":
            depth += 1
        elif char in ")}]":
            if depth == 0:
                return None
            depth -= 1
        elif char == ";" and depth == 0:
            return index
    return None


def _flatten(events: list[dict], *, reachable: bool = True) -> list[dict]:
    out = []
    for event in events:
        if event["op"] == "while_false":
            out.append({"op": "while_false", "reachable": False})
            continue
        if event["op"] == "while_not":
            out.append({**event, "reachable": reachable})
            for inner in event["body"]:
                out.append({**inner, "reachable": reachable, "in_while": event["guard"]})
            continue
        out.append({**event, "reachable": reachable})
    return out


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


def _role_events(body: str) -> dict:
    parsed = _parse_sequence(body)
    if parsed["unknown"]:
        return parsed
    return {**parsed, "flat": _flatten(parsed["events"])}


def _wait_clause(w1: str, w2: str, params: dict, rows) -> dict:
    if rows is None:
        return _check("unknown", "static_subset", reason="shared roots were not established")
    for body, role in ((w1, "w1"), (w2, "w2")):
        if len(params[role]) < 2:
            return _check("unknown", "static_subset", role=role)
        verdict = _waiter_wait(body, params[role][0], params[role][1])
        if verdict["status"] != "pass":
            verdict["detail"]["role"] = role
            return verdict
    return _check("pass", "static_subset")


def _waiter_wait(body: str, mutex: str, cv: str) -> dict:
    parsed = _role_events(body)
    if parsed.get("unknown"):
        return _check("unknown", "static_subset", reason=parsed.get("reason"))
    guard = None
    for event in parsed["flat"]:
        if event["op"] == "lock" and event["mutex"] == mutex and event.get("reachable", True):
            guard = event["guard"]
            break
    waits = [event for event in parsed["flat"] if event["op"] == "wait" and event.get("reachable", True)]
    if any(event["op"] == "while_false" for event in parsed["flat"]) and not waits:
        return _check("fail", "static_subset", reason="wait is inside a constant-false loop and is not reachable")
    if guard is None or not waits:
        return _check("fail", "static_subset", reason="no reachable wait on the lock held by this waiter")
    for event in waits:
        if event.get("in_while") == guard and event["cv"] == cv and event["guard"] == guard and event["arg"] == guard:
            return _check("pass", "static_subset")
    return _check("fail", "static_subset", reason="reachable wait is not on the shared guard inside while !*guard")


def _notify_clause(notifier: str) -> dict:
    parsed = _role_events(notifier)
    if parsed.get("unknown"):
        return _check("unknown", "static_subset", reason=parsed.get("reason"))
    reachable = [event for event in parsed["flat"] if event.get("reachable", True)]
    has_all = any(event["op"] == "notify_all" for event in reachable)
    has_one = any(event["op"] == "notify_one" for event in reachable)
    if has_all and not has_one:
        return _check("pass", "static_subset", note="reachable notify_all; this is not an all-interleaving proof")
    if has_one and not has_all:
        return _check("fail", "static_subset", reason="notify_one does not wake every blocked waiter")
    return _check("fail", "static_subset", reason="no reachable notify_all")


def _lock_at_wait(w1: str, w2: str, params: dict) -> dict:
    for body, role in ((w1, "w1"), (w2, "w2")):
        if len(params[role]) < 2:
            return _check("unknown", "static_subset", role=role)
        verdict = _waiter_wait(body, params[role][0], params[role][1])
        if verdict["status"] != "pass":
            verdict["detail"]["role"] = role
            return verdict
        parsed = _role_events(body)
        flat = parsed["flat"]
        mutex, cv = params[role][0], params[role][1]
        guard = next(event["guard"] for event in flat if event["op"] == "lock" and event["mutex"] == mutex)
        wait_at = next(i for i, event in enumerate(flat)
                       if event["op"] == "wait" and event.get("in_while") == guard and event["cv"] == cv)
        lock_at = next(i for i, event in enumerate(flat) if event["op"] == "lock" and event["guard"] == guard)
        drop_at = next((i for i, event in enumerate(flat)
                        if event["op"] == "drop" and event["name"] == guard and i > wait_at), None)
        if drop_at is None or not (lock_at < wait_at < drop_at):
            return _check("fail", "static_subset", role=role, reason="the shared guard is not held across the reachable wait")
    return _check("pass", "static_subset")


def _notifier_lock(notifier: str, params: dict) -> dict:
    if len(params["notifier"]) < 2:
        return _check("unknown", "static_subset")
    parsed = _role_events(notifier)
    if parsed.get("unknown"):
        return _check("unknown", "static_subset", reason=parsed.get("reason"))
    mutex, cv = params["notifier"][0], params["notifier"][1]
    flat = [event for event in parsed["flat"] if event.get("reachable", True)]
    lock_at = next((i for i, event in enumerate(flat) if event["op"] == "lock" and event["mutex"] == mutex), None)
    notify_at = next((i for i, event in enumerate(flat) if event["op"] == "notify_all" and event["cv"] == cv), None)
    if lock_at is None or notify_at is None or notify_at < lock_at:
        return _check("fail", "static_subset", reason="reachable notify_all is not after the shared lock")
    guard = flat[lock_at]["guard"]
    drop_at = next((i for i, event in enumerate(flat) if event["op"] == "drop" and event["name"] == guard and i > notify_at), None)
    if drop_at is None:
        return _check("fail", "static_subset", reason="the shared guard is not dropped after notify_all")
    return _check("pass", "static_subset")


def _permit_effect(events: list[dict], sem: str) -> dict:
    """Persistent acquire_count versus permits the same role returns before notify."""

    persistent = 0
    supplied = 0
    held: dict[str, str] = {}
    returned = 0
    for event in events:
        op = event["op"]
        if op == "release_count" and event["sem"] == sem:
            supplied += event["n"]
        elif op == "acquire_count" and event["sem"] == sem:
            if event["n"] <= 0:
                return {"status": "unknown", "reason": "non-positive acquire_count"}
            persistent += event["n"]
        elif op == "raii_acquire" and event["sem"] == sem:
            held[event["permit"]] = sem
        elif op == "permit_release" and event["permit"] in held:
            held.pop(event["permit"])
            returned += 1
        elif op == "drop" and event["name"] in held:
            held.pop(event["name"])
            returned += 1
        elif op in {"raii_acquire", "acquire_count", "release_count"} and event.get("sem") not in {None, sem}:
            continue
    return {"status": "ok", "persistent": persistent, "supplied": supplied,
            "returned": returned, "held": len(held)}


def _readiness(w1: str, w2: str, notifier: str, params: dict, g12_root: str | None) -> dict:
    if g12_root is None or any(len(params[role]) < 4 for role in ("w1", "w2", "notifier")):
        return _check("unknown", "static_subset", reason="g12 root was not resolved at the call")
    initial = _sem_initial(g12_root)
    if initial is None:
        return _check("unknown", "static_subset", reason="g12 initial count is not a literal at the used construction")
    waiter_ready = True
    for role, body in (("w1", w1), ("w2", w2)):
        parsed = _role_events(body)
        if parsed.get("unknown"):
            return _check("unknown", "static_subset", role=role, reason=parsed.get("reason"))
        verdict = _waiter_wait(body, params[role][0], params[role][1])
        if verdict["status"] == "unknown":
            return verdict
        if verdict["status"] != "pass":
            return _check("fail", "static_subset", role=role,
                          reason="the waiter is not reachably waiting, so a permit release is not readiness")
        sem = params[role][2]
        before = []
        for event in parsed["flat"]:
            if event["op"] == "while_not":
                break
            before.append(event)
        effect = _permit_effect(before, sem)
        if effect["status"] != "ok":
            return _check("unknown", "static_subset", role=role, reason=effect["reason"])
        if effect["supplied"] != 1 or effect["persistent"] or effect["returned"] or effect["held"]:
            waiter_ready = False
    note_body = _role_events(notifier)
    if note_body.get("unknown"):
        return _check("unknown", "static_subset", reason=note_body.get("reason"))
    sem = params["notifier"][2]
    cv = params["notifier"][1]
    before_notify = []
    saw_notify = False
    for event in note_body["flat"]:
        if event["op"] == "notify_all" and event.get("cv") == cv and event.get("reachable", True):
            saw_notify = True
            break
        if event.get("reachable", True):
            before_notify.append(event)
    if not saw_notify:
        return _check("fail", "static_subset", reason="notifier has no reachable notify_all")
    effect = _permit_effect(before_notify, sem)
    if effect["status"] != "ok":
        return _check("unknown", "static_subset", reason=effect["reason"])
    detail = {key: value for key, value in effect.items() if key != "status"}
    if effect["supplied"] > 0:
        return _check("fail", "static_subset",
                      reason="the notifier releases permits before notify_all, so the acquire need not wait for the waiters",
                      **detail)
    if effect["returned"] and effect["persistent"] != 2:
        return _check("fail", "static_subset",
                      reason="RAII acquire/release returns the permit before notify_all; it is not a persistent consume of two waiter permits",
                      **detail)
    if effect["held"]:
        return _check("unknown", "static_subset",
                      reason="a RAII permit is still held at notify_all; its net effect is not the supported explicit protocol",
                      **detail)
    if initial > 0 and effect["persistent"] <= initial:
        return _check("fail", "static_subset",
                      reason="the notifier's persistent acquire can finish from the initial count",
                      initial=initial, **detail)
    if initial == 0 and waiter_ready and effect["persistent"] == 2 and effect["returned"] == 0:
        return _check("pass", "static_subset", initial=0, persistent=effect["persistent"],
                      note="explicit acquire_count only; R8 and R9 stay uncovered")
    if effect["persistent"] < 2 and effect["held"] == 0:
        return _check("fail", "static_subset",
                      reason="notifier reaches notify_all before a persistent acquire of 2 waiter permits",
                      **detail)
    return _check("unknown", "static_subset", initial=initial, waiter_ready=waiter_ready, **detail)


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
