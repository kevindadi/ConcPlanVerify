"""Audit log: usage parsing, missing-vs-zero, incremental, raw payloads."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.audit import AuditLog, estimate_tokens, parse_usage, read_events


class UsageParseTests(unittest.TestCase):
    def test_chat_completions_fields(self):
        u = parse_usage({"prompt_tokens": 100, "completion_tokens": 20,
                         "total_tokens": 120})
        self.assertEqual(u.input_tokens, 100)
        self.assertEqual(u.output_tokens, 20)
        self.assertEqual(u.total_tokens, 120)
        self.assertTrue(u.complete)
        self.assertEqual(u.source, "server")

    def test_responses_fields_and_reasoning_subitem(self):
        u = parse_usage({"input_tokens": 50, "output_tokens": 10,
                         "total_tokens": 60,
                         "output_tokens_details": {"reasoning_tokens": 4}})
        self.assertEqual(u.input_tokens, 50)
        self.assertEqual(u.reasoning_tokens, 4)
        # reasoning is a sub-item; total is not recomputed.
        self.assertEqual(u.total_tokens, 60)

    def test_cache_read_is_subitem_not_added(self):
        u = parse_usage({"prompt_tokens": 100, "completion_tokens": 5,
                         "prompt_tokens_details": {"cached_tokens": 80}})
        self.assertEqual(u.cache_read_tokens, 80)
        self.assertEqual(u.total_tokens, None)

    def test_missing_usage_is_null_not_zero(self):
        u = parse_usage(None)
        self.assertIsNone(u.input_tokens)
        self.assertIsNone(u.output_tokens)
        self.assertIsNone(u.total_tokens)
        self.assertFalse(u.complete)
        self.assertEqual(u.source, "unknown")

    def test_empty_dict_is_unknown(self):
        u = parse_usage({})
        self.assertEqual(u.source, "unknown")
        self.assertIsNone(u.input_tokens)

    def test_incremental_flag_default_true(self):
        u = parse_usage({"prompt_tokens": 1, "completion_tokens": 1})
        self.assertTrue(u.incremental)

    def test_local_estimate_is_marked(self):
        u = parse_usage({"input_tokens": estimate_tokens("abcd" * 4)},
                        source="local-estimate")
        self.assertEqual(u.source, "local-estimate")


class AuditLogTests(unittest.TestCase):
    def test_event_records_payload_hashes_and_paths(self):
        with tempfile.TemporaryDirectory() as td:
            log = AuditLog(Path(td) / "events.jsonl")
            log.model_call(
                run_id="r1", cell_id="c1", model="Kimi", provider="moonshot",
                transport="opencode", arm="G3", task_id="lock-order/x",
                replicate=0, stage="cir", requested_model="kimi-k3",
                returned_model="kimi-k3",
                usage_raw={"prompt_tokens": 10, "completion_tokens": 3,
                           "total_tokens": 13},
                started_at=1.0, ended_at=1.5, prompt="SYS\nUSER",
                response="candidate", candidate_round=1)
            events = read_events(Path(td) / "events.jsonl")
            self.assertEqual(len(events), 1)
            e = events[0]
            self.assertTrue(e["identity_confirmed"])
            self.assertEqual(e["usage"]["input_tokens"], 10)
            self.assertEqual(e["kind"], "model-call")
            self.assertTrue(Path(e["prompt_path"]).is_file())
            self.assertTrue(Path(e["response_path"]).is_file())

    def test_mismatch_event_is_not_confirmed(self):
        with tempfile.TemporaryDirectory() as td:
            log = AuditLog(Path(td) / "events.jsonl")
            log.model_call(
                run_id="r1", cell_id="c1", model="Kimi", provider="moonshot",
                transport="opencode", arm="G3", task_id="t", replicate=0,
                stage="cir", requested_model="kimi-k3", returned_model="glm-5.3",
                usage_raw=None, started_at=0.0, ended_at=0.1, prompt="p",
                response="r")
            e = read_events(Path(td) / "events.jsonl")[0]
            self.assertFalse(e["identity_confirmed"])
            self.assertEqual(e["usage"]["source"], "unknown")

    def test_tool_step_has_no_tokens(self):
        with tempfile.TemporaryDirectory() as td:
            log = AuditLog(Path(td) / "events.jsonl")
            log.tool_step(run_id="r1", cell_id="c1", arm="G3", task_id="t",
                          replicate=0, stage="code", tool="concir-conform",
                          started_at=0.0, ended_at=0.2, result="PASS")
            e = read_events(Path(td) / "events.jsonl")[0]
            self.assertEqual(e["kind"], "tool-step")
            self.assertIsNone(e["usage"]["input_tokens"])
            self.assertIsNone(e["usage"]["total_tokens"])

    def test_retry_attempts_are_separate_events(self):
        with tempfile.TemporaryDirectory() as td:
            log = AuditLog(Path(td) / "events.jsonl")
            for attempt in (1, 2):
                log.model_call(
                    run_id="r1", cell_id="c1", model="Kimi", provider="moonshot",
                    transport="opencode", arm="G3", task_id="t", replicate=0,
                    stage="cir", requested_model="kimi-k3",
                    returned_model="kimi-k3", usage_raw=None, started_at=0.0,
                    ended_at=0.1, prompt="p", response="r",
                    transport_attempt=attempt)
            events = read_events(Path(td) / "events.jsonl")
            self.assertEqual([e["transport_attempt"] for e in events], [1, 2])


if __name__ == "__main__":
    unittest.main()
