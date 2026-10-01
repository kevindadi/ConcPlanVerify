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
from cir_workflow.transport import server_model_id

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
                self.last_transport_log = []
                for index in range(reserves):
                    budget.reserve()
                    failed = index + 1 < reserves
                    self.last_transport_log.append({
                        "index": index + 1,
                        "status": "error" if failed else "ok",
                        "error_type": "RetryableStatus" if failed else None,
                        "error": "retryable" if failed else None,
                    })
                text = complete(spec)
                model = None if spec.model_id == "deepseek-flash" and getattr(Fake, "hide_identity", False) else server_model_id(spec)
                outcome = _outcome(text, model or "missing")
                outcome.transport_attempt = reserves
                return outcome

        Fake.hide_identity = getattr(build_client, "hide_identity", False)
        if Fake.hide_identity and spec.model_id == "deepseek-flash":
            def complete_hidden(self, system, user):
                self.last_transport_log = []
                for index in range(reserves):
                    budget.reserve()
                    self.last_transport_log.append({"index": index + 1, "status": "ok"})
                outcome = _outcome(complete(spec), None)
                outcome.transport_attempt = reserves
                return outcome
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

    def test_toolchain_eval_uses_the_arm_case_id(self):
        entry = _load_entry()
        seen = {}

        def fake(_source, _cir, _contract, _work, **kwargs):
            seen["cell"] = kwargs.get("cell_id")
            return {"ledger": {}}

        with patch.object(entry, "evaluate_candidate", fake):
            entry._eval("fn main() {}", {
                "id": "channel/send_while_holding_mutex",
                "cir_path": "/tmp/cir.json", "contract_path": "/tmp/contract.json",
            }, "/tmp/work")
        self.assertEqual(seen["cell"], "channel/send_while_holding_mutex")

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
        models = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "GLM 5.3 Flash"]

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
            self.assertLessEqual(json.loads((out / "batch_state.json").read_text())["concurrency_peak"], 3)

    def test_identity_block_and_budget_cap_are_shared(self):
        entry = _load_entry()
        models = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "GLM 5.3 Flash"]

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
            state = json.loads((out / "batch_state.json").read_text())
            self.assertEqual(state["global_stop"], "global_request_budget_exhausted")
            self.assertLessEqual(state["concurrency_peak"], 3)
            events = [json.loads(line) for line in (out / "REQUEST_EVENTS.jsonl").read_text().splitlines() if line.strip()]
            retries = [event for event in events if event.get("parent_id")]
            self.assertGreaterEqual(len(retries), 1)
            self.assertTrue(all(event["attempt_id"] != event["parent_id"] for event in retries))
            self.assertEqual(sum(1 for event in events if event.get("status") == "ok"), 24)


class BudgetTests(unittest.TestCase):
    def test_two_instances_share_one_atomic_counter(self):
        from cir_workflow.live import BudgetExhausted, LiveBudget
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "budget.json"
            first = LiveBudget(path, max_requests=1, max_seconds=60)
            second = LiveBudget(path, max_requests=1, max_seconds=60)
            self.assertEqual(first.reserve(), 1)
            with self.assertRaises(BudgetExhausted):
                second.reserve()
            saved = json.loads(path.read_text())
            self.assertEqual(saved["requests_used"], 1)
            deadline = saved["deadline_epoch"]
            restarted = LiveBudget(path, max_requests=99, max_seconds=1)
            self.assertEqual(restarted.requests_used, 1)
            self.assertEqual(restarted.deadline_epoch, deadline)
            self.assertEqual(restarted.max_requests, 1)

    def test_cross_process_cap_allows_one_reserve(self):
        import subprocess
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "budget.json"
            script = (
                "import sys\n"
                "from pathlib import Path\n"
                "from cir_workflow.live import BudgetExhausted, LiveBudget\n"
                "budget = LiveBudget(Path(sys.argv[1]), max_requests=1, max_seconds=30)\n"
                "try:\n"
                "    print(budget.reserve())\n"
                "except BudgetExhausted:\n"
                "    print('exhausted')\n"
            )
            env = __import__("os").environ.copy()
            env["PYTHONPATH"] = "python"
            procs = [
                subprocess.Popen(
                    [sys.executable, "-c", script, str(path)],
                    cwd=str(REPO), env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                )
                for _ in range(2)
            ]
            finished = [proc.communicate() for proc in procs]
            texts = sorted(out.strip() for out, _err in finished)
            self.assertEqual(texts, ["1", "exhausted"])
            self.assertEqual(json.loads(path.read_text())["requests_used"], 1)

    def test_deepseek_retry_reserves_and_audits_each_attempt(self):
        from cir_workflow.audit import AuditLog
        from cir_workflow.channels import AuditedClient
        from cir_workflow.live import ALLOWED_MODEL, DeepSeekFlashClient, LiveBudget
        from cir_workflow.transport import require_experiment_model, build_registry

        class Retryable(Exception):
            status_code = 503

        class SDK:
            def __init__(self):
                self.calls = 0
                self.chat = self
                self.completions = self

            def create(self, **kwargs):
                self.calls += 1
                if self.calls == 1:
                    raise Retryable("503")
                return SimpleNamespace(
                    id="req-2", model=ALLOWED_MODEL,
                    usage=SimpleNamespace(prompt_tokens=2, completion_tokens=1, total_tokens=3),
                    choices=[SimpleNamespace(finish_reason="stop",
                                             message=SimpleNamespace(content="fn main() {}"))])

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = LiveBudget(root / "budget.json", max_requests=4, max_seconds=30)
            inner = DeepSeekFlashClient(api_key="test-key", budget=budget,
                                        evidence_dir=root / "llm", sdk_client=SDK(),
                                        max_transport_retries=1)
            audit = AuditLog(root / "REQUEST_EVENTS.jsonl")
            spec = require_experiment_model(build_registry(), "DeepSeek Flash")
            client = AuditedClient(inner, audit=audit, run_id="retry", cell_id="task/deepseek-flash/A",
                                   spec=spec, arm="A", task_id="task", replicate=0)
            outcome = client.complete("system", "user")
            self.assertEqual(outcome.transport_attempt, 2)
            self.assertEqual(budget.requests_used, 2)
            events = [json.loads(line) for line in (root / "REQUEST_EVENTS.jsonl").read_text().splitlines()]
            self.assertEqual(len(events), 2)
            self.assertEqual(events[0]["status"], "error")
            self.assertEqual(events[1]["status"], "ok")
            self.assertEqual(events[1]["parent_id"], events[0]["attempt_id"])
            self.assertEqual(events[1]["attempt_id"], "task/deepseek-flash/A-r1-a1")


if __name__ == "__main__":
    unittest.main()
