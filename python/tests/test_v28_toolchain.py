"""v28 toolchain: no silent fallback to target/release; explicit config only."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from cir_workflow import toolchain


class ToolchainTests(unittest.TestCase):
    def test_resolve_refuses_without_config(self):
        saved = {k: os.environ.pop(k, None) for k in
                 ("CONCIR_TOOLCHAIN", "CONCIR_BACKEND", "CONCIR_INSTRUMENT", "CONCIR_BIND_CHECK")}
        try:
            with self.assertRaises(toolchain.ToolchainError):
                toolchain.resolve()
        finally:
            for k, v in saved.items():
                if v is not None:
                    os.environ[k] = v

    def test_explicit_dir_resolves_and_is_not_legacy(self):
        d = Path("/Users/kevin/local-repos/ConcIR/target/v28-verified/release")
        if not (d / "concir-backend").is_file():
            self.skipTest("v28 toolchain not built")
        tc = toolchain.resolve(d)
        self.assertFalse(tc.is_legacy_release)
        self.assertEqual(tc.backend.parent, d)

    def test_env_dir_is_honored(self):
        d = Path("/Users/kevin/local-repos/ConcIR/target/v28-verified/release")
        if not (d / "concir-backend").is_file():
            self.skipTest("v28 toolchain not built")
        os.environ["CONCIR_TOOLCHAIN"] = str(d)
        try:
            self.assertEqual(toolchain.resolve().backend.parent, d)
        finally:
            os.environ.pop("CONCIR_TOOLCHAIN", None)

    def test_legacy_is_marked(self):
        self.assertTrue(toolchain.legacy_toolchain().is_legacy_release)


if __name__ == "__main__":
    unittest.main()
