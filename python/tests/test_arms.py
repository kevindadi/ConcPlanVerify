"""Offline arm orchestration tests (scripted provider + real backend/cargo)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.arms import run_cir_arm, run_rust_arm, rust_oracle
from cir_workflow.concir_client import ConcirClient
from cir_workflow.experiments_v2 import ARM_DIRECT, ARM_OURS_REVISION
from cir_workflow.providers import ScriptedProvider
from tests._helpers import REPO, real_binary

PATTERNS = REPO / "benchmarks/patterns"

GOOD_RUST = """
fn main() { println!("ok"); }
"""


class ArmTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = real_binary()
        if cls.binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)
        cls.contract = json.loads((PATTERNS / "P1/contract.json").read_text(encoding="utf-8"))
        cls.spec = (PATTERNS / "P1/spec.md").read_text(encoding="utf-8")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def test_a3_revision_accepts_fixed_cir(self):
        fixed = (PATTERNS / "P1/fixed.cir.json").read_text(encoding="utf-8")
        provider = ScriptedProvider([{"text": fixed}])
        client = ConcirClient(self.binary, workdir=self.root / "calls", timeout=30.0)
        run = run_cir_arm(client, provider, arm=ARM_OURS_REVISION, task="P1",
                          spec=self.spec, contract=self.contract,
                          out_dir=self.root / "a3")
        self.assertTrue(run.accepted, run.error)
        self.assertEqual(run.accepted_round, 1)
        self.assertEqual(run.notes["revision_status"], "accepted")

    def test_a0_direct_accepts_building_rust(self):
        provider = ScriptedProvider([{"text": GOOD_RUST}])
        run = run_rust_arm(provider, arm=ARM_DIRECT, task="P1", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a0")
        self.assertTrue(run.accepted, run.error)
        self.assertTrue(run.notes["tool_rounds"][0]["build_ok"])

    def test_rust_oracle_does_not_guess_bug_presence(self):
        path = self.root / "prog.rs"
        path.write_text(GOOD_RUST, encoding="utf-8")
        verdict = rust_oracle(path, run_miri=False)
        self.assertTrue(verdict["build_ok"])
        self.assertIsNone(verdict["bug_present"])


if __name__ == "__main__":
    unittest.main()
