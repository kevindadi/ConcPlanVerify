"""Explore feedback must keep the backend's resolved names (no 0::2, no None)."""

from __future__ import annotations

import unittest
from types import SimpleNamespace

from cir_workflow.generation import _explore_signature
from cir_workflow.prompts import build_explore_feedback


def _result(payload):
    return SimpleNamespace(payload=payload, outcome="FAIL", complete=True,
                           status="valid", kind="semantic", error=None)


class FeedbackNamingTests(unittest.TestCase):
    def test_counterexample_names_preserved(self):
        payload = {"diagnostics": [{
            "property": "no-deadlock", "outcome": "FAIL",
            "counterexample_names": ["main::waiter::s2", "main::notifier::s3"],
            "cir_statements": [{"module": "main", "function": "waiter", "sid": "s2"},
                               {"module": "main", "function": "notifier", "sid": None}],
            "blocked": [{"thread": 1, "kind": "condvar", "resource": 1,
                         "resource_name": "main::cv", "detail": "waiting for a lock"}],
            "proven_facts": ["explored 32 reachable states"],
            "repair_hints": ["ensure a notifier runs"],
        }]}
        fb = build_explore_feedback(_result(payload))
        d = fb["diagnostics"][0]
        self.assertEqual(d["counterexample_names"],
                         ["main::waiter::s2", "main::notifier::s3"])
        # null sid becomes an explicit unknown, never the string "None"
        self.assertEqual(d["cir_statements"][1]["sid"], "<unknown>")
        self.assertEqual(d["blocked"][0]["resource"], "main::cv")
        self.assertIn("explored 32", d["proven_facts"][0])

    def test_no_numeric_function_names(self):
        payload = {"diagnostics": [{
            "property": "no-deadlock", "outcome": "FAIL",
            "counterexample": [{"origin": {"module": 0, "function": 2, "sid": 0}}],
        }]}
        fb = build_explore_feedback(_result(payload))
        rendered = str(fb)
        self.assertNotIn("0::2", rendered)
        self.assertNotIn(".None", rendered)


class SignatureTests(unittest.TestCase):
    def test_signature_is_stable_and_sensitive(self):
        a = {"diagnostics": [{"property": "no-deadlock",
                              "counterexample_names": ["main::waiter::s2"],
                              "blocked": [{"resource_name": "main::cv"}]}]}
        b = {"diagnostics": [{"property": "no-deadlock",
                              "counterexample_names": ["main::waiter::s3"],
                              "blocked": [{"resource_name": "main::cv"}]}]}
        self.assertEqual(_explore_signature(a), _explore_signature(dict(a)))
        self.assertNotEqual(_explore_signature(a), _explore_signature(b))


if __name__ == "__main__":
    unittest.main()
