"""Offline tests for the isolated Rust arm (cargo only; no miri/lockbud/network)."""

from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from cir_workflow.rust_arm import (
    RustArmProject, ToolRun, classify_detection, classify_tool_run, parse_test_result,
)

GOOD = """
fn add(a: i32, b: i32) -> i32 { a + b }

#[test]
fn adds() { assert_eq!(add(1, 2), 3); }

fn main() {}
"""

NO_TESTS = """
fn main() { println!("nothing tested"); }
"""

BAD = """
fn main() { let x: i32 = "not an int"; }
"""


class ThreadLeakTests(unittest.TestCase):
    def test_thread_leak_is_classified(self):
        root = Path(__file__).resolve().parents[2] / "experiments"
        marker = "terminated without waiting for all remaining threads"
        sample = None
        if root.is_dir():
            for path in root.rglob("stderr.txt"):
                try:
                    if marker in path.read_text(encoding="utf-8", errors="ignore"):
                        sample = path
                        break
                except OSError:
                    continue
        if sample is None:
            raise unittest.SkipTest("no recorded Miri thread-leak stderr found")
        run = ToolRun(tool="miri", argv=[], exit_code=1, wall_ms=1, timed_out=False,
                      stdout="", stderr=sample.read_text(encoding="utf-8"),
                      stdout_sha256="", stderr_sha256="")
        result = classify_tool_run(run)
        self.assertEqual(result["status"], "thread_leak")
        self.assertTrue(result["thread_leak"])
        self.assertEqual(result["detected"], [])


class ParseTests(unittest.TestCase):
    def test_parses_counts(self):
        text = "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured"
        self.assertEqual(parse_test_result(text), (3, 0))
        self.assertEqual(parse_test_result("no summary here"), (None, None))

    def test_tool_error_is_not_clean(self):
        run = ToolRun(tool="miri", argv=["cargo", "miri"], exit_code=1, wall_ms=5,
                      timed_out=False, stdout="", stderr="error: unsupported API",
                      stdout_sha256="", stderr_sha256="")
        self.assertEqual(classify_tool_run(run)["status"], "tool_error")

    def test_detection_classifier(self):
        self.assertIn("deadlock", classify_detection("error: deadlock detected"))
        self.assertIn("data_race", classify_detection("found a data race here"))
        self.assertEqual(classify_detection("all good"), [])

    def test_task_name_in_path_is_not_a_detection(self):
        # The cargo warning prints the probe path, which contains the task name
        # `partial_deadlock_bystander`; that must not count as a deadlock.
        text = ("warning: binary `cir_arm_probe`\n"
                "1 | .../partial_deadlock_bystander_fixed/probe/target/miri/.../x\n"
                "error: the main thread terminated without waiting for all remaining threads")
        self.assertEqual(classify_detection(text), [])


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
        self.assertEqual(record["behavior_test_passed"], 1)
        self.assertEqual(record["behavior_test_reason"], "ok")

    def test_zero_tests_is_unknown_not_pass(self):
        project = RustArmProject(self.root, NO_TESTS, name="notests")
        record = project.analyze(run_miri=False, run_lockbud=False)
        self.assertTrue(record["build_ok"])
        self.assertIsNone(record["behavior_test_ok"])
        self.assertEqual(record["behavior_test_reason"], "no_tests")

    def test_build_failure_is_recorded_not_raised(self):
        project = RustArmProject(self.root, BAD, name="bad")
        record = project.analyze(run_miri=False, run_lockbud=False)
        self.assertFalse(record["build_ok"])
        self.assertIsNone(record["behavior_test"])
        self.assertEqual(record["lockbud"]["status"], "lockbud_unavailable")

    def test_raw_output_archived_with_argv_and_env(self):
        project = RustArmProject(self.root, GOOD, name="evidence")
        record = project.analyze(run_miri=False, run_lockbud=False)
        build = record["build"]
        self.assertIsNotNone(build["evidence_dir"])
        files = build["files"]
        for name in ("argv.json", "env.json", "stdout.txt", "stderr.txt", "exit.txt"):
            self.assertTrue(Path(files[name]).is_file(), name)
        argv = json.loads(Path(files["argv.json"]).read_text())
        self.assertIn("cargo", argv["argv"][0])
        # build does not set MIRIFLAGS; the env file still records the filtered env
        env = json.loads(Path(files["env.json"]).read_text())
        self.assertNotIn("MIRIFLAGS", env)


if __name__ == "__main__":
    unittest.main()
