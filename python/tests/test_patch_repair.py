"""Offline single-patch repair loop: scripted provider + real concir-backend."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient
from cir_workflow.patch_repair import ExternalPatchRepairWorkflow
from cir_workflow.providers import ScriptedPatchProvider
from tests._helpers import FIXTURES, real_binary


def _candidate(context, **over):
    t2 = next(f for f in context["functions"] if f["function"] == "t2")
    candidate = {
        "schema_version": "concir-external-patch-candidate-v1",
        "context_fingerprint": context["context_fingerprint"],
        "patch": {
            "module": "main", "function": "t2", "original_hash": t2["original_hash"],
            "changes": [{"kind": "swap_statements", "a": "s1", "b": "s2"}],
        },
    }
    candidate["patch"].update(over)
    return json.dumps(candidate)


class PatchRepairOfflineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = real_binary()
        if cls.binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)
        cls.model = FIXTURES / "abba.json"
        cls.contract = FIXTURES / "abba_contract.json"

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def context(self):
        client = ConcirClient(self.binary, workdir=self.root / "ctx", timeout=30.0)
        result = client.repair_context(self.model, self.contract)
        self.assertEqual(result.status, "context")
        return result.payload

    def test_rejection_feedback_then_accept(self):
        context = self.context()
        bad = _candidate(context, original_hash="deadbeefdeadbeef")
        good = _candidate(context)
        provider = ScriptedPatchProvider([{"text": bad}, {"text": good}])
        client = ConcirClient(self.binary, workdir=self.root / "run", timeout=30.0)
        result = ExternalPatchRepairWorkflow(
            client, provider, out_dir=self.root / "out", max_rounds=3).run(
            self.model, self.contract)
        self.assertEqual(result.status, "accepted", result.error)
        self.assertTrue(result.accepted)
        self.assertEqual(result.candidate_source, "scripted")
        self.assertEqual(result.validator, "tool")
        self.assertEqual(result.repair_mode, "external_single_patch")
        # the second request actually carried the real rejection reason
        self.assertEqual(len(provider.calls), 2)
        self.assertIsNone(provider.calls[0].feedback)
        self.assertIsNotNone(provider.calls[1].feedback)
        self.assertEqual(result.rounds[0].reject_reason["code"], "apply_error")
        self.assertIn("apply_error", provider.calls[1].feedback)
        self.assertEqual(result.rounds[0].evaluate_status, "rejected")
        self.assertEqual(result.rounds[1].evaluate_status, "accepted")
        self.assertEqual(result.replay["status"], "replayed")

    def test_all_rounds_rejected(self):
        context = self.context()
        bad = _candidate(context, original_hash="deadbeefdeadbeef")
        provider = ScriptedPatchProvider([{"text": bad}, {"text": bad}, {"text": bad}])
        client = ConcirClient(self.binary, workdir=self.root / "run2", timeout=30.0)
        result = ExternalPatchRepairWorkflow(
            client, provider, out_dir=self.root / "out2", max_rounds=3).run(
            self.model, self.contract)
        self.assertEqual(result.status, "rejected")
        self.assertFalse(result.accepted)
        self.assertEqual(len(result.rounds), 3)

    def test_bad_shape_rejected_and_not_success(self):
        context = self.context()
        two = json.dumps({
            "schema_version": "concir-external-patch-candidate-v1",
            "context_fingerprint": context["context_fingerprint"],
            "patch": {
                "module": "main", "function": "t2",
                "original_hash": next(f for f in context["functions"] if f["function"] == "t2")["original_hash"],
                "changes": [
                    {"kind": "swap_statements", "a": "s1", "b": "s2"},
                    {"kind": "delete_statement", "sid": "s3"},
                ],
            },
        })
        provider = ScriptedPatchProvider([{"text": two}])
        client = ConcirClient(self.binary, workdir=self.root / "run3", timeout=30.0)
        result = ExternalPatchRepairWorkflow(
            client, provider, out_dir=self.root / "out3", max_rounds=1).run(
            self.model, self.contract)
        self.assertEqual(result.status, "rejected")
        self.assertEqual(result.rounds[0].reject_reason["code"], "patch_shape")

    def test_accepted_artifact_replays_and_tamper_fails(self):
        context = self.context()
        client = ConcirClient(self.binary, workdir=self.root / "run4", timeout=30.0)
        good = self.root / "good.json"
        good.write_text(_candidate(context))
        context_path = self.root / "context.json"
        context_path.write_text(json.dumps(context))
        evaluated = client.evaluate_patch(context_path, good)
        self.assertEqual(evaluated.status, "accepted")
        replay = client.replay(evaluated.artifact_path)
        self.assertEqual((replay.kind, replay.status), ("semantic", "replayed"))
        self.assertTrue(replay.payload["accepted_ok"])
        artifact = json.loads(Path(evaluated.artifact_path).read_text())
        artifact["accepted"] = False
        tampered = self.root / "tampered_ext.json"
        tampered.write_text(json.dumps(artifact))
        bad = client.replay(tampered)
        self.assertEqual(bad.kind, "semantic")
        self.assertEqual(bad.status, "replay_failed")


if __name__ == "__main__":
    unittest.main()
