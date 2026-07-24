import unittest
from types import SimpleNamespace

from cir_workflow.llm import DeepSeekClient, QwenClient, create_llm_client
from cir_workflow.models import ModelConfig


class FakeChatCompletions:
    def __init__(self):
        self.kwargs = None

    def create(self, **kwargs):
        self.kwargs = kwargs
        return SimpleNamespace(
            choices=[SimpleNamespace(message=SimpleNamespace(content='{"ok":true}'))],
            usage=SimpleNamespace(model_dump=lambda: {"total_tokens": 7}),
        )


class FakeResponses:
    def __init__(self):
        self.kwargs = None

    def create(self, **kwargs):
        self.kwargs = kwargs
        return SimpleNamespace(output_text='{"ok":true}', usage=None)


class FakeSdkClient:
    def __init__(self):
        self.chat = SimpleNamespace(completions=FakeChatCompletions())
        self.responses = FakeResponses()


class LlmClientTests(unittest.TestCase):
    def test_deepseek_uses_chat_completions_and_thinking_options(self):
        sdk = FakeSdkClient()
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
        sdk = FakeSdkClient()
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
                sdk_client=FakeSdkClient(),
            ),
            DeepSeekClient,
        )
        self.assertIsInstance(
            create_llm_client(
                ModelConfig("q", "qwen", "m", "KEY", "https://example"),
                sdk_client=FakeSdkClient(),
            ),
            QwenClient,
        )
        with self.assertRaises(ValueError):
            create_llm_client(
                ModelConfig("x", "openai", "m", "KEY", "https://example"),
                sdk_client=FakeSdkClient(),
            )


if __name__ == "__main__":
    unittest.main()
