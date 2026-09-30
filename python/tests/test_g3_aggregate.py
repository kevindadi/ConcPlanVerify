"""run_g3_v2 must read the shared code-stage ledger."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from cir_workflow.generation import load_gen_tasks, run_g3_v2

REPO = Path(__file__).resolve().parents[2]


def _cir_ok(task):
    def run(*args, **kwargs):
        return {"accepted": True, "status": "accepted",
                "cir_path": str(task.reference_cir_path),
                "rounds": [{"round": 1, "decision": "accepted"}],
                "model": {"outcome": "PASS", "complete": True},
                "coverage": {"rf": 1.0}}
    return run


class _Client:
    def complete(self, system, user):
        return SimpleNamespace(text="```rust\nfn main() {}\n```", usage=None,
                               wall_ms=1, response_model="deepseek-flash")


class G3AggregateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        self.task = tasks["lock-order/abba_2lock"]

    def tearDown(self):
        self.tmp.cleanup()

    def _run(self, code):
        with mock.patch("cir_workflow.generation.run_g3", side_effect=_cir_ok(self.task)), \
                mock.patch("cir_workflow.generation.run_llmcode_from_cir", return_value=code):
            return run_g3_v2(_Client(), Path("/bin/true"), self.task, Path(self.tmp.name))

    def test_accepted_candidate_is_not_reported_as_hang(self):
        code = {"accepted": True, "status": "accepted",
                "rounds": [{"round": 1, "decision": "accepted"}],
                "ledger": {"run": {"state": "completed", "started": 32, "completed": 32,
                                   "hang": False},
                           "current_evaluation": "satisfied_bounded",
                           "properties": [{
                               "independent_requirement_result": "PASS_bounded",
                               "obligation_state": "satisfied"}]}}
        record = self._run(code)
        self.assertTrue(record["accepted"])
        self.assertFalse(record["oracle"]["hang"])
        self.assertEqual(record["oracle"]["runs_completed"], 32)
        self.assertEqual(record["coverage"]["basis"], "bounded_monitor_statuses")
        self.assertNotEqual(record["coverage"].get("basis"), "model_pass")

    def test_tool_failure_and_capability_gap_are_not_deadlock(self):
        tool = {"accepted": False, "status": "binding_tool_error", "action": "tool_failure",
                "rounds": [{"round": 1, "decision": "binding_tool_error"}],
                "ledger": {"run": {"state": "not_run", "started": 0, "completed": 0, "hang": False},
                           "current_evaluation": "tool_error", "properties": []}}
        gap = {"accepted": False, "status": "capability_gap", "action": "capability_gap",
               "rounds": [{"round": 1, "decision": "capability_gap"}],
               "ledger": {"run": {"state": "completed", "started": 32, "completed": 32, "hang": False},
                          "current_evaluation": "inconclusive",
                          "properties": [{"independent_requirement_result": "unsupported",
                                          "obligation_state": "unresolved"}]}}
        for code in (tool, gap):
            record = self._run(code)
            self.assertFalse(record["oracle"]["hang"])
            self.assertIsNone(record["coverage"])
            self.assertTrue(record["coverage_reason"])
            self.assertNotIn("deadlock", record["oracle"]["failure_stage"])


if __name__ == "__main__":
    unittest.main()
