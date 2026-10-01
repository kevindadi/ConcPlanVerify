"""Requirement checks for channel/send_while_holding_mutex.

The expected output line comes from the requirement text. A control program
does not decide which clauses are checked. Finite runs are not a proof that
every interleaving terminates. Matching one CIR's statement order is not a
requirement.
"""

from __future__ import annotations

import re
from pathlib import Path

from .candidate_eval import capture_program_stdout
from .pilot_oracle import OracleUnknown, strip_rust, _function_body

EXPECTED_STDOUT = "DONE done=1"
CLAUSES = ("R1", "R2", "R3", "R4", "R5")


def _body(source: str, name: str) -> str:
    return _function_body(source, name)


def _channel_kinds(main_body: str) -> list[str]:
    kinds = []
    for match in re.finditer(r"\b(sync_channel|channel)\s*(::\s*<[^;]*>)?\s*\(([^)]*)\)", main_body):
        name, _ty, arg = match.group(1), match.group(2), match.group(3).strip()
        if name == "channel" and arg == "":
            kinds.append("unbounded")
        elif name == "sync_channel" and arg == "0":
            kinds.append("rendezvous")
        else:
            kinds.append("unknown")
    return kinds


def _guard_violation(body: str) -> str:
    """Return fail, pass, or unknown for lock-held channel waits in one function."""

    if re.search(r"\b(if|match|while|for|loop|async)\b", body):
        return "unknown"
    guards = 0
    depth_guards: list[int] = []
    for raw in body.split(";"):
        stmt = raw.strip()
        if not stmt:
            continue
        while stmt.startswith("{"):
            depth_guards.append(guards)
            stmt = stmt[1:].strip()
        if re.search(r"\.lock\s*\(", stmt) and "let " in stmt:
            guards += 1
            continue
        if re.search(r"\.(send|recv)\s*\(", stmt):
            return "fail" if guards else "pass_step"
        if stmt == "}":
            if not depth_guards:
                return "unknown"
            guards = depth_guards.pop()
    # Closing braces may sit on the split remainder.
    if guards and "{" in body:
        # A block that ends before send drops the guard. Re-walk with brace depth.
        return _guard_blocks(body)
    return "pass" if re.search(r"\.(send|recv)\s*\(", body) else "fail"


def _guard_blocks(body: str) -> str:
    if re.search(r"\b(if|match|while|for|loop|async)\b", body):
        return "unknown"
    events = []
    depth = 0
    buf = []
    i = 0
    text = body
    while i < len(text):
        ch = text[i]
        if ch == "{":
            if depth == 0 and not "".join(buf).strip():
                events.append(("block-start", ""))
            depth += 1
            buf.append(ch)
        elif ch == "}":
            depth -= 1
            if depth == 0:
                events.append(("block-end", ""))
                buf = []
            else:
                buf.append(ch)
        elif ch == ";" and depth == 0:
            events.append(("stmt", "".join(buf).strip()))
            buf = []
        else:
            buf.append(ch)
        i += 1
    tail = "".join(buf).strip()
    if tail:
        events.append(("stmt", tail))
    guards = 0
    stack: list[int] = []
    saw = False
    for kind, stmt in events:
        if kind == "block-start":
            stack.append(guards)
            continue
        if kind == "block-end":
            if not stack:
                return "unknown"
            guards = stack.pop()
            continue
        if re.search(r"\.lock\s*\(", stmt):
            guards += 1
            continue
        if re.search(r"\.(send|recv)\s*\(", stmt):
            saw = True
            if guards:
                return "fail"
    return "pass" if saw else "fail"


