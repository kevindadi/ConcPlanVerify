"""Requirement clauses for the one verified channel defect."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from cir_workflow.send_holding_requirements import (
    EXPECTED_STDOUT, combine, evaluate_requirements, execution_checks, structural_checks,
)

ROOT = Path("/Users/kevin/paper-review/papers/ConcPlanVerify")
DEFECT = (ROOT / "notes/strong-link-v5/reexec-deepseekflash/DeepSeek Flash"
          / "channel__send_while_holding_mutex/rep0/source.rs").read_text()
DERIVED = (ROOT / "notes/strong-link-v11/derived_control.rs").read_text()

ALTERNATE = DERIVED.replace(
    """fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch1.send(1).unwrap();
    let _ = ch2.recv().unwrap();
}""",
    """fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>, m: Arc<Mutex<()>>) {
    ch1.send(1).unwrap();
    let _ = ch2.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
}""",
).replace(
    """fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _ = ch1.recv().unwrap();
    ch2.send(1).unwrap();
}""",
    """fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>, m: Arc<Mutex<()>>) {
    let _ = ch1.recv().unwrap();
    ch2.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
}""",
)


def _ok_run(stdout=EXPECTED_STDOUT):
    return {"kind": "completed", "stdout": stdout + "\n", "returncode": 0, "timed_out": False}


class RequirementTests(unittest.TestCase):
    def test_print_only_does_not_pass(self):
        src = 'fn main() { println!("DONE done=1"); }\n'
        result = combine(structural_checks(src), execution_checks([_ok_run(), _ok_run()]))
        self.assertEqual(result["status"], "fail")

    def test_deleted_communication_does_not_pass(self):
        src = DERIVED.replace("ch1.send(1).unwrap();\n", "").replace("ch2.send(1).unwrap();\n", "")
        result = combine(structural_checks(src), execution_checks([_ok_run()]))
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["checks"]["R2"]["status"], "fail")

    def test_unbounded_channels_fail_rendezvous(self):
        structural = structural_checks(DEFECT)
        self.assertEqual(structural["R2"]["status"], "fail")
        result = combine(structural, execution_checks([_ok_run(), _ok_run()]))
        self.assertNotEqual(result["status"], "bounded_covered_satisfied")

    def test_one_bad_stdout_fails_even_if_the_last_is_correct(self):
        runs = [_ok_run("WRONG"), _ok_run(), _ok_run()]
        execution = execution_checks(runs)
        self.assertEqual(execution["R5"]["status"], "fail")
        result = combine(structural_checks(DERIVED), execution)
        self.assertEqual(result["status"], "fail")

    def test_removed_shared_lock_does_not_pass(self):
        block = "    {\n        let _guard = m.lock().unwrap();\n    }\n"
        src = DERIVED.replace(block, "")
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_requirements(src, Path(td) / "nolock")
        self.assertEqual(result["checks"]["R1"]["status"], "fail")
        self.assertNotEqual(result["status"], "bounded_covered_satisfied")
        self.assertTrue(result["runs"])
        self.assertTrue(all(run.get("kind") == "completed" for run in result["runs"]))

    def test_unreachable_workers_do_not_pass(self):
        src = DERIVED.replace(
            "fn main() {",
            'fn main() {\n    println!("DONE done=1");\n    return;',
            1)
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_requirements(src, Path(td) / "dead")
        self.assertEqual(result["checks"]["R1"]["status"], "fail")
        self.assertEqual(result["checks"]["R2"]["status"], "fail")
        self.assertNotEqual(result["status"], "bounded_covered_satisfied")
        self.assertEqual(result["checks"]["R5"]["status"], "pass")

    def test_distinct_mutex_roots_do_not_pass(self):
        src = DERIVED.replace(
            "let m_r = Arc::clone(&m);",
            "let m_r = Arc::new(Mutex::new(()));")
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_requirements(src, Path(td) / "two-locks")
        self.assertEqual(result["checks"]["R1"]["status"], "fail")
        self.assertNotEqual(result["status"], "bounded_covered_satisfied")
        self.assertTrue(all(run.get("kind") == "completed" for run in result["runs"]))

    def test_derived_control_passes_covered_checks(self):
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_requirements(DERIVED, Path(td))
        self.assertEqual(result["status"], "bounded_covered_satisfied")
        self.assertIn("R4 universal", result["uncovered"][0])

    def test_alternate_order_is_not_a_requirement_failure(self):
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_requirements(ALTERNATE, Path(td))
        self.assertEqual(result["status"], "bounded_covered_satisfied")
        self.assertNotEqual(structural_checks(ALTERNATE)["R2"]["status"], "fail")


if __name__ == "__main__":
    unittest.main()
