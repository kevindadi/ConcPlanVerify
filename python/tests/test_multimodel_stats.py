"""Statistics/attribution fixes: failure stage, unknown usage, evidence verdicts."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

from cir_workflow.evidence import classify_property, cir_observability

REPO = Path(__file__).resolve().parents[2]


def _load_script(name: str):
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


runner = _load_script("run_multimodel_pilot")


class FailureStageTests(unittest.TestCase):
    def test_cir_failure_is_not_code_failure(self):
        rec = {"accepted": False, "cir_stage": {"accepted": False, "status": "generation_failed"},
               "code_stage": None}
        self.assertEqual(runner._failure_stage(rec), ("cir", "generation_failed"))

    def test_code_failure_after_cir_pass(self):
        rec = {"accepted": False, "cir_stage": {"accepted": True},
               "code_stage": {"accepted": False,
                              "rounds": [{"decision": "instrumented_build_failed"}]}}
        self.assertEqual(runner._failure_stage(rec), ("code", "instrumented_build_failed"))

    def test_accepted_is_classified_accepted(self):
        self.assertEqual(runner._failure_stage({"accepted": True}), ("accepted", None))


class SumCallsTests(unittest.TestCase):
    def test_missing_usage_counted_unknown_not_zero(self):
        events = [
            {"cell_id": "c", "kind": "model-call", "latency_ms": 10,
             "usage": {"input_tokens": 5, "output_tokens": 2, "total_tokens": 7}},
            {"cell_id": "c", "kind": "model-call", "latency_ms": 20, "usage": {}},
            {"cell_id": "c", "kind": "tool-step", "latency_ms": 99, "usage": {}},
        ]
        n, inp, out, tot, latency, unknown = runner._sum_calls(events, "c")
        self.assertEqual((n, inp, out, tot, unknown), (2, 5, 2, 7, 1))
        self.assertEqual(latency, 30)  # tool-step excluded


class EvidenceVerdictTests(unittest.TestCase):
    def test_cir_proved_but_no_events_is_inconclusive(self):
        prop = {"id": "both-increments", "kind": "always_reachable", "req": ["R2"],
                "status": "unsupported"}
        v = classify_property(prop, cir_outcome="PASS", cir_complete=True,
                              impl="insufficient", behavior_ok=True)
        self.assertEqual(v.verdict, "inconclusive")

    def test_cir_proved_and_correspondence_supported_is_supported(self):
        prop = {"id": "no-deadlock", "kind": "deadlock_free", "req": ["R4"],
                "status": "deferred"}
        v = classify_property(prop, cir_outcome="PASS", cir_complete=True,
                              impl="supported", behavior_ok=True)
        self.assertEqual(v.verdict, "supported")

    def test_monitor_fail_is_violated(self):
        prop = {"id": "x", "kind": "safety", "req": [], "status": "FAIL"}
        v = classify_property(prop, cir_outcome="PASS", cir_complete=True,
                              impl="supported", behavior_ok=True)
        self.assertEqual(v.verdict, "violated")

    def test_empty_correspondence_for_observable_cir(self):
        # A CIR with a mutex op has observable operations.
        import json
        import tempfile
        cir = {"modules": [{"functions": [{"body": [
            {"kind": "mutex_lock"}, {"kind": "mutex_unlock"}]}]}]}
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "c.cir.json"
            p.write_text(json.dumps(cir))
            ops, spawns = cir_observability(p)
        self.assertEqual((ops, spawns), (2, 0))


if __name__ == "__main__":
    unittest.main()
