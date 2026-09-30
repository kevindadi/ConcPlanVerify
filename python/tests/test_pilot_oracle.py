"""Limited independent checks: renaming must not turn a defect into a pass."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from cir_workflow.pilot_cases import evaluate_role, load_case
from cir_workflow.pilot_oracle import requirement_hold, requirement_terminates


class OracleRegressionTests(unittest.TestCase):
    def test_renamed_guard_stays_a_hold_failure(self):
        src = Path("python/tests/fixtures/pilot_v7/early_release/defect.rs").read_text()
        renamed = (src.replace("let ga ", "let guard_a ")
                   .replace("drop(ga)", "drop(guard_a)")
                   .replace("let gb ", "let guard_b ")
                   .replace("drop(gb)", "drop(guard_b)"))
        result = requirement_hold(renamed, "w", "a", "b", build_ok=True)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["observed"], "released_before_second_lock")

    def test_comment_and_string_are_not_operations(self):
        src = '''
fn w(a: std::sync::Arc<std::sync::Mutex<i32>>, b: std::sync::Arc<std::sync::Mutex<i32>>) {
    let note = "a.lock()"; // b.lock()
    /* h.join() */
    let guard_a = a.lock().unwrap();
    drop(guard_a);
    let guard_b = b.lock().unwrap();
}
fn main() {}
'''
        result = requirement_hold(src, "w", "a", "b", build_ok=True)
        self.assertEqual(result["status"], "fail")

    def test_scope_exit_releases_the_lock(self):
        src = '''
fn w(a: std::sync::Arc<std::sync::Mutex<i32>>, b: std::sync::Arc<std::sync::Mutex<i32>>) {
    { let guard_a = a.lock().unwrap(); }
    let guard_b = b.lock().unwrap();
}
'''
        result = requirement_hold(src, "w", "a", "b", build_ok=True)
        self.assertEqual(result["status"], "fail")

    def test_unsupported_syntax_is_unknown(self):
        src = '''
fn w(a: std::sync::Arc<std::sync::Mutex<i32>>, b: std::sync::Arc<std::sync::Mutex<i32>>) {
    if true { let g = a.lock().unwrap(); }
    let h = b.lock().unwrap();
}
'''
        result = requirement_hold(src, "w", "a", "b", build_ok=True)
        self.assertEqual(result["status"], "unknown")

    def test_build_failure_blocks_a_pass(self):
        case = load_case("lock_order")
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_role(case, "this is not rust\n", Path(td))
        self.assertNotEqual(result["requirement"]["status"], "pass")
        self.assertFalse(result["build_ok"])

    def test_renamed_handle_still_joins(self):
        case = load_case("lock_order")
        renamed = (case["control"]
                   .replace("let h =", "let worker_handle =")
                   .replace("h.join()", "worker_handle.join()"))
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_role(case, renamed, Path(td))
        self.assertEqual(result["requirement"]["status"], "pass")
        self.assertEqual(result["design"]["status"], "pass")

    def test_comment_join_and_skipped_worker_do_not_pass(self):
        case = load_case("no_join")
        commented = '''
fn w(m: std::sync::Arc<std::sync::Mutex<i32>>) { let _g = m.lock().unwrap(); }
fn main() {
    let m = std::sync::Arc::new(std::sync::Mutex::new(0));
    let h = std::thread::spawn(move || { let _note = "h.join()"; });
    // h.join()
    let _ = h;
}
'''
        with tempfile.TemporaryDirectory() as td:
            result = evaluate_role(case, commented, Path(td))
        self.assertNotEqual(result["requirement"]["status"], "pass")
        self.assertNotEqual(result["design"]["status"], "pass")

    def test_controls_pass_and_crash_does_not(self):
        case = load_case("compute_value")
        with tempfile.TemporaryDirectory() as td:
            good = evaluate_role(case, case["control"], Path(td) / "c")
        self.assertEqual(good["requirement"]["status"], "pass")
        crashed = '''
fn main() { println!("DONE done=6"); panic!("boom"); }
'''
        result = requirement_terminates.__wrapped__ if False else None
        from cir_workflow.pilot_oracle import requirement_stdout
        with tempfile.TemporaryDirectory() as td:
            out = requirement_stdout(crashed, Path(td), "DONE done=6")
        self.assertNotEqual(out["status"], "pass")


if __name__ == "__main__":
    unittest.main()
