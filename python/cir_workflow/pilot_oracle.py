"""Independent oracles for the feedback-mechanism pilot.

These checks do not call conform or monitor. A passing design oracle does not
by itself make the requirement oracle pass.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from .candidate_eval import capture_program_stdout


class OracleUnknown(Exception):
    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


def strip_rust(source: str) -> str:
    """Remove comments and string/char literals. Raw strings are unsupported."""

    if 'r"' in source or "r#" in source:
        raise OracleUnknown("raw string")
    out: list[str] = []
    i = 0
    while i < len(source):
        if source.startswith("//", i):
            nxt = source.find("\n", i)
            i = len(source) if nxt < 0 else nxt
            continue
        if source.startswith("/*", i):
            nxt = source.find("*/", i + 2)
            if nxt < 0:
                raise OracleUnknown("unterminated comment")
            i = nxt + 2
            continue
        if source[i] in "\"'":
            quote = source[i]
            i += 1
            while i < len(source):
                if source[i] == "\\":
                    i += 2
                    continue
                if source[i] == quote:
                    i += 1
                    break
                i += 1
            out.append('""')
            continue
        out.append(source[i])
        i += 1
    return "".join(out)


def _function_body(source: str, name: str) -> str:
    text = strip_rust(source)
    match = re.search(rf"fn\s+{re.escape(name)}\b[^{{]*\{{", text)
    if not match:
        raise OracleUnknown(f"function {name} not found")
    depth = 1
    index = match.end()
    while index < len(text) and depth:
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
        index += 1
    return text[match.end():index - 1]


def _split_block(text: str) -> list[tuple[str, str]]:
    items: list[tuple[str, str]] = []
    buf: list[str] = []
    depth = 0
    i = 0
    while i < len(text):
        char = text[i]
        if char == "{":
            if depth == 0 and not "".join(buf).strip():
                end = i + 1
                inner = 1
                while end < len(text) and inner:
                    if text[end] == "{":
                        inner += 1
                    elif text[end] == "}":
                        inner -= 1
                    end += 1
                items.append(("block", text[i + 1:end - 1]))
                i = end
                buf = []
                continue
            depth += 1
        elif char == "}":
            depth = max(0, depth - 1)
        elif char == ";" and depth == 0:
            items.append(("stmt", "".join(buf).strip()))
            buf = []
            i += 1
            continue
        buf.append(char)
        i += 1
    tail = "".join(buf).strip()
    if tail:
        items.append(("stmt", tail))
    return items


def _walk(text: str, guards: dict[str, str], declared: set[str]) -> list[tuple]:
    events: list[tuple] = []
    for kind, chunk in _split_block(text):
        if kind == "block":
            inner: dict[str, str] = {}
            events.extend(_walk(chunk, inner, set()))
            for mutex in inner.values():
                events.append(("rel", mutex))
            continue
        stmt = chunk.strip()
        if not stmt or stmt in {"move ||", "||"}:
            continue
        if re.search(r"\b(if|match|while|for|loop|async|await|unsafe)\b", stmt):
            if ".lock" in stmt or "spawn" in stmt or "join" in stmt:
                raise OracleUnknown(f"unsupported control around synchronization: {stmt[:40]}")
            continue
        locks = re.findall(r"([A-Za-z_]\w*)\.lock\s*\(", stmt)
        if len(locks) > 1:
            raise OracleUnknown("multiple locks in one statement")
        binding = re.match(
            r"let\s+(?:mut\s+)?([A-Za-z_]\w*)\s*=\s*([A-Za-z_]\w*)\.lock\s*\(", stmt)
        if binding:
            name, mutex = binding.group(1), binding.group(2)
            guards[name] = mutex
            declared.add(name)
            events.append(("acq", mutex))
            continue
        if locks:
            events.append(("acq", locks[0]))
            events.append(("rel", locks[0]))
            continue
        dropped = re.match(r"drop\s*\(\s*([A-Za-z_]\w*)\s*\)", stmt)
        if dropped:
            name = dropped.group(1)
            if name not in guards:
                raise OracleUnknown(f"drop of unbound name {name}")
            events.append(("rel", guards.pop(name)))
            continue
        spawn = re.match(
            r"let\s+(?:mut\s+)?([A-Za-z_]\w*)\s*=\s*(?:std\s*::\s*)?thread\s*::\s*spawn\s*\(",
            stmt)
        if spawn:
            handle = spawn.group(1)
            calls = re.findall(r"\b([A-Za-z_]\w*)\s*\(", stmt)
            events.append(("spawn", handle, [c for c in calls if c != "spawn"]))
            continue
        joined = re.match(r"([A-Za-z_]\w*)\.join\s*\(", stmt)
        if joined:
            events.append(("join", joined.group(1)))
            continue
    return events


def analyze_function(source: str, function: str) -> dict[str, Any]:
    try:
        body = _function_body(source, function)
        events = _walk(body, {}, set())
    except OracleUnknown as exc:
        return {"status": "unknown", "reason": exc.reason, "events": []}
    return {"status": "ok", "reason": None, "events": events}


def lock_sites(source: str, function: str) -> list[str]:
    found = analyze_function(source, function)
    if found["status"] != "ok":
        raise OracleUnknown(found["reason"] or "unparsed")
    return [mutex for kind, mutex in found["events"] if kind == "acq"]


def cir_mutex_locks(cir: dict, function: str) -> list[str]:
    names = []
    for module in cir.get("modules") or []:
        for fn in module.get("functions") or []:
            if fn.get("name") != function:
                continue
            for step in fn.get("body") or []:
                if step.get("kind") == "mutex_lock":
                    resource = str(step.get("resource") or "")
                    names.append(resource.split("::")[-1])
    return names


def cir_has_kind(cir: dict, kind: str) -> bool:
    for module in cir.get("modules") or []:
        for fn in module.get("functions") or []:
            for step in fn.get("body") or []:
                if step.get("kind") == kind:
                    return True
    return False


def support_of(source: str) -> dict[str, Any]:
    """Support is read from the source, not from the task id."""

    unsupported = []
    if "thread::scope" in source:
        unsupported.append("thread_scope")
    if re.search(r"\basync\b", source) or ".await" in source:
        unsupported.append("async")
    if "RwLock" in source:
        unsupported.append("rwlock")
    features = []
    if "Mutex" in source:
        features.append("mutex")
    if "thread::spawn" in source:
        features.append("spawn")
    if "Condvar" in source:
        features.append("condvar")
    if "mpsc::" in source:
        features.append("channel")
    return {"supported": not unsupported, "unsupported": unsupported, "features": features}


def _build(source: str, work: Path) -> tuple[bool, str]:
    from . import rust_oracle
    work.mkdir(parents=True, exist_ok=True)
    (work / "src").mkdir(exist_ok=True)
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    try:
        return rust_oracle.cargo_build(work)
    except OSError as exc:
        return False, f"tool_error: {exc}"


def run_source(source: str, work: Path, timeout: float) -> dict[str, Any]:
    built, log = _build(source, work)
    if not built:
        kind = "tool_unavailable" if str(log).startswith("tool_error:") else "candidate_build_failed"
        return {"kind": kind, "stdout": None, "stderr": log, "returncode": None,
                "timed_out": False, "build_ok": False}
    run = capture_program_stdout(work / "target/debug/probe", timeout=timeout)
    run["build_ok"] = True
    return run


def _static(status: str, oracle: str, observed, expected, reason: str | None = None,
            checks: list | None = None) -> dict[str, Any]:
    return {"oracle": oracle, "status": status, "observed": observed, "expected": expected,
            "reason": reason, "checks": checks or [{"id": oracle, "status": status}]}


def _apply_build(result: dict, build_ok: bool | None) -> dict[str, Any]:
    if build_ok is None:
        return result
    result["build"] = "pass" if build_ok else "fail"
    if result["status"] == "pass" and not build_ok:
        result["status"] = "fail"
        result["reason"] = "build_failed"
    return result


def _events(source: str, function: str) -> dict[str, Any]:
    return analyze_function(source, function)


def requirement_lock_order(source: str, function: str, expected: list[str],
                           build_ok: bool | None = None) -> dict[str, Any]:
    found = _events(source, function)
    if found["status"] != "ok":
        return _apply_build(_static("unknown", "requirement_lock_order", None, expected,
                                    found["reason"]), build_ok)
    observed = [mutex for kind, mutex in found["events"] if kind == "acq"]
    status = "pass" if observed == expected else "fail"
    return _apply_build(_static(status, "requirement_lock_order", observed, expected), build_ok)


def requirement_hold(source: str, function: str, first: str, second: str,
                     build_ok: bool | None = None) -> dict[str, Any]:
    found = _events(source, function)
    if found["status"] != "ok":
        return _apply_build(_static("unknown", "requirement_hold", None, "held",
                                    found["reason"]), build_ok)
    held: set[str] = set()
    seen_second = False
    observed = "missing_second_lock"
    ok = False
    for event in found["events"]:
        if event[0] == "acq":
            if event[1] == second:
                seen_second = True
                ok = first in held
                observed = "held" if ok else "released_before_second_lock"
                break
            held.add(event[1])
        elif event[0] == "rel":
            held.discard(event[1])
    if not seen_second:
        ok = False
        observed = "missing_second_lock"
    return _apply_build(_static("pass" if ok else "fail", "requirement_hold", observed, "held"),
                        build_ok)


def requirement_locks_present(source: str, function: str, names: list[str],
                              build_ok: bool | None = None) -> dict[str, Any]:
    return requirement_lock_order(source, function, names, build_ok=build_ok)


def requirement_stdout(source: str, work: Path, expected: str, timeout: float = 8.0) -> dict[str, Any]:
    run = run_source(source, work, timeout)
    stdout = run.get("stdout")
    stripped = stdout.strip() if isinstance(stdout, str) else None
    ok = (run.get("kind") == "completed" and run.get("returncode") == 0
          and not run.get("timed_out") and stripped == expected)
    if run.get("kind") == "tool_unavailable":
        status = "tool_error"
    elif run.get("timed_out"):
        status = "fail"
    else:
        status = "pass" if ok else "fail"
    return {"oracle": "requirement_stdout", "status": status,
            "observed": {"stdout": stripped, "returncode": run.get("returncode"),
                         "timed_out": bool(run.get("timed_out"))},
            "expected": expected}


def requirement_terminates(source: str, work: Path, timeout: float = 2.0) -> dict[str, Any]:
    run = run_source(source, work, timeout)
    ok = run.get("kind") == "completed" and run.get("returncode") == 0 and not run.get("timed_out")
    if run.get("kind") == "tool_unavailable":
        status = "tool_error"
    else:
        status = "pass" if ok else "fail"
    observed = "completed" if ok else ("timeout" if run.get("timed_out") else run.get("kind"))
    return {"oracle": "requirement_terminates", "status": status, "observed": observed,
            "expected": "completed"}


def _spawn_join(source: str) -> dict[str, Any]:
    found = analyze_function(source, "main")
    if found["status"] != "ok":
        return {"status": "unknown", "reason": found["reason"], "joined": False, "calls": []}
    spawned: dict[str, list[str]] = {}
    joined: set[str] = set()
    for event in found["events"]:
        if event[0] == "spawn":
            spawned[event[1]] = event[2]
        elif event[0] == "join":
            joined.add(event[1])
    calls = [name for names in spawned.values() for name in names]
    all_joined = bool(spawned) and set(spawned) <= joined
    return {"status": "ok", "joined": all_joined, "calls": calls, "handles": list(spawned)}


def design_lock_order(source: str, cir: dict, function: str,
                      build_ok: bool | None = None) -> dict[str, Any]:
    cir_order = cir_mutex_locks(cir, function)
    found = _events(source, function)
    if found["status"] != "ok":
        return _apply_build(_static("unknown", "design_lock_order", None, cir_order,
                                    found["reason"]), build_ok)
    observed = [mutex for kind, mutex in found["events"] if kind == "acq"]
    status = "pass" if observed == cir_order else "fail"
    return _apply_build(_static(status, "design_lock_order", observed, cir_order), build_ok)


def design_join(source: str, cir: dict, build_ok: bool | None = None) -> dict[str, Any]:
    wants_join = cir_has_kind(cir, "join")
    parsed = _spawn_join(source)
    if parsed["status"] != "ok":
        return _apply_build(_static("unknown", "design_join", None,
                                    "join" if wants_join else "no_join", parsed["reason"]),
                            build_ok)
    has_join = parsed["joined"]
    ok = bool(wants_join) and has_join
    return _apply_build(_static("pass" if ok else "fail", "design_join",
                                "join" if has_join else "no_join",
                                "join" if wants_join else "no_join"), build_ok)


def design_sync_matches_cir(source: str, cir: dict, function: str) -> dict[str, Any]:
    """Synchronization skeleton only. A wrong stored value is not this oracle."""

    locks = design_lock_order(source, cir, function)
    join = design_join(source, cir)
    ok = locks["status"] == "pass" and join["status"] == "pass"
    return {"oracle": "design_sync", "status": "pass" if ok else "fail",
            "observed": {"locks": locks["observed"], "join": join["observed"]},
            "expected": {"locks": locks["expected"], "join": join["expected"]}}


