"""Requirement checks for channel/send_while_holding_mutex.

The expected output line comes from the requirement text. A control program
does not decide which clauses are checked. Finite runs are not a proof that
every interleaving terminates. Matching one CIR's statement order is not a
requirement.
"""

from __future__ import annotations

import hashlib
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


def _reachable(body: str) -> tuple[str, str]:
    """Return the straight-line prefix and ``ok`` or ``unknown``.

    A ``return`` ends the prefix. ``if`` / loops make control flow unknown.
    Statements after ``return`` are not evidence.
    """

    if re.search(r"\b(if|match|while|for|loop|async)\b", body):
        return body, "unknown"
    depth_brace = 0
    depth_paren = 0
    index = 0
    while index < len(body):
        char = body[index]
        if char == "{":
            depth_brace += 1
        elif char == "}":
            depth_brace = max(0, depth_brace - 1)
        elif char == "(":
            depth_paren += 1
        elif char == ")":
            depth_paren = max(0, depth_paren - 1)
        elif body.startswith("return", index) and depth_brace == 0 and depth_paren == 0:
            before = body[index - 1] if index else " "
            after = body[index + 6] if index + 6 < len(body) else " "
            if (not before.isalnum() and before != "_") and (not after.isalnum() and after != "_"):
                return body[:index], "ok"
        index += 1
    return body, "ok"


def _split_params(blob: str) -> list[str]:
    parts: list[str] = []
    buf: list[str] = []
    depth = 0
    for char in blob:
        if char in "<(":
            depth += 1
            buf.append(char)
        elif char in ">)":
            depth = max(0, depth - 1)
            buf.append(char)
        elif char == "," and depth == 0:
            parts.append("".join(buf))
            buf = []
        else:
            buf.append(char)
    if "".join(buf).strip():
        parts.append("".join(buf))
    return parts


def _params(stripped: str, name: str) -> list[str]:
    match = re.search(rf"fn\s+{re.escape(name)}\b\s*(?:<[^>]*>)?\s*\(", stripped)
    if not match:
        return []
    index = match.end()
    depth = 1
    while index < len(stripped) and depth:
        if stripped[index] == "(":
            depth += 1
        elif stripped[index] == ")":
            depth -= 1
        index += 1
    names = []
    for part in _split_params(stripped[match.end():index - 1]):
        ident = part.split(":")[0].strip()
        if ident.startswith("mut "):
            ident = ident[4:].strip()
        if re.fullmatch(r"[A-Za-z_]\w*", ident):
            names.append(ident)
    return names


def _lookup_root(scopes: list[dict[str, str]], name: str) -> str | None:
    for scope in reversed(scopes):
        if name in scope:
            return scope[name]
    return None


def _skip_gap(text: str, index: int) -> int:
    while index < len(text):
        if text.startswith("//", index):
            newline = text.find("\n", index)
            index = len(text) if newline < 0 else newline + 1
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            index = len(text) if end < 0 else end + 2
            continue
        if text[index] in " \t\r\n":
            index += 1
            continue
        break
    return index


def _skip_string(text: str, index: int) -> int | None:
    if index >= len(text) or text[index] != '"':
        return None
    index += 1
    while index < len(text):
        if text[index] == "\\":
            index += 2
            continue
        if text[index] == '"':
            return index + 1
        index += 1
    return None


def _ident_boundary(text: str, index: int) -> bool:
    if index <= 0:
        return True
    prev = text[index - 1]
    return not (prev.isalnum() or prev == "_")


def _keyword(text: str, index: int) -> bool:
    for word in ("if", "match", "while", "for", "loop", "async"):
        if text.startswith(word, index) and _ident_boundary(text, index):
            after = index + len(word)
            if after >= len(text) or not (text[after].isalnum() or text[after] == "_"):
                return True
    return False


def _matching_brace(text: str, index: int) -> tuple[str, int] | None:
    if index >= len(text) or text[index] != "{":
        return None
    depth = 0
    cursor = index
    while cursor < len(text):
        if text.startswith("//", cursor) or text.startswith("/*", cursor):
            cursor = _skip_gap(text, cursor)
            continue
        skipped = _skip_string(text, cursor)
        if skipped is not None:
            cursor = skipped
            continue
        if text[cursor] == "{":
            depth += 1
        elif text[cursor] == "}":
            depth -= 1
            if depth == 0:
                return text[index + 1:cursor], cursor + 1
        cursor += 1
    return None


