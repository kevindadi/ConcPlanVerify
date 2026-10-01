"""Recovery and identity for the three-arm entry, through AuditedClient."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.audit import AuditLog, read_events
from cir_workflow.channels import AuditedClient
from cir_workflow.conditional_arms import SYSTEM, feedback_for, plan_row, render_prompt, run_repairs
from cir_workflow.feedback_runner import _sha_text
from cir_workflow.live import LiveBudget
from cir_workflow.transport import build_registry, require_experiment_model

DEFECT = "fn main() { println!(\"DONE done=1\"); }\n"
CONTROL = "fn main() { println!(\"CONTROL\"); }\n"


def _score(source, _work):
    if "CONTROL" in source:
        return {"status": "bounded_covered_satisfied", "runs": []}
    if "UNKNOWN" in source:
        return {"status": "unknown", "runs": []}
    return {"status": "fail", "runs": [{"kind": "completed", "stdout": "DONE done=1\n",
                                       "returncode": 0, "timed_out": False}]}


class _Inner:
    def __init__(self, text, model):
        self.text = text
        self.model = model
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(text=self.text, response_model=self.model, usage=None,
                               wall_ms=1, finish_reason="stop")


def _client(inner, spec, root, arm="A"):
    audit = AuditLog(root / "events.jsonl")
    return AuditedClient(inner, audit=audit, run_id="run-1", cell_id=f"{spec.model_id}/{arm}",
                         spec=spec, arm=arm, task_id="channel/send_while_holding_mutex",
                         replicate=0, stage="repair")


def _case():
    return {"task": "channel/send_while_holding_mutex", "defect": DEFECT,
            "requirements": "print DONE done=1", "cir_text": "{}",
            "model_name": "DeepSeek Flash"}


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_completed_cell_restart_sends_nothing(self):
        inner = _Inner("```rust\n" + CONTROL + "```", self.spec.model_id)
        dest = self.root / "cell"
        client = _client(inner, self.spec, self.root)
        first = run_repairs(_case(), "A", client, dest, evaluate_candidate=None, score=_score)
        self.assertEqual(first["stop"], "bounded_covered_satisfied")
        self.assertEqual(inner.calls, 1)
        again = _Inner("```rust\n" + CONTROL + "```", self.spec.model_id)
        client2 = _client(again, self.spec, self.root)
        second = run_repairs(_case(), "A", client2, dest, evaluate_candidate=None, score=_score,
                             audit_events=read_events(client.audit.path))
        self.assertEqual(again.calls, 0)
        self.assertEqual(second["actual_model_requests_this_run"], 0)
        self.assertEqual(second["stop"], "bounded_covered_satisfied")

    def test_audited_response_is_recovered_without_resend(self):
        feedback = feedback_for("A", _score(DEFECT, None), None)
        prompt = render_prompt(arm="A", requirements="print DONE done=1", cir_text="{}",
                               previous=DEFECT, feedback=feedback)
        audit = AuditLog(self.root / "recover-events.jsonl")
        audit.model_call(
            run_id="run-1", cell_id=f"{self.spec.model_id}/A", model=self.spec.display_name,
            provider=self.spec.provider, transport=self.spec.channel, arm="A",
            task_id="channel/send_while_holding_mutex", replicate=0, stage="repair",
            requested_model=self.spec.model_id, returned_model=self.spec.model_id,
            usage_raw={"prompt_tokens": 3, "completion_tokens": 4},
            started_at=1.0, ended_at=1.2, prompt=SYSTEM + "\n\n" + prompt.strip(),
            response="```rust\n" + CONTROL + "```", candidate_round=1, attempt_id="a1",
            status="ok")
        events = read_events(audit.path)
        dest = self.root / "recover"
        dest.mkdir()
        (dest / "state.json").write_text(json.dumps({
            "requests": [],
            "pending": {"round": 1, "arm": "A", "attempt": 1, "run_id": "run-1",
                        "cell_id": f"{self.spec.model_id}/A", "model": self.spec.model_id,
                        "prompt_sha256": _sha_text(prompt),
                        "audit_prompt_sha256": events[0]["prompt_sha256"],
                        "feedback": feedback},
        }), encoding="utf-8")
        inner = _Inner("should-not-send", self.spec.model_id)
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, dest, evaluate_candidate=None, score=_score,
                             audit_events=events)
        self.assertEqual(inner.calls, 0)
        self.assertEqual(result["stop"], "bounded_covered_satisfied")
        state = json.loads((dest / "state.json").read_text())
        self.assertEqual(state["requests"][0]["round"], events[0]["candidate_round"])
        self.assertTrue(state["requests"][0]["recovered_from_audit"])

    def test_missing_response_model_is_not_success(self):
        inner = _Inner("```rust\n" + CONTROL + "```", None)
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, self.root / "missing",
                             evaluate_candidate=None, score=_score)
        self.assertEqual(result["stop"], "identity_unconfirmed")
        self.assertNotEqual(result["stop"], "bounded_covered_satisfied")
        events = read_events(client.audit.path)
        self.assertFalse(events[0]["identity_confirmed"])

    def test_identity_mismatch_blocks_only_that_model(self):
        inner = _Inner("```rust\n" + CONTROL + "```", "other-model")
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, self.root / "mismatch",
                             evaluate_candidate=None, score=_score)
        self.assertEqual(result["stop"], "identity_mismatch")
        rows = [
            plan_row({"model_id": "deepseek-flash", "arm": "B"}, global_stop=None,
                     model_stop="identity_mismatch", executed=None),
            plan_row({"model_id": "qwen3.8-flash", "arm": "A"}, global_stop=None,
                     model_stop=None, executed={"stop": "bounded_covered_satisfied"}),
        ]
        self.assertEqual(rows[0]["status"], "blocked")
        self.assertEqual(rows[1]["status"], "executed")

    def test_nonempty_directory_without_state_is_refused(self):
        dest = self.root / "dirty"
        dest.mkdir()
        (dest / "prompt-1.txt").write_text("old", encoding="utf-8")
        inner = _Inner("```rust\n" + CONTROL + "```", self.spec.model_id)
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, dest, evaluate_candidate=None, score=_score)
        self.assertEqual(result["stop"], "refuse_nonempty_without_state")
        self.assertEqual(inner.calls, 0)

    def test_changed_freeze_file_is_refused_before_send(self):
        target = self.root / "scorer.py"
        target.write_text("v1", encoding="utf-8")
        frozen = {str(target): "0" * 64}
        inner = _Inner("```rust\n" + CONTROL + "```", self.spec.model_id)
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, self.root / "freeze", evaluate_candidate=None,
                             score=_score, freeze_files=frozen)
        self.assertEqual(result["stop"], "frozen_input_changed")
        self.assertEqual(inner.calls, 0)
        self.assertEqual(result["actual_model_requests_this_run"], 0)

    def test_empty_body_is_not_a_code_candidate(self):
        inner = _Inner("", self.spec.model_id)
        inner.finish = "stop"
        client = _client(inner, self.spec, self.root)
        result = run_repairs(_case(), "A", client, self.root / "empty", evaluate_candidate=None,
                             score=_score, max_repairs=1)
        classes = [row.get("response_class") for row in result["rounds"]]
        self.assertIn("empty", classes)
        self.assertTrue(all(not row.get("new_candidate") for row in result["rounds"] if row.get("response_class") == "empty"))
        self.assertNotEqual(result["stop"], "bounded_covered_satisfied")

    def test_budget_file_survives_a_new_object(self):
        path = self.root / "budget.json"
        first = LiveBudget(path, max_requests=24, max_seconds=3600)
        first.reserve()
        second = LiveBudget(path, max_requests=24, max_seconds=3600)
        self.assertEqual(second.requests_used, 1)
        self.assertEqual(second.deadline_epoch, first.deadline_epoch)

    def test_plan_keeps_not_started_rows(self):
        row = plan_row({"model_id": "kimi-k2.7-code", "arm": "A"},
                       global_stop="global_request_budget_exhausted",
                       model_stop=None, executed=None)
        self.assertEqual(row["status"], "not_started")
        self.assertEqual(row["stop"], "global_request_budget_exhausted")
