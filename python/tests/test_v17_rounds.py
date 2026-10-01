"""Same-round design metrics and the 24-cell entry. No model requests."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from cir_workflow.conditional_arms import run_repairs
from cir_workflow.feedback_runner import design_round_status
from cir_workflow.live import BudgetExhausted

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
CHANNEL = {
    "task": "channel/send_while_holding_mutex",
    "source_path": str(NOTES / "strong-link-v5/reexec-deepseekflash/DeepSeek Flash/channel__send_while_holding_mutex/rep0/source.rs"),
    "requirements_path": str(REPO / "benchmarks/families/channel/send_while_holding_mutex/generation_input/REQUIREMENTS.md"),
    "cir_path": str(REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash/DeepSeek Flash/channel__send_while_holding_mutex/rep0/cir/revision-1.cir.json"),
    "contract_path": str(REPO / "benchmarks/families/channel/send_while_holding_mutex/contract.json"),
}
CONDVAR = {
    "task": "condvar/notify_one_multi_waiter_wrong_pick",
    "source_path": str(NOTES / "strong-link-v5/reexec-gpt6luna/GPT 6 Luna/condvar__notify_one_multi_waiter_wrong_pick/rep0/source.rs"),
    "requirements_path": str(REPO / "benchmarks/families/condvar/notify_one_multi_waiter_wrong_pick/generation_input/REQUIREMENTS.md"),
    "cir_path": str(REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-gpt6luna/GPT 6 Luna/condvar__notify_one_multi_waiter_wrong_pick/rep0/cir/revision-1.cir.json"),
    "contract_path": str(REPO / "benchmarks/families/condvar/notify_one_multi_waiter_wrong_pick/contract.json"),
}


def _load_entry():
    spec = importlib.util.spec_from_file_location("v17_entry", REPO / "scripts/v12_three_arm.py")
    entry = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(entry)
    return entry


def _outcome(text, model="deepseek-flash"):
    return SimpleNamespace(text="```rust\n" + text + "\n```", response_model=model,
                           usage={"prompt_tokens": 3, "completion_tokens": 1}, wall_ms=1,
                           finish_reason="stop")


def _ledger(complete: bool, reject: bool = False) -> dict:
    return {"ledger": {
        "delivery_status": "reject" if reject else "deliver_bounded",
        "design_correspondence": {"complete": complete, "uncovered_sync": [] if complete else ["Barrier"]},
    }, "followup": {}, "binding": {}}


class DesignStatusTests(unittest.TestCase):
    def test_complete_pass_differs_from_uncovered_and_missing(self):
        self.assertEqual(design_round_status(_ledger(True)), "pass")
        self.assertEqual(design_round_status(_ledger(False)), "incomplete")
        self.assertEqual(design_round_status(_ledger(False, reject=True)), "fail")
        self.assertEqual(design_round_status({"ledger": {}}), "not_observed")
        self.assertEqual(design_round_status(None), "not_observed")


class RoundMetricTests(unittest.TestCase):
    def _run(self, texts, design_for):
        class Client:
            calls = 0

            def complete(self, system, user):
                Client.calls += 1
                return _outcome(texts[Client.calls - 1])

        def score(source, _work):
            ok = "REQ" in source or "BOTH" in source
            return {"status": "bounded_covered_satisfied" if ok else "fail",
                    "runs": [] if ok else [{"kind": "completed", "returncode": 1, "stdout": "", "timed_out": False}]}

        def evaluate(source, _case, _work):
            return design_for(source)

        case = {"task": "channel/send_while_holding_mutex", "defect": "fn main() {}",
                "requirements": "reqs", "cir_text": "{}"}
        return run_repairs(case, "A", Client(), Path(tempfile.mkdtemp()) / "cell",
                           score=score, evaluate_candidate=evaluate), Client

    def test_same_round_both_and_split_rounds(self):
        both, client = self._run(["fn main() { /* BOTH */ }"], lambda source: _ledger("BOTH" in source))
        self.assertEqual(client.calls, 1)
        self.assertEqual(both["first_requirement_round"], 1)
        self.assertEqual(both["first_design_round"], 1)
        self.assertEqual(both["first_both_round"], 1)

        def design_for(source):
            if "DESIGN" in source:
                return _ledger(True)
            if "REQ" in source:
                return _ledger(False, reject=True)
            return _ledger(False, reject=True)

        split, split_client = self._run(
            ["fn main() { /* DESIGN */ }", "fn main() { /* REQ */ }"], design_for)
        self.assertEqual(split_client.calls, 2)
        self.assertEqual(split["first_requirement_round"], 2)
        self.assertEqual(split["first_design_round"], 1)
        self.assertIsNone(split["first_both_round"])

    def test_unknown_and_not_observed_are_not_passes(self):
        class Client:
            calls = 0

            def complete(self, system, user):
                Client.calls += 1
                return _outcome("fn main() { if true {} }")

        def score(source, _work):
            if "if true" in source:
                return {"status": "unknown", "runs": []}
            return {"status": "fail", "runs": [{"kind": "completed", "returncode": 1, "stdout": "", "timed_out": False}]}

        case = {"task": "channel/send_while_holding_mutex", "defect": "fn main() {}",
                "requirements": "reqs", "cir_text": "{}"}
        result = run_repairs(case, "B", Client(), Path(tempfile.mkdtemp()) / "unk",
                             score=score, evaluate_candidate=lambda *args: {"ledger": {}})
        self.assertEqual(result["stop"], "unknown")
        self.assertIsNone(result["first_requirement_round"])
        self.assertIsNone(result["first_design_round"])
        self.assertIsNone(result["first_both_round"])

    def test_restart_keeps_the_recorded_rounds(self):
        directory = Path(tempfile.mkdtemp()) / "again"

        class Client:
            calls = 0

            def complete(self, system, user):
                Client.calls += 1
                return _outcome("fn main() { /* BOTH */ }")

        def score(source, _work):
            ok = "BOTH" in source
            return {"status": "bounded_covered_satisfied" if ok else "fail", "runs": []}

        case = {"task": "channel/send_while_holding_mutex", "defect": "fn main() {}",
                "requirements": "reqs", "cir_text": "{}"}
        first = run_repairs(case, "C", Client(), directory, score=score,
                            evaluate_candidate=lambda source, case, work: _ledger("BOTH" in source))
        calls = Client.calls
        second = run_repairs(case, "C", Client(), directory, score=score,
                             evaluate_candidate=lambda source, case, work: _ledger("BOTH" in source))
        self.assertEqual(Client.calls, calls)
        self.assertEqual(second["first_requirement_round"], first["first_requirement_round"])
        self.assertEqual(second["first_design_round"], first["first_design_round"])
        self.assertEqual(second["first_both_round"], first["first_both_round"])


def _entry_patches(entry, models, complete, score, reserves=1):
    def build_client(spec, *, budget, **kwargs):
        class Fake:
            def complete(self, system, user):
                for _ in range(reserves):
                    budget.reserve()
                text = complete(spec)
                model = None if spec.model_id == "deepseek-flash" and getattr(Fake, "hide_identity", False) else spec.model_id
                return _outcome(text, model or "missing")

        Fake.hide_identity = getattr(build_client, "hide_identity", False)
        if Fake.hide_identity and spec.model_id == "deepseek-flash":
            def complete_hidden(self, system, user):
                for _ in range(reserves):
                    budget.reserve()
                return _outcome(complete(spec), None)
            Fake.complete = complete_hidden
        return Fake()

    return [
        patch.object(entry, "MODELS", models),
        patch.object(entry, "load_dotenv"),
        patch.object(entry, "key_for", return_value="fake-no-secret"),
        patch.object(entry, "build_client", side_effect=build_client),
        patch.object(entry, "_eval", side_effect=score[0]),
        patch.object(entry, "score_for_task", side_effect=score[1]),
    ]


class EntryTests(unittest.TestCase):
    def _selected(self, root: Path) -> Path:
        path = root / "selected.json"
        path.write_text(json.dumps({"selected_records": [CHANNEL, CONDVAR]}), encoding="utf-8")
        return path

    def _run(self, entry, selected: Path, out: Path, confirm: bool) -> int:
        argv = ["v12_three_arm.py", "--selected", str(selected), "--out", str(out)]
        if confirm:
            argv.append("--confirm-paid-run")
        with patch.object(sys, "argv", argv):
            return entry.main()

    def test_dry_run_plans_24_cells_and_does_not_rewrite(self):
        entry = _load_entry()
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            selected = self._selected(root)
            out = root / "plan"
            with patch.object(sys, "argv", ["v12", "--selected", str(selected), "--out", str(out)]):
                self.assertEqual(entry.main(), 0)
            rows = json.loads((out / "RESULTS.json").read_text())
            protocol = json.loads((out / "PROTOCOL.json").read_text())
            self.assertEqual(len(rows), 24)
            self.assertEqual(protocol["max_physical"], 48)
            self.assertEqual(len({row["cell_key"] for row in rows}), 24)
            self.assertEqual(sorted({row["task"] for row in rows}), sorted({CHANNEL["task"], CONDVAR["task"]}))
            before = (out / "RESULTS.json").read_bytes()
            with patch.object(sys, "argv", ["v12", "--selected", str(selected), "--out", str(out)]):
                self.assertEqual(entry.main(), 0)
            self.assertEqual((out / "RESULTS.json").read_bytes(), before)

    def test_fake_entry_runs_all_24_and_restart_sends_nothing(self):
        entry = _load_entry()
        models = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "Kimi 2.7 Code"]

        def complete(_spec):
            return "fn main() { /* repaired */ }"

        def evaluate(source, _case, _work):
            return _ledger("repaired" in source, reject="repaired" not in source)

        def score(task, source, _work):
            if "repaired" in source:
                return {"status": "bounded_covered_satisfied", "runs": [], "task_id": task}
            return {"status": "fail", "runs": [{"kind": "completed", "returncode": 1, "stdout": "", "timed_out": False}],
                    "task_id": task}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            selected = self._selected(root)
            out = root / "run"
            patches = _entry_patches(entry, models, complete, (evaluate, score))
            with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5]:
                self.assertEqual(self._run(entry, selected, out, False), 0)
                self.assertEqual(self._run(entry, selected, out, True), 0)
            rows = json.loads((out / "RESULTS.json").read_text())
            self.assertEqual(len(rows), 24)
            self.assertTrue(all(row["status"] == "executed" for row in rows))
            self.assertTrue(all(row["first_requirement_round"] == 1 for row in rows))
            self.assertTrue(all(row["first_design_round"] == 1 for row in rows))
            self.assertTrue(all(row["first_both_round"] == 1 for row in rows))
            budget = json.loads((out / "budget.json").read_text())
            self.assertEqual(budget["requests_used"], 24)
            events = [json.loads(line) for line in (out / "REQUEST_EVENTS.jsonl").read_text().splitlines() if line.strip()]
            self.assertEqual(len(events), 24)
            saved = (out / "RESULTS.json").read_bytes()
            with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5]:
                self.assertEqual(self._run(entry, selected, out, True), 0)
            self.assertEqual((out / "RESULTS.json").read_bytes(), saved)
            self.assertEqual(json.loads((out / "budget.json").read_text())["requests_used"], 24)

    def test_identity_block_and_budget_cap_are_shared(self):
        entry = _load_entry()
        models = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "Kimi 2.7 Code"]

        def evaluate(source, _case, _work):
            return _ledger(False, reject=True)

        def score(task, source, _work):
            return {"status": "fail", "runs": [{"kind": "completed", "returncode": 1, "stdout": "", "timed_out": False}],
                    "task_id": task}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            selected = self._selected(root)
            out = root / "cap"
            patches = _entry_patches(entry, models, lambda spec: "fn main() { /* still */ }", (evaluate, score), reserves=2)
            with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5]:
                self.assertEqual(self._run(entry, selected, out, False), 0)
                self.assertEqual(self._run(entry, selected, out, True), 0)
            budget = json.loads((out / "budget.json").read_text())
            self.assertEqual(budget["requests_used"], 48)
            self.assertEqual(budget["max_requests"], 48)
            rows = json.loads((out / "RESULTS.json").read_text())
            self.assertTrue(any(row["status"] == "not_started" for row in rows))
            self.assertEqual(json.loads((out / "batch_state.json").read_text())["global_stop"],
                             "global_request_budget_exhausted")


if __name__ == "__main__":
    unittest.main()
