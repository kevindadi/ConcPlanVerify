"""v27 DeepSeek ablation switches: goal gate off, explore off (unit level)."""

from __future__ import annotations

import unittest

from cir_workflow.generation import _goal_flags, _safety_only_pass


CONTRACT = {
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R1"]},
        {"kind": "always_reachable", "id": "goal", "req": ["R2"],
         "goal": {"kind": "var_eq", "resource": "main::c", "value": 2}},
    ],
    "preserved": [
        {"kind": "reachable", "id": "p", "req": ["R3"],
         "goal": {"kind": "function_completed", "function": "main::w1"}},
    ],
}


class _Explore:
    def __init__(self, outcomes, complete=True):
        self.complete = complete
        self.payload = {"properties": [{"outcome": o} for o in outcomes]}


class AblationTests(unittest.TestCase):
    def test_goal_flags(self):
        self.assertEqual(_goal_flags(CONTRACT), [False, True, True])

    def test_safety_only_pass_ignores_goal_failures(self):
        # safety PASS, goal/preservation FAIL -> accepted under without_goal_gate
        self.assertTrue(_safety_only_pass(CONTRACT, _Explore(["PASS", "FAIL", "FAIL"])))

    def test_safety_only_pass_rejects_safety_failure(self):
        self.assertFalse(_safety_only_pass(CONTRACT, _Explore(["FAIL", "PASS", "PASS"])))

    def test_safety_only_pass_requires_complete(self):
        self.assertFalse(_safety_only_pass(CONTRACT, _Explore(["PASS", "PASS", "PASS"], complete=False)))


if __name__ == "__main__":
    unittest.main()
