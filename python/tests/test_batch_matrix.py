"""Batch control through the real paid entry. No network."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO))

from cir_workflow import feedback_runner  # noqa: E402
from cir_workflow.live import BudgetExhausted  # noqa: E402
from scripts import run_feedback_pilot as batch  # noqa: E402
from scripts.feedback_pilot import freeze_inputs  # noqa: E402

_REAL_SCORE = feedback_runner.evaluate_role
_REAL_EVAL = feedback_runner._default_evaluate
_SCORE_CACHE: dict = {}
_EVAL_CACHE: dict = {}


def _cached_score(case, source, work):
    key = (case["id"], hashlib.sha256(source.encode()).hexdigest())
    if key not in _SCORE_CACHE:
        _SCORE_CACHE[key] = _REAL_SCORE(case, source, work)
    return _SCORE_CACHE[key]


def _cached_eval(source, case, work):
    key = (case["id"], hashlib.sha256(source.encode()).hexdigest())
    if key not in _EVAL_CACHE:
        _EVAL_CACHE[key] = _REAL_EVAL(source, case, work)
    return _EVAL_CACHE[key]


def _fence(source: str) -> str:
    return "```rust\n" + source + "\n```"


def _case_in(user: str, sources: dict[str, str]) -> str:
    found = [name for name, source in sources.items() if source in user]
    if len(found) != 1:
        raise AssertionError(f"prompt matched {found}")
    return found[0]


def _is_counterexample(user: str) -> bool:
    feedback = user.split("\nFeedback:\n", 1)[-1]
    return any(token in feedback for token in (
        "checker_next=", "functional.observed_stdout=", "execution.started=",
        "cargo.compiler_log="))


class _Repair:
    """Return the defect twice on verdict_only and the control on the other arm."""

    def __init__(self, spec, defects, controls):
        self.model_id = spec.model_id
        self.defects = defects
        self.controls = controls
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        case_id = _case_in(user, self.defects)
        source = self.controls[case_id] if _is_counterexample(user) else self.defects[case_id]
        return SimpleNamespace(text=_fence(source), usage=None, wall_ms=None,
                               response_model=self.model_id, request_id="fake")


class _Passing:
    qwen_calls = 0

    def __init__(self, spec, defects, controls, mismatch_model=False):
        self.model_id = spec.model_id
        self.defects = defects
        self.controls = controls
        self.mismatch_model = mismatch_model
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        case_id = _case_in(user, self.defects)
        returned = self.model_id
        if self.mismatch_model and self.model_id == "qwen3.8-flash":
            _Passing.qwen_calls += 1
            if _Passing.qwen_calls == 1:
                returned = "kimi-k3"
        return SimpleNamespace(text=_fence(self.controls[case_id]), usage=None, wall_ms=None,
                               response_model=returned, request_id="fake")


class _Boom:
    def __init__(self, message):
        self.message = message
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        raise BudgetExhausted(self.message)


class BatchEntryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        config, rows, _ignored = freeze_inputs()
        self.config = config
        self.rows = rows
        self.defect = Path(rows[0]["defect"]["path"]).read_text(encoding="utf-8")
        self.control = Path(rows[0]["control"]["path"]).read_text(encoding="utf-8")
        # Every case has its own control; the scripted client reads the case
        # from the user prompt's previous program only for the shared shape.
        self.controls = {
            row["id"]: Path(row["control"]["path"]).read_text(encoding="utf-8") for row in rows
        }
        self.defects = {
            row["id"]: Path(row["defect"]["path"]).read_text(encoding="utf-8") for row in rows
        }
        (self.root / "config.json").write_text(json.dumps(config), encoding="utf-8")
        (self.root / "inputs.jsonl").write_text(
            "".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
        self._old_build = batch.build_client
        self._old_key = batch.key_for
        self._old_env = batch.load_dotenv
        self._old_score = feedback_runner.evaluate_role
        self._old_eval = feedback_runner._default_evaluate
        batch.key_for = lambda *args, **kwargs: "test-key"
        batch.load_dotenv = lambda *args, **kwargs: None
        _Passing.qwen_calls = 0
        # Reuse real local results for the same source. The first call still
        # runs evaluate_role and evaluate_candidate.
        feedback_runner.evaluate_role = _cached_score
        feedback_runner._default_evaluate = _cached_eval

    def tearDown(self):
        batch.build_client = self._old_build
        batch.key_for = self._old_key
        batch.load_dotenv = self._old_env
        feedback_runner.evaluate_role = self._old_score
        feedback_runner._default_evaluate = self._old_eval
        self.tmp.cleanup()

    def _run(self, out: Path) -> dict:
        argv = sys.argv
        sys.argv = ["run_feedback_pilot.py", "--config", str(self.root / "config.json"),
                    "--inputs", str(self.root / "inputs.jsonl"), "--out", str(out),
                    "--confirm-paid-run"]
        try:
            code = batch.main()
        finally:
            sys.argv = argv
        self.assertEqual(code, 0)
        return json.loads((out / "RESULTS.json").read_text(encoding="utf-8"))

    def _keys(self, payload):
        return [(row["model_id"], row["case"], row["arm"]) for row in payload["results"]]

    def test_cell_repair_failure_does_not_stop_the_model(self):
        batch.build_client = lambda spec, **kwargs: _Repair(spec, self.defects, self.controls)
        payload = self._run(self.root / "repair")
        keys = self._keys(payload)
        self.assertEqual(len(keys), 48)
        self.assertEqual(len(set(keys)), 48)
        self.assertEqual(payload["planned_cells"], 48)
        first = payload["results"][0]
        second = payload["results"][1]
        self.assertEqual(first["arm"], "verdict_only")
        self.assertEqual(first["stop"], "cell_repair_budget_exhausted")
        self.assertEqual(second["arm"], "counterexample")
        self.assertEqual(second["stop"], "requirement_pass")
        self.assertEqual(second["case"], first["case"])
        self.assertTrue(all(row["status"] == "executed" for row in payload["results"]))
        self.assertEqual({row["case"] for row in payload["results"]},
                         {row["id"] for row in self.rows})

    def test_global_budget_keeps_unstarted_rows(self):
        def build_client(spec, **kwargs):
            return _Boom("request budget reached (1/1)")

        batch.build_client = build_client
        payload = self._run(self.root / "budget")
        self.assertEqual(len(payload["results"]), 48)
        self.assertEqual(payload["results"][0]["stop"], "global_request_budget_exhausted")
        self.assertEqual(payload["results"][0]["status"], "executed")
        self.assertTrue(all(row["status"] == "not_started" for row in payload["results"][1:]))
        self.assertEqual(payload["http_attempts"], 1)
        self.assertEqual(len(self._keys(payload)), 48)

    def test_identity_failure_blocks_only_that_model(self):
        batch.build_client = lambda spec, **kwargs: _Passing(
            spec, self.defects, self.controls, mismatch_model=True)
        payload = self._run(self.root / "identity")
        self.assertEqual(len(payload["results"]), 48)
        qwen = [row for row in payload["results"] if row["model_id"] == "qwen3.8-flash"]
        self.assertEqual(len(qwen), 12)
        self.assertEqual(qwen[0]["stop"], "identity_mismatch")
        self.assertEqual(qwen[0]["status"], "executed")
        self.assertTrue(all(row["status"] == "blocked" for row in qwen[1:]))
        others = [row for row in payload["results"] if row["model_id"] != "qwen3.8-flash"]
        self.assertTrue(all(row["status"] == "executed" for row in others))
        self.assertTrue(all(row["stop"] == "requirement_pass" for row in others))


if __name__ == "__main__":
    unittest.main()
