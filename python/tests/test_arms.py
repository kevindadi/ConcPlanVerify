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
from tests._helpers import REPO, broken_program, real_binary

PATTERNS = REPO / "benchmarks/legacy-paper-patterns"

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

    def test_a3_round_records_are_populated(self):
        buggy = (PATTERNS / "P1/buggy.cir.json").read_text(encoding="utf-8")
        broken = json.dumps(broken_program())
        fixed = (PATTERNS / "P1/fixed.cir.json").read_text(encoding="utf-8")
        provider = ScriptedProvider([{"text": buggy}, {"text": broken}, {"text": fixed}])
        client = ConcirClient(self.binary, workdir=self.root / "calls2", timeout=30.0)
        run = run_cir_arm(client, provider, arm=ARM_OURS_REVISION, task="P1",
                          spec=self.spec, contract=self.contract,
                          out_dir=self.root / "a3rounds")
        self.assertTrue(run.accepted, run.error)
        self.assertEqual(run.accepted_round, 3)
        self.assertEqual(len(run.rounds), 3)
        self.assertEqual([r.decision for r in run.rounds],
                         ["explore_fail", "check_invalid", "accepted"])
        for record in run.rounds:
            self.assertIsNotNone(record.request_sha256)
            self.assertIsNotNone(record.tool_output_sha256)

    def test_a1_no_issues_accepts_without_compiling_sentinel(self):
        provider = ScriptedProvider([{"text": GOOD_RUST}, {"text": "NO_ISSUES"}])
        run = run_rust_arm(provider, arm="A1_self_iter", task="P1", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a1", k=4)
        self.assertTrue(run.accepted)
        self.assertEqual(run.accepted_round, 2)
        self.assertEqual(len(run.rounds), 2)
        self.assertEqual(run.rounds[1].decision, "self_no_issues")
        self.assertTrue(run.notes.get("self_no_issues"))

    def test_a1_fragment_is_format_error(self):
        fragment = "let _g1 = a.lock().unwrap(); let _g2 = b.lock().unwrap();"
        provider = ScriptedProvider([{"text": fragment}, {"text": GOOD_RUST}])
        run = run_rust_arm(provider, arm="A1_self_iter", task="P1", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a1f", k=2)
        self.assertEqual(run.rounds[0].decision, "format_error")
        self.assertIsNotNone(run.rounds[0].feedback_sha256)

    def test_a2_accepts_when_tools_are_clean(self):
        fixed_rs = (PATTERNS / "P1/fixed.rs").read_text(encoding="utf-8")
        provider = ScriptedProvider([{"text": fixed_rs}])
        run = run_rust_arm(provider, arm="A2_tools_iter", task="P1", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a2")
        self.assertTrue(run.accepted, run.error)
        self.assertEqual(run.accepted_round, 1)
        self.assertEqual(run.rounds[0].decision, "tools_green_ml")

    def test_a0_direct_accepts_building_rust(self):
        provider = ScriptedProvider([{"text": GOOD_RUST}])
        run = run_rust_arm(provider, arm=ARM_DIRECT, task="P1", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a0")
        self.assertTrue(run.accepted, run.error)
        self.assertTrue(run.notes["tool_rounds"][0]["build_ok"])

    def test_rust_oracle_uses_behavior_for_bug_presence(self):
        path = self.root / "prog.rs"
        path.write_text(GOOD_RUST, encoding="utf-8")
        verdict = rust_oracle(path, run_miri=False)
        self.assertTrue(verdict["build_ok"])
        self.assertEqual(verdict["behavior_status"], "terminated_ok")
        self.assertFalse(verdict["bug_present"])
        # A program that cannot terminate is a hang and still has the defect.
        hang = self.root / "hang.rs"
        hang.write_text("fn main() { loop {} }", encoding="utf-8")
        bad = rust_oracle(hang, run_miri=False)
        self.assertEqual(bad["behavior_status"], "hang")
        self.assertTrue(bad["bug_present"])


if __name__ == "__main__":
    unittest.main()
