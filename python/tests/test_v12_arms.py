"""Fake-transport check of the three-arm entry."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.candidate_eval import evaluate_candidate
from cir_workflow.conditional_arms import run_repairs
from cir_workflow.live import BudgetExhausted
from cir_workflow.send_holding_requirements import evaluate_requirements

ROOT = Path("/Users/kevin/paper-review/papers/ConcPlanVerify")
REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")
DEFECT = (ROOT / "notes/strong-link-v5/reexec-deepseekflash/DeepSeek Flash"
          / "channel__send_while_holding_mutex/rep0/source.rs").read_text()
DERIVED = (ROOT / "notes/strong-link-v11/derived_control.rs").read_text()
CIR = (REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash/DeepSeek Flash"
       / "channel__send_while_holding_mutex/rep0/cir/revision-1.cir.json")
CONTRACT = REPO / "benchmarks/families/channel/send_while_holding_mutex/contract.json"
REQUIREMENTS = (REPO / "benchmarks/families/channel/send_while_holding_mutex/generation_input/REQUIREMENTS.md").read_text()
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")


def _case():
    return {"task": "channel/send_while_holding_mutex", "defect": DEFECT,
            "requirements": REQUIREMENTS, "cir_text": CIR.read_text(encoding="utf-8"),
            "cir_path": str(CIR), "contract_path": str(CONTRACT)}


def _eval(source, case, work):
    return evaluate_candidate(source, Path(case["cir_path"]), Path(case["contract_path"]), Path(work),
                              binary=BIN, instrument=INS, n_runs=1, run_timeout=6, cell_id="v12")


class _Fake:
    def __init__(self, text):
        self.text = text
        self.calls = 0
        self.prompts = []

    def complete(self, system, user):
        self.calls += 1
        self.prompts.append(user)
        return SimpleNamespace(text="```rust\n" + self.text + "\n```", usage=None, wall_ms=1,
                               response_model="deepseek-flash")


class ArmTests(unittest.TestCase):
    def test_three_arms_share_the_defect_and_split_feedback(self):
        case = _case()
        prompts = {}
        for arm in ("A", "B", "C"):
            client = _Fake(DERIVED)
            with tempfile.TemporaryDirectory() as td:
                result = run_repairs(case, arm, client, Path(td), evaluate_candidate=_eval,
                                     score=evaluate_requirements)
            self.assertEqual(result["stop"], "bounded_covered_satisfied")
            self.assertEqual(result["first_requirement_round"], 1)
            self.assertEqual(client.calls, 1)
            self.assertIn(DEFECT.strip(), client.prompts[0])
            prompts[arm] = client.prompts[0]
        self.assertNotIn('"capacity": 0', prompts["A"])
        self.assertIn('"capacity": 0', prompts["B"])
        self.assertIn('"capacity": 0', prompts["C"])
        a_feedback = prompts["A"].split("\nFeedback:\n", 1)[1]
        b_head, b_feedback = prompts["B"].split("\nFeedback:\n", 1)
        c_head, c_feedback = prompts["C"].split("\nFeedback:\n", 1)
        self.assertEqual(a_feedback, b_feedback)
        self.assertEqual(b_head.split("Verified CIR:", 1)[1], c_head.split("Verified CIR:", 1)[1])
        self.assertNotIn("checker_next=", a_feedback)
        self.assertIn("bounded_trace_deviation=", c_feedback)

    def test_unknown_and_budget_are_kept(self):
        case = _case()
        unknown_src = DERIVED.replace("ch1.send(1).unwrap();", "if true { ch1.send(1).unwrap(); }")

        class _Unknown:
            def complete(self, system, user):
                return SimpleNamespace(text="```rust\n" + unknown_src + "\n```", usage=None,
                                       wall_ms=1, response_model="deepseek-flash")

        with tempfile.TemporaryDirectory() as td:
            unknown = run_repairs(case, "A", _Unknown(), Path(td), evaluate_candidate=None,
                                  score=evaluate_requirements)
        self.assertEqual(unknown["stop"], "unknown")

        class _Budget:
            def complete(self, system, user):
                raise BudgetExhausted("request budget reached", physical_attempts=0)

        with tempfile.TemporaryDirectory() as td:
            budget = run_repairs(case, "B", _Budget(), Path(td), evaluate_candidate=None,
                                 score=evaluate_requirements)
        self.assertEqual(budget["stop"], "global_request_budget_exhausted")
        self.assertEqual(budget["send_status"], "not_sent")


if __name__ == "__main__":
    unittest.main()
