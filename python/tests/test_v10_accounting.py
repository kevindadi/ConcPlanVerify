"""Offline checks for persisted request identity and transport counts."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from cir_workflow.audit import AuditLog, read_events
from cir_workflow.channels import AuditedClient
from cir_workflow.feedback_runner import run_arm
from cir_workflow.live import BudgetExhausted, DeepSeekFlashClient, LiveBudget
from cir_workflow.pilot_cases import load_case
from cir_workflow.transport import build_registry, require_experiment_model
from tests.test_feedback_runner import _Fake, _fence, _ok_eval


class _Crash(BaseException):
    pass


def _wrap(inner, audit, spec, root_name):
    return AuditedClient(
        inner, audit=audit, run_id=root_name,
        cell_id="deepseek-flash/lock_order/verdict_only", spec=spec, arm="verdict_only",
        task_id="lock_order", replicate=0, stage="repair")


class _StatusError(Exception):
    def __init__(self, status_code):
        super().__init__(f"status {status_code}")
        self.status_code = status_code


class _Sdk:
    def __init__(self, items):
        self.items = list(items)
        self.calls = 0
        self.chat = SimpleNamespace(completions=SimpleNamespace(create=self.create))

    def create(self, **kwargs):
        self.calls += 1
        item = self.items.pop(0)
        if isinstance(item, Exception):
            raise item
        return item


def _chat(text, model, usage=None):
    return SimpleNamespace(
        model=model, id="sdk-req", usage=usage,
        choices=[SimpleNamespace(message=SimpleNamespace(content=text), finish_reason="stop")])


class AccountingTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.case = load_case("lock_order")
        self.spec = require_experiment_model(build_registry(), "DeepSeek Flash")

    def tearDown(self):
        self.tmp.cleanup()

    def test_new_client_records_round_two_and_recovery_does_not_recount(self):
        audit = AuditLog(self.root / "events.jsonl")
        dest = self.root / "cell"
        first_inner = _Fake([_fence(self.case["defect"])])
        first = run_arm(self.case, self.spec, "verdict_only",
                        _wrap(first_inner, audit, self.spec, "acct"), dest,
                        evaluate=_ok_eval, halt_after_new_requests=1)
        self.assertEqual(first["stop"], "halted")
        self.assertEqual(first_inner.calls, 1)

        class _Second:
            calls = 0

            def complete(self, system, user):
                self.calls += 1
                return SimpleNamespace(
                    text=_fence(self_case_control), usage={"prompt_tokens": 5, "completion_tokens": 9},
                    wall_ms=4, response_model=self_model, request_id="round-2")

        self_case_control = self.case["control"]
        self_model = self.spec.model_id
        second_inner = _Second()
        real_save = __import__("cir_workflow.feedback_runner", fromlist=["_save_state"])._save_state

        def crash(path, state):
            if any(item.get("round") == 2 for item in state.get("requests") or []) and not state.get("pending"):
                raise _Crash()
            return real_save(path, state)

        with patch("cir_workflow.feedback_runner._save_state", crash):
            with self.assertRaises(_Crash):
                run_arm(self.case, self.spec, "verdict_only",
                        _wrap(second_inner, audit, self.spec, "acct"), dest, evaluate=_ok_eval)
        self.assertEqual(second_inner.calls, 1)
        events = read_events(audit.path)
        self.assertEqual([event["candidate_round"] for event in events], [1, 2])
        self.assertEqual([event["attempt_id"] for event in events], ["a1", "a2"])
        pending = json.loads((dest / "state.json").read_text())["pending"]
        self.assertEqual(pending["round"], 2)
        self.assertEqual(pending["attempt"], 2)

        third = _Fake([])
        recovered = run_arm(self.case, self.spec, "verdict_only",
                            _wrap(third, audit, self.spec, "acct"), dest,
                            evaluate=_ok_eval, audit_events=events)
        self.assertEqual(third.calls, 0)
        self.assertEqual(recovered["stop"], "requirement_pass")
        self.assertEqual(recovered["physical_attempts_this_run"], 0)
        tokens = recovered["physical_attempts_cumulative"]
        again = run_arm(self.case, self.spec, "verdict_only",
                        _wrap(_Fake([]), audit, self.spec, "acct"), dest,
                        evaluate=_ok_eval, audit_events=events)
        self.assertEqual(again["physical_attempts_this_run"], 0)
        self.assertEqual(again["physical_attempts_cumulative"], tokens)
        self.assertEqual(again["logical_calls_this_run"], 0)
        saved = json.loads((dest / "state.json").read_text())["requests"]
        self.assertEqual(len([item for item in saved if item.get("round") == 2]), 1)
        self.assertEqual(saved[-1]["usage"]["input_tokens"], 5)
        self.assertEqual(saved[-1]["usage"]["output_tokens"], 9)

    def _run_sdk(self, name, items, max_requests):
        dest = self.root / name
        budget = LiveBudget(dest / "budget.json", max_requests=max_requests, max_seconds=600)
        sdk = _Sdk(items)
        inner = DeepSeekFlashClient(api_key="offline", budget=budget, evidence_dir=dest / "llm",
                                    sdk_client=sdk, max_transport_retries=1)
        client = AuditedClient(
            inner, audit=AuditLog(dest / "events.jsonl"), run_id=name, cell_id="cell",
            spec=self.spec, arm="verdict_only", task_id="lock_order", replicate=0, stage="repair")
        result = run_arm(self.case, self.spec, "verdict_only", client, dest / "cell", evaluate=_ok_eval)
        events = read_events(dest / "events.jsonl")
        return result, sdk, budget, events

    def test_budget_refusal_before_send_has_zero_physical_attempts(self):
        result, sdk, budget, events = self._run_sdk(
            "refuse", [_chat(_fence(self.case["control"]), self.spec.model_id)], 0)
        self.assertEqual(sdk.calls, 0)
        self.assertEqual(budget.requests_used, 0)
        self.assertEqual(result["physical_attempts_this_run"], 0)
        self.assertEqual(result["logical_calls_this_run"], 0)
        self.assertEqual(result["stop"], "global_request_budget_exhausted")
        self.assertEqual(events[0]["candidate_round"], 1)
        self.assertEqual(events[0]["transport_attempt"], 0)
        self.assertEqual(events[0]["transport_log"], [])

    def test_one_retry_is_two_physical_attempts_and_one_repair_round(self):
        usage = {"prompt_tokens": 3, "completion_tokens": 4}
        result, sdk, budget, events = self._run_sdk(
            "retry", [_StatusError(500), _chat(_fence(self.case["control"]), self.spec.model_id, usage)], 4)
        self.assertEqual(sdk.calls, 2)
        self.assertEqual(budget.requests_used, 2)
        self.assertEqual(result["physical_attempts_this_run"], 2)
        self.assertEqual(result["logical_calls_this_run"], 1)
        self.assertEqual(result["semantic_repair_rounds"], 1)
        self.assertEqual(result["stop"], "requirement_pass")
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["attempt_id"], "a1")
        self.assertEqual(len(events[0]["transport_log"]), 2)
        self.assertEqual(events[0]["transport_log"][0]["cell_id"], "cell")
        self.assertEqual(events[0]["transport_log"][0]["candidate_round"], 1)
        self.assertIsNone(events[0]["transport_log"][0]["usage"]["input_tokens"])
        self.assertEqual(events[0]["transport_log"][1]["usage"]["input_tokens"], 3)
        self.assertEqual(result["usage"][-1]["input_tokens"], 3)
        self.assertIsNone(result["usage"][-1]["cache_read_tokens"])

    def test_budget_exhausted_before_retry_keeps_the_sent_attempt(self):
        result, sdk, budget, _events = self._run_sdk(
            "mid", [_StatusError(500), _chat(_fence(self.case["control"]), self.spec.model_id)], 1)
        self.assertEqual(sdk.calls, 1)
        self.assertEqual(budget.requests_used, 1)
        self.assertEqual(result["physical_attempts_this_run"], 1)
        self.assertEqual(result["stop"], "global_request_budget_exhausted")


if __name__ == "__main__":
    unittest.main()
