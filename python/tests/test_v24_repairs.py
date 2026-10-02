"""Regression controls for the partial thinking-batch diagnosis; no network."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from cir_workflow.arms import run_rust_arm
from cir_workflow.candidate_eval import decide_followup
from cir_workflow.generation import _cir_plan
from cir_workflow.multi_gen import GenerationProvider, execute_matrix
from cir_workflow.providers import CandidateRequest
from test_evidence_v2 import _ev, _result
from test_v23_matrix import _Mock, _config


class _SelfReviewer:
    def __init__(self):
        self.users = []

    def complete(self, system, user):
        self.users.append(user)
        text = "NO_ISSUES" if "reply exactly NO_ISSUES" in user else "fn main() {}"
        return SimpleNamespace(text=text, response_model="m", usage=None, wall_ms=1)


class RepairTests(unittest.TestCase):
    def test_isolated_binding_tool_can_be_selected_without_replacing_release(self):
        from cir_workflow.binding import default_binary
        with patch.dict("os.environ", {"CONCIR_BIND_CHECK": "/private/tmp/tool-version/bind_check"}):
            self.assertEqual(default_binary(), Path("/private/tmp/tool-version/bind_check"))

    def test_self_review_stops_on_the_previously_built_candidate(self):
        client = _SelfReviewer()
        with tempfile.TemporaryDirectory() as td, patch(
                "cir_workflow.rust_arm.RustArmProject.analyze", return_value={"build_ok": True}):
            run = run_rust_arm(GenerationProvider(client, "self", "m"),
                               arm="A1_self_iter", task="t", spec="requirements",
                               contract={}, out_dir=Path(td), k=3)
        self.assertTrue(run.accepted)
        self.assertEqual(len(client.users), 2)
        self.assertIn("Review the program for concurrency defects", client.users[1])

    def test_attribute_retry_includes_expected_and_observed_counts(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"s": "main::s"},
                       "ambiguous": [], "violated": {}, "attributes": [{
                           "resource_id": "main::s", "status": "mismatch",
                           "expected": "semaphore_initial", "expected_capacity": 1,
                           "observed": 2, "observed_capacity": 2, "construction_site": "10"}]})
            led = _ev(r)
            followup = decide_followup(led, r)
        self.assertEqual(followup["action"], "repair")
        self.assertIn('"expected_capacity": 1', followup["feedback"])
        self.assertIn('"observed_capacity": 2', followup["feedback"])
        self.assertFalse(led.all_obligations_satisfied)

    def test_plan_preserves_updates_calls_and_control_flow(self):
        cir = {"modules": [{"name": "main", "functions": [{"name": "main", "body": [
            {"sid": "s1", "kind": "call", "func": "helper"},
            {"sid": "s2", "kind": "var_write", "resource": "c", "value": 2},
            {"sid": "s3", "kind": "loop", "body": [{"kind": "var_write", "value": 3}]}
        ]}]}]}
        plan = _cir_plan(cir)
        for term in ('call', 'helper', 'var_write', '"value": 2', 'loop', '"value": 3'):
            self.assertIn(term, plan)
        self.assertNotIn("no operations", plan)

    def test_anonymous_worker_is_protocol_retry_not_semantic_failure(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"m": "main::m"}, "violated": {},
                       "ambiguous": [{"rust": "h1#10", "entry": None,
                                      "reason": "thread entry is not unambiguous"}]})
            led = _ev(r)
            followup = decide_followup(led, r)
        self.assertEqual(followup["action"], "repair_protocol")
        self.assertFalse(followup["semantic_retry"])
        self.assertIn("named Rust function", followup["feedback"])
        self.assertFalse(led.all_obligations_satisfied)

    def test_unresolved_resource_is_not_guessed_or_blindly_repaired(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {}, "violated": {},
                       "ambiguous": [{"rust": "a", "reason": "duplicate runtime resource name"}]})
            led = _ev(r)
            followup = decide_followup(led, r)
        self.assertEqual(followup["category"], "capability_gap")
        self.assertEqual(followup["action"], "stop")

    def test_baseline_oracle_does_not_guess_identity_by_order(self):
        from cir_workflow import rust_oracle
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            cir = root / "c.json"; contract = root / "ct.json"
            cir.write_text("{}")
            contract.write_text("{}")
            unresolved = {"unrelated#1": {"reason": "no structural match"}}
            binding = {"verified": {}, "unresolved": unresolved, "violated": {}}
            wrapped = {"annotated": "fn main(){}", "runtime": "", "limitations": [],
                       "resources": [{"name": "unrelated#1", "kind": "Mutex"}]}
            with patch.object(rust_oracle, "instrument_wrappers", return_value=wrapped), \
                    patch.object(rust_oracle, "cargo_build", return_value=(True, "ok")), \
                    patch.object(rust_oracle, "run_native", return_value=[]), \
                    patch.object(rust_oracle, "auto_mapping", side_effect=AssertionError("order guessing")), \
                    patch("cir_workflow.binding.bind", return_value=binding), \
                    patch("cir_workflow.bounded_monitor.run_monitor", return_value={"properties": []}):
                result = rust_oracle.evaluate("fn main(){}", contract, cir, root / "work")
            self.assertEqual(result["mapping"], {})
            self.assertEqual(result["mapping_provenance"]["unresolved"], unresolved)
            self.assertFalse(result["behavior_ok"])

    def test_live_report_does_not_treat_stale_summary_as_current_results(self):
        import importlib.util
        from cir_workflow.multi_gen import write_cell_cache
        script = Path(__file__).resolve().parents[2] / "scripts/diagnose_generation.py"
        spec = importlib.util.spec_from_file_location("diagnose_generation", script)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "SUMMARY.json").write_text(json.dumps({"cells": [{"accepted": True}] * 72}))
            write_cell_cache(root / "t" / "m" / "G3_concir" / "rep0",
                             {"status": "executed", "fingerprint": "f", "accepted": False,
                              "record_status": "capability_gap", "calls": 1})
            result = module.read_report(root)
        self.assertEqual(result["completed_cells"], 1)
        self.assertEqual(result["groups"][0]["arm_accepted"], 0)

    def test_live_entry_does_not_report_partial_matrix_as_success(self):
        import importlib.util
        import contextlib
        import io
        script = Path(__file__).resolve().parents[2] / "scripts/v23_live.py"
        spec = importlib.util.spec_from_file_location("live_partial_control", script)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        cfg = _config(["t"])
        result = {"real_requests": 0, "stop": None, "cells": [{
            "cell": {"task": "t", "model_id": "qwen3.8-flash", "arm": "G0_direct", "rep": 0},
            "status": "transport_error", "error": "control", "calls": 0, "record": {}}]}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            with patch.object(module, "OUT", root), patch.object(module, "load_dotenv"), \
                    patch.object(module, "load_config", return_value=cfg), \
                    patch.object(module, "load_gen_tasks", return_value=[]), \
                    patch.object(module, "preflight", return_value=[]), \
                    patch.object(module, "execute_matrix", return_value=result), \
                    patch("sys.argv", ["v23_live.py"]), contextlib.redirect_stdout(io.StringIO()):
                code = module.main()
            saved = json.loads((root / "SUMMARY.json").read_text())
        self.assertEqual(code, 3)
        self.assertFalse(saved["matrix_complete"])
        self.assertEqual(saved["unfinished_by_status"], {"transport_error": 1})

    def test_live_entry_checks_run_version_before_paid_preflight(self):
        import importlib.util
        import contextlib
        import io
        script = Path(__file__).resolve().parents[2] / "scripts/v23_live.py"
        spec = importlib.util.spec_from_file_location("live_pin_control", script)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            state = root / "t" / "m" / "G0_direct" / "rep0" / "state.json"
            state.parent.mkdir(parents=True); state.write_text("{}")
            with patch.object(module, "OUT", root), patch.object(module, "load_dotenv"), \
                    patch.object(module, "load_config", return_value=_config(["t"])), \
                    patch.object(module, "preflight", side_effect=AssertionError("paid call")), \
                    patch("sys.argv", ["v23_live.py"]), contextlib.redirect_stdout(io.StringIO()):
                code = module.main()
        self.assertEqual(code, 2)

    def test_unversioned_partial_run_blocks_pending_cells_before_client_creation(self):
        made = []
        def factory(*args):
            made.append(1)
            raise AssertionError("must not construct a client")
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            state = root / "t" / "m" / "G0_direct" / "rep0" / "state.json"
            state.parent.mkdir(parents=True)
            state.write_text('{"fingerprint":"old","calls":[]}')
            before = state.read_bytes()
            r = execute_matrix(_config(["t"], arms=["G0_direct"]), root,
                               client_factory=factory, tasks_by_id={"t": None}, binary=Path("missing"))
            self.assertEqual(r["stop"], "protocol_stop")
            self.assertEqual(r["calls"], 0)
            self.assertEqual(state.read_bytes(), before)
            self.assertFalse((root / "WORKFLOW.json").exists())
        self.assertEqual(made, [])

    def test_tool_content_change_blocks_cache_resume_without_new_calls(self):
        calls = []
        def factory(spec, cell, budget, out_dir):
            return _Mock(spec, budget)
        def runner(cell, client, task, out_dir):
            client.complete("s", "u")
            calls.append(1)
            return {"accepted": True}
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            binary = root / "tool"
            binary.write_bytes(b"tool-v1")
            cfg = _config(["t"], arms=["G0_direct"])
            execute_matrix(cfg, root / "run", client_factory=factory, tasks_by_id={"t": None},
                           binary=binary, stage_runner=runner)
            before = (root / "run").glob("*/**/cache.json")
            snapshots = {p: p.read_bytes() for p in before}
            binary.write_bytes(b"tool-v2")
            second = execute_matrix(cfg, root / "run", client_factory=factory,
                                    tasks_by_id={"t": None}, binary=binary, stage_runner=runner)
            self.assertEqual(second["cells"][0]["status"], "protocol_stop")
            self.assertTrue(all(i["status"] == "not_started" for i in second["cells"][1:]))
            self.assertEqual(second["stop"], "protocol_stop")
            self.assertEqual(second["calls"], 0)
            self.assertTrue(all(p.read_bytes() == body for p, body in snapshots.items()))
        self.assertEqual(len(calls), len(cfg["models"]))


if __name__ == "__main__":
    unittest.main()
