"""Consolidated tests for core utilities (env, json_utils, llm, prompts) and CIR normalization."""

from __future__ import annotations

import copy
import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from concplan.core.concir_client import ConcirClient
from concplan.core.env import load_dotenv
from concplan.core.json_utils import extract_json, sha256_bytes, sha256_file, sha256_text
from concplan.core.llm import DeepSeekClient, QwenClient, _resolve_api_key, create_llm_client
from concplan.core.models import Consumption, ModelConfig, normalize_token_usage
from concplan.core.prompts import (
    ACTIVE_PROMPT_ASSETS,
    hole_repair_system_prompt,
    local_revision_system_prompt,
    patch_system_prompt,
    prompt_asset_record,
    scir_generation_system_prompt,
)
from concplan.model_verify.normalize import SID_RE, normalize
from tests._helpers import REPO, real_binary


class DotenvTests(unittest.TestCase):
    def test_does_not_override_existing_environment(self):
        key = "CIR_WORKFLOW_TEST_KEY"
        old = os.environ.get(key)
        os.environ[key] = "existing"
        try:
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / ".env"
                path.write_text(f'{key}="from-file"\nNEW_CIR_WORKFLOW_KEY=loaded\n', encoding="utf-8")
                load_dotenv(path)
            self.assertEqual(os.environ[key], "existing")
            self.assertEqual(os.environ["NEW_CIR_WORKFLOW_KEY"], "loaded")
        finally:
            if old is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = old
            os.environ.pop("NEW_CIR_WORKFLOW_KEY", None)


class JsonAndHashUtilsTests(unittest.TestCase):
    def test_plain_object_with_trailing_text(self):
        self.assertEqual(extract_json('{"program": "demo"}\nDone.'), '{"program": "demo"}')

    def test_markdown_fence(self):
        self.assertEqual(
            extract_json('```json\n{"program": "demo"}\n```'),
            '{"program": "demo"}',
        )

    def test_thinking_block(self):
        self.assertEqual(
            extract_json('<think>reasoning</think>\n{"program":"demo"}'),
            '{"program":"demo"}',
        )

    def test_invalid_text_is_preserved_for_repair_feedback(self):
        text = "I cannot produce JSON"
        self.assertEqual(extract_json(text), text)

    def test_hashing_helpers_consistent(self):
        data = b"concir-v4"
        self.assertEqual(sha256_text("concir-v4"), sha256_bytes(data))
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "sample.bin"
            p.write_bytes(data)
            self.assertEqual(sha256_file(p), sha256_bytes(data))


class PromptAssetTests(unittest.TestCase):
    def test_all_four_active_prompts_load_and_hash(self):
        record = prompt_asset_record()
        self.assertEqual(set(record.keys()), set(ACTIVE_PROMPT_ASSETS))
        self.assertEqual(len(record), 4)
        for sha in record.values():
            self.assertEqual(len(sha), 64)

        self.assertIn("lock_guard", scir_generation_system_prompt())
        self.assertIn("seq_hole", scir_generation_system_prompt())
        self.assertIn("escalate", hole_repair_system_prompt())
        self.assertIn("swap_statements", patch_system_prompt())
        self.assertIn("new_resources", local_revision_system_prompt())


class _FakeChatCompletions:
    def __init__(self):
        self.kwargs = None

    def create(self, **kwargs):
        self.kwargs = kwargs
        return SimpleNamespace(
            choices=[SimpleNamespace(message=SimpleNamespace(content='{"ok":true}'))],
            usage=SimpleNamespace(model_dump=lambda: {"total_tokens": 7}),
        )


class _FakeResponses:
    def __init__(self):
        self.kwargs = None

    def create(self, **kwargs):
        self.kwargs = kwargs
        return SimpleNamespace(output_text='{"ok":true}', usage=None)


class _FakeSdkClient:
    def __init__(self):
        self.chat = SimpleNamespace(completions=_FakeChatCompletions())
        self.responses = _FakeResponses()