def _expr_until_semi(text: str, index: int) -> tuple[str, int] | None:
    depth = 0
    cursor = index
    while cursor < len(text):
        if text.startswith("//", cursor) or text.startswith("/*", cursor):
            cursor = _skip_gap(text, cursor)
            continue
        skipped = _skip_string(text, cursor)
        if skipped is not None:
            cursor = skipped
            continue
        char = text[cursor]
        if char in "({[":
            depth += 1
        elif char in ")}]":
            depth -= 1
        elif char == ";" and depth == 0:
            return text[index:cursor], cursor + 1
        cursor += 1
    return None


def _arg_root(arg: str, scopes: list[dict[str, str]]) -> str:
    arg = arg.strip()
    if re.fullmatch(r"[A-Za-z_]\w*", arg):
        return _lookup_root(scopes, arg) or "absent"
    clone = re.fullmatch(r"Arc::clone\s*\(\s*&([A-Za-z_]\w*)\s*\)", arg)
    if clone:
        return _lookup_root(scopes, clone.group(1)) or "unknown"
    return "unknown"


def _split_args(blob: str) -> list[str] | None:
    parts: list[str] = []
    buf: list[str] = []
    depth = 0
    for char in blob:
        if char in "({[<":
            depth += 1
            buf.append(char)
        elif char in ")}]>":
            depth = max(0, depth - 1)
            buf.append(char)
        elif char == "," and depth == 0:
            parts.append("".join(buf).strip())
            buf = []
        else:
            buf.append(char)
    if "".join(buf).strip():
        parts.append("".join(buf).strip())
    return parts


def _leading_root(expr: str, scopes: list[dict[str, str]], origin: int, space: str) -> str:
    stripped = expr.lstrip()
    site = f"{space}:{origin + (len(expr) - len(stripped))}"
    if re.match(r"Arc::new\s*\(\s*Mutex::new\b", stripped):
        return f"mutex:{site}"
    if re.match(r"Arc::new\s*\(\s*Condvar::new\b", stripped):
        return f"condvar:{site}"
    sem = re.match(r"Semaphore::new\s*\(\s*(-?\d+)\s*\)", stripped)
    if sem:
        return f"sem:{sem.group(1)}:{site}"
    clone = re.match(r"Arc::clone\s*\(\s*&([A-Za-z_]\w*)\s*\)\s*$", stripped)
    if clone:
        return _lookup_root(scopes, clone.group(1)) or "unknown"
    if re.fullmatch(r"[A-Za-z_]\w*", stripped):
        found = _lookup_root(scopes, stripped)
        return found if found is not None else "unknown"
    return "unknown"


def _scan_expr(expr: str, scopes: list[dict[str, str]], calls: dict[str, list[list[str]]],
               interest: set[str], locks: list[str], origin: int, space: str) -> tuple[bool, str]:
    """Walk one expression. A call records the roots visible at that call."""

    stripped = expr.lstrip()
    pad = len(expr) - len(stripped)
    if stripped.startswith("{"):
        found = _matching_brace(expr, pad)
        if found is None:
            return False, "unknown"
        inner, end = found
        if expr[end:].strip():
            return False, "unknown"
        scopes.append({})
        ok, root = _scan_statements(inner, scopes, calls, interest, locks, origin + pad + 1, space)
        scopes.pop()
        return ok, root
    if _keyword(stripped, 0):
        return False, "unknown"
    root = _leading_root(expr, scopes, origin, space)
    index = 0
    while index < len(expr):
        if expr.startswith("//", index) or expr.startswith("/*", index):
            index = _skip_gap(expr, index)
            continue
        skipped = _skip_string(expr, index)
        if skipped is not None:
            index = skipped
            continue
        if expr[index] == "{":
            found = _matching_brace(expr, index)
            if found is None:
                return False, "unknown"
            inner, end = found
            scopes.append({})
            ok, _inner_root = _scan_statements(inner, scopes, calls, interest, locks, origin + index + 1, space)
            scopes.pop()
            if not ok:
                return False, "unknown"
            index = end
            continue
        if _keyword(expr, index):
            return False, "unknown"
        call = re.match(r"([A-Za-z_]\w*)\s*\(", expr[index:])
        if call and _ident_boundary(expr, index) and expr[index - 1:index] != ".":
            name = call.group(1)
            open_at = index + call.end() - 1
            depth = 0
            cursor = open_at
            while cursor < len(expr):
                if expr[cursor] == "(":
                    depth += 1
                elif expr[cursor] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                cursor += 1
            else:
                return False, "unknown"
            if name in interest:
                args = _split_args(expr[open_at + 1:cursor])
                if args is None:
                    return False, "unknown"
                calls.setdefault(name, []).append([_arg_root(arg, scopes) for arg in args])
            lock = re.match(r"([A-Za-z_]\w*)\s*\.\s*lock\s*\(", expr[index:])
            if lock and _ident_boundary(expr, index):
                locks.append(_lookup_root(scopes, lock.group(1)) or "unknown")
            index += 1
            continue
        lock = re.match(r"([A-Za-z_]\w*)\s*\.\s*lock\s*\(", expr[index:])
        if lock and _ident_boundary(expr, index):
            locks.append(_lookup_root(scopes, lock.group(1)) or "unknown")
            index += lock.end()
            continue
        index += 1
    return True, root


