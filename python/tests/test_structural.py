"""Task-level structural fidelity checks (no Rust/CLI needed)."""

from __future__ import annotations

import copy
import json
import unittest

from cir_workflow.structural import run_structural_check
from tests._helpers import FIXTURES


def _abba_program():
    return json.loads((FIXTURES / "abba.json").read_text())


def _main_func(program, name):
    for module in program["modules"]:
        for function in module["functions"]:
            if function["name"] == name:
                return function
    raise KeyError(name)


class StructuralFidelityTests(unittest.TestCase):
    def test_faithful_abba_is_ok(self):
        result = run_structural_check("abba_inversion", _abba_program())
        self.assertTrue(result["ok"], result)

    def test_deleting_a_scope_member_is_not_faithful(self):
        program = _abba_program()
        scope = _main_func(program, "main")["body"][0]
        scope["funcs"] = ["t1"]  # t2 defined but never started
        result = run_structural_check("abba_inversion", program)
        self.assertFalse(result["ok"], result)
        self.assertFalse(result["criteria"]["scope_ok"])

    def test_unspawned_reversed_function_is_not_faithful(self):
        program = _abba_program()
        _main_func(program, "main")["body"][0]["funcs"] = ["t1"]  # t2 not in scope
        result = run_structural_check("abba_inversion", program)
        self.assertFalse(result["ok"], result)

    def test_early_release_of_first_lock_is_not_faithful(self):
        program = _abba_program()
        t1 = _main_func(program, "t1")
        t1["body"] = [copy.deepcopy(s) for s in [t1["body"][0], t1["body"][2],
                                                 t1["body"][1], t1["body"][3], t1["body"][4]]]
        # now: lock a, unlock a, lock b, unlock b
        result = run_structural_check("abba_inversion", program)
        self.assertFalse(result["ok"], result)
        self.assertFalse(result["criteria"]["t1_ok"])

    def test_same_order_pair_accepts_consistent_model(self):
        program = _abba_program()
        _main_func(program, "t2")["body"] = _main_func(program, "t1")["body"]
        result = run_structural_check("same_order_pair", program)
        self.assertTrue(result["ok"], result)

    def test_spawn_instead_of_scope_is_unknown(self):
        program = _abba_program()
        main = _main_func(program, "main")
        main["body"][0] = {"sid": "s1", "kind": "spawn", "func": "t1", "handle": "h1"}
        result = run_structural_check("abba_inversion", program)
        self.assertIsNone(result["ok"], result)

    def test_cross_module_local_shadow_is_not_faithful(self):
        program = {
            "program": "cross", "version": "3.5.0", "entry": "main::main",
            "modules": [
                {"name": "main",
                 "provides": {"resources": ["a"], "functions": ["main", "t1"]},
                 "requires": {"resources": [], "functions": ["other::t2"]},
                 "resources": [{"name": "a", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
                 "protection": [],
                 "functions": [
                     {"name": "main", "kind": "normal", "body": [
                         {"sid": "s1", "kind": "scope", "funcs": ["t1", "other::t2"]},
                         {"sid": "s2", "kind": "return"}]},
                     {"name": "t1", "kind": "normal", "form": "closure", "body": [
                         {"sid": "s1", "kind": "mutex_lock", "resource": "a"},
                         {"sid": "s2", "kind": "mutex_lock", "resource": "b"},
                         {"sid": "s3", "kind": "mutex_unlock", "resource": "b"},
                         {"sid": "s4", "kind": "mutex_unlock", "resource": "a"},
                         {"sid": "s5", "kind": "return"}]}]},
                {"name": "other",
                 "provides": {"resources": ["b"], "functions": ["t2"]},
                 "requires": {"resources": [], "functions": []},
                 "resources": [{"name": "b", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
                 "protection": [],
                 "functions": [
                     {"name": "t2", "kind": "normal", "form": "closure", "body": [
                         {"sid": "s1", "kind": "mutex_lock", "resource": "b"},
                         {"sid": "s2", "kind": "mutex_lock", "resource": "a"},
                         {"sid": "s3", "kind": "mutex_unlock", "resource": "a"},
                         {"sid": "s4", "kind": "mutex_unlock", "resource": "b"},
                         {"sid": "s5", "kind": "return"}]}]},
            ],
        }
        # bare `a`/`b` in both modules: ambiguity + missing requires -> not faithful
        result = run_structural_check("cross_module_abba", program)
        self.assertFalse(result["ok"], result)


if __name__ == "__main__":
    unittest.main()