class LlmClientAndModelsTests(unittest.TestCase):
    def test_deepseek_uses_chat_completions_and_thinking_options(self):
        sdk = _FakeSdkClient()
        model = ModelConfig(
            name="deepseek",
            provider="deepseek",
            model_id="deepseek-v4-pro",
            api_key_env="DEEPSEEK_API_KEY",
            base_url="https://api.deepseek.com",
            reasoning_effort="high",
            thinking_enabled=True,
        )
        content, usage = DeepSeekClient(sdk_client=sdk, model=model).chat("system", "user")
        self.assertEqual(content, '{"ok":true}')
        self.assertEqual(usage["total_tokens"], 7)
        self.assertEqual(sdk.chat.completions.kwargs["model"], "deepseek-v4-pro")
        self.assertEqual(sdk.chat.completions.kwargs["reasoning_effort"], "high")
        self.assertEqual(
            sdk.chat.completions.kwargs["extra_body"],
            {"thinking": {"type": "enabled"}},
        )

    def test_qwen_uses_responses_with_string_input(self):
        sdk = _FakeSdkClient()
        model = ModelConfig(
            name="qwen",
            provider="qwen",
            model_id="qwen3.7-plus",
            api_key_env="DASHSCOPE_API_KEY",
            base_url="https://workspace.example/v1",
        )
        content, _ = QwenClient(sdk_client=sdk, model=model).chat("system", "user")
        self.assertEqual(content, '{"ok":true}')
        self.assertEqual(sdk.responses.kwargs["model"], "qwen3.7-plus")
        self.assertIn("[SYSTEM INSTRUCTIONS]", sdk.responses.kwargs["input"])
        self.assertIn("[USER REQUEST]", sdk.responses.kwargs["input"])
        self.assertNotIn("temperature", sdk.responses.kwargs)

    def test_provider_factory_accepts_only_supported_clients(self):
        self.assertIsInstance(
            create_llm_client(
                ModelConfig("d", "deepseek", "m", "KEY", "https://example"),
                sdk_client=_FakeSdkClient(),
            ),
            DeepSeekClient,
        )
        self.assertIsInstance(
            create_llm_client(
                ModelConfig("q", "qwen", "m", "KEY", "https://example"),
                sdk_client=_FakeSdkClient(),
            ),
            QwenClient,
        )
        with self.assertRaises(ValueError):
            create_llm_client(
                ModelConfig("x", "openai", "m", "KEY", "https://example"),
                sdk_client=_FakeSdkClient(),
            )

    def test_deepseek_apikey_alias_fallback(self):
        old_std = os.environ.pop("DEEPSEEK_API_KEY", None)
        old_alias = os.environ.get("DEEPSEEK_APIKEY")
        os.environ["DEEPSEEK_APIKEY"] = "alias-secret-token"
        try:
            self.assertEqual(_resolve_api_key("DEEPSEEK_API_KEY"), "alias-secret-token")
            os.environ["DEEPSEEK_API_KEY"] = "primary-secret-token"
            self.assertEqual(_resolve_api_key("DEEPSEEK_API_KEY"), "primary-secret-token")
        finally:
            os.environ.pop("DEEPSEEK_API_KEY", None)
            os.environ.pop("DEEPSEEK_APIKEY", None)
            if old_std is not None:
                os.environ["DEEPSEEK_API_KEY"] = old_std
            if old_alias is not None:
                os.environ["DEEPSEEK_APIKEY"] = old_alias

    def test_consumption_and_token_normalization(self):
        self.assertEqual(normalize_token_usage({"prompt_tokens": 10, "completion_tokens": 5}), (10, 5))
        self.assertEqual(normalize_token_usage({"input_tokens": 4, "output_tokens": 6}), (4, 6))
        c = Consumption()
        c.add_usage({"prompt_tokens": 10, "completion_tokens": 5})
        self.assertEqual(c.total_tokens, 15)
        c.add_usage(None)
        self.assertIsNone(c.total_tokens)


def _bare_wait() -> dict:
    return json.loads(
        (REPO / "tests/fixtures/lost_wakeup/buggy.cir.json")
        .read_text(encoding="utf-8")
    )


class NormalizeTests(unittest.TestCase):
    def test_expr_object_to_string(self):
        program = _bare_wait()
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared" and "expr" in stmt:
                    stmt["expr"] = {"kind": "bool", "value": True}
        out, records, _ = normalize(program)
        exprs = [
            s["expr"]
            for m in out["modules"]
            for f in m["functions"]
            for s in f.get("body", [])
            if s.get("kind") == "write_shared"
        ]
        self.assertIn("true", exprs)
        self.assertTrue(any(r["rule"] == "expr_object_to_string" for r in records))

    def test_field_alias_var_to_resource(self):
        program = _bare_wait()
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared" and "resource" in stmt:
                    stmt["var"] = stmt.pop("resource")
        out, records, _ = normalize(program)
        for fn in out["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared":
                    self.assertIn("resource", stmt)
                    self.assertNotIn("var", stmt)
        self.assertTrue(any(r["rule"] == "field_alias" for r in records))

    def test_missing_base_inferred(self):
        program = _bare_wait()
        for res in program["modules"][0]["resources"]:
            if res.get("type") == "Var":
                res.pop("base", None)
                res["init"] = True
        out, records, _ = normalize(program)
        var = [r for r in out["modules"][0]["resources"] if r.get("type") == "Var"][0]
        self.assertEqual(var["base"], "Bool")
        self.assertTrue(any(r["rule"] == "infer_base" for r in records))

    def test_bad_sid_is_renamed_and_recorded(self):
        program = _bare_wait()
        stmt = program["modules"][0]["functions"][-1]["body"][0]
        stmt["sid"] = "b6"
        out, records, issues = normalize(program)
        self.assertNotEqual(out["modules"][0]["functions"][-1]["body"][0]["sid"], "b6")
        self.assertEqual(issues, [])
        self.assertTrue(any(r["rule"] == "sid_rename" and r.get("from") == "b6" for r in records))

    def test_missing_sid_filled_and_goto_rewritten(self):
        program = _bare_wait()
        fn = program["modules"][0]["functions"][-1]
        fn["body"] = [
            {"sid": "b6", "kind": "goto", "target": "b6"},
            {"kind": "return"},
        ]
        out, _records, issues = normalize(program)
        body = out["modules"][0]["functions"][-1]["body"]
        self.assertTrue(all(SID_RE.match(s["sid"]) for s in body))
        self.assertIn(body[0]["target"], [s["sid"] for s in body])
        self.assertEqual(issues, [])

    def test_normalized_program_is_statically_valid(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend binary not found")
        program = _bare_wait()
        for res in program["modules"][0]["resources"]:
            if res.get("type") == "Var":
                res.pop("base", None)
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared":
                    stmt["var"] = stmt.pop("resource")
                    stmt["expr"] = {"kind": "bool", "value": True}
        out, records, _ = normalize(copy.deepcopy(program))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "normalized.json"
            path.write_text(json.dumps(out), encoding="utf-8")
            client = ConcirClient(binary, workdir=Path(tmp) / "calls", timeout=30.0)
            result = client.check(path)
            self.assertEqual(result.status, "valid", result.payload)
        self.assertTrue(records)


if __name__ == "__main__":
    unittest.main()
