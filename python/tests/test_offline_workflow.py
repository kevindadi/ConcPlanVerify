"""Offline workflow tests: scripted provider + real concir-backend."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient, sha256_file
from cir_workflow.offline_workflow import OfflineWorkflow
from cir_workflow.providers import ScriptedProvider
from tests._helpers import FIXTURES, fixture_text, real_binary


class OfflineWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = real_binary()
        if cls.binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def workflow(self, name, responses, **kwargs):
        out = self.root / name
        client = ConcirClient(self.binary, workdir=out / "calls", timeout=30.0)
        provider = ScriptedProvider(responses)
        return OfflineWorkflow(client, provider, out_dir=out, **kwargs), provider

    def contract(self, name):
        return json.loads((FIXTURES / name).read_text())

    def test_bad_json_then_valid_then_tool_repair(self):
        wf, provider = self.workflow(
            "retry",
            [{"text": "this is not json"}, {"text": fixture_text("single_cycle.json")}],
            max_generation_rounds=3,
        )
        result = wf.run("two locks in opposite order", self.contract("single_cycle_contract.json"))
        self.assertEqual(result.status, "repaired")
        self.assertEqual(len(provider.calls), 2)
        self.assertIsNone(provider.calls[0].feedback)
        self.assertIsNotNone(provider.calls[1].feedback)
        sources = {step.step.split("[")[0]: step.source for step in result.steps}
        self.assertEqual(sources.get("generate"), "scripted")
        self.assertEqual(sources.get("check"), "tool")
        self.assertEqual(sources.get("repair"), "tool")
        self.assertTrue(result.repaired_by_tool)
        self.assertEqual(result.replay["status"], "replayed")

    def test_already_satisfied_stops_without_repair(self):
        wf, provider = self.workflow(
            "ok", [{"text": fixture_text("already_correct.json")}]
        )
        result = wf.run("already correct", self.contract("already_correct_contract.json"))
        self.assertEqual(result.status, "already_satisfied")
        self.assertIsNone(result.repair)
        self.assertFalse(any(s.step == "repair" for s in result.steps))

    def test_bounds_unknown_stops(self):
        wf, _ = self.workflow("unknown", [{"text": fixture_text("finite_call_loop.json")}])
        result = wf.run("bounded analysis", self.contract("tiny_bounds_contract.json"))
        self.assertEqual(result.status, "unknown")
        self.assertIsNone(result.repair)

    def test_rwlock_unsupported_stops(self):
        wf, _ = self.workflow("unsupported", [{"text": fixture_text("rwlock_unsupported.json")}])
        result = wf.run("rwlock case", self.contract("rwlock_unsupported_contract.json"))
        self.assertIn(result.status, {"unsupported", "invalid"})
        self.assertIsNone(result.repair)

    def test_contract_is_unchanged(self):
        contract = self.contract("single_cycle_contract.json")
        wf, _ = self.workflow("contract", [{"text": fixture_text("single_cycle.json")}])
        result = wf.run("fix", contract)
        contract_path = Path(result.out_dir) / "contract.json"
        self.assertEqual(sha256_file(contract_path), result.contract_sha256)
        self.assertEqual(json.loads(contract_path.read_text()), contract)

    def test_retry_budget_is_finite(self):
        wf, provider = self.workflow(
            "finite", [{"text": "bad"}, {"text": "still bad"}, {"text": "bad again"}],
            max_generation_rounds=2,
        )
        result = wf.run("never valid", self.contract("single_cycle_contract.json"))
        self.assertEqual(result.status, "generation_failed")
        self.assertEqual(len(provider.calls), 2)

    def test_non_repair_outcome_is_reported_as_such(self):
        wf, _ = self.workflow(
            "budget", [{"text": fixture_text("single_cycle.json")}],
            repair_config={"candidate_budget": 0, "verification_budget": 1,
                           "max_depth": 4, "max_total_edits": 4},
        )
        result = wf.run("fix", self.contract("single_cycle_contract.json"))
        self.assertIn(result.status, {"no_acceptable_candidate", "budget_exhausted",
                                      "analysis_unknown"})
        self.assertNotEqual(result.status, "repaired")


if __name__ == "__main__":
    unittest.main()
