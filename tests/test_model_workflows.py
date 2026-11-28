"""Consolidated tests for model-level iterative revision workflow."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from concplan.core.concir_client import ConcirClient
from concplan.core.providers import ScriptedProvider
from concplan.model_verify.revision_workflow import (
    WholeArtifactRevisionWorkflow,
    _merge_local,
)
from tests._helpers import REPO, broken_program, real_binary


# ---------------------------------------------------------------------------
# RevisionWorkflow tests (using tests/fixtures/abba_2lock)
# ---------------------------------------------------------------------------

ABBA_DIR = REPO / "tests/fixtures/abba_2lock"
ABBA_BUGGY = ABBA_DIR / "buggy.cir.json"
ABBA_FIXED = ABBA_DIR / "fixed.cir.json"
ABBA_CONTRACT = ABBA_DIR / "contract.json"


class RevisionWorkflowOfflineTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = real_binary()
        if cls.binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)
        cls.contract = json.loads(ABBA_CONTRACT.read_text(encoding="utf-8"))
        cls.fixed_text = ABBA_FIXED.read_text(encoding="utf-8")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def _run(self, responses, *, max_rounds=3, diagnostics=True, initial=ABBA_BUGGY):
        provider = ScriptedProvider(responses)
        client = ConcirClient(self.binary, workdir=self.root / "calls", timeout=30.0)
        workflow = WholeArtifactRevisionWorkflow(
            client,
            provider,
            out_dir=self.root / "out",
            max_rounds=max_rounds,
            diagnostics=diagnostics,
        )
        return provider, workflow.run(
            "two tasks take two locks in opposite order",
            self.contract,
            task_id="abba_2lock",
            initial_program=initial,
        )

    def test_fail_then_revision_pass(self):
        provider, result = self._run([{"text": self.fixed_text}])
        self.assertEqual(result.status, "accepted", result.error)
        self.assertEqual(result.repair_mode, "llm_revision")
        self.assertEqual(result.accepted_version, 2)
        self.assertEqual(result.consumption.first_correct_round, 2)
        self.assertEqual(result.versions[0].explore_outcome, "FAIL")
        self.assertEqual(result.versions[1].explore_outcome, "PASS")
        self.assertTrue(result.versions[1].accepted)
        self.assertEqual(len(provider.calls), 1)

    def test_static_error_feedback_then_fix(self):
        broken = json.dumps(broken_program())
        provider, result = self._run([{"text": broken}, {"text": self.fixed_text}])
        self.assertEqual(result.status, "accepted", result.error)
        self.assertEqual(result.accepted_version, 3)
        self.assertEqual(result.versions[1].decision, "check_invalid")
        self.assertEqual(len(provider.calls), 2)
        self.assertIn("diagnostics", provider.calls[1].feedback or "")

    def test_three_round_records_fail_static_error_pass(self):
        buggy = ABBA_BUGGY.read_text(encoding="utf-8")
        broken = json.dumps(broken_program())
        provider, result = self._run(
            [{"text": buggy}, {"text": broken}, {"text": self.fixed_text}],
            max_rounds=4,
            initial=None,
        )
        self.assertEqual(result.status, "accepted", result.error)
        self.assertEqual(result.accepted_version, 3)
        self.assertEqual(len(result.versions), 3)
        self.assertEqual(
            [v.decision for v in result.versions],
            ["explore_fail", "check_invalid", "accepted"],
        )
        for v in result.versions:
            self.assertIsNotNone(v.request_sha256)
            self.assertIsNotNone(v.tool_output_sha256)
        self.assertIsNotNone(result.versions[0].feedback_sha256)
        self.assertIsNotNone(result.versions[1].feedback_sha256)

    def test_schema_error_is_feedback_not_tool_error(self):
        fixed = json.loads(
            (REPO / "tests/fixtures/lost_wakeup/fixed.cir.json")
            .read_text(encoding="utf-8")
        )
        contract = json.loads(
            (REPO / "tests/fixtures/lost_wakeup/contract.json")
            .read_text(encoding="utf-8")
        )
        corrupt = copy.deepcopy(fixed)
        for res in corrupt["modules"][0]["resources"]:
            if res.get("type") == "Var":
                res.pop("base", None)
                res.pop("init", None)
        provider = ScriptedProvider([{"text": json.dumps(corrupt)}, {"text": json.dumps(fixed)}])
        client = ConcirClient(self.binary, workdir=self.root / "schema", timeout=30.0)
        workflow = WholeArtifactRevisionWorkflow(
            client, provider, out_dir=self.root / "schema-out", max_rounds=3
        )
        result = workflow.run("waiter/notifier with a ready flag", contract, task_id="lost_wakeup")
        self.assertEqual(result.status, "accepted", result.error)
        self.assertEqual(result.accepted_version, 2)
        self.assertEqual(result.versions[0].decision, "check_invalid")
        feedback = provider.calls[1].feedback or ""
        self.assertIn("expected_shapes", feedback)
        self.assertNotIn("revision-1", feedback)

    def test_local_merge_keeps_unmentioned_functions_and_rejects_unreachable(self):
        base = json.loads(ABBA_BUGGY.read_text(encoding="utf-8"))
        original_t1 = next(f for f in base["modules"][0]["functions"] if f["name"] == "t1")
        new_body = [{"sid": "s1", "kind": "return"}]
        merged, errors = _merge_local(base, {"functions": {"main::t1": new_body}})
        self.assertEqual(errors, [])
        t1 = next(f for f in merged["modules"][0]["functions"] if f["name"] == "t1")
        t2 = next(f for f in merged["modules"][0]["functions"] if f["name"] == "t2")
        self.assertEqual(t1["body"], new_body)
        self.assertEqual(
            t2["body"],
            next(f for f in base["modules"][0]["functions"] if f["name"] == "t2")["body"],
        )
        self.assertEqual(original_t1["body"][0]["kind"], "mutex_lock")

        _m2, errs2 = _merge_local(
            base, {"functions": {"main::ghost": [{"sid": "s1", "kind": "return"}]}}
        )
        self.assertTrue(any("not reachable" in e for e in errs2))

    def test_local_mode_undeclared_resource_is_feedback_not_crash(self):
        bad_body = [
            {"sid": "s1", "kind": "mutex_lock", "resource": "main::a"},
            {"sid": "s2", "kind": "write_shared", "resource": "main::ghost", "expr": "1"},
            {"sid": "s3", "kind": "mutex_unlock", "resource": "main::a"},
            {"sid": "s4", "kind": "return"},
        ]
        provider = ScriptedProvider([{"text": json.dumps({"functions": {"main::t2": bad_body}})}])
        client = ConcirClient(self.binary, workdir=self.root / "local", timeout=30.0)
        workflow = WholeArtifactRevisionWorkflow(
            client,
            provider,
            out_dir=self.root / "local-out",
            max_rounds=2,
            reply_format="local",
        )
        result = workflow.run(
            "two tasks take two locks",
            self.contract,
            task_id="abba_2lock",
            initial_program=ABBA_BUGGY,
        )
        self.assertEqual(result.status, "exhausted")
        self.assertEqual(result.versions[0].decision, "explore_fail")
        self.assertEqual(result.versions[1].decision, "check_invalid")

    def test_k_rounds_exhausted_and_nodiag(self):
        broken = json.dumps(broken_program())
        _prov, result = self._run([{"text": broken}, {"text": broken}], max_rounds=3)
        self.assertEqual(result.status, "exhausted")
        self.assertIsNone(result.accepted_version)
        self.assertEqual(len(result.versions), 3)

        provider, result2 = self._run([{"text": self.fixed_text}], diagnostics=False)
        self.assertEqual(result2.status, "accepted")
        feedback = provider.calls[0].feedback or ""
        self.assertIn("FAIL", feedback)
        self.assertNotIn("diagnostics", feedback)


if __name__ == "__main__":
    unittest.main()
