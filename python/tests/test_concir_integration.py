"""Integration tests against the real ``concir-backend`` binary."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient
from tests._helpers import (
    FIXTURES,
    broken_program,
    fixture_text,
    real_binary,
)


class ConcirIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = real_binary()
        if cls.binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.work = Path(cls._tmp.name)

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def client(self):
        return ConcirClient(self.binary, workdir=self.work / "calls", timeout=30.0)

    def test_check_valid_and_invalid(self):
        c = self.client()
        r = c.check(FIXTURES / "already_correct.json")
        self.assertEqual((r.kind, r.status), ("semantic", "valid"))
        broken = self.work / "broken.json"
        broken.write_text(json.dumps(broken_program()))
        r = c.check(broken)
        self.assertEqual((r.kind, r.status), ("semantic", "invalid"))
        self.assertTrue(r.payload.get("diagnostics"))

    def test_support_supported_and_unsupported(self):
        c = self.client()
        r = c.support(FIXTURES / "single_cycle.json")
        self.assertEqual((r.kind, r.status), ("semantic", "supported"))
        r = c.support(FIXTURES / "rwlock_unsupported.json")
        self.assertEqual((r.kind, r.status), ("semantic", "unsupported"))

    def test_explore_all_outcomes(self):
        c = self.client()
        r = c.explore(FIXTURES / "single_cycle.json", FIXTURES / "single_cycle_contract.json")
        self.assertEqual((r.kind, r.status), ("semantic", "fail"))
        self.assertTrue(r.complete)
        r = c.explore(FIXTURES / "already_correct.json", FIXTURES / "already_correct_contract.json")
        self.assertEqual((r.kind, r.status), ("semantic", "pass"))
        r = c.explore(FIXTURES / "finite_call_loop.json", FIXTURES / "tiny_bounds_contract.json")
        self.assertEqual((r.kind, r.status), ("semantic", "unknown"))
        r = c.explore(FIXTURES / "rwlock_unsupported.json", FIXTURES / "rwlock_unsupported_contract.json")
        self.assertEqual((r.kind, r.status), ("semantic", "unsupported"))

    def test_repair_and_replay_round_trip(self):
        c = self.client()
        r = c.repair(FIXTURES / "single_cycle.json", FIXTURES / "single_cycle_contract.json",
                     strategy="c")
        self.assertEqual((r.kind, r.status), ("semantic", "repaired"))
        self.assertIsNotNone(r.artifact_path)
        artifact = json.loads(Path(r.artifact_path).read_text())
        self.assertEqual(artifact["schema_version"], "concir-repair-artifact-v1")
        rp = c.replay(r.artifact_path)
        self.assertEqual((rp.kind, rp.status), ("semantic", "replayed"))

    def test_replay_detects_tampering(self):
        c = self.client()
        r = c.repair(FIXTURES / "single_cycle.json", FIXTURES / "single_cycle_contract.json")
        artifact = json.loads(Path(r.artifact_path).read_text())
        artifact["stop_reason"] = "tampered"
        bad = self.work / "tampered_artifact.json"
        bad.write_text(json.dumps(artifact))
        rp = c.replay(bad)
        self.assertEqual(rp.kind, "semantic")
        self.assertEqual(rp.status, "replay_failed")


if __name__ == "__main__":
    unittest.main()
