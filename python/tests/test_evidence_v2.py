"""Evidence state machine: layered, conservative, no false upgrade."""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.evidence_v2 import evaluate_reexecution, hash_verification

CONTRACT = {"properties": [{"id": "p1", "kind": "safety", "req": ["R1"]}]}


def _files(tmp: Path) -> dict:
    src = tmp / "s.rs"; cir = tmp / "c.json"; con = tmp / "ct.json"
    src.write_text("fn main(){}")
    cir.write_text("{}")
    con.write_text("{}")
    return {"source_path": str(src), "cir_path": str(cir), "contract_path": str(con),
            "source_sha256": hashlib.sha256(src.read_bytes()).hexdigest(),
            "cir_sha256": hashlib.sha256(cir.read_bytes()).hexdigest(),
            "contract_sha256": hashlib.sha256(con.read_bytes()).hexdigest()}


def _result(tmp: Path, **over) -> dict:
    base = {
        **_files(tmp), "cell": "m/t",
        "stages": {"source_build": "ok", "instrumented_build": "ok"},
        "n_runs": 32, "runs_completed": 32, "hang": False,
        "raw_events": 10, "projected_events": 10,
        "conform": {"traces": 1, "statuses": {"conformant": 1}},
        "monitor": {"status": "ok", "properties": [["p1", "PASS_bounded"]]},
        "binding": {"mapping": {"m": "main::m"}, "ambiguous": []},
        "thread_lifecycle": {"spawn": 1},
    }
    base.update(over)
    return base


class StateMachineTests(unittest.TestCase):
    def test_1_binding_unresolved_blocks_obligations(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {}, "ambiguous": [{"rust": "x"}]})
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertFalse(led.all_obligations_satisfied)
        self.assertTrue(led.identity["relevant_unresolved"])

    def test_2_tool_error_is_not_a_violation(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), conform={"traces": 1, "statuses": {"error": 1}})
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.trace["state"], "tool_error")
        self.assertEqual(led.current_evaluation, "inconclusive")

    def test_3_empty_contract_never_passes(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            led = evaluate_reexecution(r, {"properties": [], "preserved": []},
                                       accepted=True, cir_props={}, cir_complete=True)
        self.assertFalse(led.all_obligations_satisfied)
        self.assertEqual(led.current_evaluation, "inconclusive")

    def test_4_model_failed_property_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "FAIL"}, cir_complete=True)
        self.assertFalse(led.model["verified"])
        self.assertEqual(led.model["failed_properties"], ["p1"])

    def test_5_hash_mismatch_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), source_sha256="WRONG_NONEMPTY")
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertFalse(led.hashes["verified"])
        self.assertFalse(led.all_obligations_satisfied)

    def test_6_no_completed_runs_is_failure(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), n_runs=32, runs_completed=0, hang=False)
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.run["state"], "crash")
        self.assertEqual(led.current_evaluation, "explicit_failure")

    def test_7_empty_projection_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), projected_events=0, raw_events=64)
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.trace["state"], "empty_projection")
        self.assertFalse(led.all_obligations_satisfied)

    def test_8_historical_accepted_does_not_override_violation(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), conform={"traces": 1, "statuses": {"violation": 1}})
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertTrue(led.historical_acceptance)
        self.assertEqual(led.current_evaluation, "explicit_failure")

    def test_9_historical_rejection_is_preserved(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), conform={"traces": 1, "statuses": {"violation": 1}})
            led = evaluate_reexecution(r, CONTRACT, accepted=False,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertFalse(led.historical_acceptance)
        self.assertEqual(led.current_evaluation, "not_accepted")

    def test_happy_path_all_obligations_satisfied(self):
        with tempfile.TemporaryDirectory() as td:
            led = evaluate_reexecution(_result(Path(td)), CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertTrue(led.all_obligations_satisfied)
        self.assertEqual(led.current_evaluation, "satisfied_bounded")
        self.assertEqual(led.properties[0].guarantee, "bounded_observation")


if __name__ == "__main__":
    unittest.main()
