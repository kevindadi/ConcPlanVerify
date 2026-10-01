"""OpenCode budget, usage zeros, and failed-request persistence."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import httpx

from cir_workflow.audit import AuditLog, parse_usage
from cir_workflow.channels import AuditedClient
from cir_workflow.feedback_runner import _usage, run_arm
from cir_workflow.live import LiveBudget
from cir_workflow.opencode_go import OpenCodeGoClient, OpenCodeGoResponsesClient
from cir_workflow.pilot_cases import load_case
from cir_workflow.transport import build_registry, require_experiment_model, resolve_model
from tests.test_feedback_runner import _Fake, _fence, _ok_eval


def _http_client(handler):
    return httpx.Client(transport=httpx.MockTransport(handler))


def _install(client, handler):
    client._client = client._client.with_options(max_retries=0, http_client=_http_client(handler))


class OpenCodeBudgetTests(unittest.TestCase):
    def test_chat_and_responses_stop_at_one_http_when_budget_is_one(self):
        for kind in ("chat", "responses"):
            with self.subTest(kind=kind):
                root = Path(tempfile.mkdtemp())
                budget = LiveBudget(root / "budget.json", max_requests=1, max_seconds=60)
                wire = []

                def handler(request):
                    wire.append(request.url.path)
                    if len(wire) < 3:
                        return httpx.Response(500, json={"error": {"message": "offline", "type": "server_error"}})
                    if kind == "chat":
                        body = {"id": "r", "object": "chat.completion", "created": 0,
                                "model": "kimi-k2.7-code",
                                "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"},
                                             "finish_reason": "stop"}],
                                "usage": {"prompt_tokens": 2, "completion_tokens": 1, "total_tokens": 3}}
                    else:
                        body = {"id": "r", "object": "response", "status": "completed",
                                "model": "gpt-6-luna",
                                "output": [{"type": "message", "role": "assistant",
                                            "content": [{"type": "output_text", "text": "ok"}]}],
                                "usage": {"input_tokens": 2, "output_tokens": 1, "total_tokens": 3}}
                    return httpx.Response(200, json=body)

                if kind == "chat":
                    client = OpenCodeGoClient(api_key="offline", budget=budget, evidence_dir=root,
                                              model="kimi-k2.7-code")
                else:
                    client = OpenCodeGoResponsesClient(api_key="offline", budget=budget,
                                                       evidence_dir=root, model="gpt-6-luna")
                self.assertEqual(client._client.max_retries, 0)
                _install(client, handler)
                with self.assertRaises(Exception):
                    client.complete("system", "user")
                self.assertEqual(len(wire), 1)
                self.assertEqual(budget.requests_used, 1)
                self.assertEqual(len(client.last_transport_log), 1)


class UsageAndPersistenceTests(unittest.TestCase):
    def test_zero_cache_is_kept_and_missing_stays_null(self):
        present = parse_usage({
            "prompt_tokens": 10, "completion_tokens": 2,
            "prompt_cache_hit_tokens": 0,
            "prompt_tokens_details": {"cached_tokens": 0},
        })
        self.assertEqual(present.cache_read_tokens, 0)
        nested = parse_usage({
            "prompt_tokens": 10, "completion_tokens": 2,
            "prompt_tokens_details": {"cached_tokens": 512},
        })
        self.assertEqual(nested.cache_read_tokens, 512)
        missing = parse_usage({"prompt_tokens": 10, "completion_tokens": 2})
        self.assertIsNone(missing.cache_read_tokens)
        outcome = type("O", (), {"usage": {
            "prompt_tokens": 10, "completion_tokens": 2, "prompt_cache_hit_tokens": 0,
        }, "wall_ms": 3})()
        usage = _usage(outcome)
        self.assertEqual(usage["cache_read_tokens"], 0)
        self.assertIsNone(_usage(type("O", (), {"usage": None, "wall_ms": None})())["input_tokens"])

    def test_failed_request_fields_survive_restart(self):
        case = load_case("lock_order")
        spec = resolve_model(build_registry(), "Kimi 2.7 Code")
        for name, inner in (
            ("identity", _Fake([_fence(case["control"])], model_id="kimi-k3")),
            ("error", _Boom()),
        ):
            with self.subTest(name=name):
                root = Path(tempfile.mkdtemp())
                audit = AuditLog(root / "events.jsonl")
                client = AuditedClient(
                    inner, audit=audit, run_id=name, cell_id="c", spec=spec,
                    arm="verdict_only", task_id="lock_order", replicate=0, stage="repair")
                first = run_arm(case, spec, "verdict_only", client, root / "cell", evaluate=_ok_eval)
                saved = json.loads((root / "cell" / "state.json").read_text())["requests"][0]
                self.assertIn("physical_attempts", saved)
                self.assertIn("send_status", saved)
                self.assertIn("transport_log", saved)
                self.assertEqual(first["physical_attempts_cumulative"], saved["physical_attempts"])
                again = run_arm(case, spec, "verdict_only", client, root / "cell", evaluate=_ok_eval)
                third = run_arm(case, spec, "verdict_only", client, root / "cell", evaluate=_ok_eval)
                self.assertEqual(again["physical_attempts_this_run"], 0)
                self.assertEqual(third["physical_attempts_this_run"], 0)
                self.assertEqual(again["physical_attempts_cumulative"], first["physical_attempts_cumulative"])
                self.assertEqual(third["physical_attempts_cumulative"], first["physical_attempts_cumulative"])
                self.assertEqual(inner.calls, 1)


class _Boom:
    calls = 0

    def complete(self, system, user):
        self.calls += 1
        self.last_transport_log = [{"transport_index": 1, "status": "error", "usage_raw": None}]
        raise RuntimeError("offline post-send error")


if __name__ == "__main__":
    unittest.main()
