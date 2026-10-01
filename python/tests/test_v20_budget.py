"""Budget lifecycle on the four experiment transports. No network."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import MagicMock

sys.modules.setdefault("openai", MagicMock())

from cir_workflow.direct import DirectChatClient
from cir_workflow.live import DeepSeekFlashClient, LiveBudget
from cir_workflow.opencode_go import OpenCodeGoClient, OpenCodeGoResponsesClient


class _SDK:
    def __init__(self, *, fail_times: int = 0, error: Exception | None = None):
        self.fail_times = fail_times
        self.error = error or TimeoutError("timed out")
        self.calls = 0
        self.chat = self
        self.completions = self
        self.responses = self

    def create(self, **kwargs):
        self.calls += 1
        if self.calls <= self.fail_times:
            raise self.error
        return SimpleNamespace(
            id="req", model=kwargs.get("model"),
            output_text="fn main() {}",
            usage=SimpleNamespace(prompt_tokens=2, completion_tokens=1, total_tokens=3,
                                  model_dump=lambda: {"prompt_tokens": 2, "completion_tokens": 1}),
            choices=[SimpleNamespace(finish_reason="stop",
                                     message=SimpleNamespace(content="fn main() {}"))])


def _bind(client, attempt="cell-r1-a1"):
    client.call_context = {"run_id": "run", "cell_id": "cell", "attempt_id": attempt}


class LifecycleTests(unittest.TestCase):
    def _budget(self, root: Path) -> LiveBudget:
        return LiveBudget(root / "budget.json", max_requests=6, max_seconds=30)

    def test_deepseek_marks_returned_and_keeps_the_link(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            client = DeepSeekFlashClient(api_key="k", budget=budget, evidence_dir=root / "llm",
                                        sdk_client=_SDK(), max_transport_retries=0)
            _bind(client)
            client.complete("s", "u")
            rows = budget.attempt_rows()
            self.assertEqual([row["status"] for row in rows], ["returned"])
            self.assertEqual(rows[0]["cell_id"], "cell")
            self.assertEqual(rows[0]["logical_attempt"], "cell-r1-a1")
            self.assertEqual(rows[0]["transport_retry"], 1)

    def test_deepseek_retry_then_return_are_separate_reservations(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            client = DeepSeekFlashClient(api_key="k", budget=budget, evidence_dir=root / "llm",
                                        sdk_client=_SDK(fail_times=1), max_transport_retries=1)
            _bind(client)
            client.complete("s", "u")
            rows = budget.attempt_rows()
            self.assertEqual([row["status"] for row in rows], ["awaiting_reconciliation", "returned"])
            self.assertEqual([row["transport_retry"] for row in rows], [1, 2])
            self.assertEqual(budget.requests_used, 2)

    def test_direct_and_both_opencode_surfaces_mark_returned(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            direct = DirectChatClient(api_key="k", base_url="http://example.invalid", model="qwen3.8-flash",
                                     budget=budget, evidence_dir=root / "direct")
            direct._client = _SDK()
            _bind(direct, "qwen-r1-a1")
            direct.complete("s", "u")
            chat = OpenCodeGoClient(api_key="k", budget=budget, evidence_dir=root / "chat",
                                   model="glm-5.3-flash", reasoning_effort="low")
            chat._client = _SDK()
            _bind(chat, "glm-r1-a1")
            chat.complete("s", "u")
            responses = OpenCodeGoResponsesClient(api_key="k", budget=budget, evidence_dir=root / "resp",
                                                 model="gpt-6-luna")
            responses._client = _SDK()
            _bind(responses, "gpt-r1-a1")
            responses.complete("s", "u")
            rows = budget.attempt_rows()
            self.assertEqual([row["status"] for row in rows], ["returned", "returned", "returned"])
            self.assertEqual([row["logical_attempt"] for row in rows],
                             ["qwen-r1-a1", "glm-r1-a1", "gpt-r1-a1"])

    def test_uncertain_send_is_not_refunded(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            client = DeepSeekFlashClient(api_key="k", budget=budget, evidence_dir=root / "llm",
                                        sdk_client=_SDK(fail_times=1, error=RuntimeError("boom")),
                                        max_transport_retries=0)
            with self.assertRaises(Exception):
                client.complete("s", "u")
            self.assertEqual(budget.requests_used, 1)
            self.assertEqual(budget.attempt_rows()[0]["status"], "awaiting_reconciliation")

    def test_timeout_is_not_refunded_or_resent(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            sdk = _SDK(fail_times=1, error=TimeoutError("timed out"))
            client = DeepSeekFlashClient(api_key="k", budget=budget, evidence_dir=root / "llm",
                                        sdk_client=sdk, max_transport_retries=0)
            with self.assertRaises(Exception):
                client.complete("s", "u")
            self.assertEqual(sdk.calls, 1)
            self.assertEqual(budget.requests_used, 1)
            self.assertEqual(budget.attempt_rows()[0]["status"], "awaiting_reconciliation")

    def test_evidence_write_failure_still_returns_the_response(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            sdk = _SDK()
            client = DeepSeekFlashClient(api_key="k", budget=budget, evidence_dir=root / "llm",
                                        sdk_client=sdk, max_transport_retries=0)
            _bind(client)

            def fail_write(_record):
                raise OSError("disk")

            client._record = fail_write
            outcome = client.complete("s", "u")
            self.assertIn("fn main", outcome.text)
            self.assertEqual(sdk.calls, 1)
            self.assertEqual(budget.requests_used, 1)
            self.assertEqual(budget.attempt_rows()[0]["status"], "returned")
            self.assertIn("OSError", budget.lifecycle_error or "")

    def test_mirror_failure_keeps_the_reservation(self):
        import cir_workflow.live as live
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = self._budget(root)
            original = live.os.replace

            def fail_replace(src, dst):
                if str(dst).endswith("budget.json"):
                    raise OSError("mirror")
                return original(src, dst)

            live.os.replace = fail_replace
            try:
                used = budget.reserve({"run_id": "r", "cell_id": "c", "logical_attempt": "a",
                                       "transport_retry": 1})
            finally:
                live.os.replace = original
            self.assertEqual(used, 1)
            self.assertEqual(budget.requests_used, 1)
            self.assertIn("OSError", budget.mirror_error or "")
            self.assertEqual(budget.attempt_rows()[0]["cell_id"], "c")

    def test_threads_share_one_request_cap(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "budget.json"
            LiveBudget(path, max_requests=1, max_seconds=30)
            outcomes = []

            def once():
                budget = LiveBudget(path, max_requests=99, max_seconds=1)
                try:
                    outcomes.append(("ok", budget.reserve()))
                except Exception as exc:
                    outcomes.append((type(exc).__name__, str(exc)))

            import threading
            threads = [threading.Thread(target=once) for _ in range(2)]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
            self.assertEqual(sum(1 for kind, _text in outcomes if kind == "ok"), 1)
            self.assertEqual(json.loads(path.read_text())["requests_used"], 1)
