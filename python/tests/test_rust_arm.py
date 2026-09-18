"""Offline tests for the isolated Rust arm (cargo only; no miri/lockbud/network)."""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

from cir_workflow.rust_arm import RustArmProject, classify_detection

GOOD = """
fn add(a: i32, b: i32) -> i32 { a + b }

#[test]
fn adds() { assert_eq!(add(1, 2), 3); }

fn main() {}
"""

BAD = """
fn main() { let x: i32 = "not an int"; }
"""


@unittest.skipUnless(shutil.which("cargo"), "cargo not available")
class RustArmTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def test_build_and_test_green(self):
        project = RustArmProject(self.root, GOOD, name="good")
        record = project.analyze(run_miri=False, run_lockbud=False)
        self.assertTrue(record["build_ok"])
        self.assertTrue(record["behavior_test_ok"])

    def test_build_failure_is_recorded_not_raised(self):
        project = RustArmProject(self.root, BAD, name="bad")
        record = project.analyze(run_miri=False, run_lockbud=False)
        self.assertFalse(record["build_ok"])
        self.assertIsNone(record["behavior_test"])
        self.assertEqual(record["lockbud"]["status"], "lockbud_unavailable")

    def test_detection_classifier(self):
        self.assertIn("deadlock", classify_detection("error: deadlock detected"))
        self.assertIn("data_race", classify_detection("found a data race here"))
        self.assertEqual(classify_detection("all good"), [])


if __name__ == "__main__":
    unittest.main()
