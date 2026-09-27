"""Evidence state machine v4: layered, conservative, evidence-only verdict."""

from __future__ import annotations

import hashlib
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
    arts = [{"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
            for p in (src, cir, con)]
    return {"source_path": str(src), "cir_path": str(cir), "contract_path": str(con),
            "artifacts": arts}


def _result(tmp: Path, **over) -> dict:
    base = {
        **_files(tmp), "cell": "m/t",
        "stages": {"source_build": "ok", "instrument": "ok", "instrumented_build": "ok"},
        "n_runs": 32, "runs_started": 32, "runs_completed": 32, "hang": False,
        "raw_events": 10, "projected_events": 10,
        "conform": {"traces": 1, "statuses": {"conformant": 1}, "violations": []},
        "monitor": {"status": "ok", "properties": [["p1", "PASS_bounded"]]},
        "binding": {"mapping": {"m": "main::m"}, "ambiguous": [], "violated": []},
        "thread_lifecycle": {"spawn": 1},
    }
    base.update(over)
    return base


def _ev(r, contract=CONTRACT, accepted=True, props=None, complete=True):
    return evaluate_reexecution(r, contract, accepted=accepted,
                                cir_props=({"p1": "PASS"} if props is None else props),
                                cir_complete=complete)


class StateMachineTests(unittest.TestCase):
    def test_happy_path(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)))
        self.assertEqual(led.current_evaluation, "satisfied_bounded")
        self.assertTrue(led.all_obligations_satisfied)

    def test_binding_violated_blocks_obligations(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"m": "main::m"},
                                           "ambiguous": [], "violated": ["m"]})
            led = _ev(r)
        self.assertEqual(led.current_evaluation, "binding_declaration_error")
        self.assertFalse(led.all_obligations_satisfied)

    def test_model_unknown_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), props={"p1": "UNKNOWN"}, complete=True)
        self.assertFalse(led.model["verified"])
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")

    def test_model_missing_property_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), props={}, complete=True)
        self.assertFalse(led.model["verified"])
        self.assertEqual(led.model["missing_properties"], ["p1"])

    def test_verdict_independent_of_historical_acceptance(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), conform={"traces": 1, "statuses": {"violation": 1},
                                           "violations": [{"status": "violation",
                                                           "got": "mutex_lock:m",
                                                           "resource": "m"}]})
            a = _ev(r, accepted=True)
            b = _ev(r, accepted=False)
        self.assertEqual(a.current_evaluation, "explicit_failure")
        self.assertEqual(b.current_evaluation, "explicit_failure")
        self.assertNotEqual(a.historical_acceptance, b.historical_acceptance)

    def test_source_build_failure_is_not_runtime_crash(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), stages={"source_build": "failed"},
                        runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "source_build_failed")
        self.assertNotEqual(led.run["state"], "runtime_crash")

    def test_instrument_build_failure_distinct(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), stages={"source_build": "ok", "instrument": "ok",
                                          "instrumented_build": "failed"},
                        runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "instrument_build_failed")

    def test_planned_runs_not_started_is_not_run(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), n_runs=32, runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "not_run")

    def test_empty_projection_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), projected_events=0, raw_events=64)
            led = _ev(r)
        self.assertEqual(led.trace["state"], "empty_projection")
        self.assertFalse(led.all_obligations_satisfied)

    def test_binding_gap_violation_is_unresolved_not_independent(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {}, "ambiguous": [{"rust": "tx"}],
                                           "violated": []},
                        conform={"traces": 1, "statuses": {"violation": 1},
                                 "violations": [{"status": "violation",
                                                 "got": "channel_send:tx",
                                                 "resource": "tx"}]})
            led = _ev(r)
        self.assertEqual(led.trace["state"], "binding_unresolved")
        self.assertFalse(led.trace["independent_violation"])

    def test_binding_gap_and_independent_both_preserved(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"m": "main::m"},
                                           "ambiguous": [{"rust": "tx"}],
                                           "violated": []},
                        conform={"traces": 2, "statuses": {"violation": 2},
                                 "violations": [
                                     {"status": "violation", "got": "channel_send:tx",
                                      "resource": "tx"},
                                     {"status": "violation", "got": "mutex_lock:m",
                                      "resource": "m"}]})
            led = _ev(r)
        self.assertTrue(led.trace["binding_gap"])
        self.assertTrue(led.trace["independent_violation"])
        self.assertEqual(led.current_evaluation, "explicit_failure")

    def test_hash_mismatch_blocks(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            r["artifacts"][0]["sha256"] = "WRONG"
            led = _ev(r)
        self.assertFalse(led.hashes["verified"])
        self.assertFalse(led.all_obligations_satisfied)

    def test_empty_contract_never_passes(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), contract={"properties": [], "preserved": []},
                      props={})
        self.assertFalse(led.all_obligations_satisfied)
        self.assertEqual(led.current_evaluation, "inconclusive")


if __name__ == "__main__":
    unittest.main()
