"""Statistics/attribution fixes: failure stage, unknown usage, evidence verdicts."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

from cir_workflow.evidence import cir_observability

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


if __name__ == "__main__":
    unittest.main()
