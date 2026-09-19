#!/usr/bin/env python3
"""Build the capability-family benchmark from the current ConcIR capability set.

This is benchmark tooling, not experiment code. It authors/reuses modular CIR
cases, runs ``check``/``support``/``explore`` (petri + interp) on every case,
compares against the pre-registered expectation, rebuilds
``benchmarks/MANIFEST.json`` and writes ``benchmarks/FAMILIES.md``.

Run from the repository root: ``python3 benchmarks/build_families.py``.

Exit code is non-zero if any ready case fails validation or the petri/interp
outcomes disagree. A pre-registration mismatch is recorded (never hidden) but
does not by itself abort.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "benchmarks/families"
CONCIR = Path("/Users/kevin/local-repos/ConcIR")
LEGACY = ROOT / "benchmarks/legacy-paper-patterns"
PATTERNS = ROOT / "benchmarks/patterns"
FROZEN = ROOT / "experiments/deepseek-flash-repair-v1/frozen-inputs"

BOUNDS = {"max_threads": 8, "max_frames_per_thread": 8, "max_states": 20000,
          "max_depth": 64, "max_boundary_events": 256}
SCOPE = {"allow_lock_reorder": True}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


# ─────────────────── R-1: de-leaked repair inputs ───────────────────

FAMILY_LETTER = {"lock-order": "a", "condvar": "b", "channel": "c", "semaphore": "d",
                 "atomic-data": "e", "structure": "f", "boundary": "g",
                 "real-cases": "h"}
FORBIDDEN_WORDS = ("deadlock", "lost", "leak", "bug", "fix", "wrong", "order")

# Human-written, neutral requirements: expected behaviour and termination only.
SANITIZED_REQUIREMENTS = {
    "lock-order/abba_2lock":
        "Two worker threads each need exclusive access to two shared mutexes, A and "
        "B. Each worker acquires both mutexes, performs its work, releases both, and "
        "returns. The main thread starts both workers and waits for both to finish. "
        "Every interleaving must terminate and both workers must complete.",
    "lock-order/cycle_3lock":
        "Three workers need three shared mutexes. Worker 1 uses A and B, worker 2 "
        "uses B and C, worker 3 uses C and A. Each worker holds its two mutexes at the "
        "same time, releases them, and returns. Main starts all three and waits. Every "
        "interleaving must terminate and all three workers must complete.",
    "lock-order/cross_module_cycle":
        "Two modules share resources: module main owns resource a and module other "
        "owns resource b. Two tasks each need both resources and must declare the "
        "cross-module dependency. Every interleaving must terminate and both tasks "
        "must complete.",
    "lock-order/partial_deadlock_bystander":
        "Two workers A and B use two mutexes with an intermediate semaphore "
        "handshake, and an independent third task keeps making progress. Main starts "
        "all three. Every reachable state must still allow A and B to complete, and "
        "both workers must complete.",
    "lock-order/two_independent_cycles":
        "Several workers share mutexes and two separate acquisition patterns are "
        "present. Every interleaving must terminate and all workers must complete.",
    "condvar/lost_wakeup_notify_before_wait":
        "A waiter and a notifier share a mutex, a condition variable, and a boolean "
        "flag guarded by the mutex. The notifier makes the flag true and signals; the "
        "waiter waits until the flag is true and then completes. The waiter must "
        "complete even if the notifier signals before the waiter begins waiting.",
    "condvar/bare_wait_no_predicate":
        "A waiter blocks on a condition variable until a predicate guarded by a mutex "
        "becomes true; a notifier makes it true and signals. The waiter must complete "
        "even if the signal arrives before it waits.",
    "condvar/notify_one_multi_waiter_wrong_pick":
        "Several waiters share a mutex and a condition variable, and a notifier wakes "
        "one waiter. Every interleaving must terminate and every waiter must complete.",
    "channel/rendezvous_both_send":
        "Two tasks communicate over a channel with no buffering. One task sends, the "
        "other receives. Both must pair and terminate.",
    "channel/send_while_holding_mutex":
        "A sender and a receiver communicate over a channel, and both occasionally "
        "need a shared mutex. Every interleaving must terminate; the receiver must not "
        "block on the channel while holding the lock the sender needs.",
    "channel/bounded_backpressure_lock_held":
        "A sender and a receiver use a channel of capacity one, plus a mutex both "
        "occasionally need. The sender sends two values. Every interleaving must "
        "terminate; neither side may block on the channel while holding the lock the "
        "other needs.",
    "semaphore/permit_leak":
        "Two workers share a counting semaphore with one permit. Each acquires the "
        "permit, does its work, releases the permit, and returns. Every interleaving "
        "must terminate and both workers must complete.",
    "semaphore/acquire_twice_no_release":
        "Two workers share a semaphore with one permit. A worker may acquire the "
        "permit more than once but must release it as many times before returning. "
        "Every interleaving must terminate and both workers must complete.",
    "atomic-data/counter_overflow_safety":
        "Two workers each add one to a bounded integer counter (declared range 0..2) "
        "under a mutex, while an invariant requires the counter never to exceed 1. "
        "Every reachable state must satisfy the invariant and every interleaving must "
        "terminate.",
    "atomic-data/atomic_lost_update":
        "Two workers each add one to a shared atomic counter starting from zero, "
        "using a compare-and-swap retry so no update is dropped. From every reachable "
        "state it must still be possible for the counter to reach two, and every "
        "interleaving must terminate.",
    "structure/nested_scope_lock_order":
        "A worker starts a nested group of two tasks; the two inner tasks both need "
        "mutexes A and B and must not form a circular wait. Every interleaving must "
        "terminate and the outer worker must complete.",
    "structure/scope_worker_abba":
        "A group of two workers both acquire mutexes A and B. Every interleaving must "
        "terminate and both workers must complete.",
    "boundary/rwlock_unsupported":
        "A program using a reader-writer lock and shared state. The model must either "
        "verify it or report the construct as unsupported.",
    "boundary/async_select_unsupported":
        "A program using async tasks and channel selection. The model must either "
        "verify it or report the construct as unsupported.",
    "real-cases/rmw-zenoh-998":
        "A reduced real-world program with shared state and synchronization. Every "
        "interleaving must terminate and the modelled goals must be reachable.",
    "real-cases/dashmap-369":
        "A reduced real-world program using a reader-writer lock. The model must "
        "either verify it or report the construct as unsupported.",
}


# Observable terminal line per case (F-3): the program must print exactly this
# line when it completes, so the watchdog can tell "finished" from "finished
# with the right result", without inspecting internals.
TERMINAL = {
    "lock-order/abba_2lock": "DONE t1=1 t2=1",
    "lock-order/partial_deadlock_bystander": "DONE a=1 b=1",
    "condvar/bare_wait_no_predicate": "DONE ready=true",
    "condvar/lost_wakeup_notify_before_wait": "DONE ready=true",
    "semaphore/permit_leak": "DONE permits=1",
    "condvar/notify_one_multi_waiter_wrong_pick": "DONE waiters=0",
}
DEFAULT_TERMINAL = "DONE done=1"

TERMINAL_SENTENCE = (
    "On completion the program must print exactly one line `{line}` and then exit; "
    "it must terminate.")


def strip_comments(source: str) -> str:
    out: list[str] = []
    i, n = 0, len(source)
    in_line = in_block = in_str = esc = False
    while i < n:
        c = source[i]
        if in_line:
            if c == "\n":
                in_line = False
                out.append(c)
            i += 1
            continue
        if in_block:
            if source[i:i + 2] == "*/":
                in_block = False
                i += 2
            else:
                i += 1
            continue
        if in_str:
            out.append(c)
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
            i += 1
            continue
        if source[i:i + 2] == "//":
            in_line = True
            i += 2
            continue
        if source[i:i + 2] == "/*":
            in_block = True
            i += 2
            continue
        if c == '"':
            in_str = True
        out.append(c)
        i += 1
    # compress blank lines
    lines = [ln.rstrip() for ln in "".join(out).splitlines()]
    compressed: list[str] = []
    for ln in lines:
        if ln == "" and compressed and compressed[-1] == "":
            continue
        compressed.append(ln)
    return "\n".join(compressed).strip() + "\n"


def strip_fields(obj):
    if isinstance(obj, dict):
        return {k: strip_fields(v) for k, v in obj.items()
                if k not in ("description", "note")}
    if isinstance(obj, list):
        return [strip_fields(x) for x in obj]
    return obj


def sanitize_case(family: str, case: str, task_dir: Path, seq: int,
                  contract_rel: str, ground_truth: dict,
                  base_rel: str | None = None) -> dict:
    """Generate ``repair_input/`` with comments/names/qualitative wording removed."""

    rin = task_dir / "repair_input"
    rin.mkdir(exist_ok=True)
    program_name = f"case_{FAMILY_LETTER.get(family, 'z')}{seq}"

    buggy_rs = task_dir / "rust/buggy.rs"
    input_rust = rin / "input.rs"
    if buggy_rs.is_file():
        cleaned = strip_comments(buggy_rs.read_text(encoding="utf-8"))
        input_rust.write_text(cleaned, encoding="utf-8")
        subprocess.run(["rustfmt", "--edition", "2021", str(input_rust)],
                       capture_output=True, text=True)

    buggy_cir = task_dir / "buggy.cir.json"
    input_cir = rin / "input.cir.json"
    if buggy_cir.is_file():
        program = strip_fields(json.loads(buggy_cir.read_text(encoding="utf-8")))
        program["program"] = program_name
        write_json(input_cir, program)

    key = f"{family}/{case}"
    requirements = SANITIZED_REQUIREMENTS.get(key)
    if requirements is None:
        raise KeyError(f"no sanitized requirements for {family}/{case}")
    line = TERMINAL.get(key, DEFAULT_TERMINAL)
    requirements = requirements.rstrip() + " " + TERMINAL_SENTENCE.format(line=line)
    (rin / "requirements.txt").write_text(requirements + "\n", encoding="utf-8")

    base = base_rel or f"families/{family}/{case}"
    write_json(task_dir / "repair_task.json", {
        "requirements_file": f"{base}/repair_input/requirements.txt",
        "input_cir": f"{base}/repair_input/input.cir.json",
        "input_rust": (f"{base}/repair_input/input.rs"
                       if input_rust.is_file() else None),
        "contract": contract_rel,
        "ground_truth": ground_truth,
    })

    files = {}
    for name in ("requirements.txt", "input.cir.json", "input.rs"):
        path = rin / name
        if path.is_file():
            files[f"{base}/repair_input/{name}"] = sha256(path)
    files[f"{base}/repair_task.json"] = sha256(task_dir / "repair_task.json")
    return {"requirements_sha256": files[f"{base}/repair_input/requirements.txt"],
            "program": program_name, "files": files}


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


# ─────────────────────────── CIR builders ───────────────────────────

def res_mutex(name: str) -> dict:
    return {"name": name, "kind": "sync", "type": "Mutex", "mode": "Sync"}


def res_condvar(name: str) -> dict:
    return {"name": name, "kind": "sync", "type": "Condvar", "mode": "Sync"}


def res_sem(name: str, count: int) -> dict:
    return {"name": name, "kind": "sync", "type": "Semaphore", "mode": "Sync", "count": count}


def res_channel(name: str, capacity: int, base: Any = "Int") -> dict:
    return {"name": name, "kind": "sync", "type": "Channel", "mode": "Sync",
            "base": base, "capacity": capacity}


def res_var(name: str, base: Any, init: Any) -> dict:
    return {"name": name, "kind": "var", "type": "Var", "base": base, "init": init}


def res_atomic(name: str, base: Any, init: Any) -> dict:
    return {"name": name, "kind": "var", "type": "Atomic", "base": base, "init": init}


def module(resources: list[dict], functions: list[dict], *, name: str = "main",
           protection: list[dict] | None = None, requires: dict | None = None) -> dict:
    return {
        "name": name,
        "provides": {"resources": [r["name"] for r in resources],
                     "functions": [f["name"] for f in functions]},
        "requires": requires or {"resources": [], "functions": []},
        "resources": resources,
        "protection": protection or [],
        "functions": functions,
    }


def program(modules: list[dict], *, name: str, entry: str = "main::main") -> dict:
    return {"program": name, "version": "3.5.0", "entry": entry, "modules": modules}


def lock_fn(name: str, pairs: list[tuple[str, str]], *, prefix: str = "main") -> dict:
    body: list[dict] = []
    counter = {"n": 0}

    def sid() -> str:
        counter["n"] += 1
        return f"s{counter['n']}"

    for (a, b) in pairs:
        body.append({"sid": sid(), "kind": "mutex_lock", "resource": f"{prefix}::{a}"})
        body.append({"sid": sid(), "kind": "mutex_lock", "resource": f"{prefix}::{b}"})
    for (a, b) in reversed(pairs):
        body.append({"sid": sid(), "kind": "mutex_unlock", "resource": f"{prefix}::{b}"})
        body.append({"sid": sid(), "kind": "mutex_unlock", "resource": f"{prefix}::{a}"})
    body.append({"sid": sid(), "kind": "return"})
    return {"name": name, "kind": "normal", "form": "closure", "body": body}


def main_scope(funcs: list[str]) -> dict:
    return {"name": "main", "kind": "normal", "body": [
        {"sid": "s1", "kind": "scope", "funcs": [f"main::{f}" if "::" not in f else f
                                                 for f in funcs]},
        {"sid": "s2", "kind": "return"}]}


def contract(name: str, *, properties: list[dict], preserved: list[dict] | None = None,
             bounds: dict | None = None) -> dict:
    return {"name": name, "properties": properties, "preserved": preserved or [],
            "assumptions": {"sequential_consistency": True, "no_spurious_wakeups": True},
            "bounds": bounds or dict(BOUNDS), "allowed_scope": dict(SCOPE)}


def completed(fn: str, prefix: str = "main") -> dict:
    return {"kind": "function_completed", "function": f"{prefix}::{fn}"}


def deadlock(pid: str = "no-deadlock") -> dict:
    return {"kind": "deadlock_free", "id": pid}


def preserved_all(fns: list[str], prefix: str = "main") -> list[dict]:
    return [{"kind": "reachable", "description": f"{prefix}::{f} completes",
             "goal": completed(f, prefix)} for f in fns]


# ─────────────────────────── authored cases ───────────────────────────

def case_abba() -> dict:
    mod = module([res_mutex("a"), res_mutex("b")],
                 [main_scope(["t1", "t2"]),
                  lock_fn("t1", [("a", "b")]), lock_fn("t2", [("a", "b")])])
    fixed = program([mod], name="abba_2lock")
    buggy_mod = module([res_mutex("a"), res_mutex("b")],
                       [main_scope(["t1", "t2"]),
                        lock_fn("t1", [("a", "b")]), lock_fn("t2", [("b", "a")])])
    buggy = program([buggy_mod], name="abba_2lock")
    c = contract("abba-2lock", properties=[deadlock()],
                 preserved=preserved_all(["t1", "t2"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a program with two mutexes A and B and two worker threads. "
                    "Each worker acquires both mutexes, works inside the critical "
                    "section, releases both, and terminates. Main spawns both workers "
                    "and joins them. Every interleaving must terminate and both workers "
                    "must complete.",
            "ground_truth": {"defect_family": "lock_order", "resources": ["main::a", "main::b"],
                             "statements": ["main::t2.l1", "main::t2.l1b"],
                             "expected_outcome_buggy": "FAIL", "expected_outcome_fixed": "PASS",
                             "expected_repair": "unify lock order",
                             "provenance": "authored; buggy CIR equals the LLM-generated "
                                           "pilot-v1 t2_abba model (sha d2d4958b...)"},
            "rust": {"buggy": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/mutex_deadlock/buggy.rs",
                     "fixed": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/mutex_deadlock/fixed.rs"}}


def case_cycle3() -> dict:
    def prog(t3: tuple[str, str]) -> dict:
        mod = module([res_mutex("a"), res_mutex("b"), res_mutex("c")],
                     [main_scope(["t1", "t2", "t3"]),
                      lock_fn("t1", [("a", "b")]), lock_fn("t2", [("b", "c")]),
                      lock_fn("t3", [t3])])
        return program([mod], name="cycle_3lock")
    c = contract("cycle-3lock", properties=[deadlock()],
                 preserved=preserved_all(["t1", "t2", "t3"]))
    return {"buggy": prog(("c", "a")), "fixed": prog(("a", "c")), "contract": c,
            "spec": "Design a program with three mutexes A, B, C and three worker "
                    "threads. Worker 1 uses A and B, worker 2 uses B and C, worker 3 "
                    "uses C and A. Each worker holds its two mutexes simultaneously, "
                    "releases them, and terminates. Main spawns all three and joins "
                    "them. Every interleaving must terminate and all three workers "
                    "must complete.",
            "ground_truth": {"defect_family": "lock_order",
                             "resources": ["main::a", "main::b", "main::c"],
                             "statements": ["main::t3.l1", "main::t3.l1b"],
                             "expected_outcome_buggy": "FAIL", "expected_outcome_fixed": "PASS",
                             "expected_repair": "global lock order",
                             "provenance": "authored"},
            "rust": {"buggy": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/three_way_deadlock/buggy.rs",
                     "fixed": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/three_way_deadlock/fixed.rs"}}


def case_cross_module() -> dict:
    buggy = json.loads((FROZEN / "t3_cross_module_abba_frozen.cir.json").read_text())
    # Fixed twin: the cross-module task acquires in a single global order.
    fixed = json.loads(json.dumps(buggy))
    for mod in fixed["modules"]:
        for fn in mod.get("functions", []):
            if fn["name"] == "t2":
                body = fn["body"]
                locks = [s for s in body if "lock" in s.get("kind", "") and
                         "unlock" not in s.get("kind", "")]
                if len(locks) >= 2 and locks[0]["resource"] == "other::b":
                    locks[0]["resource"], locks[1]["resource"] = "main::a", "other::b"
                unlocks = [s for s in body if s.get("kind") == "mutex_unlock"]
                if len(unlocks) >= 2 and unlocks[0]["resource"] == "main::a":
                    unlocks[0]["resource"], unlocks[1]["resource"] = "other::b", "main::a"
    c = json.loads((FROZEN / "t3_cross_module_abba_contract.json").read_text())
    return {"buggy": buggy, "contract": c, "fixed": fixed,
            "spec": "Design two modules that share cross-module resources. Module main "
                    "owns resource a, module other owns resource b. Two tasks each "
                    "acquire both resources (declared in requires.resources) and "
                    "release them. Every interleaving must terminate and both tasks "
                    "must complete across the module boundary.",
            "ground_truth": {"defect_family": "lock_order",
                             "resources": ["main::a", "other::b"],
                             "statements": ["main::t1.l1", "other::t2.l1"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "unify cross-module lock order",
                             "provenance": "LLM-generated pilot-v1 t3_cross_module_abba "
                                           "(frozen) + authored fixed twin, declared "
                                           "llm_generated"}}


def case_partial_bystander() -> dict:
    def worker(prefix: str, first_mutex: str, wait_sem: str, release_sem: str,
               second_mutex: str) -> dict:
        return {"name": prefix, "kind": "normal", "form": "closure", "body": [
            {"sid": "s1", "kind": "mutex_lock", "resource": f"main::{first_mutex}"},
            {"sid": "s2", "kind": "semaphore_release", "resource": f"main::{release_sem}"},
            {"sid": "s3", "kind": "semaphore_acquire", "resource": f"main::{wait_sem}"},
            {"sid": "s4", "kind": "mutex_lock", "resource": f"main::{second_mutex}"},
            {"sid": "s5", "kind": "mutex_unlock", "resource": f"main::{second_mutex}"},
            {"sid": "s6", "kind": "mutex_unlock", "resource": f"main::{first_mutex}"},
            {"sid": "s7", "kind": "return"},
        ]}

    def fixed_worker(name: str) -> dict:
        return lock_fn(name, [("a", "b")])

    # The bystander loops but has a statically present exit (the flag is never
    # set), so the validator's "infinite loop with no exit" check is satisfied
    # while the loop still keeps the system globally live.
    bystander = {"name": "bystander", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "branch", "cond": "flag == true", "then": "s3", "else": "s2"},
        {"sid": "s2", "kind": "goto", "target": "s1"},
        {"sid": "s3", "kind": "return"},
    ]}
    common = [res_mutex("a"), res_mutex("b"), res_sem("sa", 0), res_sem("sb", 0),
              res_var("flag", "Bool", False)]
    buggy_mod = module(
        common,
        [main_scope(["a", "b", "bystander"]),
         worker("a", "a", "sb", "sa", "b"),
         worker("b", "b", "sa", "sb", "a"),
         bystander])
    fixed_mod = module(
        common,
        [main_scope(["a", "b", "bystander"]),
         fixed_worker("a"), fixed_worker("b"), bystander])
    c = contract("partial-bystander",
                 properties=[deadlock(),
                             {"kind": "always_reachable", "id": "a-completes",
                              "goal": completed("a")},
                             {"kind": "always_reachable", "id": "b-completes",
                              "goal": completed("b")}],
                 preserved=preserved_all(["a", "b"]))
    return {"buggy": program([buggy_mod], name="partial_bystander"),
            "fixed": program([fixed_mod], name="partial_bystander"), "contract": c,
            "spec": "Design two workers A and B that each take two mutexes with an "
                    "intermediate semaphore handshake, plus an independent bystander "
                    "task that keeps making progress forever. Main spawns all three. "
                    "Although the bystander keeps the system globally live, every "
                    "reachable state must still be able to finish A and B, and both "
                    "workers must complete.",
            "ground_truth": {"defect_family": "goal_layer",
                             "resources": ["main::a", "main::b", "main::sa", "main::sb"],
                             "statements": ["main::a.s3", "main::b.s3"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "break the semaphore cross-wait",
                             "preregistered": {"buggy": {"deadlock_free": "PASS",
                                                         "always_reachable": "FAIL"},
                                               "fixed": {"deadlock_free": "PASS",
                                                         "always_reachable": "PASS"}},
                             "provenance": "authored"},
            "rust": {"buggy": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/partial_deadlock/buggy.rs",
                     "fixed": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/partial_deadlock/fixed.rs"}}


def case_lost_wakeup() -> dict:
    buggy_waiter = {"name": "waiter", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "condvar_wait", "condvar": "main::cv", "lock": "main::m"},
        {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s4", "kind": "return"},
    ]}
    fixed_waiter = {"name": "waiter", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "branch", "cond": "ready == true", "then": "s5", "else": "s3"},
        {"sid": "s3", "kind": "condvar_wait", "condvar": "main::cv", "lock": "main::m"},
        {"sid": "s4", "kind": "goto", "target": "s2"},
        {"sid": "s5", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s6", "kind": "return"},
    ]}
    fixed_notifier = {"name": "notifier", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "write_shared", "resource": "main::ready", "expr": "true"},
        {"sid": "s3", "kind": "condvar_notify", "condvar": "main::cv"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s5", "kind": "return"},
    ]}
    buggy_notifier = {"name": "notifier", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "condvar_notify", "condvar": "main::cv"},
        {"sid": "s3", "kind": "write_shared", "resource": "main::ready", "expr": "true"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s5", "kind": "return"},
    ]}
    resources = [res_mutex("m"), res_condvar("cv"),
                 res_var("ready", "Bool", False)]
    protection = [{"var": "ready", "lock": "m"}]
    buggy = program([module(resources, [main_scope(["waiter", "notifier"]),
                                        buggy_waiter, buggy_notifier],
                             protection=protection)], name="lost_wakeup")
    fixed = program([module(resources, [main_scope(["waiter", "notifier"]),
                                        fixed_waiter, fixed_notifier],
                            protection=protection)], name="lost_wakeup")
    c = contract("lost-wakeup", properties=[deadlock(),
                 {"kind": "reachability", "id": "ready-set",
                  "goal": {"kind": "var_eq", "resource": "main::ready", "value": True}}],
                 preserved=preserved_all(["waiter", "notifier"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a waiter/notifier pair sharing a mutex, a condition "
                    "variable and a boolean flag ready protected by the mutex. The "
                    "notifier sets ready and signals the condition variable; the "
                    "waiter must block until ready is true and then finish. The waiter "
                    "must terminate even if the notifier signals before the waiter "
                    "starts waiting.",
            "ground_truth": {"defect_family": "condvar",
                             "resources": ["main::m", "main::cv", "main::ready"],
                             "statements": ["main::waiter.s2", "main::notifier.s2"],
                             "expected_outcome_buggy": "FAIL", "expected_outcome_fixed": "PASS",
                             "expected_repair": "set the predicate before notify; "
                                                "re-check the predicate in a loop",
                             "provenance": "authored"}}


def case_rendezvous_both_send() -> dict:
    send = lambda n: {"name": n, "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "channel_send", "channel": "main::ch", "value": "1"},
        {"sid": "s2", "kind": "return"}]}
    recv = {"name": "r", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "channel_recv", "channel": "main::ch", "dst": "_"},
        {"sid": "s2", "kind": "return"}]}
    resources = [res_channel("ch", 0)]
    buggy = program([module(resources, [main_scope(["s1", "s2"]), send("s1"), send("s2")])],
                    name="rendezvous_both_send")
    fixed = program([module(resources, [main_scope(["s1", "r"]), send("s1"), recv])],
                    name="rendezvous_both_send")
    c = contract("rendezvous-both-send", properties=[deadlock()],
                 preserved=preserved_all(["s1"]) + (
                     [{"kind": "reachable", "description": "main::r completes",
                       "goal": completed("r")}]
                     if False else []))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a sender and a receiver that communicate over a "
                    "zero-capacity channel (rendezvous). Both sides must eventually "
                    "pair and terminate; no execution may block forever.",
            "ground_truth": {"defect_family": "channel",
                             "resources": ["main::ch"], "statements": ["main::s2.s1"],
                             "expected_outcome_buggy": "FAIL", "expected_outcome_fixed": "PASS",
                             "expected_repair": "pair each send with a receive",
                             "provenance": "authored"}}


def case_permit_leak() -> dict:
    resources = [res_sem("s", 1)]
    leak = {"name": "w1", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
        {"sid": "s2", "kind": "return"}]}
    fixed = {"name": "w1", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
        {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
        {"sid": "s3", "kind": "return"}]}
    other = {"name": "w2", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
        {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
        {"sid": "s3", "kind": "return"}]}
    buggy = program([module(resources, [main_scope(["w1", "w2"]), leak, other])],
                    name="permit_leak")
    ok = program([module(resources, [main_scope(["w1", "w2"]), fixed, other])],
                 name="permit_leak")
    c = contract("permit-leak", properties=[deadlock()], preserved=preserved_all(["w1", "w2"]))
    return {"buggy": buggy, "fixed": ok, "contract": c,
            "spec": "Design two workers that share a counting semaphore with one "
                    "permit. Each worker acquires the permit, does its work, releases "
                    "the permit and returns. Every interleaving must terminate and "
                    "both workers must complete.",
            "ground_truth": {"defect_family": "semaphore", "resources": ["main::s"],
                             "statements": ["main::w1.s2"],
                             "expected_outcome_buggy": "FAIL", "expected_outcome_fixed": "PASS",
                             "expected_repair": "release the permit on every path",
                             "provenance": "authored"}}


def case_throttle() -> dict:
    worker = lambda n: {"name": n, "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
        {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
        {"sid": "s3", "kind": "return"}]}
    resources = [res_sem("s", 2)]
    correct = program([module(resources, [main_scope(["w1", "w2", "w3"]),
                                          worker("w1"), worker("w2"), worker("w3")])],
                      name="throttle")
    c = contract("throttle", properties=[deadlock()],
                 preserved=preserved_all(["w1", "w2", "w3"]))
    return {"correct": correct, "contract": c,
            "spec": "Design three worker threads throttled by a counting semaphore "
                    "with two permits. Each worker acquires one permit, does its work, "
                    "releases the permit and returns. All interleavings must terminate.",
            "ground_truth": {"defect_family": "semaphore", "resources": ["main::s"],
                             "statements": [], "expected_outcome_correct": "PASS",
                             "provenance": "authored"}}


def case_counter_invariant() -> dict:
    worker = lambda n: {"name": n, "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "write_shared", "resource": "main::c", "expr": "c + 1"},
        {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s4", "kind": "return"}]}
    resources = [res_mutex("m"), res_var("c", {"Int": [0, 2]}, 0)]
    correct = program([module(resources, [main_scope(["w1", "w2"]),
                                          worker("w1"), worker("w2")],
                              protection=[{"var": "c", "lock": "m"}])],
                      name="counter_invariant")
    c = contract("counter-invariant", properties=[
        deadlock(),
        {"kind": "safety", "id": "counter-bound",
         "invariant": {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 2}}],
        preserved=preserved_all(["w1", "w2"]))
    return {"correct": correct, "contract": c,
            "spec": "Design two workers that each increment a bounded integer counter "
                    "(range 0..2) while holding a mutex. Every reachable state must "
                    "keep the counter within its declared bound and every interleaving "
                    "must terminate.",
            "ground_truth": {"defect_family": "atomic_data", "resources": ["main::c"],
                             "statements": [], "expected_outcome_correct": "PASS",
                             "provenance": "authored; exercises bounded Int + safety "
                                           "invariant (a capability with no paper Table 1 "
                                           "coverage)"}}


def case_scope_bound() -> dict:
    workers = [{"name": f"w{i}", "kind": "normal", "form": "closure", "bound": 2,
                "body": [{"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
                         {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
                         {"sid": "s3", "kind": "return"}]} for i in range(1, 4)]
    resources = [res_sem("s", 1)]
    correct = program([module(resources, [main_scope(["w1", "w2", "w3"])] + workers)],
                      name="scope_bound")
    c = contract("scope-bound", properties=[deadlock()],
                 preserved=preserved_all(["w1", "w2", "w3"]))
    return {"correct": correct, "contract": c,
            "spec": "Design a scope that starts three worker roles, where each role is "
                    "bounded to two concurrent activations, sharing a one-permit "
                    "semaphore. Every interleaving must terminate.",
            "ground_truth": {"defect_family": "structure", "resources": ["main::s"],
                             "statements": [], "expected_outcome_correct": "PASS",
                             "provenance": "authored; exercises scope + function bound"}}


def case_acquire_twice_no_release() -> dict:
    def w1(buggy: bool) -> dict:
        if buggy:
            body = [{"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
                    {"sid": "s2", "kind": "semaphore_acquire", "resource": "main::s"},
                    {"sid": "s3", "kind": "semaphore_release", "resource": "main::s"},
                    {"sid": "s4", "kind": "return"}]
        else:
            body = [{"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
                    {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
                    {"sid": "s3", "kind": "semaphore_acquire", "resource": "main::s"},
                    {"sid": "s4", "kind": "semaphore_release", "resource": "main::s"},
                    {"sid": "s5", "kind": "return"}]
        return {"name": "w1", "kind": "normal", "form": "closure", "body": body}

    w2 = {"name": "w2", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "semaphore_acquire", "resource": "main::s"},
        {"sid": "s2", "kind": "semaphore_release", "resource": "main::s"},
        {"sid": "s3", "kind": "return"}]}
    resources = [res_sem("s", 1)]
    buggy = program([module(resources, [main_scope(["w1", "w2"]), w1(True), w2])],
                    name="acquire_twice")
    fixed = program([module(resources, [main_scope(["w1", "w2"]), w1(False), w2])],
                    name="acquire_twice")
    c = contract("acquire-twice", properties=[deadlock()],
                 preserved=preserved_all(["w1", "w2"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design two workers sharing a counting semaphore with one permit. "
                    "Each worker may acquire the permit more than once, but must "
                    "release it the same number of times before returning. Every "
                    "interleaving must terminate and both workers must complete.",
            "ground_truth": {"defect_family": "semaphore", "resources": ["main::s"],
                             "statements": ["main::w1.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "release every acquired permit",
                             "provenance": "authored"}}


def case_bare_wait_no_predicate() -> dict:
    waiter_buggy = {"name": "waiter", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "condvar_wait", "condvar": "main::cv", "lock": "main::m"},
        {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s4", "kind": "return"}]}
    waiter_fixed = {"name": "waiter", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "branch", "cond": "ready == true", "then": "s5", "else": "s3"},
        {"sid": "s3", "kind": "condvar_wait", "condvar": "main::cv", "lock": "main::m"},
        {"sid": "s4", "kind": "goto", "target": "s2"},
        {"sid": "s5", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s6", "kind": "return"}]}
    notifier_buggy = {"name": "notifier", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "condvar_notify", "condvar": "main::cv"},
        {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s4", "kind": "return"}]}
    notifier_fixed = {"name": "notifier", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "write_shared", "resource": "main::ready", "expr": "true"},
        {"sid": "s3", "kind": "condvar_notify", "condvar": "main::cv"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s5", "kind": "return"}]}
    protection = [{"var": "ready", "lock": "m"}]
    resources = [res_mutex("m"), res_condvar("cv"), res_var("ready", "Bool", False)]
    buggy = program([module(resources, [main_scope(["waiter", "notifier"]),
                                        waiter_buggy, notifier_buggy],
                             protection=protection)], name="bare_wait")
    fixed = program([module(resources, [main_scope(["waiter", "notifier"]),
                                        waiter_fixed, notifier_fixed],
                            protection=protection)], name="bare_wait")
    c = contract("bare-wait", properties=[deadlock(),
                 {"kind": "reachability", "id": "ready-set",
                  "goal": {"kind": "var_eq", "resource": "main::ready", "value": True}}],
                 preserved=preserved_all(["waiter", "notifier"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a waiter that blocks on a condition variable until a "
                    "predicate protected by the mutex becomes true, and a notifier "
                    "that makes the predicate true and signals. The waiter must "
                    "terminate even if the notifier signals before the waiter waits.",
            "ground_truth": {"defect_family": "condvar",
                             "resources": ["main::m", "main::cv"],
                             "statements": ["main::waiter.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "re-check a predicate in a loop and "
                                                "set it before notify",
                             "provenance": "authored"},
            "rust": {"buggy": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/signal_loss/buggy.rs",
                     "fixed": LEGACY.parent / "legacy-cir2cvn/benchmarks/rust/signal_loss/fixed.rs"}}


def case_bounded_backpressure_lock_held() -> dict:
    sender_buggy = {"name": "sender", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "channel_send", "channel": "main::ch", "value": "1"},
        {"sid": "s3", "kind": "channel_send", "channel": "main::ch", "value": "2"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s5", "kind": "return"}]}
    receiver_buggy = {"name": "receiver", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
        {"sid": "s2", "kind": "channel_recv", "channel": "main::ch", "dst": "_"},
        {"sid": "s3", "kind": "channel_recv", "channel": "main::ch", "dst": "_"},
        {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
        {"sid": "s5", "kind": "return"}]}
    sender_fixed = {"name": "sender", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "channel_send", "channel": "main::ch", "value": "1"},
        {"sid": "s2", "kind": "channel_send", "channel": "main::ch", "value": "2"},
        {"sid": "s3", "kind": "return"}]}
    receiver_fixed = {"name": "receiver", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "channel_recv", "channel": "main::ch", "dst": "_"},
        {"sid": "s2", "kind": "channel_recv", "channel": "main::ch", "dst": "_"},
        {"sid": "s3", "kind": "return"}]}
    resources = [res_mutex("m"), res_channel("ch", 1)]
    buggy = program([module(resources, [main_scope(["sender", "receiver"]),
                                        sender_buggy, receiver_buggy])], name="backpressure")
    fixed = program([module(resources, [main_scope(["sender", "receiver"]),
                                        sender_fixed, receiver_fixed])], name="backpressure")
    c = contract("backpressure", properties=[deadlock()],
                 preserved=preserved_all(["sender", "receiver"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a sender and a receiver over a channel of capacity one, "
                    "plus one mutex both occasionally need. The sender sends two "
                    "values. Every interleaving must terminate: no side may block on "
                    "the channel while holding the lock the other needs.",
            "ground_truth": {"defect_family": "channel", "resources": ["main::m", "main::ch"],
                             "statements": ["main::sender.s3"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "release the lock before the channel op",
                             "provenance": "authored"}}


def case_counter_overflow_safety() -> dict:
    def worker(name: str, guarded: bool) -> dict:
        if guarded:
            body = [
                {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
                {"sid": "s2", "kind": "branch", "cond": "c < 1", "then": "s3", "else": "s4"},
                {"sid": "s3", "kind": "write_shared", "resource": "main::c", "expr": "c + 1"},
                {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
                {"sid": "s5", "kind": "return"},
            ]
        else:
            body = [
                {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
                {"sid": "s2", "kind": "write_shared", "resource": "main::c", "expr": "c + 1"},
                {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m"},
                {"sid": "s4", "kind": "return"},
            ]
        return {"name": name, "kind": "normal", "form": "closure", "body": body}

    resources = [res_mutex("m"), res_var("c", {"Int": [0, 2]}, 0)]
    protection = [{"var": "c", "lock": "m"}]
    buggy = program([module(resources, [main_scope(["w1", "w2"]), worker("w1", False),
                                        worker("w2", False)], protection=protection)],
                    name="counter_overflow")
    fixed = program([module(resources, [main_scope(["w1", "w2"]), worker("w1", True),
                                        worker("w2", True)], protection=protection)],
                    name="counter_overflow")
    c = contract("counter-overflow", properties=[
        deadlock(),
        {"kind": "safety", "id": "counter-bound",
         "invariant": {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 1}}],
        preserved=preserved_all(["w1", "w2"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design two workers that each increment a bounded integer counter "
                    "(declared range 0..2) under a mutex, while a safety invariant "
                    "requires the counter never to exceed 1. Every reachable state "
                    "must keep the invariant and every interleaving must terminate.",
            "ground_truth": {"defect_family": "atomic_data", "resources": ["main::c"],
                             "statements": ["main::w1.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "guard the increment with the "
                                                "invariant bound",
                             "provenance": "authored; bounded Int + safety invariant"}}

def case_atomic_lost_update() -> dict:
    def worker(name: str, use_cas: bool) -> dict:
        if use_cas:
            body = [
                {"sid": "s1", "kind": "atomic_load", "resource": "main::c", "dst": "l"},
                {"sid": "s2", "kind": "atomic_cas", "resource": "main::c",
                 "expected": "l", "desired": "l + 1", "dst": "l2"},
                {"sid": "s3", "kind": "branch", "cond": "l2 == l",
                 "then": "s5", "else": "s1"},
                {"sid": "s5", "kind": "return"},
            ]
        else:
            body = [
                {"sid": "s1", "kind": "atomic_load", "resource": "main::c", "dst": "l"},
                {"sid": "s2", "kind": "atomic_store", "resource": "main::c", "value": "l + 1"},
                {"sid": "s3", "kind": "return"},
            ]
        return {"name": name, "kind": "normal", "form": "closure",
                "locals": [{"name": "l", "type": "Int", "modeled": True},
                           {"name": "l2", "type": "Int", "modeled": True}],
                "body": body}

    resources = [res_atomic("c", {"Int": [0, 2]}, 0)]
    buggy = program([module(resources, [main_scope(["w1", "w2"]),
                                        worker("w1", False), worker("w2", False)])],
                    name="lost_update")
    fixed = program([module(resources, [main_scope(["w1", "w2"]),
                                        worker("w1", True), worker("w2", True)])],
                    name="lost_update")
    c = contract("lost-update", properties=[
        deadlock(),
        {"kind": "always_reachable", "id": "both-increments",
         "goal": {"kind": "var_eq", "resource": "main::c", "value": 2}}],
        preserved=preserved_all(["w1", "w2"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design two workers that each increment a shared atomic counter by "
                    "exactly one, starting from zero, using a compare-and-swap retry "
                    "loop so no update is lost. From every reachable state it must "
                    "still be possible for the counter to reach two, and every "
                    "interleaving must terminate.",
            "ground_truth": {"defect_family": "atomic_data", "resources": ["main::c"],
                             "statements": ["main::w1.s2", "main::w2.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "use compare-and-swap with retry "
                                                "instead of load-then-store",
                             "provenance": "authored; atomics + always_reachable"}}

def case_nested_scope_lock_order() -> dict:
    def x2(order: tuple[str, str]) -> dict:
        return lock_fn("x2", [order])

    def outer() -> dict:
        return {"name": "outer", "kind": "normal", "form": "closure", "body": [
            {"sid": "s1", "kind": "scope", "funcs": ["main::x1", "main::x2"]},
            {"sid": "s2", "kind": "return"}]}

    resources = [res_mutex("a"), res_mutex("b")]
    buggy = program([module(resources, [main_scope(["outer"]), outer(),
                                        lock_fn("x1", [("a", "b")]), x2(("b", "a"))])],
                    name="nested_scope")
    fixed = program([module(resources, [main_scope(["outer"]), outer(),
                                        lock_fn("x1", [("a", "b")]), x2(("a", "b"))])],
                    name="nested_scope")
    c = contract("nested-scope", properties=[deadlock()],
                 preserved=preserved_all(["outer"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a worker that starts a nested scope of two tasks; the two "
                    "inner tasks both need mutexes A and B and must not form a circular "
                    "wait. Every interleaving must terminate and the outer worker must "
                    "complete.",
            "ground_truth": {"defect_family": "structure",
                             "resources": ["main::a", "main::b"],
                             "statements": ["main::x2.s1", "main::x2.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "unify the inner lock order",
                             "provenance": "authored; nested scope"}}


def case_scope_worker_abba() -> dict:
    resources = [res_mutex("a"), res_mutex("b")]
    buggy = program([module(resources, [main_scope(["w1", "w2"]),
                                        lock_fn("w1", [("a", "b")]),
                                        lock_fn("w2", [("b", "a")])])], name="scope_abba")
    fixed = program([module(resources, [main_scope(["w1", "w2"]),
                                        lock_fn("w1", [("a", "b")]),
                                        lock_fn("w2", [("a", "b")])])], name="scope_abba")
    c = contract("scope-abba", properties=[deadlock()], preserved=preserved_all(["w1", "w2"]))
    return {"buggy": buggy, "fixed": fixed, "contract": c,
            "spec": "Design a scope of two workers that both acquire mutexes A and B. "
                    "Every interleaving must terminate and both workers must complete.",
            "ground_truth": {"defect_family": "structure", "resources": ["main::a", "main::b"],
                             "statements": ["main::w2.s1", "main::w2.s2"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_outcome_fixed": "PASS",
                             "expected_repair": "unify worker lock order",
                             "provenance": "authored; scope structure"}}


def case_worker_with_payload() -> dict:
    """A correct case whose worker calls a body-less helper, so codegen has a HOLE."""

    def worker(name: str) -> dict:
        return {"name": name, "kind": "normal", "form": "closure", "body": [
            {"sid": "s1", "kind": "mutex_lock", "resource": "main::m"},
            {"sid": "s2", "kind": "call", "func": "main::compute"},
            {"sid": "s3", "kind": "write_shared", "resource": "main::acc", "expr": "acc + 1"},
            {"sid": "s4", "kind": "mutex_unlock", "resource": "main::m"},
            {"sid": "s5", "kind": "return"}]}

    compute = {"name": "compute", "kind": "normal", "body": []}
    resources = [res_mutex("m"), res_var("acc", "Int", 0)]
    protection = [{"var": "acc", "lock": "m"}]
    correct = program([module(resources, [main_scope(["w1", "w2"]),
                                          worker("w1"), worker("w2"), compute],
                              protection=protection)], name="worker_payload")
    c = contract("worker-payload", properties=[deadlock()],
                 preserved=preserved_all(["w1", "w2"]))
    return {"correct": correct, "contract": c,
            "spec": "Design a group of two workers. Each worker acquires a mutex, "
                    "calls a sequential helper that performs local computation, "
                    "updates a shared counter under the mutex, releases the mutex and "
                    "returns. All interleavings must terminate and both workers must "
                    "complete.",
            "ground_truth": {"defect_family": "structure", "resources": ["main::acc"],
                             "statements": [],
                             "expected_outcome_correct": "PASS",
                             "provenance": "authored; nobody helper gives codegen a HOLE"}}


def case_unbounded_unknown() -> dict:
    w = {"name": "w", "kind": "normal", "form": "closure", "body": [
        {"sid": "s1", "kind": "write_shared", "resource": "main::x", "expr": "x + 1"},
        {"sid": "s2", "kind": "goto", "target": "s1"}]}
    resources = [res_var("x", "Int", 0)]
    unknown = program([module(resources, [main_scope(["w"]), w],
                              protection=[{"var": "x", "lock": "main::m"}] if False else [])],
                      name="unbounded_int")
    c = contract("unbounded-int", properties=[deadlock()],
                 bounds={"max_threads": 8, "max_frames_per_thread": 8,
                         "max_states": 5000, "max_depth": 64,
                         "max_boundary_events": 256})
    return {"correct": unknown, "contract": c,
            "spec": "Design a program that keeps incrementing an unbounded shared "
                    "integer while a worker loops. The bounded analysis is expected to "
                    "hit its state budget; report UNKNOWN rather than a verdict.",
            "ground_truth": {"defect_family": "boundary", "resources": ["main::x"],
                             "statements": [], "expected_outcome_correct": "UNKNOWN",
                             "provenance": "authored; negative control for the analysis "
                                           "bound"}}

# ─────────────────────────── reused seed cases ───────────────────────────

REUSE = [
    ("lock-order", "two_independent_cycles",
     "tests/repro_bench/two_cycles.json", "tests/repro_bench/two_cycles_contract.json",
     "FAIL", "concir regression seed (two independent cycles)"),
    ("condvar", "same_cv_different_locks",
     "tests/repro_round3/r2_multi_lock_cv.json", "tests/repro_round3/r2_multi_lock_cv_contract.json",
     "PASS", "concir round3 regression seed (precise wait-set: same cv, different "
             "locks; correct by construction)"),
    ("condvar", "notify_one_multi_waiter_wrong_pick",
     "tests/repro_round2/notify_choice_false_pass.json",
     "tests/repro_round2/notify_choice_false_pass_contract.json",
     "FAIL", "concir round2 regression seed (notify_one choice)"),
    ("channel", "send_while_holding_mutex",
     "tests/repro_bench/channel_deadlock.json", "tests/repro_bench/channel_deadlock_contract.json",
     "FAIL", "concir regression seed (channel rendezvous mismatch)"),
    ("structure", "finite_call_loop",
     "tests/repro_round2/finite_call_loop.json", "tests/repro_round2/finite_call_loop_contract.json",
     "PASS", "concir round2 regression seed (finite call loop)"),
    ("structure", "spawn_join_loop_finite",
     "tests/repro_round3/r2_spawn_join_loop.json", "tests/repro_round3/r2_spawn_join_loop_contract.json",
     "PASS", "concir round3 regression seed (finite spawn/join loop)"),
    ("boundary", "rwlock_unsupported",
     "examples/complex_rwlock.json", None, "UNSUPPORTED",
     "ConcIR example; RwLock is unsupported in the backend"),
    ("boundary", "async_select_unsupported",
     "examples/async_workers.json", None, "UNSUPPORTED",
     "ConcIR example; async/await is unsupported in the backend"),
]


# ─────────────────────────── runner ───────────────────────────

def binary() -> Path:
    env = os.environ.get("CONCIR_BACKEND")
    if env and Path(env).is_file():
        return Path(env)
    for candidate in (CONCIR / "target/release/concir-backend",
                      CONCIR / "target/debug/concir-backend"):
        if candidate.is_file():
            return candidate
    raise FileNotFoundError("concir-backend not found")


def run_cmd(command: str, program_path: Path) -> dict:
    proc = subprocess.run([str(binary()), command, str(program_path)],
                          capture_output=True, text=True, timeout=120)
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"outcome": "PROTOCOL_ERROR", "exit": proc.returncode,
                "stderr": proc.stderr[-400:]}
    payload["exit"] = proc.returncode
    return payload


def run_explore(program_path: Path, contract_path: Path, engine: str) -> dict:
    proc = subprocess.run(
        [str(binary()), "explore", str(program_path), str(contract_path), engine],
        capture_output=True, text=True, timeout=120)
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"outcome": "PROTOCOL_ERROR", "exit": proc.returncode,
                "stderr": proc.stderr[-400:]}
    payload["exit"] = proc.returncode
    return payload


def _holds(fn: str, *resources: str, prefix: str = "main") -> dict:
    return {"kind": "reachable",
            "description": f"{fn} holds [{', '.join(resources)}] at once",
            "goal": {"kind": "holds_all", "function": fn,
                     "resources": [r if "::" in r else f"{prefix}::{r}" for r in resources]}}


def _var_eq(resource: str, value) -> dict:
    return {"kind": "reachable", "description": f"{resource} == {value}",
            "goal": {"kind": "var_eq", "resource": resource, "value": value}}


# Design intent that a patch must preserve (F-1). Keyed "family/case".
DESIGN_INTENT: dict[str, tuple[list[dict], str]] = {
    "lock-order/abba_2lock": ([_holds("main::t1", "main::a", "main::b"),
                               _holds("main::t2", "main::a", "main::b")],
                              "Each worker must hold both mutexes at once; a fix that "
                              "removes the nested critical section is not a repair."),
    "lock-order/cycle_3lock": ([_holds("main::t1", "main::a", "main::b"),
                                _holds("main::t2", "main::b", "main::c"),
                                _holds("main::t3", "main::c", "main::a")],
                               "Every worker must hold its pair of mutexes at once."),
    "lock-order/cross_module_cycle": ([_holds("main::t1", "main::a", "other::b"),
                                       _holds("other::t2", "main::a", "other::b")],
                                      "Both cross-module tasks must hold both resources "
                                      "at once."),
    "lock-order/partial_deadlock_bystander": ([_holds("main::a", "main::a", "main::b"),
                                               _holds("main::b", "main::a", "main::b")],
                                              "Each worker must still enter a critical "
                                              "section holding both mutexes."),
    "lock-order/two_independent_cycles": ([_holds("main::t1", "a", "b"),
                                           _holds("main::t2", "a", "b"),
                                           _holds("main::t3", "c", "d"),
                                           _holds("main::t4", "c", "d")],
                                          "Each worker must hold its pair at once."),
    "structure/scope_worker_abba": ([_holds("main::w1", "a", "b"),
                                     _holds("main::w2", "a", "b")],
                                    "Each worker must hold both mutexes at once."),
    "structure/nested_scope_lock_order": ([_holds("main::x1", "a", "b"),
                                           _holds("main::x2", "a", "b")],
                                          "The inner tasks must hold both mutexes at "
                                          "once."),
    "semaphore/permit_leak": ([_holds("main::w1", "main::s")],
                              "A worker must hold the permit while it works."),
    "semaphore/acquire_twice_no_release": ([_holds("main::w1", "main::s")],
                                           "A worker must hold the permit while it "
                                           "works."),
    "condvar/lost_wakeup_notify_before_wait": ([_var_eq("main::ready", True)],
                                               "The design requires the ready flag to "
                                               "become true."),
    "condvar/bare_wait_no_predicate": ([_var_eq("main::ready", True)],
                                       "The design requires the ready flag to become "
                                       "true."),
    "condvar/notify_one_multi_waiter_wrong_pick": ([_holds("main::w1", "main::m")],
                                                   "A waiter must hold the mutex while "
                                                   "it waits."),
    "atomic-data/atomic_lost_update": ([_var_eq("main::c", 2)],
                                       "Both increments must be observable in the "
                                       "counter's reachable values."),
    "channel/rendezvous_both_send": ([
        {"kind": "reachable", "description": "the channel is drained",
         "goal": {"kind": "channel_empty", "resource": "main::ch"}}],
        "The message must be delivered, leaving the channel drained."),
}


def _apply_design_intent(family: str, case: str, data: dict) -> None:
    key = f"{family}/{case}"
    entry = DESIGN_INTENT.get(key)
    if not entry or data.get("buggy") is None:
        return
    predicates, why = entry
    data["contract"].setdefault("preserved", []).extend(predicates)
    data["spec"] = data["spec"].rstrip() + " " + why


def write_case(family: str, case: str, data: dict) -> dict:
    _apply_design_intent(family, case, data)
    task = OUT / family / case
    task.mkdir(parents=True, exist_ok=True)
    # remove stale variants so an old buggy/fixed/correct file can never linger
    for stale in ("buggy.cir.json", "fixed.cir.json", "correct.cir.json"):
        (task / stale).unlink(missing_ok=True)
    (task / "spec.md").write_text(data["spec"].strip() + "\n", encoding="utf-8")
    write_json(task / "contract.json", data["contract"])
    if data.get("buggy") is not None:
        write_json(task / "buggy.cir.json", data["buggy"])
    if data.get("fixed") is not None:
        write_json(task / "fixed.cir.json", data["fixed"])
    if data.get("correct") is not None:
        write_json(task / "correct.cir.json", data["correct"])
    write_json(task / "ground_truth.json", data["ground_truth"])
    # rust references
    rust_src = data.get("rust")
    if rust_src:
        rdir = task / "rust"
        rdir.mkdir(exist_ok=True)
        if rust_src.get("buggy") and Path(rust_src["buggy"]).is_file():
            shutil.copyfile(rust_src["buggy"], rdir / "buggy.rs")
        if rust_src.get("fixed") and Path(rust_src["fixed"]).is_file():
            shutil.copyfile(rust_src["fixed"], rdir / "fixed.rs")
    # repair task (for every case that has a buggy variant)
    if data.get("buggy") is not None:
        rust_rel = f"{family}/{case}/rust/buggy.rs"
        write_json(task / "repair_task.json", {
            "requirements": data["spec"].strip(),
            "input_cir": f"{family}/{case}/buggy.cir.json",
            "contract": f"{family}/{case}/contract.json",
            "input_rust": rust_rel if (task / "rust/buggy.rs").is_file() else None,
            "ground_truth": data["ground_truth"],
        })

    files = {}
    for name in ("spec.md", "contract.json", "buggy.cir.json", "fixed.cir.json",
                 "correct.cir.json", "ground_truth.json", "repair_task.json",
                 "rust/buggy.rs", "rust/fixed.rs"):
        path = task / name
        if path.is_file():
            files[f"families/{family}/{case}/{name}"] = sha256(path)
    return {"family": family, "case": case, "directory": f"families/{family}/{case}",
            "files": files,
            "spec": f"families/{family}/{case}/spec.md",
            "contract": f"families/{family}/{case}/contract.json"}


def add_reuse() -> list[dict]:
    entries = []
    for family, case, prog_rel, contract_rel, expected, provenance in REUSE:
        src_prog = CONCIR / prog_rel
        if not src_prog.is_file():
            print(f"  ! missing seed {src_prog}", file=sys.stderr)
            continue
        prog = json.loads(src_prog.read_text(encoding="utf-8"))
        if contract_rel:
            contract_spec = json.loads((CONCIR / contract_rel).read_text(encoding="utf-8"))
        else:
            contract_spec = contract("boundary", properties=[deadlock()])
        data = {"spec": f"Seed case {family}/{case}. Design intent: the modelled plan "
                        f"must terminate and satisfy the declared properties.",
                "contract": contract_spec, "ground_truth": {
                    "defect_family": family, "resources": [], "statements": [],
                    "expected_outcome_buggy": expected,
                    "provenance": provenance},
                "rust": None}
        if expected == "PASS":
            data["correct"] = prog
        else:
            data["buggy"] = prog
        entries.append(write_case(family, case, data))
    return entries


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    tasks: list[dict] = []
    mismatches: list[str] = []
    failures: list[str] = []

    authored = [
        ("lock-order", "abba_2lock", case_abba),
        ("lock-order", "cycle_3lock", case_cycle3),
        ("lock-order", "cross_module_cycle", case_cross_module),
        ("lock-order", "partial_deadlock_bystander", case_partial_bystander),
        ("condvar", "lost_wakeup_notify_before_wait", case_lost_wakeup),
        ("channel", "rendezvous_both_send", case_rendezvous_both_send),
        ("semaphore", "permit_leak", case_permit_leak),
        ("semaphore", "acquire_twice_no_release", case_acquire_twice_no_release),
        ("semaphore", "throttle_n_permits", case_throttle),
        ("condvar", "bare_wait_no_predicate", case_bare_wait_no_predicate),
        ("channel", "bounded_backpressure_lock_held", case_bounded_backpressure_lock_held),
        ("atomic-data", "bounded_counter_invariant", case_counter_invariant),
        ("atomic-data", "counter_overflow_safety", case_counter_overflow_safety),
        ("atomic-data", "atomic_lost_update", case_atomic_lost_update),
        ("structure", "scope_bound_k_workers", case_scope_bound),
        ("structure", "nested_scope_lock_order", case_nested_scope_lock_order),
        ("structure", "scope_worker_abba", case_scope_worker_abba),
        ("structure", "worker_with_payload", case_worker_with_payload),
        ("boundary", "unbounded_int_unknown", case_unbounded_unknown),
    ]
    for family, case, builder in authored:
        try:
            data = builder()
        except Exception as exc:  # noqa: BLE001
            failures.append(f"{family}/{case}: build error: {exc}")
            continue
        tasks.append(write_case(family, case, data))

    tasks.extend(add_reuse())

    # validate every case
    for entry in tasks:
        task_dir = ROOT / "benchmarks/families" / entry["family"] / entry["case"]
        contract_path = task_dir / "contract.json"
        variants = []
        if (task_dir / "buggy.cir.json").is_file():
            variants.append(("buggy", task_dir / "buggy.cir.json"))
        if (task_dir / "fixed.cir.json").is_file():
            variants.append(("fixed", task_dir / "fixed.cir.json"))
        if (task_dir / "correct.cir.json").is_file():
            variants.append(("correct", task_dir / "correct.cir.json"))
        results = {}
        for label, path in variants:
            check = run_cmd("check", path)
            support = run_cmd("support", path)
            if check.get("valid") is not True:
                failures.append(f"{entry['family']}/{entry['case']} {label}: "
                                f"check not valid: {check.get('diagnostics')}")
            p = run_explore(path, contract_path, "petri")
            i = run_explore(path, contract_path, "interp")
            entry.setdefault("results", {})[label] = {
                "check_valid": check.get("valid"), "support": support.get("supported"),
                "petri": p.get("outcome"), "interp": i.get("outcome"),
                "petri_complete": p.get("complete"), "interp_complete": i.get("complete"),
                "states": p.get("states_explored"), "wall_ms": p.get("wall_ms"),
            }
            if p.get("outcome") != i.get("outcome"):
                failures.append(f"{entry['family']}/{entry['case']} {label}: "
                                f"petri {p.get('outcome')} != interp {i.get('outcome')}")
            results[label] = p.get("outcome")
        # compare with ground truth
        gt = json.loads((task_dir / "ground_truth.json").read_text(encoding="utf-8"))
        for label, actual in results.items():
            key = f"expected_outcome_{label}"
            expected = gt.get(key)
            if expected is not None and actual != expected:
                mismatches.append(f"{entry['family']}/{entry['case']} {label}: "
                                  f"expected {expected} got {actual}")
        entry["status"] = "ready"
        entry["ground_truth"] = gt
        # compatibility fields for the v2 manifest loader
        base = f"families/{entry['family']}/{entry['case']}"
        entry["id"] = f"{entry['family']}/{entry['case']}"
        entry["directory"] = base
        entry["spec"] = f"{base}/spec.md"
        entry["contract"] = f"{base}/contract.json"
        entry["buggy_cir"] = (f"{base}/buggy.cir.json"
                              if (task_dir / "buggy.cir.json").is_file() else None)
        entry["fixed_cir"] = (f"{base}/fixed.cir.json"
                              if (task_dir / "fixed.cir.json").is_file() else None)
        entry["correct_cir"] = (f"{base}/correct.cir.json"
                                if (task_dir / "correct.cir.json").is_file() else None)
        entry["buggy_rs"] = (f"{base}/rust/buggy.rs"
                             if (task_dir / "rust/buggy.rs").is_file() else None)
        entry["fixed_rs"] = (f"{base}/rust/fixed.rs"
                             if (task_dir / "rust/fixed.rs").is_file() else None)

    tasks.extend(_legacy_entries())
    tasks.extend(_real_case_entries())

    # R-1: generate de-leaked repair inputs for every ready buggy case.
    counters: dict[str, int] = {}
    for entry in tasks:
        if entry.get("status") != "ready" or not entry.get("buggy_cir"):
            continue
        fam = entry.get("family") or entry["directory"].split("/")[0]
        case = entry.get("case") or entry["directory"].split("/")[-1]
        counters[fam] = counters.get(fam, 0) + 1
        task_dir = ROOT / "benchmarks" / entry["directory"]
        try:
            info = sanitize_case(
                fam, case, task_dir, counters[fam],
                entry.get("contract") or f"{entry['directory']}/contract.json",
                entry.get("ground_truth") or {}, base_rel=entry["directory"])
        except KeyError as exc:
            failures.append(str(exc))
            continue
        entry.setdefault("files", {}).update(info["files"])
        entry["repair_input"] = {k: v for k, v in info.items() if k != "files"}
        entry["requirements_sha256"] = info["requirements_sha256"]

    manifest = {"version": 2, "description":
                "Capability-family benchmark for the current ConcIR. status=ready "
                "means check/support/explore passed; per-case results and "
                "pre-registration mismatches are recorded.",
                "bounds": BOUNDS, "tasks": tasks, "mismatches": mismatches}
    write_json(ROOT / "benchmarks/MANIFEST.json", manifest)

    _write_families_md(tasks, mismatches)
    print(f"families: {len(tasks)} cases, {len(mismatches)} pre-registration mismatches")
    for m in mismatches:
        print(f"  mismatch: {m}")
    if failures:
        print("FAILURES:", file=sys.stderr)
        for f in failures:
            print(f"  {f}", file=sys.stderr)
        return 1
    return 0


def _legacy_entries() -> list[dict]:
    """The retired paper Table 1 patterns, kept but excluded from v2 runs."""

    legacy_dir = ROOT / "benchmarks/legacy-paper-patterns"
    entries = []
    for path in sorted(legacy_dir.glob("P*")):
        if not path.is_dir():
            continue
        files = {}
        for name in ("spec.md", "contract.json", "buggy.cir.json", "fixed.cir.json",
                     "buggy.rs", "fixed.rs", "ground_truth.json", "legacy_brief.md"):
            full = path / name
            if full.is_file():
                files[f"legacy-paper-patterns/{path.name}/{name}"] = sha256(full)
        gt = {}
        gt_path = path / "ground_truth.json"
        if gt_path.is_file():
            gt = json.loads(gt_path.read_text(encoding="utf-8"))
        gt.setdefault("provenance", "paper Table 1 pattern; status=legacy, excluded "
                                    "from v2 experiments")
        entries.append({
            "id": path.name, "status": "legacy",
            "directory": f"legacy-paper-patterns/{path.name}",
            "family": "legacy", "case": path.name,
            "spec": f"legacy-paper-patterns/{path.name}/spec.md"
                    if (path / "spec.md").is_file() else None,
            "contract": f"legacy-paper-patterns/{path.name}/contract.json"
                        if (path / "contract.json").is_file() else None,
            "buggy_cir": None, "fixed_cir": None, "correct_cir": None,
            "buggy_rs": None, "fixed_rs": None,
            "ground_truth": gt, "files": files, "results": {}})
    return entries


def _real_case_entries() -> list[dict]:
    entries = []
    base_dir = ROOT / "benchmarks/real-cases/cases"
    for case in ("rmw-zenoh-998", "dashmap-369"):
        base = base_dir / case
        if not base.is_dir():
            continue
        files = {}
        for name in ("buggy.cir.json", "fixed.cir.json", "contract.json", "expected.json"):
            if (base / name).is_file():
                files[f"real-cases/cases/{case}/{name}"] = sha256(base / name)
        gt = {}
        if (base / "expected.json").is_file():
            gt = json.loads((base / "expected.json").read_text(encoding="utf-8"))
        entries.append({
            "id": f"real-cases/{case}", "status": "ready",
            "directory": f"real-cases/cases/{case}",
            "family": "real-cases", "case": case,
            "spec": None,
            "contract": f"real-cases/cases/{case}/contract.json",
            "buggy_cir": f"real-cases/cases/{case}/buggy.cir.json",
            "fixed_cir": (f"real-cases/cases/{case}/fixed.cir.json"
                          if (base / "fixed.cir.json").is_file() else None),
            "correct_cir": None, "buggy_rs": None, "fixed_rs": None,
            "ground_truth": gt, "files": files, "results": {}})
    return entries


def _write_families_md(tasks: list[dict], mismatches: list[str]) -> None:
    lines = ["# FAMILIES — capability-family benchmark", "",
             "Cases are validated with `check`/`support`/`explore` (petri and interp).",
             "`status=ready` means both engines agreed and the files are frozen in",
             "`benchmarks/MANIFEST.json`. Pre-registration mismatches are listed.",
             "", "| family | case | variants | petri results | source |",
             "| --- | --- | --- | --- | --- |"]
    for entry in tasks:
        variants = ", ".join((entry.get("results") or {}).keys())
        results = ", ".join(f"{k}={v}" for k, v in (entry.get("results") or {}).items())
        provenance = (entry.get("ground_truth") or {}).get("provenance", "")
        lines.append(f"| {entry['family']} | {entry['case']} | {variants} | {results} | "
                     f"{provenance} |")
    if mismatches:
        lines += ["", "## Pre-registration mismatches", ""]
        lines += [f"- {m}" for m in mismatches]
    lines += ["", "## Capabilities covered that the paper's Table 1 did not",
              "",
              "- precise condvar wait-set (same cv, different locks; notify choice);",
              "- bounded channel (rendezvous and capacity >= 1);",
              "- counting semaphore (throttle, permit leak);",
              "- bounded Int data domains and `safety` / `var_cmp` invariants;",
              "- `scope` + function `bound`, finite spawn/join/goto loops;",
              "- `always_reachable` (AG EF) at the goal layer;",
              "- `UNSUPPORTED` (RwLock, async/await) and `UNKNOWN` (unbounded Int) "
              "boundary controls."]
    (ROOT / "benchmarks/FAMILIES.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
