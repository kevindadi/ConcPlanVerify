"""v26: DeepSeek thinking pass-through and nested reasoning usage."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from cir_workflow.audit import parse_usage
from cir_workflow.live import DeepSeekFlashClient, LiveBudget


class DeepSeekThinkingTests(unittest.TestCase):
    def _client(self, td: str, thinking=None):
        budget = LiveBudget(Path(td) / "budget.json", max_requests=5, max_seconds=60)
        return DeepSeekFlashClient(api_key="k", budget=budget,
                                   evidence_dir=Path(td) / "ev", thinking=thinking)

    def test_default_keeps_thinking_disabled(self):
        with tempfile.TemporaryDirectory() as td:
            client = self._client(td)
            kwargs = client._request_kwargs([{"role": "user", "content": "x"}])
        self.assertEqual(kwargs["extra_body"], {"thinking": {"type": "disabled"}})

    def test_configured_thinking_reaches_the_wire(self):
        with tempfile.TemporaryDirectory() as td:
            client = self._client(td, thinking={"thinking": {"type": "enabled"}})
            kwargs = client._request_kwargs([{"role": "user", "content": "x"}])
        self.assertEqual(kwargs["extra_body"], {"thinking": {"type": "enabled"}})

    def test_nested_reasoning_is_read_and_not_double_counted(self):
        usage = {"prompt_tokens": 10, "completion_tokens": 100, "total_tokens": 110,
                 "completion_tokens_details": {"reasoning_tokens": 40}}
        parsed = parse_usage(usage)
        self.assertEqual(parsed.output_tokens, 100)
        self.assertEqual(parsed.reasoning_tokens, 40)
        # reasoning is a sub-item of output, never added again
        self.assertEqual(parsed.total_tokens, 110)

    def test_missing_reasoning_is_null(self):
        parsed = parse_usage({"prompt_tokens": 1, "completion_tokens": 2})
        self.assertIsNone(parsed.reasoning_tokens)


if __name__ == "__main__":
    unittest.main()