def _scan_statements(text: str, scopes: list[dict[str, str]], calls: dict[str, list[list[str]]],
                     interest: set[str], locks: list[str], origin: int, space: str) -> tuple[bool, str]:
    index = 0
    while index < len(text):
        index = _skip_gap(text, index)
        if index >= len(text):
            break
        if _keyword(text, index):
            return False, "unknown"
        if text[index] == "{":
            found = _matching_brace(text, index)
            if found is None:
                return False, "unknown"
            inner, end = found
            scopes.append({})
            ok, _root = _scan_statements(inner, scopes, calls, interest, locks, origin + index + 1, space)
            scopes.pop()
            if not ok:
                return False, "unknown"
            index = end
            continue
        if text[index] == "}":
            return False, "unknown"
        # A zero-argument closure that returns a tuple of clones, then a
        # destructure of that call. This is one supported way to pass the same
        # objects into several threads. Any other closure is not guessed.
        closure = re.match(
            r"let\s+(?:mut\s+)?(\w+)\s*=\s*\|\|\s*\(([^;]*)\)\s*;", text[index:])
        if closure and _ident_boundary(text, index):
            parts = _split_args(closure.group(2)) or []
            roots = [_arg_root(part, scopes) for part in parts]
            scopes[-1][closure.group(1)] = "tuple:" + "|".join(roots)
            index += closure.end()
            continue
        destructure = re.match(
            r"let\s+\(([^)]*)\)\s*=\s*(\w+)\s*\(\s*\)\s*;", text[index:])
        if destructure and _ident_boundary(text, index):
            names = [part.strip() for part in destructure.group(1).split(",") if part.strip()]
            source = _lookup_root(scopes, destructure.group(2)) or ""
            parts = source[6:].split("|") if source.startswith("tuple:") else []
            if len(parts) == len(names) and parts:
                for name, root in zip(names, parts):
                    scopes[-1][name] = root
            else:
                for name in names:
                    scopes[-1][name] = "unknown"
            index += destructure.end()
            continue
        let = re.match(r"let\s+(?:mut\s+)?(\w+)\s*=\s*", text[index:])
        assign = None if let else re.match(r"(\w+)\s*=\s*", text[index:])
        if assign and _lookup_root(scopes, assign.group(1)) is None:
            assign = None
        if let or assign:
            match = let or assign
            expr_at = index + match.end()
            parsed = _expr_until_semi(text, expr_at)
            if parsed is None:
                ok, root = _scan_expr(text[expr_at:], scopes, calls, interest, locks, origin + expr_at, space)
                if let or assign:
                    scopes[-1][match.group(1)] = root
                return ok, root
            expr, end = parsed
            ok, root = _scan_expr(expr, scopes, calls, interest, locks, origin + expr_at, space)
            if not ok:
                return False, "unknown"
            scopes[-1][match.group(1)] = root
            index = end
            continue
        parsed = _expr_until_semi(text, index)
        if parsed is None:
            return _scan_expr(text[index:], scopes, calls, interest, locks, origin + index, space)
        expr, end = parsed
        ok, _root = _scan_expr(expr, scopes, calls, interest, locks, origin + index, space)
        if not ok:
            return False, "unknown"
        index = end
    return True, "unknown"


