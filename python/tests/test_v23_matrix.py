"""strong-link-v23 executor gate: config, rounds, caps, resume, timeout, identity.

Fake transport only (no network, no cargo). Mirrors test_v22_matrix with the v23
round caps and adds the resume-zero-new-call and hard-timeout checks.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from cir_workflow.generation import load_gen_tasks
from cir_workflow.live import BudgetExhausted
from cir_workflow.multi_gen import (CELL_CAPS, ROUND_CAPS, ConfigError,
                                    execute_matrix, load_config, matrix_from_config,
                                    pilot_tasks)
from cir_workflow.audit import AuditLog
from cir_workflow.channels import AuditedClient
from cir_workflow.live import LiveBudget, reservation_link
from cir_workflow.transport import ModelSpec
from cir_workflow.opencode_go import OpenCodeGoClient, OpenCodeGoResponsesClient
from cir_workflow.multi_gen import SnapshotClient
from cir_workflow.hard_timeout import HardTimeoutClient, HardTimeoutError
from cir_workflow.v23_specs import config_spec
import threading
import time

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")
V23_CAPS = {"G0_direct": 2, "G1_self_iter": 6, "G2_tools_iter": 6, "G3_concir": 12}
V23_ROUNDS = {"G0_direct": 1, "G1_self_iter": 3, "G2_tools_iter": 3, "G3_concir": 3}
MODELS = [
    {"display_name": "Qwen", "model_id": "qwen3.8-flash"},
    {"display_name": "GPT 6 Luna", "model_id": "gpt-6-luna"},
    {"display_name": "GLM 5.3 Flash", "model_id": "glm-5.3-flash"},
]


def _config(tasks, models=None, arms=None, caps=None, rounds=None, matrix_cap=600):
    return {
        "models": models if models is not None else MODELS,
        "tasks": [{"task": t} for t in tasks],
        "arms": arms or ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"],
        "reps": [0],
        "cell_caps": caps or V23_CAPS,
        "rounds": rounds or V23_ROUNDS,
        "g3_code_rounds": 3,
        "global_physical_cap": matrix_cap,
        "matrix_cap": matrix_cap,
        "wall_seconds": 60,
        "concurrency": 1,
        "_sha256": "cfg",
    }


class _Mock:
    def __init__(self, spec, budget):
        self.model = spec.model_id
        self.max_tokens = 4096
        self.temperature = 0
        self.budget = budget
        self.calls = 0
        self.stage = "code"

    def set_stage(self, stage):
        self.stage = stage

    def complete(self, system, user):
        self.budget.reserve({"cell_id": self.model})
        self.budget.mark_attempt("returned")
        self.calls += 1
        return SimpleNamespace(text="fn main() {}\n", response_model=self.model,
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", transport_attempt=1)


class MatrixV23Tests(unittest.TestCase):
    def test_round_and_cell_caps_are_config_driven(self):
        cells = matrix_from_config(_config(["lock-order/abba_2lock"]))
        by_arm = {c["arm"]: c for c in cells}
        for arm in ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"):
            self.assertEqual(by_arm[arm]["cell_cap"], V23_CAPS[arm])
            self.assertEqual(by_arm[arm]["round_cap"], V23_ROUNDS[arm])

    def test_default_round_caps_exclude_first_generation(self):
        # defaults are 1/3/3/3: the cap includes the first generation
        self.assertEqual(ROUND_CAPS, {"G0_direct": 1, "G1_self_iter": 3,
                                      "G2_tools_iter": 3, "G3_concir": 3})

    def test_mock_matrix_72_cells(self):
        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        chosen = [item["task"] for item in pilot_tasks(REPO / "benchmarks/GENERATION_MANIFEST.json")]

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)

        def runner(cell, client, task, out_dir):
            # spend exactly the configured round cap, as the real runner would
            for _ in range(cell["round_cap"]):
                client.complete("s", "u")
            return {"accepted": False, "status": "not_accepted"}

        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(_config(chosen), Path(td), client_factory=factory,
                                    tasks_by_id=tasks, binary=Path("missing"),
                                    stage_runner=runner)
        self.assertEqual(len(result["cells"]), 72)
        by_arm = {}
        for item in result["cells"]:
            by_arm.setdefault(item["cell"]["arm"], 0)
            by_arm[item["cell"]["arm"]] += 1
        self.assertEqual(by_arm, {"G0_direct": 18, "G1_self_iter": 18,
                                  "G2_tools_iter": 18, "G3_concir": 18})
        self.assertTrue(all(item["status"] == "executed" for item in result["cells"]))

    def test_resume_is_zero_new_physical_calls(self):
        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        task_id = "atomic-data/atomic_lost_update"
        calls = []

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)

        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            calls.append(cell["arm"])
            return {"accepted": False}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cfg = _config([task_id], arms=["G0_direct"])
            first = execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                                   binary=Path("missing"), stage_runner=runner)
            after_first = len(calls)
            second = execute_matrix(cfg, root, client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(first["cells"][0]["status"], "executed")
        self.assertEqual(second["cells"][0]["status"], "executed")
        # the resumed cell is executed from the saved snapshot: no new model call
        self.assertEqual(len(calls), after_first)

    def test_timeout_is_recorded_and_does_not_hang(self):
        tasks = {t.id: t for t in load_gen_tasks(REPO)}

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)

        def runner(cell, client, task, out_dir):
            raise TimeoutError("simulated hard timeout")

        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(_config(["atomic-data/atomic_lost_update"]),
                                    Path(td), client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        self.assertEqual(result["cells"][0]["status"], "infrastructure_error")
        self.assertIn("TimeoutError", result["cells"][0]["error"])

    def test_second_coordinator_is_refused(self):
        import fcntl
        import subprocess
        import os
        import sys
        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            handle = (root / "coordinator.lock").open("a", encoding="utf-8")
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            script = (
                "import sys\nfrom pathlib import Path\n"
                "from cir_workflow.multi_gen import execute_matrix, ConfigError\n"
                "cfg={'models':[{'display_name':'Qwen','model_id':'qwen3.8-flash'}],"
                "'tasks':[{'task':'atomic-data/atomic_lost_update'}],'arms':['G0_direct'],"
                "'reps':[0],'cell_caps':{'G0_direct':2},'rounds':{'G0_direct':1},"
                "'global_physical_cap':4,'matrix_cap':4,'wall_seconds':30,'concurrency':1,"
                "'_sha256':'cfg'}\n"
                "try:\n"
                "    execute_matrix(cfg, Path(sys.argv[1]), client_factory=lambda *a: None,"
                " tasks_by_id={}, binary=Path('missing'))\n"
                "    print('ran')\n"
                "except Exception as exc:\n"
                "    print(type(exc).__name__, exc)\n")
            proc = subprocess.run([sys.executable, "-c", script, str(root)], cwd=str(REPO),
                                  env={**os.environ, "PYTHONPATH": "python"},
                                  capture_output=True, text=True)
            handle.close()
        self.assertIn("another coordinator", proc.stdout + proc.stderr)

    def test_attempt_row_links_run_cell_stage_round_retry(self):
        class _Inner:
            def __init__(self, budget):
                self.budget = budget
                self.stage = "cir"
                self.model = "qwen3.8-flash"
                self.last_transport_log = []

            def complete(self, system, user):
                self.budget.reserve(reservation_link(self, 2))
                self.budget.mark_attempt("returned")
                return SimpleNamespace(text="ok", response_model="qwen3.8-flash",
                                       usage={"prompt_tokens": 1, "completion_tokens": 1},
                                       wall_ms=1, request_id="r", transport_attempt=1,
                                       finish_reason="stop")

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            budget = LiveBudget(root / "budget.json", max_requests=10, max_seconds=60)
            spec = ModelSpec("Qwen", "qwen", "opencode-go", "qwen3.8-flash")
            inner = _Inner(budget)
            snapshot = SnapshotClient(inner, root / "state.json", "fp")
            timed = HardTimeoutClient(snapshot, seconds=30)
            client = AuditedClient(timed, audit=AuditLog(root / "audit.jsonl"),
                                   run_id="r1", cell_id="t/G0/rep0", spec=spec,
                                   arm="G0_direct", task_id="t", replicate=0, stage="cir")
            client.complete("s", "u", candidate_round=3)
            rows = budget.attempt_rows()
        self.assertEqual(len(rows), 1)
        row = rows[0]
        self.assertEqual(row["run_id"], "r1")
        self.assertEqual(row["cell_id"], "t/G0/rep0")
        self.assertEqual(row["stage"], "cir")
        self.assertEqual(row["model"], "qwen3.8-flash")
        self.assertTrue(str(row["logical_attempt"]).endswith("-r3-a3"))
        self.assertEqual(row["transport_retry"], 2)

    def test_chat_thinking_params_are_sent_and_recorded(self):
        captured = {}

        class _Completions:
            def create(self, **kwargs):
                captured.update(kwargs)
                return SimpleNamespace(
                    choices=[SimpleNamespace(message=SimpleNamespace(content="ok"),
                                             finish_reason="stop")],
                    usage=SimpleNamespace(model_dump=lambda: {"prompt_tokens": 1,
                                                              "completion_tokens": 2}),
                    model="qwen3.8-flash", id="resp-1")

        with tempfile.TemporaryDirectory() as td:
            budget = LiveBudget(Path(td) / "budget.json", max_requests=5, max_seconds=60)
            client = OpenCodeGoClient(api_key="k", budget=budget,
                                      evidence_dir=Path(td) / "ev", model="qwen3.8-flash",
                                      max_tokens=65536,
                                      thinking={"enable_thinking": True,
                                                "reasoning_effort": "high"})
            client._client = SimpleNamespace(chat=SimpleNamespace(completions=_Completions()))
            client.complete("s", "u")
            record = client.last_request_params
        self.assertEqual(captured["max_tokens"], 65536)
        self.assertEqual(captured["reasoning_effort"], "high")
        self.assertTrue(captured["extra_body"]["enable_thinking"])
        self.assertEqual(record["reasoning_effort"], "high")

    def test_responses_thinking_params_are_sent_and_recorded(self):
        captured = {}

        class _Responses:
            def create(self, **kwargs):
                captured.update(kwargs)
                return SimpleNamespace(output_text="ok",
                                       usage=SimpleNamespace(model_dump=lambda: {"input_tokens": 1,
                                                                                 "output_tokens": 2}),
                                       model="gpt-6-luna", id="resp-2")

        with tempfile.TemporaryDirectory() as td:
            budget = LiveBudget(Path(td) / "budget.json", max_requests=5, max_seconds=60)
            client = OpenCodeGoResponsesClient(
                api_key="k", budget=budget, evidence_dir=Path(td) / "ev",
                model="gpt-6-luna", max_tokens=65536,
                thinking={"reasoning": {"effort": "medium"}})
            client._client = SimpleNamespace(responses=_Responses())
            client.complete("s", "u")
            record = client.last_request_params
        self.assertEqual(captured["max_output_tokens"], 65536)
        self.assertEqual(captured["reasoning"], {"effort": "medium"})
        self.assertEqual(record["reasoning"], {"effort": "medium"})

    def test_hard_timeout_aborts_and_continues(self):
        class _Blocking:
            def __init__(self, budget):
                self.budget = budget
                self.stage = "code"
                self.closed = threading.Event()
                self.last_transport_log = []

            def complete(self, system, user):
                self.budget.reserve(reservation_link(self, 1))
                self.budget.mark_attempt("dispatched")
                self.closed.wait(10)
                raise RuntimeError("aborted by close")

            def close(self):
                self.closed.set()

        with tempfile.TemporaryDirectory() as td:
            budget = LiveBudget(Path(td) / "budget.json", max_requests=10, max_seconds=60)
            client = HardTimeoutClient(_Blocking(budget), seconds=0.3, grace=1.0)
            started = time.time()
            with self.assertRaises(HardTimeoutError):
                client.complete("s", "u")
            elapsed = time.time() - started

            # the unknown attempt still consumes its reservation (no refund)
            self.assertEqual(len(budget.attempt_rows()), 1)
            self.assertIn(budget.attempt_rows()[0]["status"], ("reserved", "dispatched"))

            # the coordinator can continue with a fresh client
            class _Fast:
                def __init__(self):
                    self.stage = "code"

                def complete(self, system, user):
                    return SimpleNamespace(text="ok", response_model="m", usage=None,
                                           wall_ms=1, request_id="r")

            self.assertEqual(HardTimeoutClient(_Fast(), 5).complete("s", "u").text, "ok")
        self.assertLess(elapsed, 3.0)

    def test_config_resolver_uses_config_channel(self):
        spec = config_spec({"display_name": "Qwen", "model_id": "qwen3.8-flash",
                            "channel": "opencode-go", "surface": "chat"})
        self.assertEqual(spec.channel, "opencode-go")   # not the registry default
        self.assertEqual(spec.model_id, "qwen3.8-flash")
        self.assertEqual(spec.surface, "chat")

    def test_transport_error_is_recoverable_and_continues(self):
        class APIConnectionError(Exception):
            pass

        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        seen = []

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)

        def runner(cell, client, task, out_dir):
            seen.append(cell["arm"])
            if cell["arm"] == "G1_self_iter":
                raise APIConnectionError("Connection error.")
            return {"accepted": True}

        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(_config(["atomic-data/atomic_lost_update"]),
                                    Path(td), client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        statuses = {i["cell"]["arm"]: i["status"] for i in result["cells"]}
        self.assertEqual(statuses["G1_self_iter"], "transport_error")
        # a transport error in one cell must not stop the rest of the batch
        self.assertIsNone(result["stop"])
        self.assertEqual(len(seen), 12)  # 3 models x 4 arms, all attempted

    def test_cell_cap_exhaustion_does_not_stop_batch(self):
        tasks = {t.id: t for t in load_gen_tasks(REPO)}

        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)

        def runner(cell, client, task, out_dir):
            for _ in range(cell["cell_cap"] + 1):  # overrun the per-cell cap once
                client.complete("s", "u")
            return {"accepted": False}

        with tempfile.TemporaryDirectory() as td:
            result = execute_matrix(_config(["atomic-data/atomic_lost_update"],
                                            caps={"G0_direct": 2, "G1_self_iter": 6,
                                                  "G2_tools_iter": 6, "G3_concir": 12}),
                                    Path(td), client_factory=factory, tasks_by_id=tasks,
                                    binary=Path("missing"), stage_runner=runner)
        statuses = [i["status"] for i in result["cells"]]
        self.assertIn("budget_exhausted", statuses)
        self.assertIsNone(result["stop"])  # a per-cell cap must not halt the matrix

    def test_snapshot_replay_outcome_has_provider_fields(self):
        class _Inner:
            def complete(self, system, user):
                return SimpleNamespace(text="ok", response_model="m", usage=None, wall_ms=1,
                                       finish_reason="stop", request_id="r1",
                                       prompt_sha256="p", cost=None, transport_attempt=1)

        with tempfile.TemporaryDirectory() as td:
            state = Path(td) / "state.json"
            SnapshotClient(_Inner(), state, "fp").complete("s", "u")
            replayed = SnapshotClient(_Inner(), state, "fp").complete("s", "u")
        # providers read .request_id/.prompt_sha256 directly; a replay must expose them
        self.assertEqual(replayed.text, "ok")
        self.assertIsNone(replayed.request_id)
        self.assertTrue(hasattr(replayed, "cost"))


if __name__ == "__main__":
    unittest.main()
