"""Semantics audit: unknown/reachability/deadlock guarantee direction.

These tests pin the *actual* backend behaviour on minimal models. They do not
assert a guarantee that the backend does not provide.
"""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
FIX = Path(__file__).resolve().parent / "fixtures" / "semantics"
CONTRACT = FIX / "contract.json"


def _binary() -> Path | None:
    p = REPO.parent / "ConcIR" / "target" / "release" / "concir-backend"
    return p if p.is_file() else None


def _explore(program: Path) -> dict:
    binary = _binary()
    proc = subprocess.run([str(binary), "explore", str(program), str(CONTRACT), "petri"],
                          capture_output=True, text=True, timeout=60)
    return json.loads(proc.stdout)


class SemanticsAuditTests(unittest.TestCase):
    def setUp(self):
        if _binary() is None:
            self.skipTest("concir-backend not built")

    def test_concrete_unreachable_goal_is_fail_not_pass(self):
        d = _explore(FIX / "concrete.json")
        # x stays 0, so x==1 is unreachable: decided FAIL, not a spurious PASS.
        self.assertEqual(d["outcome"], "FAIL")
        self.assertTrue(d["complete"])
        self.assertEqual(d["properties"][0]["outcome"], "FAIL")

    def test_unknown_source_is_unsupported_not_spurious_pass(self):
        # A body-less external call (a source of unknown) is refused, not turned
        # into a reachable goal.
        d = _explore(FIX / "unknown_reach.json")
        self.assertEqual(d["outcome"], "UNSUPPORTED")
        self.assertFalse(d["complete"])
        self.assertEqual(d["properties"], [])
        self.assertTrue(d["unsupported"])


if __name__ == "__main__":
    unittest.main()
