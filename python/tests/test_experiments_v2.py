"""Unit tests for the v2 accounting/oracle types (no network, no backend)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow import experiments_v2 as v2


class ConsumptionTests(unittest.TestCase):
    def test_missing_usage_is_unknown_not_zero(self):
        c = v2.Consumption()
        c.add_usage(None)
        self.assertIsNone(c.prompt_tokens)
        self.assertIsNone(c.completion_tokens)
        self.assertIsNone(c.total_tokens)
        c.add_usage({"prompt_tokens": 10, "completion_tokens": 5})
        self.assertIsNone(c.prompt_tokens)  # stays unknown once seen as unknown
        self.assertIsNone(c.total_tokens)

    def test_usage_accumulates(self):
        c = v2.Consumption()
        c.add_usage({"prompt_tokens": 10, "completion_tokens": 5})
        c.add_usage({"input_tokens": 3, "output_tokens": 2})
        self.assertEqual(c.prompt_tokens, 13)
        self.assertEqual(c.completion_tokens, 7)
        self.assertEqual(c.total_tokens, 20)


class DerivedFlagTests(unittest.TestCase):
    def test_false_accept_and_conservative_reject(self):
        flags = v2.derived_flags(arm_accepted=True, oracle_bug_present=True,
                                 behavior_test_ok=True)
        self.assertTrue(flags["false_accept"])
        self.assertFalse(flags["behavior_loss"])
        flags = v2.derived_flags(arm_accepted=False, oracle_bug_present=False,
                                 behavior_test_ok=None)
        self.assertTrue(flags["conservative_reject"])
        self.assertIsNone(flags["behavior_loss"])

    def test_behavior_loss(self):
        flags = v2.derived_flags(arm_accepted=True, oracle_bug_present=False,
                                 behavior_test_ok=False)
        self.assertTrue(flags["behavior_loss"])
        self.assertFalse(flags["false_accept"])


class ManifestTests(unittest.TestCase):
    def _manifest(self, root: Path, payload: dict) -> Path:
        task_dir = root / "P1"
        task_dir.mkdir(parents=True)
        (task_dir / "a.json").write_text(json.dumps(payload), encoding="utf-8")
        digest = v2.sha256_file(task_dir / "a.json")
        manifest = {
            "tasks": [{
                "id": "P1", "status": "ready", "directory": "P1",
                "spec": "P1/a.json", "ground_truth": {"bug": "deadlock"},
                "files": {"P1/a.json": digest},
            }],
        }
        path = root / "MANIFEST.json"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        return path

    def test_load_and_verify(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = self._manifest(root, {"program": "x"})
            tasks = v2.load_manifest(path)
            self.assertEqual([t.id for t in tasks], ["P1"])
            self.assertEqual(tasks[0].ground_truth["bug"], "deadlock")

    def test_hash_mismatch_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = self._manifest(root, {"program": "x"})
            (root / "P1" / "a.json").write_text(json.dumps({"program": "y"}),
                                                encoding="utf-8")
            with self.assertRaises(ValueError):
                v2.load_manifest(path)


if __name__ == "__main__":
    unittest.main()
