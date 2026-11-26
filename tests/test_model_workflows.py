"""Consolidated tests for model-level workflows: offline, iterative revision, and single-patch repair."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from concplan.core.concir_client import (
    ConcirClient,
    ConcirIdentity,
    ConcirResult,
    sha256_file,
)
from concplan.core.providers import ScriptedPatchProvider, ScriptedProvider
from concplan.model_verify.offline_workflow import OfflineWorkflow
from concplan.model_verify.patch_repair import ExternalPatchRepairWorkflow
from concplan.model_verify.revision_workflow import (
    WholeArtifactRevisionWorkflow,
    _merge_local,
)
from tests._helpers import FIXTURES, REPO, broken_program, fixture_text, real_binary


# ---------------------------------------------------------------------------
# 1. OfflineWorkflow tests
# ---------------------------------------------------------------------------

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
        return json.loads((FIXTURES / name).read_text(encoding="utf-8"))

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
        wf, _ = self.workflow("ok", [{"text": fixture_text("already_correct.json")}])
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
        self.assertEqual(json.loads(contract_path.read_text(encoding="utf-8")), contract)

    def test_retry_budget_is_finite(self):
        wf, provider = self.workflow(
            "finite",
            [{"text": "bad"}, {"text": "still bad"}, {"text": "bad again"}],
            max_generation_rounds=2,
        )
        result = wf.run("never valid", self.contract("single_cycle_contract.json"))
        self.assertEqual(result.status, "generation_failed")
        self.assertEqual(len(provider.calls), 2)

    def test_non_repair_outcome_is_reported_as_such(self):
        wf, _ = self.workflow(
            "budget",
            [{"text": fixture_text("single_cycle.json")}],
            repair_config={
                "candidate_budget": 0,
                "verification_budget": 1,
                "max_depth": 4,
                "max_total_edits": 4,
            },
        )
        result = wf.run("fix", self.contract("single_cycle_contract.json"))
        self.assertIn(
            result.status,
            {"no_acceptable_candidate", "budget_exhausted", "analysis_unknown"},
        )
        self.assertNotEqual(result.status, "repaired")


class _MissingArtifactClient(ConcirClient):
    def repair(self, *args, **kwargs):
        result = super().repair(*args, **kwargs)
        if result.artifact_path:
            Path(result.artifact_path).unlink()
        result.artifact_path = None
        return result


class _BadReplayClient(ConcirClient):
    def replay(self, artifact):
        return ConcirResult(
            command="replay",
            argv=[],
            exit_code=0,
            status="protocol_error",
            kind="protocol_error",
            payload=None,
            identity=ConcirIdentity(
                binary=str(self.binary),
                binary_sha256="0" * 64,
                argv=[],
                cwd="",
            ),
            error="invalid_replay_payload",
        )


class OfflineAcceptanceRegressionTests(unittest.TestCase):
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

    def contract(self):
        return json.loads((FIXTURES / "single_cycle_contract.json").read_text(encoding="utf-8"))

    def test_repaired_without_artifact_is_tool_error(self):
        out = self.root / "missing"
        client = _MissingArtifactClient(self.binary, workdir=out / "calls", timeout=30.0)
        provider = ScriptedProvider([{"text": fixture_text("single_cycle.json")}])
        result = OfflineWorkflow(client, provider, out_dir=out).run("fix", self.contract())
        self.assertEqual(result.status, "tool_error")
        self.assertFalse(result.repaired_by_tool)
        self.assertIsNone(result.replay)

    def test_bad_replay_result_is_tool_error(self):
        out = self.root / "badreplay"
        client = _BadReplayClient(self.binary, workdir=out / "calls", timeout=30.0)
        provider = ScriptedProvider([{"text": fixture_text("single_cycle.json")}])
        result = OfflineWorkflow(client, provider, out_dir=out).run("fix", self.contract())
        self.assertEqual(result.status, "tool_error")
        self.assertFalse(result.repaired_by_tool)

    def test_each_run_has_exclusive_output_dir(self):
        out = self.root / "exclusive"
        dirs = []
        for _ in range(2):
            client = ConcirClient(self.binary, workdir=out / f"calls-{len(dirs)}", timeout=30.0)
            provider = ScriptedProvider([{"text": fixture_text("single_cycle.json")}])
            result = OfflineWorkflow(client, provider, out_dir=out).run("fix", self.contract())
            dirs.append(Path(result.out_dir))
        self.assertNotEqual(dirs[0], dirs[1])
        for d in dirs:
            self.assertTrue((d / "contract.json").is_file())
            self.assertTrue((d / "frozen_initial.cir.json").is_file())


# ---------------------------------------------------------------------------
# 2. RevisionWorkflow tests (using active benchmarks/families/lock-order/abba_2lock)
# ---------------------------------------------------------------------------

ABBA_DIR = REPO / "benchmarks/families/lock-order/abba_2lock"
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
            (REPO / "benchmarks/families/condvar/lost_wakeup_notify_before_wait/fixed.cir.json")
            .read_text(encoding="utf-8")
        )
        contract = json.loads(
            (REPO / "benchmarks/families/condvar/lost_wakeup_notify_before_wait/contract.json")
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


# ---------------------------------------------------------------------------
# 3. ExternalPatchRepairWorkflow tests
# ---------------------------------------------------------------------------

def _patch_candidate(context, **over):
    t2 = next(f for f in context["functions"] if f["function"] == "t2")
    candidate = {
        "schema_version": "concir-external-patch-candidate-v1",
        "context_fingerprint": context["context_fingerprint"],
        "patch": {
            "module": "main",
            "function": "t2",
            "original_hash": t2["original_hash"],
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
        bad = _patch_candidate(context, original_hash="deadbeefdeadbeef")
        good = _patch_candidate(context)
        provider = ScriptedPatchProvider([{"text": bad}, {"text": good}])
        client = ConcirClient(self.binary, workdir=self.root / "run", timeout=30.0)
        result = ExternalPatchRepairWorkflow(
            client, provider, out_dir=self.root / "out", max_rounds=3
        ).run(self.model, self.contract)
        self.assertEqual(result.status, "accepted", result.error)
        self.assertTrue(result.accepted)
        self.assertEqual(result.candidate_source, "scripted")
        self.assertEqual(result.validator, "tool")
        self.assertEqual(result.repair_mode, "external_single_patch")
        self.assertEqual(len(provider.calls), 2)
        self.assertIsNone(provider.calls[0].feedback)
        self.assertIsNotNone(provider.calls[1].feedback)
        self.assertEqual(result.rounds[0].reject_reason["code"], "apply_error")
        self.assertIn("apply_error", provider.calls[1].feedback)
        self.assertEqual(result.rounds[0].evaluate_status, "rejected")
        self.assertEqual(result.rounds[1].evaluate_status, "accepted")
        self.assertEqual(result.replay["status"], "replayed")

    def test_all_rounds_rejected_and_bad_shape_rejected(self):
        context = self.context()
        bad = _patch_candidate(context, original_hash="deadbeefdeadbeef")
        provider = ScriptedPatchProvider([{"text": bad}, {"text": bad}, {"text": bad}])
        client = ConcirClient(self.binary, workdir=self.root / "run2", timeout=30.0)
        result = ExternalPatchRepairWorkflow(
            client, provider, out_dir=self.root / "out2", max_rounds=3
        ).run(self.model, self.contract)
        self.assertEqual(result.status, "rejected")
        self.assertFalse(result.accepted)
        self.assertEqual(len(result.rounds), 3)

        two = json.dumps({
            "schema_version": "concir-external-patch-candidate-v1",
            "context_fingerprint": context["context_fingerprint"],
            "patch": {
                "module": "main",
                "function": "t2",
                "original_hash": next(
                    f for f in context["functions"] if f["function"] == "t2"
                )["original_hash"],
                "changes": [
                    {"kind": "swap_statements", "a": "s1", "b": "s2"},
                    {"kind": "delete_statement", "sid": "s3"},
                ],
            },
        })
        provider2 = ScriptedPatchProvider([{"text": two}])
        result2 = ExternalPatchRepairWorkflow(
            client, provider2, out_dir=self.root / "out3", max_rounds=1
        ).run(self.model, self.contract)
        self.assertEqual(result2.status, "rejected")
        self.assertEqual(result2.rounds[0].reject_reason["code"], "patch_shape")

    def test_accepted_artifact_replays_and_tamper_fails(self):
        context = self.context()
        client = ConcirClient(self.binary, workdir=self.root / "run4", timeout=30.0)
        good = self.root / "good.json"
        good.write_text(_patch_candidate(context), encoding="utf-8")
        context_path = self.root / "context.json"
        context_path.write_text(json.dumps(context), encoding="utf-8")
        evaluated = client.evaluate_patch(context_path, good)
        self.assertEqual(evaluated.status, "accepted")
        replay = client.replay(evaluated.artifact_path)
        self.assertEqual((replay.kind, replay.status), ("semantic", "replayed"))
        self.assertTrue(replay.payload["accepted_ok"])

        artifact = json.loads(Path(evaluated.artifact_path).read_text(encoding="utf-8"))
        artifact["accepted"] = False
        tampered = self.root / "tampered_ext.json"
        tampered.write_text(json.dumps(artifact), encoding="utf-8")
        bad = client.replay(tampered)
        self.assertEqual(bad.kind, "semantic")
        self.assertEqual(bad.status, "replay_failed")


if __name__ == "__main__":
    unittest.main()
