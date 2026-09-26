"""Transport registry: discovery, blocked entries, identity enforcement."""

from __future__ import annotations

import unittest

from cir_workflow import transport


class RegistryTests(unittest.TestCase):
    def test_deepseek_is_main_and_discovered(self):
        specs = transport.build_registry()
        ds = transport.resolve_model(specs, "DeepSeek Flash")
        self.assertEqual(ds.model_id, "deepseek-flash")
        self.assertEqual(ds.channel, "deepseek-direct")
        self.assertEqual(ds.status, "available")
        self.assertTrue(ds.discovered)

    def test_qwen_and_composer_are_blocked_with_reasons(self):
        blocked = {s.display_name: s for s in transport.blocked_models()}
        self.assertIn("Qwen", blocked)
        self.assertIn("Composer 2.5", blocked)
        self.assertIn("401", blocked["Qwen"].blocked_reason)
        self.assertIn("cursor_sdk", blocked["Composer 2.5"].blocked_reason)

    def test_blocked_models_have_no_available_id(self):
        for spec in transport.blocked_models():
            self.assertFalse(spec.model_id and spec.status == "available")

    def test_opencode_models_are_discovered(self):
        specs = transport.build_registry()
        for display in ("Kimi", "GLM", "GPT 6 Luna", "Grok 4.7"):
            spec = transport.resolve_model(specs, display)
            self.assertEqual(spec.channel, "opencode-go")
            self.assertTrue(spec.discovered, display)
            self.assertIn(spec.model_id, transport.DISCOVERED_MODELS["opencode-go"])

    def test_no_model_routes_deepseek_or_qwen_through_opencode(self):
        for spec in transport.build_registry():
            if spec.provider in {"deepseek", "qwen"}:
                self.assertNotEqual(spec.channel, "opencode-go")


class IdentityTests(unittest.TestCase):
    def test_exact_match_confirms(self):
        self.assertTrue(transport.verify_identity("kimi-k3", "kimi-k3"))

    def test_missing_model_is_unconfirmed(self):
        self.assertFalse(transport.verify_identity("kimi-k3", None))

    def test_mismatch_raises_and_does_not_fall_back(self):
        with self.assertRaises(transport.ModelIdentityError):
            transport.verify_identity("kimi-k3", "glm-5.3")


if __name__ == "__main__":
    unittest.main()