def structural_checks(source: str) -> dict:
    try:
        strip_rust(source)
        main = _body(source, "main")
        sender = _body(source, "s")
        receiver = _body(source, "r")
    except OracleUnknown as exc:
        if "not found" in exc.reason:
            return {
                "R1": {"status": "fail", "reason": exc.reason},
                "R2": {"status": "fail", "reason": "sender and receiver are not both present"},
                "R3": {"status": "unknown", "reason": exc.reason},
            }
        return {clause: {"status": "unknown", "reason": exc.reason} for clause in ("R1", "R2", "R3")}
    spawns = re.findall(r"thread::spawn\s*\([^;]*\)", main, flags=re.S)
    calls_s = any(re.search(r"\bs\s*\(", item) for item in spawns)
    calls_r = any(re.search(r"\br\s*\(", item) for item in spawns)
    joins = len(re.findall(r"\.join\s*\(", main))
    r1 = "pass" if calls_s and calls_r and joins >= 2 else "fail"
    kinds = _channel_kinds(main)
    if not kinds or "unknown" in kinds:
        r2 = "unknown"
    elif any(kind == "unbounded" for kind in kinds) or not all(kind == "rendezvous" for kind in kinds):
        r2 = "fail"
    elif not (re.search(r"\.send\s*\(", sender) and re.search(r"\.recv\s*\(", sender)
              and re.search(r"\.send\s*\(", receiver) and re.search(r"\.recv\s*\(", receiver)):
        r2 = "fail"
    else:
        r2 = "pass"
    g1, g2 = _guard_blocks(sender), _guard_blocks(receiver)
    if "unknown" in (g1, g2):
        r3 = "unknown"
    elif "fail" in (g1, g2):
        r3 = "fail"
    elif g1 == "pass" and g2 == "pass":
        r3 = "pass"
    else:
        r3 = "fail"
    return {
        "R1": {"status": r1, "detail": {"spawns_s": calls_s, "spawns_r": calls_r, "joins": joins}},
        "R2": {"status": r2, "detail": {"channels": kinds}},
        "R3": {"status": r3, "detail": {"s": g1, "r": g2}},
    }


def execution_checks(runs: list[dict]) -> dict:
    if not runs:
        return {"R4": {"status": "unknown", "reason": "no runs"},
                "R5": {"status": "unknown", "reason": "no runs"}}
    rows = []
    for run in runs:
        stdout = run.get("stdout")
        stripped = stdout.strip() if isinstance(stdout, str) else None
        rows.append({"stdout": stripped, "returncode": run.get("returncode"),
                     "timed_out": bool(run.get("timed_out")), "kind": run.get("kind")})
    terminated = all(item["kind"] == "completed" and item["returncode"] == 0 and not item["timed_out"]
                     for item in rows)
    printed = all(item["stdout"] == EXPECTED_STDOUT and item["returncode"] == 0 and not item["timed_out"]
                  for item in rows)
    return {
        "R4": {"status": "observed_finite" if terminated else "fail",
               "coverage": "finite runs only; not a proof of every interleaving",
               "runs": rows},
        "R5": {"status": "pass" if printed else "fail", "expected": EXPECTED_STDOUT, "runs": rows},
    }


def combine(structural: dict, execution: dict) -> dict:
    checks = {**structural, **execution}
    statuses = [checks[clause]["status"] for clause in CLAUSES]
    if any(status == "fail" for status in statuses):
        overall = "fail"
    elif any(status == "unknown" for status in statuses):
        overall = "unknown"
    elif all(status == "pass" for clause, status in zip(CLAUSES, statuses) if clause != "R4") \
            and checks["R4"]["status"] == "observed_finite":
        overall = "bounded_covered_satisfied"
    else:
        overall = "unknown"
    return {
        "status": overall,
        "label": ("已覆盖需求在有限检查中满足" if overall == "bounded_covered_satisfied" else overall),
        "checks": checks,
        "uncovered": ["R4 universal interleavings"],
        "not_a_complete_proof": True,
    }


def execute_source(source: str, work: Path, repeats: int = 3, timeout: float = 4.0) -> list[dict]:
    from . import rust_oracle
    work.mkdir(parents=True, exist_ok=True)
    (work / "src").mkdir(exist_ok=True)
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    built, log = rust_oracle.cargo_build(work)
    if not built:
        return [{"kind": "candidate_build_failed", "stdout": None, "returncode": None,
                 "timed_out": False, "log": log[-500:]}] * 1
    binary = work / "target/debug/probe"
    return [capture_program_stdout(binary, timeout=timeout) for _ in range(repeats)]


def evaluate_requirements(source: str, work: Path, repeats: int = 3) -> dict:
    structural = structural_checks(source)
    runs = execute_source(source, work, repeats=repeats)
    if runs and runs[0].get("kind") == "candidate_build_failed":
        execution = {"R4": {"status": "fail", "reason": "build_failed", "runs": runs},
                     "R5": {"status": "fail", "reason": "build_failed", "runs": runs}}
    elif runs and runs[0].get("kind") == "tool_unavailable":
        execution = {"R4": {"status": "unknown", "reason": "tool_unavailable", "runs": runs},
                     "R5": {"status": "unknown", "reason": "tool_unavailable", "runs": runs}}
    else:
        execution = execution_checks(runs)
    result = combine(structural, execution)
    result["runs"] = runs
    return result
