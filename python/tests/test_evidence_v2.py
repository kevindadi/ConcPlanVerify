"""Evidence ledger v2: finite evidence never upgrades into whole correspondence."""

from __future__ import annotations

import unittest

from cir_workflow.evidence_v2 import evaluate_reexecution

CONTRACT = {"properties": [{"id": "p1", "kind": "safety", "req": ["R1"]}]}
HASHES = {"source_sha256": "a", "cir_sha256": "b", "contract_sha256": "c"}


def _result(**over):
    base = {
        **HASHES, "cell": "m/t", "stages": {"source_build": "ok",
                                            "instrumented_build": "ok"},
        "raw_events": 10, "projected_events": 10,
        "conform": {"traces": 1, "statuses": {"conformant": 1}},
        "monitor": {"status": "ok", "properties": [["p1", "PASS_bounded"]]},
        "binding": {"mapping": {"m": "main::m"}, "ambiguous": []},
        "thread_lifecycle": {"spawn": 1},
    }
    base.update(over)
    return base


class LedgerV2RegressionTests(unittest.TestCase):
    def test_A_no_conform_result_is_not_sufficient(self):
        # source + one event + PASS_bounded, but no conform result at all
        r = _result(conform=None)
        led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "insufficient")
        self.assertFalse(led.observed_trace_conformant)

    def test_B_conform_failure_not_overridden_by_accepted(self):
        r = _result(conform={"traces": 1, "statuses": {"violation": 1}})
        led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "explicit_failure")

    def test_C_deadlock_model_proof_needs_correspondence(self):
        contract = {"properties": [{"id": "d", "kind": "deadlock_free", "req": ["R1"]}]}
        r = _result(projected_events=0,
                    conform={"traces": 32, "statuses": {"conformant": 32}},
                    monitor={"status": "ok", "properties": [["d", "deferred"]]})
        led = evaluate_reexecution(r, contract, accepted=True,
                                   cir_props={"d": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "insufficient")
        self.assertEqual(led.properties[0].final_claim, "inconclusive")

    def test_D_all_unsupported_is_not_sufficient(self):
        r = _result(monitor={"status": "ok", "properties": [["p1", "unsupported"]]})
        led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "insufficient")
        self.assertFalse(led.properties[0].implementation_obligations_satisfied)

    def test_E_empty_projection_is_not_correspondence(self):
        r = _result(projected_events=0, raw_events=64,
                    conform={"traces": 32, "statuses": {"conformant": 32}})
        led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "insufficient")
        self.assertIn("empty projection", " ".join(led.reasons))

    def test_F_missing_hashes_block_reuse(self):
        r = _result(source_sha256=None)
        led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "insufficient")

    def test_happy_path_is_sufficient(self):
        led = evaluate_reexecution(_result(), CONTRACT, accepted=True,
                                   cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(led.verdict, "sufficient")
        self.assertTrue(led.observed_trace_conformant)
        self.assertTrue(led.properties[0].implementation_obligations_satisfied)


if __name__ == "__main__":
    unittest.main()
