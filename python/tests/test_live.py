"""Unit tests for the controlled DeepSeek Flash client (no network)."""

from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.live import (
    ALLOWED_MODEL,
    ALLOWED_PROVIDER,
    BudgetExhausted,
    DeepSeekFlashClient,
    LiveBudget,
    ModelIdentityError,
    NonRetryableLlmError,
    TransientLlmError,
    assert_allowed_model,
    run_live_pilot,
)
from cir_workflow.__main__ import main as cli_main


def _response(text="{\"ok\": true}", model=ALLOWED_MODEL, rid="req-1", finish="stop", usage=None):
    return SimpleNamespace(
        model=model, id=rid, usage=usage,
        choices=[SimpleNamespace(message=SimpleNamespace(content=text), finish_reason=finish)],
    )


class _FakeCompletions:
    def __init__(self, items):
        self.items = list(items)
        self.calls = []

    def create(self, **kwargs):
        self.calls.append(kwargs)
        item = self.items.pop(0)
        if isinstance(item, Exception):
            raise item
        return item


class _FakeSdk:
    def __init__(self, items):
        self.completions = _FakeCompletions(items)
        self.chat = SimpleNamespace(completions=self.completions)


class _StatusError(Exception):
    def __init__(self, status_code):
        super().__init__(f"status {status_code}")
        self.status_code = status_code


class LiveClientTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def client(self, items, *, max_requests=12, max_transport_retries=1, key="sk-secret-value"):
        budget = LiveBudget(self.root / "budget.json", max_requests=max_requests, max_seconds=600)
        sdk = _FakeSdk(items)
        client = DeepSeekFlashClient(
            api_key=key, budget=budget, evidence_dir=self.root, sdk_client=sdk,
            max_transport_retries=max_transport_retries,
        )
        return client, sdk, budget

    def test_only_flash_is_allowed(self):
        with self.assertRaises(ValueError):
            assert_allowed_model(ALLOWED_PROVIDER, "deepseek-pro")
        with self.assertRaises(ValueError):
            assert_allowed_model("openai", ALLOWED_MODEL)

    def test_thinking_is_explicitly_disabled_and_usage_missing_is_none(self):
        client, sdk, _ = self.client([_response(usage=None)])
        outcome = client.complete("system", "user")
        self.assertEqual(outcome.usage, None)
        kwargs = sdk.completions.calls[0]
        self.assertEqual(kwargs["model"], ALLOWED_MODEL)
        self.assertEqual(kwargs["extra_body"], {"thinking": {"type": "disabled"}})
        self.assertEqual(kwargs["max_tokens"], 4096)

    def test_request_log_is_sanitized(self):
        client, _, _ = self.client([_response()])
        client.complete("system-text", "user-text")
        log = (self.root / "llm_requests.jsonl").read_text()
        self.assertNotIn("sk-secret-value", log)
        self.assertNotIn("Authorization", log)
        record = json.loads(log.splitlines()[0])
        self.assertEqual(record["requested_model"], ALLOWED_MODEL)
        self.assertEqual(record["response_model"], ALLOWED_MODEL)
        self.assertEqual(record["thinking"], {"thinking": {"type": "disabled"}})
        self.assertIn("prompt_sha256", record)

    def test_budget_stops_before_sending(self):
        client, sdk, budget = self.client([_response()], max_requests=1)
        client.complete("s", "u")
        with self.assertRaises(BudgetExhausted):
            client.complete("s", "u")
        self.assertEqual(len(sdk.completions.calls), 1)
        self.assertEqual(budget.requests_used, 1)

    def test_non_retryable_stops_immediately(self):
        client, sdk, _ = self.client([_StatusError(401), _response()])
        with self.assertRaises(NonRetryableLlmError):
            client.complete("s", "u")
        self.assertEqual(len(sdk.completions.calls), 1)

    def test_retryable_is_retried_once(self):
        client, sdk, budget = self.client([_StatusError(503), _response()])
        outcome = client.complete("s", "u")
        self.assertEqual(outcome.transport_attempt, 2)
        self.assertEqual(len(sdk.completions.calls), 2)
        self.assertEqual(budget.requests_used, 2)

    def test_retryable_exhausted_raises_transient(self):
        client, sdk, _ = self.client([_StatusError(503), _StatusError(503)], max_transport_retries=1)
        with self.assertRaises(TransientLlmError):
            client.complete("s", "u")
        self.assertEqual(len(sdk.completions.calls), 2)

    def test_non_flash_response_identity_stops_batch(self):
        client, _, _ = self.client([_response(model="deepseek-pro")])
        with self.assertRaises(ModelIdentityError):
            client.complete("s", "u")

    def test_run_live_pilot_stops_on_identity_error(self):
        tasks = self.root / "tasks.json"
        tasks.write_text(json.dumps({"tasks": [{
            "id": "t", "requirements": "x",
            "contract": "contract.json", "structural_check": "abba_inversion",
        }]}))
        (self.root / "contract.json").write_text("{}")
        sdk = _FakeSdk([_response(model="deepseek-pro")])
        summary = run_live_pilot(tasks, out_dir=self.root / "out", binary="/nonexistent",
                                 api_key="sk-secret-value", sdk_client=sdk)
        self.assertIn("ModelIdentityError", summary["stop_reason"])
        self.assertEqual(summary["requests_used"], 1)

    def test_cli_rejects_pro_before_any_request(self):
        with contextlib.redirect_stdout(io.StringIO()):
            code = cli_main(["live", "--tasks", str(self.root / "nope.json"),
                             "--model", "deepseek-pro", "--api-key-env", "UNSET_KEY_XYZ"])
        self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
