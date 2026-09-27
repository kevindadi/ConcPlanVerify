"""Binding-check CLI: verified / unresolved / violated, no heuristic proof."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("binding_check", REPO / "scripts" / "binding_check.py")
binding_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(binding_check)

CIR = {"modules": [{"name": "main",
                    "resources": [{"name": "a", "type": "Mutex"}],
                    "functions": [{"name": "main"}, {"name": "w1"}]}]}


class BindingCheckTests(unittest.TestCase):
    def test_manifest_claim_disagreeing_with_structure_is_violated(self):
        resources = [{"name": "a", "kind": "Mutex", "site": "10"}]
        r = binding_check.check(resources, CIR, [{"rust": "a", "cir": "main::b"}])
        self.assertIn("a", r["violated"])

    def test_manifest_claim_matching_structure_is_verified(self):
        resources = [{"name": "a", "kind": "Mutex", "site": "10"}]
        r = binding_check.check(resources, CIR, [{"rust": "a", "cir": "main::a"}])
        self.assertIn("a", r["verified"])

    def test_duplicate_name_is_unresolved_not_verified(self):
        resources = [{"name": "a", "kind": "Mutex", "site": "10"},
                     {"name": "a", "kind": "Mutex", "site": "20"}]
        r = binding_check.check(resources, CIR, None)
        self.assertEqual(r["verified"], {})
        self.assertIn("a", r["unresolved"])

    def test_structural_spawn_entry_is_verified(self):
        resources = [{"name": "spawn1", "kind": "Spawn", "entry": "w1",
                      "unique_entry": True, "site": "5"}]
        r = binding_check.check(resources, CIR, None)
        self.assertEqual(r["verified"]["spawn1"]["cir"], "main::w1")

    def test_ambiguous_closure_entry_is_unresolved(self):
        resources = [{"name": "helper", "kind": "Spawn", "entry": "helper",
                      "unique_entry": False, "site": "5"}]
        r = binding_check.check(resources, CIR, None)
        self.assertIn("helper", r["unresolved"])


if __name__ == "__main__":
    unittest.main()
