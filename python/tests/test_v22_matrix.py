"""Config-driven generation matrix. Fake transport only."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.generation import load_gen_tasks
from cir_workflow.live import BudgetExhausted
from cir_workflow.multi_gen import ConfigError, execute_matrix, load_config, pilot_tasks

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")


def _config(tasks, models=None, arms=None, caps=None):
    return {
        "models": models if models is not None else [
            {"display_name": "DeepSeek Flash", "model_id": "deepseek-flash", "max_tokens": 100},
        ],
        "tasks": [{"task": task} for task in tasks],
        "arms": arms or ["G0_direct"],
        "reps": [0],
        "cell_caps": caps or {"G0_direct": 2, "G1_self_iter": 8, "G2_tools_iter": 8, "G3_concir": 14},
        "global_physical_cap": 20,
        "matrix_cap": 20,
        "wall_seconds": 60,
        "concurrency": 1,
        "_sha256": "cfg",
    }


class _Mock:
    def __init__(self, spec, max_tokens, budget):
        self.model = spec.model_id
        self.max_tokens = max_tokens
        self.temperature = 0
        self.budget = budget
        self.calls = 0
        self.users = []
        self.stage = "code"

    def set_stage(self, stage):
        self.stage = stage

    def complete(self, system, user):
        self.budget.reserve({"cell_id": self.model})
        self.budget.mark_attempt("returned")
        self.calls += 1
        self.users.append(user)
        return SimpleNamespace(text="fn main() {}\n", response_model=self.model,
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", request_id="m", transport_attempt=1)


class MatrixTests(unittest.TestCase):
    def test_empty_config_does_not_call_default_models(self):
        cfg = _config(["atomic-data/bounded_counter_invariant"], models=[])
        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(cfg, Path(td), client_factory=lambda *args: (_ for _ in ()).throw(AssertionError("called")),
                                    tasks_by_id={}, binary=Path("missing"))
        self.assertEqual(result["calls"], 0)
        self.assertEqual(result["stop"], "empty_matrix")

    def test_missing_model_id_is_rejected(self):
        path = Path(tempfile.mkdtemp()) / "config.json"
        path.write_text(json.dumps({"models": [{"display_name": "DeepSeek Flash"}], "tasks": [], "arms": []}))
        with self.assertRaises(ConfigError):
            load_config(path)

    def test_parameter_change_blocks_resume(self):
        tasks = {task.id: task for task in load_gen_tasks(REPO)}
        task_id = "atomic-data/bounded_counter_invariant"

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, 100 if cell.get("_tokens", 100) == 100 else cell["_tokens"], budget)

        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            return {"accepted": False}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = _config([task_id])
            first = execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                                   binary=Path("missing"), stage_runner=runner)
            self.assertEqual(first["calls"], 1)
            cfg2 = _config([task_id])
            cfg2["models"][0]["max_tokens"] = 200

            def factory2(spec, cell, budget, out_dir):
                return _Mock(spec, 200, budget)

            second = execute_matrix(cfg2, root, client_factory=factory2, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(second["cells"][0]["status"], "protocol_stop")
        self.assertIn("fingerprint", second["cells"][0]["error"])

    def test_cell_cap_counts_retries(self):
        tasks = {task.id: task for task in load_gen_tasks(REPO)}

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, 100, budget)

        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            client.complete("s", "u")
            client.complete("s", "retry")
            return {"accepted": False}

        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(_config(["atomic-data/bounded_counter_invariant"], caps={"G0_direct": 2}),
                                    Path(td), client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(result["cells"][0]["status"], "budget_exhausted")
        self.assertEqual(result["calls"], 2)

    def test_snapshot_tamper_blocks(self):
        tasks = {task.id: task for task in load_gen_tasks(REPO)}

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, 100, budget)

        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            return {"accepted": False}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = _config(["atomic-data/bounded_counter_invariant"])
            execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                           binary=Path("missing"), stage_runner=runner)
            state = next(root.rglob("state.json"))
            saved = json.loads(state.read_text())
            saved["calls"][0]["response"] = "tampered"
            state.write_text(json.dumps(saved))
            second = execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(second["cells"][0]["status"], "protocol_stop")
        self.assertIn("snapshot_corrupt", second["cells"][0]["error"])

    def test_second_coordinator_is_refused(self):
        import fcntl
        import subprocess
        import sys
        tasks = {task.id: task for task in load_gen_tasks(REPO)}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            handle = (root / "coordinator.lock").open("a", encoding="utf-8")
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            script = (
                "import sys\n"
                "from pathlib import Path\n"
                "from cir_workflow.multi_gen import execute_matrix\n"
                "cfg={'models':[{'display_name':'DeepSeek Flash','model_id':'deepseek-flash'}],"
                "'tasks':[{'task':'atomic-data/bounded_counter_invariant'}],'arms':['G0_direct'],"
                "'reps':[0],'cell_caps':{'G0_direct':2},'global_physical_cap':4,'matrix_cap':4,"
                "'wall_seconds':30,'concurrency':1,'_sha256':'cfg'}\n"
                "try:\n"
                "    execute_matrix(cfg, Path(sys.argv[1]), client_factory=lambda *a: None,\n"
                "                   tasks_by_id={}, binary=Path('missing'))\n"
                "    print('ran')\n"
                "except Exception as exc:\n"
                "    print(type(exc).__name__, exc)\n"
            )
            proc = subprocess.run([sys.executable, "-c", script, str(root)], cwd=str(REPO),
                                  env={**dict(**__import__("os").environ), "PYTHONPATH": "python"},
                                  capture_output=True, text=True)
            handle.close()
        self.assertIn("another coordinator", proc.stdout + proc.stderr)

    def test_prompt_hides_contract_marker(self):
        from cir_workflow.multi_gen import GenerationProvider
        captured = {}

        class _C:
            def complete(self, system, user):
                captured["user"] = user
                return SimpleNamespace(text="fn main(){}", response_model="m", usage=None, wall_ms=1)

        provider = GenerationProvider(_C(), "direct", "deepseek-flash")
        provider.propose(type("R", (), {"requirements": "need a counter", "feedback": None,
                                        "previous_candidate": None, "attempt": 1,
                                        "contract": {"secret_marker": "CONTRACT-SECRET"}})())
        self.assertNotIn("CONTRACT-SECRET", captured["user"])

    def test_qwen_opencode_sends_thinking_off(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location(
            "v22_live", REPO / "scripts/v22_live.py")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        class _Client:
            def _kwargs(self, messages, max_tokens):
                return {"model": "qwen3.8-flash", "messages": messages,
                        "max_tokens": max_tokens}

        client = module._send_qwen_thinking_off(_Client())
        sent = client._kwargs([{"role": "user", "content": "x"}], 32)
        self.assertEqual(sent["extra_body"], {"enable_thinking": False})
        self.assertEqual(client.extra_body, {"enable_thinking": False})

    def test_executed_cell_is_not_sent_again(self):
        tasks = {task.id: task for task in load_gen_tasks(REPO)}
        sent = []

        def factory(spec, cell, budget, out_dir):
            client = _Mock(spec, 100, budget)
            original = client.complete

            def complete(system, user):
                sent.append(user)
                return original(system, user)

            client.complete = complete
            return client

        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            return {"accepted": True, "status": "accepted"}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = _config(["atomic-data/bounded_counter_invariant"])
            execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                           binary=Path("missing"), stage_runner=runner)
            second = execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(len(sent), 1)
        self.assertTrue(second["cells"][0]["cached"])
        self.assertEqual(second["calls"], 0)
        self.assertEqual(second["cells"][0]["record"]["accepted"], True)

    def test_empty_snapshot_accepts_a_new_fingerprint(self):
        from cir_workflow.multi_gen import SnapshotClient
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "state.json"
            path.write_text(json.dumps({"fingerprint": "old", "calls": []}))
            client = SnapshotClient(_Mock(SimpleNamespace(model_id="m"), 1, SimpleNamespace(
                reserve=lambda link=None: 1, mark_attempt=lambda status: None)), path, "new")
            self.assertIsNone(client.blocked)
            outcome = client.complete("s", "fresh")
        self.assertEqual(outcome.text, "fn main() {}\n")

    def test_saved_call_replays_without_a_send(self):
        from cir_workflow.multi_gen import SnapshotClient, _digest
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "state.json"
            record = {"system": "s", "user": "u", "response": "cached-program",
                      "response_model": "m"}
            record["snapshot_sha256"] = _digest(record)
            path.write_text(json.dumps({"fingerprint": "same", "calls": [record]}))
            inner = _Mock(SimpleNamespace(model_id="m"), 1, SimpleNamespace(
                reserve=lambda link=None: (_ for _ in ()).throw(AssertionError("sent"))))
            client = SnapshotClient(inner, path, "same")
            outcome = client.complete("s", "u")
        self.assertEqual(outcome.text, "cached-program")
        self.assertEqual(outcome.finish_reason, "cache")
        self.assertEqual(client.calls, 0)

    def test_six_families_are_the_manifest_first_tasks(self):
        chosen = pilot_tasks(REPO / "benchmarks/GENERATION_MANIFEST.json")
        self.assertEqual([item["family"] for item in chosen],
                         ["lock-order", "condvar", "channel", "semaphore", "atomic-data", "structure"])
        self.assertEqual(len(chosen), 6)


if __name__ == "__main__":
    unittest.main()
