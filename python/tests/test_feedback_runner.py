"""Fake-client drills for the two-arm feedback runner. No model service."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.feedback_runner import run_arm, validate_config
from cir_workflow.pilot_cases import load_case
from cir_workflow.transport import build_registry, require_experiment_model


def _ok_eval(*args, **kwargs):
    return {"followup": {"action": "continue", "category": None},
            "ledger": {"delivery_status": "reject", "current_evaluation": "explicit_failure",
                       "model": {"verified": True},
                       "trace": {"state": "observed_violation", "violations": [{
                           "status": "violation", "got": "mutex_unlock:main::a",
                           "resource": "main::a", "event_index": 1,
                           "expected": ["mutex_lock:main::b"], "detail": "checker"}]},
                       "run": {"state": "completed", "started": 1, "completed": 1, "hang": False}},
            "functional": {"status": "not_run"},
            "source_sha256": "src", "cir_sha256": "cir", "tool_time_s": None}


def _tool_eval(*args, **kwargs):
    return {"followup": {"action": "stop", "category": "tool_failure"},
            "ledger": {"delivery_status": "withhold_tool"}, "tool_time_s": None}


class _Fake:
    def __init__(self, texts, model_id="deepseek-flash"):
        self.texts = list(texts)
        self.calls = 0
        self.model_id = model_id
        self.prompts = []

    def complete(self, system, user):
        self.calls += 1
        self.prompts.append(user)
        text = self.texts.pop(0) if self.texts else self.texts
        return SimpleNamespace(text=text, usage=None, wall_ms=None,
                               response_model=self.model_id)


def _fence(source: str) -> str:
    return "```rust\n" + source + "\n```"


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.case = load_case("lock_order")
        self.spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        self.tmp = tempfile.TemporaryDirectory()
        self.out = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_repair_reaches_the_control(self):
        client = _Fake([_fence(self.case["control"])])
        result = run_arm(self.case, self.spec, "counterexample", client, self.out,
                         evaluate=_ok_eval)
        self.assertEqual(client.calls, 1)
        self.assertEqual(result["first_requirement_round"], 1)
        self.assertEqual(result["first_design_round"], 1)
        self.assertEqual(result["stop"], "requirement_pass")
        self.assertTrue(result["in_defect_denominator"])
        self.assertLess(result["actual_model_requests_this_run"], result["suggested_repairs"])
        self.assertIsNone(result["usage"][0]["input_tokens"])

    def test_budget_exhausts_on_repeated_defects(self):
        client = _Fake([_fence(self.case["defect"]), _fence(self.case["defect"])])
        result = run_arm(self.case, self.spec, "verdict_only", client, self.out,
                         evaluate=_ok_eval)
        self.assertEqual(client.calls, 2)
        self.assertEqual(result["actual_model_requests_this_run"], 2)
        self.assertIsNone(result["first_requirement_round"])
        self.assertEqual(result["stop"], "budget_exhausted")
        joined = "\n".join(client.prompts)
        self.assertNotIn(self.case["control"], joined)

    def test_correct_initial_makes_no_request(self):
        client = _Fake([_fence(self.case["defect"])])
        result = run_arm(self.case, self.spec, "verdict_only", client, self.out,
                         source=self.case["control"], evaluate=_ok_eval)
        self.assertEqual(client.calls, 0)
        self.assertEqual(result["first_requirement_round"], 0)
        self.assertFalse(result["in_defect_denominator"])

    def test_tool_failure_makes_no_request(self):
        client = _Fake([_fence(self.case["control"])])
        result = run_arm(self.case, self.spec, "verdict_only", client, self.out,
                         evaluate=_tool_eval)
        self.assertEqual(client.calls, 0)
        self.assertEqual(result["stop"], "tool_failure")
        self.assertFalse(result["in_defect_denominator"])

    def test_kimi_k3_return_is_rejected(self):
        spec = require_experiment_model(build_registry(), "Kimi 2.7 Code")
        client = _Fake([_fence(self.case["control"])], model_id="kimi-k3")
        result = run_arm(self.case, spec, "counterexample", client, self.out,
                         evaluate=_ok_eval)
        self.assertEqual(client.calls, 1)
        self.assertEqual(result["stop"], "identity_mismatch")
        self.assertFalse(result["identity"][0]["identity_confirmed"])
        self.assertEqual(result["identity"][0]["returned_model"], "kimi-k3")
        self.assertIsNone(result["first_requirement_round"])

    def test_resume_does_not_repeat_a_request(self):
        first = _Fake([_fence(self.case["defect"])])
        run_arm(self.case, self.spec, "verdict_only", first, self.out,
                evaluate=_ok_eval, halt_after_new_requests=1)
        self.assertEqual(first.calls, 1)
        second = _Fake([_fence(self.case["control"])])
        result = run_arm(self.case, self.spec, "verdict_only", second, self.out,
                         evaluate=_ok_eval)
        self.assertEqual(second.calls, 1)
        self.assertEqual(result["requests_recorded"], 2)
        self.assertEqual(result["first_requirement_round"], 2)

    def test_dry_run_rejects_a_narrow_whitelist_mismatch_and_a_bad_hash(self):
        import sys
        sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
        from scripts.feedback_pilot import freeze_inputs
        config, rows, _ignored = freeze_inputs()
        plan = validate_config(config, rows)
        self.assertTrue(plan["ok"], plan["errors"])
        self.assertEqual(plan["cells"], 48)
        self.assertEqual(plan["requests_upper_bound"], 96)
        self.assertEqual(plan["llm_calls"], 0)
        config = dict(config)
        config["models"] = ["DeepSeek Flash"]
        narrow = validate_config(config, rows)
        self.assertTrue(narrow["ok"], narrow["errors"])
        self.assertEqual(narrow["model_ids"], ["deepseek-flash"])
        self.assertEqual(narrow["cells"], 12)
        self.assertTrue(all(cell["model_id"] == "deepseek-flash" for cell in narrow["matrix"]))
        broken = json.loads(json.dumps(rows))
        broken[0]["defect"]["sha256"] = "WRONG"
        bad = validate_config(config, broken)
        self.assertFalse(bad["ok"])
        self.assertTrue(any("hash mismatch" in err for err in bad["errors"]))
        self.assertEqual(bad["requests_upper_bound"], 0)
        rejected = validate_config({**config, "models": ["kimi-k3"]}, rows)
        self.assertFalse(rejected["ok"])
        self.assertTrue(any("kimi-k3" in err for err in rejected["errors"]))

    def test_feedback_comes_from_the_toolchain_not_the_oracle(self):
        from cir_workflow.toolchain_feedback import feedback_from_toolchain
        result = _ok_eval()
        result["functional"] = {"status": "fail", "raw_stdout": "DONE done=5",
                                "returncode": 0, "timed_out": False, "evidence_path": None}
        text, items = feedback_from_toolchain("counterexample", result)
        self.assertNotIn("requirement_oracle", text)
        self.assertIn("bounded_trace_deviation=violation", text)
        self.assertIn("DONE done=5", text)
        self.assertNotIn("DONE done=6", text)
        self.assertTrue(any(item["layer"] == "external_functional" for item in items))
        verdict, verdict_items = feedback_from_toolchain("verdict_only", result)
        self.assertNotIn("mutex_unlock", verdict)
        self.assertIn("external_functional=fail", verdict)
        self.assertTrue(all(not item["detail_in_prompt"] for item in verdict_items))
        self.assertTrue(any(item["detail_in_prompt"] for item in items))

    def test_compile_failure_stays_toolchain_feedback(self):
        from cir_workflow.toolchain_feedback import feedback_from_toolchain, has_defect_signal
        log = self.out / "build.log"
        log.write_text("error[E0425]: cannot find value `nope` in this scope\n",
                       encoding="utf-8")
        result = {"ledger": {"current_evaluation": "source_build_failed",
                             "model": {"verified": True}, "trace": {}, "run": {}},
                  "source_build_log": str(log), "functional": {"status": "not_run"},
                  "source_sha256": "s", "cir_sha256": "c"}
        self.assertTrue(has_defect_signal(result))
        text, _items = feedback_from_toolchain("counterexample", result)
        verdict, _ignored = feedback_from_toolchain("verdict_only", result)
        self.assertIn("source_build=failed", verdict)
        self.assertNotIn("E0425", verdict)
        self.assertIn("error[E0425]", text)

    def test_manifest_path_is_the_source_that_runs(self):
        import sys
        sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
        from scripts.feedback_pilot import freeze_inputs
        _config, rows, _ignored = freeze_inputs()
        row = json.loads(json.dumps(rows[0]))
        other = self.out / "other.rs"
        other.write_text(self.case["control"], encoding="utf-8")
        import hashlib
        row["defect"] = {"path": str(other),
                         "sha256": hashlib.sha256(self.case["control"].encode()).hexdigest()}
        from cir_workflow.feedback_runner import load_frozen
        loaded = load_frozen(row, _config["tools"])
        self.assertEqual(loaded["defect"], self.case["control"])
        client = _Fake([_fence(self.case["defect"])])
        result = run_arm(loaded, self.spec, "verdict_only", client, self.out / "ran",
                         evaluate=_ok_eval)
        self.assertEqual(client.calls, 0)
        self.assertEqual(result["first_requirement_round"], 0)

    def test_audited_client_k3_saves_state(self):
        from cir_workflow.audit import AuditLog
        from cir_workflow.channels import AuditedClient
        spec = require_experiment_model(build_registry(), "Kimi 2.7 Code")
        inner = _Fake([_fence(self.case["control"])], model_id="kimi-k3")
        audit = AuditLog(self.out / "events.jsonl")
        client = AuditedClient(inner, audit=audit, run_id="drill", cell_id="c",
                               spec=spec, arm="counterexample", task_id="lock_order",
                               replicate=0, stage="repair")
        result = run_arm(self.case, spec, "counterexample", client, self.out / "k3",
                         evaluate=_ok_eval)
        self.assertEqual(inner.calls, 1)
        self.assertEqual(result["stop"], "identity_mismatch")
        state = json.loads((self.out / "k3" / "state.json").read_text())
        self.assertFalse(state["requests"][0]["identity"]["identity_confirmed"])
        self.assertIsNone(state.get("pending"))

    def test_pending_without_audit_is_not_resent(self):
        dest = self.out / "pending"
        dest.mkdir()
        (dest / "state.json").write_text(json.dumps({
            "fingerprint": None, "requests": [],
            "pending": {"round": 1, "arm": "verdict_only", "prompt_sha256": "abc",
                        "feedback": "x"},
        }), encoding="utf-8")
        # fingerprint will be overwritten after mismatch check only if empty.
        client = _Fake([_fence(self.case["control"])])
        result = run_arm(self.case, self.spec, "verdict_only", client, dest,
                         evaluate=_ok_eval, audit_events=[])
        self.assertEqual(result["stop"], "outcome_unknown")
        self.assertEqual(client.calls, 0)

    def _audited(self, inner, name, spec=None):
        from cir_workflow.audit import AuditLog
        from cir_workflow.channels import AuditedClient
        spec = spec or self.spec
        audit = AuditLog(self.out / name / "events.jsonl")
        client = AuditedClient(inner, audit=audit, run_id="drill", cell_id="c",
                               spec=spec, arm="verdict_only", task_id="lock_order",
                               replicate=0, stage="repair")
        return client, audit

    def test_audited_client_missing_model_id_saves_state(self):
        spec = require_experiment_model(build_registry(), "Kimi 2.7 Code")
        inner = _Fake([_fence(self.case["control"])], model_id=None)
        client, _audit = self._audited(inner, "missing", spec)
        result = run_arm(self.case, spec, "verdict_only", client, self.out / "missing-run",
                         evaluate=_ok_eval)
        self.assertEqual(inner.calls, 1)
        self.assertEqual(result["stop"], "identity_unconfirmed")
        state = json.loads((self.out / "missing-run" / "state.json").read_text())
        self.assertEqual(len(state["requests"]), 1)
        self.assertFalse(state["requests"][0]["identity"]["identity_confirmed"])
        self.assertIsNone(state.get("pending"))

    def test_audited_client_request_exception_saves_state(self):
        class _Boom:
            calls = 0

            def complete(self, system, user):
                self.calls += 1
                raise RuntimeError("network down")

        inner = _Boom()
        client, audit = self._audited(inner, "boom")
        result = run_arm(self.case, self.spec, "verdict_only", client, self.out / "boom-run",
                         evaluate=_ok_eval)
        self.assertEqual(inner.calls, 1)
        self.assertEqual(result["stop"], "request_error")
        state = json.loads((self.out / "boom-run" / "state.json").read_text())
        self.assertEqual(state["requests"][0]["error_type"], "RuntimeError")
        self.assertTrue(audit.path.is_file())
        again = _Fake([_fence(self.case["control"])])
        client2, _audit2 = self._audited(again, "boom-again")
        second = run_arm(self.case, self.spec, "verdict_only", client2, self.out / "boom-run",
                         evaluate=_ok_eval)
        self.assertEqual(again.calls, 0)
        self.assertEqual(second["stop"], "request_error")

    def test_audit_response_is_recovered_without_resend(self):
        from cir_workflow.audit import AuditLog, read_events
        from cir_workflow.feedback_runner import PROMPT_PATH, _fingerprint, _sha_text, render_prompt
        from cir_workflow.toolchain_feedback import feedback_from_toolchain
        feedback, _items = feedback_from_toolchain("verdict_only", _ok_eval())
        prompt = render_prompt(self.case["requirements"],
                               json.dumps(self.case["cir"], sort_keys=True),
                               self.case["defect"], feedback)
        system = PROMPT_PATH.read_text(encoding="utf-8").strip()
        audit = AuditLog(self.out / "recover" / "events.jsonl")
        audit.model_call(
            run_id="drill", cell_id="c", model=self.spec.display_name,
            provider=self.spec.provider, transport=self.spec.channel,
            arm="verdict_only", task_id="lock_order", replicate=0, stage="repair",
            requested_model=self.spec.model_id, returned_model=self.spec.model_id,
            usage_raw={"prompt_tokens": 3, "completion_tokens": 4},
            started_at=1.0, ended_at=1.2, prompt=system + "\n\n" + prompt.strip(),
            response=_fence(self.case["control"]), candidate_round=1, status="ok")
        events = read_events(audit.path)
        dest = self.out / "recover-run"
        dest.mkdir()
        (dest / "state.json").write_text(json.dumps({
            "fingerprint": _fingerprint(self.spec, "verdict_only", self.case, 2),
            "requests": [],
            "pending": {"round": 1, "arm": "verdict_only",
                        "prompt_sha256": _sha_text(prompt),
                        "audit_prompt_sha256": events[0]["prompt_sha256"],
                        "feedback": feedback},
        }), encoding="utf-8")
        client = _Fake([_fence(self.case["defect"])])
        result = run_arm(self.case, self.spec, "verdict_only", client, dest,
                         evaluate=_ok_eval, audit_events=events)
        self.assertEqual(client.calls, 0)
        self.assertEqual(result["stop"], "requirement_pass")
        self.assertEqual(result["first_requirement_round"], 1)
        state = json.loads((dest / "state.json").read_text())
        self.assertTrue(state["requests"][0]["recovered_from_audit"])
        self.assertIsNone(state.get("pending"))


if __name__ == "__main__":
    unittest.main()