def call_snapshots(text: str, names: set[str], initial: dict[str, str] | None = None,
                   locks: list[str] | None = None, space: str = "main") -> dict[str, list[list[str]]] | None:
    """Roots of call arguments as they were when the call was reached.

    A later ``let`` of the same name does not rewrite an earlier call.
    ``None`` means the control flow is outside the supported subset.
    """

    found: list[str] = [] if locks is None else locks
    calls: dict[str, list[list[str]]] = {}
    ok, _root = _scan_statements(text, [dict(initial or {})], calls, set(names), found, 0, space)
    if not ok:
        return None
    return calls


def _lock_roots_at_use(body: str, initial: dict[str, str]) -> str:
    locks: list[str] = []
    if call_snapshots(body, set(), initial, locks, space="worker") is None:
        return "unknown"
    if not locks:
        return "fail"
    if any(item in {None, "unknown", "absent"} for item in locks):
        return "unknown"
    if len(set(locks)) != 1:
        return "fail"
    return locks[0]


def _shared_lock(main: str, sender: str, receiver: str, stripped: str) -> str:
    snaps = call_snapshots(main, {"s", "r"})
    if snaps is None:
        return "unknown"
    s_calls = snaps.get("s", [])
    r_calls = snaps.get("r", [])
    if len(s_calls) != 1 or len(r_calls) != 1:
        return "unknown" if s_calls or r_calls else "fail"
    s_params = _params(stripped, "s")
    r_params = _params(stripped, "r")
    if len(s_params) != len(s_calls[0]) or len(r_params) != len(r_calls[0]):
        return "unknown"
    s_root = _lock_roots_at_use(sender, dict(zip(s_params, s_calls[0])))
    r_root = _lock_roots_at_use(receiver, dict(zip(r_params, r_calls[0])))
    if "unknown" in (s_root, r_root):
        return "unknown"
    if "fail" in (s_root, r_root) or s_root != r_root:
        return "fail"
    return "pass"


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
    stripped = strip_rust(source)
    main_r, main_flow = _reachable(main)
    sender_r, sender_flow = _reachable(sender)
    receiver_r, receiver_flow = _reachable(receiver)
    flow_unknown = "unknown" in (main_flow, sender_flow, receiver_flow)
    spawns = re.findall(r"thread::spawn\s*\([^;]*\)", main_r, flags=re.S)
    calls_s = any(re.search(r"\bs\s*\(", item) for item in spawns)
    calls_r = any(re.search(r"\br\s*\(", item) for item in spawns)
    joins = len(re.findall(r"\.join\s*\(", main_r))
    if flow_unknown:
        r1 = "unknown"
    elif not (calls_s and calls_r and joins >= 2):
        r1 = "fail"
    else:
        r1 = _shared_lock(main_r, sender_r, receiver_r, stripped)
    kinds = _channel_kinds(main_r)
    if flow_unknown:
        r2 = "unknown"
    elif not kinds or "unknown" in kinds:
        r2 = "unknown" if kinds else "fail"
    elif any(kind == "unbounded" for kind in kinds) or not all(kind == "rendezvous" for kind in kinds):
        r2 = "fail"
    elif not (re.search(r"\.send\s*\(", sender_r) and re.search(r"\.recv\s*\(", sender_r)
              and re.search(r"\.send\s*\(", receiver_r) and re.search(r"\.recv\s*\(", receiver_r)):
        r2 = "fail"
    else:
        r2 = "pass"
    if flow_unknown or not (calls_s and calls_r):
        g1 = g2 = "unknown"
        r3 = "unknown"
    else:
        g1, g2 = _guard_blocks(sender_r), _guard_blocks(receiver_r)
        if "unknown" in (g1, g2):
            r3 = "unknown"
        elif "fail" in (g1, g2):
            r3 = "fail"
        elif g1 == "pass" and g2 == "pass":
            r3 = "pass"
        else:
            r3 = "fail"
    return {
        "R1": {"status": r1, "detail": {"spawns_s": calls_s, "spawns_r": calls_r, "joins": joins,
                                       "reachable": not flow_unknown}},
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
    runs = []
    for _ in range(repeats):
        run = capture_program_stdout(binary, timeout=timeout)
        run["argv"] = [str(binary)]
        run["timeout_s"] = timeout
        if binary.is_file():
            run["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        runs.append(run)
    return runs


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
