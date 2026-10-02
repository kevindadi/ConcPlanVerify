"""v28 representation_adaptation feedback: safe repair, never acceptance."""

from __future__ import annotations

import unittest
from types import SimpleNamespace

from cir_workflow.candidate_eval import (
    _adaptation_feedback, _channel_protocol_feedback, decide_followup,
)


def _prop(pid, result, verified=True):
    return SimpleNamespace(property_id=pid, independent_requirement_result=result,
                           identity_relevant_verified=verified)


class _Ledger:
    def __init__(self, props):
        self.properties = props


class AdaptationTests(unittest.TestCase):
    def test_adaptation_when_identity_resolved(self):
        led = _Ledger([_prop("main::c == 2", "unsupported")])
        fb = _adaptation_feedback(led, {"binding": {"mapping": {"m": "main::m"}}})
        self.assertIsNotNone(fb)
        self.assertIn("Representation adaptation", fb)
        self.assertIn("main::c == 2", fb)
        self.assertIn("keep every synchronization operation", fb)

    def test_no_adaptation_without_resolved_identity(self):
        led = _Ledger([_prop("p1", "unsupported", verified=False)])
        self.assertIsNone(_adaptation_feedback(led, {"binding": {"mapping": {}}}))

    def test_no_adaptation_when_a_failed_property_identity_is_unresolved(self):
        led = _Ledger([_prop("p1", "unsupported", verified=False)])
        self.assertIsNone(_adaptation_feedback(led, {"binding": {"mapping": {"m": "main::m"}}}))

    def test_adaptation_forbids_atomic_to_mutex(self):
        led = _Ledger([_prop("main::c == 2", "unsupported")])
        fb = _adaptation_feedback(led, {"binding": {"mapping": {"c": "main::c"}}})
        self.assertIn("do not turn an Atomic into a Mutex", fb)

    def test_no_adaptation_when_all_properties_decided(self):
        led = _Ledger([_prop("p1", "PASS_bounded"), _prop("p2", "FAIL")])
        self.assertIsNone(_adaptation_feedback(led, {"binding": {"mapping": {"m": "main::m"}}}))

    def test_attribute_conflict_feedback_carries_constructor_mismatch(self):
        led = SimpleNamespace(current_evaluation="attribute_conflict", reasons=[],
                              delivery_status="reject")
        result = {"binding": {"attributes": [
            {"status": "mismatch", "resource_id": "main::ch", "expected_capacity": 0,
             "observed_capacity": 1, "construction_site": "42"}]}}
        out = decide_followup(led, result)
        self.assertEqual(out["action"], "repair")
        self.assertEqual(out["status"], "attribute_conflict")
        self.assertIn("expected_capacity", out["feedback"])
        self.assertIn("main::ch", out["feedback"])

    def test_channel_protocol_feedback_reports_site_and_channels(self):
        import json
        import tempfile
        from pathlib import Path
        d = Path(tempfile.mkdtemp())
        cir = d / "cir.json"
        cir.write_text(json.dumps({"modules": [
            {"name": "main", "resources": [{"name": "ch", "type": "Channel"}]}]}))
        result = {
            "cir_path": str(cir),
            "binding": {"ambiguous": [
                {"rust": "tx", "reason": "channel endpoint has no channel token",
                 "site": "42"}]},
            "resources": {"channel_semantics": {"endpoints": [
                {"name": "tx", "function": "main", "site": "42"}]}},
        }
        fb = _channel_protocol_feedback(None, result)
        self.assertIn("main::ch", fb)
        self.assertIn("'tx'", fb)
        self.assertIn("'main'", fb)
        self.assertIn("creation order", fb)

    def test_channel_protocol_feedback_ignores_non_channel_misses(self):
        result = {"binding": {"ambiguous": [
            {"rust": "m", "reason": "no structural evidence"}]}}
        self.assertIsNone(_channel_protocol_feedback(None, result))


if __name__ == "__main__":
    unittest.main()
