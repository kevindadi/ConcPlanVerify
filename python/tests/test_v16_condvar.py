"""Condvar finite-coverage scorer and task dispatch. No model requests."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.condvar_requirements import evaluate_condvar, structural_checks
from cir_workflow.conditional_arms import run_repairs
from cir_workflow.generation import _rewrite_traces
from cir_workflow.send_holding_requirements import evaluate_requirements
from cir_workflow.task_score import score_for_task

ROOT = Path("/Users/kevin/paper-review/papers/ConcPlanVerify")
HUMAN = (ROOT / "notes/strong-link-v15/human_derived_control.rs").read_text()
ORIGINAL = (ROOT / "notes/strong-link-v5/reexec-gpt6luna/GPT 6 Luna"
            / "condvar__notify_one_multi_waiter_wrong_pick/rep0/source.rs").read_text()
CHANNEL = (ROOT / "notes/strong-link-v11/derived_control.rs").read_text()
CONDVAR_REQUIREMENTS = (Path("/Users/kevin/local-repos/ConcPlanVerify")
                        / "benchmarks/families/condvar/notify_one_multi_waiter_wrong_pick/generation_input/REQUIREMENTS.md").read_text()


def _order_negative() -> str:
    return HUMAN.replace(
        """    g12.acquire_count(2).unwrap();
    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);""",
        """    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);
    g12.acquire_count(2).unwrap();""",
    )


def _alternate() -> str:
    return HUMAN.replace(
        "g12.acquire_count(2).unwrap();",
        "g12.acquire_count(1).unwrap();\n    g12.acquire_count(1).unwrap();",
    )


class CondvarScoreTests(unittest.TestCase):
    def test_original_fails_readiness_and_control_shape_passes(self):
        self.assertEqual(structural_checks(ORIGINAL)["R6"]["status"], "fail")
        self.assertEqual(structural_checks(HUMAN)["R6"]["status"], "pass")
        self.assertEqual(structural_checks(_order_negative())["R6"]["status"], "fail")
        self.assertEqual(structural_checks(_alternate())["R6"]["status"], "pass")
        unknown = HUMAN.replace("let g12 = Semaphore::new(0);", "let n = 0;\n    let g12 = Semaphore::new(n);")
        self.assertEqual(structural_checks(unknown)["R6"]["status"], "unknown")
        renamed = HUMAN.replace("proceed", "flag")
        self.assertEqual(structural_checks(renamed)["status"] if "status" in structural_checks(renamed) else structural_checks(renamed)["R6"]["status"], "pass")

    def test_three_reproduced_negatives_do_not_pass(self):
        raii = HUMAN.replace(
            "g12.acquire_count(2).unwrap();",
            "let permit = g12.acquire();\n    permit.release();\n"
            "    let permit = g12.acquire();\n    permit.release();")
        supplied = HUMAN.replace(
            "g12.acquire_count(2).unwrap();",
            "g12.release_count(2).unwrap();\n    g12.acquire_count(2).unwrap();")
        unreachable = HUMAN.replace("while !*proceed {", "while false {")
        for name, source in (("raii", raii), ("supplied", supplied), ("unreachable", unreachable)):
            checks = structural_checks(source)
            self.assertNotEqual(checks["R6"]["status"], "pass", name)
            self.assertIn(checks["R6"]["status"], {"fail", "unknown"}, name)
        self.assertEqual(structural_checks(unreachable)["R2"]["status"], "fail")
        self.assertEqual(structural_checks(unreachable)["R4"]["status"], "fail")
        early = HUMAN.replace(
            "    g12.release_count(1).unwrap();\n",
            "    g12.release_count(1).unwrap();\n    gN.release_count(1).unwrap();\n    return;\n")
        early_checks = structural_checks(early)
        self.assertEqual(early_checks["R2"]["status"], "fail")
        self.assertEqual(early_checks["R4"]["status"], "fail")
        self.assertEqual(early_checks["R6"]["status"], "fail")
        constant = HUMAN.replace(
            "    while !*proceed {",
            "    *proceed = true;\n    while !*proceed {")
        self.assertEqual(structural_checks(constant)["R2"]["status"], "fail")

    def test_finite_runs_keep_pass_fail_and_unknown_apart(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            human = evaluate_condvar(HUMAN, root / "human", repeats=1)
            original = evaluate_condvar(ORIGINAL, root / "original", repeats=1)
            alternate = evaluate_condvar(_alternate(), root / "alternate", repeats=1)
        self.assertEqual(human["status"], "bounded_covered_satisfied")
        self.assertIn("R8", " ".join(human["uncovered"]))
        self.assertIn("R9", " ".join(human["uncovered"]))
        self.assertEqual(original["status"], "fail")
        self.assertEqual(original["checks"]["R6"]["status"], "fail")
        self.assertEqual(alternate["status"], "bounded_covered_satisfied")
        self.assertNotEqual(original["status"], "bounded_covered_satisfied")

    def test_task_dispatch_does_not_cross_scorers(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            channel = score_for_task("channel/send_while_holding_mutex", CHANNEL, root / "ch")
            condvar = score_for_task("condvar/notify_one_multi_waiter_wrong_pick", HUMAN, root / "cv")
            missing = score_for_task("other/task", HUMAN, root / "missing")
        self.assertEqual(channel["status"], "bounded_covered_satisfied")
        self.assertEqual(channel["task_id"], "channel/send_while_holding_mutex")
        self.assertEqual(condvar["status"], "bounded_covered_satisfied")
        self.assertEqual(condvar["task_id"], "condvar/notify_one_multi_waiter_wrong_pick")
        self.assertEqual(missing["status"], "unknown")
        crossed = structural_checks(CHANNEL)
        self.assertNotEqual(crossed["R1"]["status"], "pass")

    def test_projection_keeps_semaphore_count(self):
        with tempfile.TemporaryDirectory() as td:
            src = Path(td) / "in"
            dst = Path(td) / "out"
            src.mkdir()
            (src / "t.jsonl").write_text(
                '{"t":"t0","op":"sem_acquire","r":"g12","n":2,"api":"count"}\n', encoding="utf-8")
            _rewrite_traces(src, dst, {"g12": "main::g12"}, set())
            event = json.loads((dst / "t.jsonl").read_text(encoding="utf-8"))
        self.assertEqual(event["r"], "main::g12")
        self.assertEqual(event["n"], 2)
        self.assertEqual(event["api"], "count")

    def test_fake_transport_uses_the_same_arm_protocol(self):
        channel_case = {"task": "channel/send_while_holding_mutex", "defect": "fn main() {}\n",
                        "requirements": "channel requirements", "cir_text": "{\"capacity\": 0}"}
        condvar_case = {"task": "condvar/notify_one_multi_waiter_wrong_pick", "defect": "fn main() {}\n",
                        "requirements": CONDVAR_REQUIREMENTS, "cir_text": "{\"count\": 0}"}

        class _Fake:
            def __init__(self, text):
                self.text = text
                self.prompts = []

            def complete(self, system, user):
                self.prompts.append(user)
                return SimpleNamespace(text="```rust\n" + self.text + "\n```", usage=None, wall_ms=1,
                                       response_model="deepseek-flash")

        def stub(_source, _case, _work):
            return {"ledger": {"layers": {}, "trace": {"state": "observed_violation",
                    "violations": [{"event_index": 0, "got": "sem_acquire:main::g12",
                                    "resource": "main::g12", "expected": ["sem_release:main::g12"]}]}},
                    "binding": {"attributes": [], "uncovered_sync": []}}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            channel = _Fake(CHANNEL)
            run_repairs(channel_case, "A", channel, root / "ch-a", evaluate_candidate=None,
                        score=evaluate_requirements)
            condvar_a = _Fake(HUMAN)
            run_repairs(condvar_case, "A", condvar_a, root / "cv-a", evaluate_candidate=None,
                        score=evaluate_condvar)
            condvar_c = _Fake(HUMAN)
            run_repairs(condvar_case, "C", condvar_c, root / "cv-c", evaluate_candidate=stub,
                        score=evaluate_condvar)
        for prompt in (channel.prompts[0], condvar_a.prompts[0], condvar_c.prompts[0]):
            self.assertIn("\nRequirements:\n", prompt)
            self.assertIn("\nPrevious Rust program:\n", prompt)
            self.assertIn("\nFeedback:\n", prompt)
        self.assertNotIn("Verified CIR:", channel.prompts[0])
        self.assertNotIn("Verified CIR:", condvar_a.prompts[0])
        self.assertIn("Verified CIR:", condvar_c.prompts[0])
        a_feedback = condvar_a.prompts[0].split("\nFeedback:\n", 1)[1]
        c_feedback = condvar_c.prompts[0].split("\nFeedback:\n", 1)[1]
        self.assertNotIn("checker_next=", a_feedback)
        self.assertNotIn("bounded_trace_deviation=", a_feedback)
        self.assertIn("bounded_trace_deviation=", c_feedback)
        self.assertNotIn("DONE waiters=0", channel.prompts[0])


if __name__ == "__main__":
    unittest.main()
