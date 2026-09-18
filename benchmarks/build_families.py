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
    c = json.loads((FROZEN / "t3_cross_module_abba_contract.json").read_text())
    return {"buggy": buggy, "contract": c, "fixed": None,
            "spec": "Design two modules that share cross-module resources. Module main "
                    "owns resource a, module other owns resource b. Two tasks each "
                    "acquire both resources (declared in requires.resources) and "
                    "release them. Every interleaving must terminate and both tasks "
                    "must complete across the module boundary.",
            "ground_truth": {"defect_family": "lock_order",
                             "resources": ["main::a", "other::b"],
                             "statements": ["main::t1.l1", "other::t2.l1"],
                             "expected_outcome_buggy": "FAIL",
                             "expected_repair": "unify cross-module lock order",
                             "provenance": "LLM-generated pilot-v1 t3_cross_module_abba "
                                           "(frozen), declared llm_generated"}}


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
                             "provenance": "authored"}}


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


def write_case(family: str, case: str, data: dict) -> dict:
    task = OUT / family / case
    task.mkdir(parents=True, exist_ok=True)
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

    files = {}
    for name in ("spec.md", "contract.json", "buggy.cir.json", "fixed.cir.json",
                 "correct.cir.json", "ground_truth.json",
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
        ("semaphore", "throttle_n_permits", case_throttle),
        ("atomic-data", "bounded_counter_invariant", case_counter_invariant),
        ("structure", "scope_bound_k_workers", case_scope_bound),
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
