"""Functional execution records timeout, nonzero exit, and a clean pass."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from cir_workflow import candidate_eval


def _proc(code, stdout):
    return subprocess.CompletedProcess(args=["probe"], returncode=code, stdout=stdout, stderr="err")


class FunctionalExecTests(unittest.TestCase):
    def test_timeout_is_recorded_and_finalized(self):
        def boom(*args, **kwargs):
            raise subprocess.TimeoutExpired(cmd="probe", timeout=1, output=b"", stderr=b"")

        def build(work, **kwargs):
            binary = Path(work) / "target/debug/probe"
            binary.parent.mkdir(parents=True, exist_ok=True)
            binary.write_bytes(b"")
            return True, "ok"

        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            with mock.patch("cir_workflow.candidate_eval.run_model_check",
                            return_value={"ok": False, "path": str(out / "missing"), "error": "x"}), \
                    mock.patch("cir_workflow.rust_oracle.cargo_build", side_effect=build), \
                    mock.patch("cir_workflow.candidate_eval.subprocess.run", side_effect=boom), \
                    mock.patch("cir_workflow.rust_oracle.instrument_wrappers",
                               side_effect=RuntimeError("stop after functional")):
                result = candidate_eval.evaluate_candidate(
                    "fn main(){ println!(\"DONE done=6\"); }\n",
                    Path(__file__).resolve().parents[1] / "tests/fixtures/stronglink/compute.cir.json",
                    Path(__file__).resolve().parents[1] / "tests/fixtures/stronglink/compute.contract.json",
                    out, binary=Path("/bin/true"),
                    functional_spec={"test_id": "stdout_eq", "expected": "DONE done=6"})
            self.assertTrue((out / "result.json").is_file())
            self.assertEqual(result["functional"]["reason"], "timeout")
            self.assertTrue(result["functional"]["timed_out"])
            self.assertNotEqual(result["functional"]["status"], "pass")

    def test_nonzero_exit_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as td:
            work = Path(td)
            with mock.patch("cir_workflow.rust_oracle.cargo_build", return_value=(True, "ok")), \
                    mock.patch("cir_workflow.candidate_eval.capture_program_stdout",
                               return_value={"stdout": "DONE done=6\n", "stderr": "boom",
                                             "returncode": 1, "timed_out": False, "elapsed_s": 0.1,
                                             "error": None, "kind": "completed"}):
                result = candidate_eval.run_functional_check(
                    "fn main(){}\n", work, {"test_id": "stdout_eq", "expected": "DONE done=6"})
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["reason"], "nonzero_exit")
        self.assertEqual(result["supports"], [])

    def test_clean_completion_passes(self):
        with tempfile.TemporaryDirectory() as td:
            work = Path(td)
            with mock.patch("cir_workflow.rust_oracle.cargo_build", return_value=(True, "ok")), \
                    mock.patch("cir_workflow.candidate_eval.capture_program_stdout",
                               return_value={"stdout": "DONE done=6\n", "stderr": "",
                                             "returncode": 0, "timed_out": False, "elapsed_s": 0.1,
                                             "error": None, "kind": "completed"}):
                result = candidate_eval.run_functional_check(
                    "fn main(){}\n", work, {"test_id": "stdout_eq", "expected": "DONE done=6"})
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["supports"], ["external_stdout"])


if __name__ == "__main__":
    unittest.main()
