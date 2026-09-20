"""Three-way A0/A1 reply classification and claims_no_issue semantics (§1)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.arms import classify_reply, run_rust_arm
from cir_workflow.experiments_v2 import ARM_DIRECT, ARM_SELF_ITER
from cir_workflow.providers import ScriptedProvider
from tests._helpers import REPO, real_binary

GOOD_RUST = "```rust\nfn main() { println!(\"ok\"); }\n```"


class ClassifyTests(unittest.TestCase):
    def test_sentinel(self):
        self.assertEqual(classify_reply("NO_ISSUES")["kind"], "claims_no_issue")
        self.assertEqual(classify_reply("  no_issues \n")["kind"], "claims_no_issue")

    def test_prose_claim(self):
        text = ("Looking at this program, both threads acquire locks in the same "
                "order, so there is no deadlock.")
        result = classify_reply(text)
        self.assertEqual(result["kind"], "claims_no_issue")
        self.assertTrue(result["matched"])

    def test_program_fenced(self):
        self.assertEqual(classify_reply(GOOD_RUST)["kind"], "program")

    def test_program_raw(self):
        result = classify_reply("fn main() { println!(\"x\"); }\n")
        self.assertEqual(result["kind"], "program")

    def test_program_plus_prose(self):
        text = "Here is the fix:\n```rust\nfn main() {}\n```\nThis addresses it."
        self.assertEqual(classify_reply(text)["kind"], "program")

    def test_empty_is_other(self):
        self.assertEqual(classify_reply("")["kind"], "other")
        self.assertEqual(classify_reply("I am not sure what to do.")["kind"], "other")

    def test_prose_claim_without_wordlist(self):
        # "no concurrency defect" matches; a neutral sentence does not.
        self.assertEqual(classify_reply("There is no bug here.")["kind"],
                         "claims_no_issue")
        self.assertEqual(classify_reply("The program has two threads.")["kind"],
                         "other")


class ClaimsNoIssueSemanticsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if real_binary() is None:
            raise unittest.SkipTest("concir-backend not found (set CONCIR_BACKEND)")
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)
        cls.contract = json.loads((REPO / "benchmarks/legacy-paper-patterns/"
                                   "P1/contract.json").read_text(encoding="utf-8"))
        cls.spec = (REPO / "benchmarks/legacy-paper-patterns/P1/spec.md").read_text(
            encoding="utf-8")

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "_tmp"):
            cls._tmp.cleanup()

    def test_a0_claims_no_issue_accepts_input(self):
        provider = ScriptedProvider([{"text": "NO_ISSUES"}])
        run = run_rust_arm(provider, arm=ARM_DIRECT, task="t", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a0",
                           initial_source="fn main() { let _ = 1; }\n")
        self.assertTrue(run.accepted)
        self.assertEqual(run.rounds[-1].decision, "claims_no_issue")
        self.assertEqual(run.notes.get("accepted_artifact"), "input")
        self.assertEqual(run.notes.get("bug_present_reason"), "by_construction")
        self.assertTrue(run.final_artifact_path.endswith("initial.rs"))

    def test_a1_claims_no_issue_unbuilt_not_accepted(self):
        provider = ScriptedProvider([{"text": "There is no bug in this program."}])
        run = run_rust_arm(provider, arm=ARM_SELF_ITER, task="t", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a1u",
                           initial_source="fn main() {}\n")
        self.assertFalse(run.accepted)
        self.assertEqual(run.rounds[-1].decision, "claims_no_issue_unbuilt")

    def test_a1_accepts_most_recent_build_ok_candidate(self):
        provider = ScriptedProvider([{"text": GOOD_RUST},
                                     {"text": "No issues found."}])
        run = run_rust_arm(provider, arm=ARM_SELF_ITER, task="t", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "a1b",
                           initial_source="fn main() {}\n")
        self.assertTrue(run.accepted)
        self.assertEqual(run.accepted_round, 1)
        self.assertEqual(run.rounds[-1].decision, "claims_no_issue")
        self.assertTrue(run.final_artifact_path.endswith("round-1/candidate.rs"))

    def test_format_retry_then_program(self):
        provider = ScriptedProvider([{"text": "I cannot decide."},
                                     {"text": GOOD_RUST}])
        run = run_rust_arm(provider, arm=ARM_DIRECT, task="t", spec=self.spec,
                           contract=self.contract, out_dir=self.root / "retry",
                           initial_source="fn main() {}\n")
        self.assertTrue(run.accepted)
        self.assertEqual(run.rounds[-1].decision, "build_ok")
        # two requests consumed for one round: an initial reply and one retry
        self.assertEqual(len(provider.calls), 2)


if __name__ == "__main__":
    unittest.main()
