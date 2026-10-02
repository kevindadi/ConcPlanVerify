"""v28 representation_adaptation feedback: safe repair, never acceptance."""

from __future__ import annotations

import unittest
from types import SimpleNamespace

from cir_workflow.candidate_eval import _adaptation_feedback


def _prop(pid, result):
    return SimpleNamespace(property_id=pid, independent_requirement_result=result)


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
        self.assertIn("keep the exact CIR synchronization", fb)

    def test_no_adaptation_without_resolved_identity(self):
        led = _Ledger([_prop("p1", "unsupported")])
        self.assertIsNone(_adaptation_feedback(led, {"binding": {"mapping": {}}}))

    def test_no_adaptation_when_all_properties_decided(self):
        led = _Ledger([_prop("p1", "PASS_bounded"), _prop("p2", "FAIL")])
        self.assertIsNone(_adaptation_feedback(led, {"binding": {"mapping": {"m": "main::m"}}}))


if __name__ == "__main__":
    unittest.main()
