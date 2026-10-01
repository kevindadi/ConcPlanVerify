"""Checkpoint, replay, and budget-mirror controls. No model requests."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.feedback_runner import run_arm
from cir_workflow.live import BudgetExhausted, LiveBudget
from cir_workflow.transport import build_registry, require_experiment_model

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")


def _case(text: str = "requirements stay fixed") -> dict:
    return {"id": "channel/send_while_holding_mutex", "defect": "fn main() { /* defect */ }",
            "requirements": text, "cir": {}, "cir_text": "{}"}


class _Fake:
    def __init__(self):
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(
            text="```rust\nfn main() { /* still */ }\n```",
            response_model="deepseek-flash",
            usage={"prompt_tokens": 3, "completion_tokens": 1},
            wall_ms=1, finish_reason="stop")


def _score_factory(stdout: dict):
    def score(_case, _source, _work):
        return {"requirement": {"status": "fail"}, "design": {"status": "not_observed"},
                "requirement_error": True, "tool_error": False, "capability_gap": False,
                "support": {"supported": True},
                "raw": {"status": "fail", "runs": [{"stdout": stdout["value"]}]}}

    def evaluate(_source, _case, _work):
        return {"ledger": {"delivery_status": "reject", "design_correspondence": {"complete": False}},
                "followup": {"action": "repair", "category": "candidate_error"},
                "binding": {}, "functional": {},
                "observed_stdout": stdout["value"]}

    return score, evaluate


class RecoveryTests(unittest.TestCase):
    def test_two_failed_rounds_are_exhausted(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        with tempfile.TemporaryDirectory() as td:
            result = run_arm(_case(), spec, "counterexample", client, Path(td), max_repairs=2,
                             evaluate=evaluate, score=score, stop_after_round=2,
                             require_defect_signal=False, system_prompt="rules",
                             feedback_builder=lambda arm, requirement, delivery: "feedback")
        self.assertEqual(result["stop"], "cell_repair_budget_exhausted")
        self.assertEqual(client.calls, 2)

    def test_stdout_change_replays_and_still_offers_round_two(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        observed = {"value": "first-stdout"}
        score, evaluate = _score_factory(observed)
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            first = run_arm(_case(), spec, "counterexample", client, out, max_repairs=2,
                            evaluate=evaluate, score=score, stop_after_round=1,
                            require_defect_signal=False, system_prompt="rules",
                            feedback_builder=lambda arm, requirement, delivery: f"feedback {observed['value']}")
            self.assertEqual(first["stop"], "phase_complete")
            self.assertEqual(client.calls, 1)
            observed["value"] = "different-stdout"
            second = run_arm(_case(), spec, "counterexample", client, out, max_repairs=2,
                             evaluate=evaluate, score=score, stop_after_round=2,
                             require_defect_signal=False, system_prompt="rules",
                             feedback_builder=lambda arm, requirement, delivery: f"feedback {observed['value']}")
        self.assertNotEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 2)
        self.assertEqual(second["stop"], "cell_repair_budget_exhausted")

    def test_changed_requirements_still_block(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case("one"), spec, "counterexample", client, out, max_repairs=2,
                    evaluate=evaluate, score=score, stop_after_round=1,
                    require_defect_signal=False, system_prompt="rules",
                    feedback_builder=lambda arm, requirement, delivery: "feedback")
            second = run_arm(_case("two"), spec, "counterexample", client, out, max_repairs=2,
                             evaluate=evaluate, score=score, stop_after_round=2,
                             require_defect_signal=False, system_prompt="rules",
                             feedback_builder=lambda arm, requirement, delivery: "feedback")
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 1)

    def test_changed_system_prompt_is_refused_before_another_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case(), spec, "counterexample", client, out, stop_after_round=1,
                    system_prompt="original system", **common)
            second = run_arm(_case(), spec, "counterexample", client, out, stop_after_round=2,
                             system_prompt="different system", **common)
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 1)

    def test_corrupted_snapshot_is_not_followed_by_a_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case(), spec, "counterexample", client, out, stop_after_round=1, **common)
            state_path = out / "state.json"
            state = json.loads(state_path.read_text())
            state["requests"][0]["evaluation_snapshot"]["requirement"]["status"] = "tampered"
            state_path.write_text(json.dumps(state))
            second = run_arm(_case(), spec, "counterexample", client, out, stop_after_round=2, **common)
        self.assertEqual(second["stop"], "snapshot_corrupt")
        self.assertEqual(client.calls, 1)

    def test_changed_initial_source_blocks_before_another_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case(), spec, "counterexample", client, out, stop_after_round=1,
                    source="fn main() { /* frozen */ }", **common)
            second = run_arm(_case(), spec, "counterexample", client, out, stop_after_round=2,
                             source="fn main() { /* replaced */ }", **common)
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 1)

    def test_changed_generation_params_block_before_another_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")

        class _Client(_Fake):
            def __init__(self, temperature):
                super().__init__()
                self.temperature = temperature
                self.max_tokens = 16384
                self.reasoning_effort = "low"

        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            first = _Client(0)
            run_arm(_case(), spec, "counterexample", first, out, stop_after_round=1, **common)
            second_client = _Client(1)
            second = run_arm(_case(), spec, "counterexample", second_client, out,
                             stop_after_round=2, **common)
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(first.calls, 1)
        self.assertEqual(second_client.calls, 0)

    def test_changed_provider_blocks_before_another_send(self):
        deepseek = require_experiment_model(build_registry(), "DeepSeek Flash")
        qwen = require_experiment_model(build_registry(), "Qwen")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case(), deepseek, "counterexample", client, out, stop_after_round=1, **common)
            second = run_arm(_case(), qwen, "counterexample", client, out, stop_after_round=2, **common)
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 1)

    def test_raising_max_repairs_does_not_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            run_arm(_case(), spec, "counterexample", client, out, max_repairs=2,
                    stop_after_round=1, **common)
            second = run_arm(_case(), spec, "counterexample", client, out, max_repairs=3,
                             stop_after_round=2, **common)
        self.assertEqual(second["stop"], "fingerprint_mismatch")
        self.assertEqual(client.calls, 1)

    def test_saved_round_replays_with_no_extra_send(self):
        spec = require_experiment_model(build_registry(), "DeepSeek Flash")
        client = _Fake()
        score, evaluate = _score_factory({"value": "same"})
        common = dict(max_repairs=2, evaluate=evaluate, score=score, require_defect_signal=False,
                      feedback_builder=lambda arm, requirement, delivery: "feedback",
                      system_prompt="rules")
        with tempfile.TemporaryDirectory() as td:
            out = Path(td)
            first = run_arm(_case(), spec, "counterexample", client, out, stop_after_round=1, **common)
            second = run_arm(_case(), spec, "counterexample", client, out, stop_after_round=1, **common)
        self.assertEqual(first["stop"], "phase_complete")
        self.assertEqual(second["stop"], "phase_complete")
        self.assertEqual(client.calls, 1)


class BudgetMirrorTests(unittest.TestCase):
    def test_concurrent_mirrors_keep_the_sqlite_count(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "budget.json"
            LiveBudget(path, max_requests=8, max_seconds=30)
            errors = []

            def once():
                try:
                    LiveBudget(path, max_requests=99, max_seconds=1).reserve()
                except BudgetExhausted as exc:
                    errors.append(str(exc))
                except OSError as exc:
                    errors.append(type(exc).__name__)

            threads = [threading.Thread(target=once) for _ in range(8)]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
            saved = json.loads(path.read_text())
            self.assertEqual(errors, [])
            self.assertEqual(saved["requests_used"], 8)

    def test_second_process_does_not_reserve_past_one(self):
        script = (
            "import sys\nfrom pathlib import Path\n"
            "from cir_workflow.live import BudgetExhausted, LiveBudget\n"
            "budget = LiveBudget(Path(sys.argv[1]), max_requests=1, max_seconds=30)\n"
            "try:\n    print(budget.reserve())\n"
            "except BudgetExhausted:\n    print('exhausted')\n"
        )
        with tempfile.TemporaryDirectory() as td:
            path = str(Path(td) / "budget.json")
            env = dict(**{k: v for k, v in __import__("os").environ.items()})
            env["PYTHONPATH"] = "python"
            procs = [subprocess.Popen([sys.executable, "-c", script, path], cwd=str(REPO),
                                      env=env, stdout=subprocess.PIPE, text=True)
                     for _ in range(2)]
            texts = sorted(proc.communicate()[0].strip() for proc in procs)
            self.assertEqual(texts, ["1", "exhausted"])
            self.assertEqual(json.loads(Path(path).read_text())["requests_used"], 1)


if __name__ == "__main__":
    unittest.main()
