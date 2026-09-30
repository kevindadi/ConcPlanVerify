"""Online entry control flow through run_llmcode_from_cir and a fake client."""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from cir_workflow import bounded_monitor, generation, rust_oracle
from cir_workflow.binding import BindingUnavailable
from cir_workflow.generation import load_gen_tasks, run_llmcode_from_cir

REPO = Path(__file__).resolve().parents[2]
FIX = REPO / "python/tests/fixtures/stronglink"
RUST = "```rust\nfn main() {}\n```"


class _FakeClient:
    def __init__(self, text: str = RUST) -> None:
        self.calls = 0
        self.text = text
        self.users: list[str] = []

    def complete(self, system, user):
        self.calls += 1
        self.users.append(user)
        return SimpleNamespace(text=self.text, usage=None, wall_ms=1,
                               prompt_sha256="x", sent=None)


def _wrapped():
    return {"annotated": "fn main() {}", "runtime": "", "resources": [],
            "limitations": []}


def _write_trace(project, traces_dir, n=32, timeout=10.0):
    traces_dir = Path(traces_dir)
    traces_dir.mkdir(parents=True, exist_ok=True)
    path = traces_dir / "native-000.jsonl"
    path.write_text('{"op":"mutex_lock","r":"m"}\n', encoding="utf-8")
    return [{"completed": True, "timed_out": False, "index": 0}]


def _conform(status: str, got: str | None = None):
    def run(binary, cir_path, traces_dir):
        traces = sorted(Path(traces_dir).glob("*.jsonl"))
        raw = []
        body = {"status": status}
        if got:
            body.update({"got": got, "event_index": 1, "expected": ["mutex_lock:main::a"]})
        text = json.dumps(body)
        for trace in traces:
            raw.append({"trace": str(trace),
                        "trace_sha256": hashlib.sha256(trace.read_bytes()).hexdigest(),
                        "stdout": text})
        count = len(raw) or 1
        item = json.loads(text)
        return {"traces": count, "statuses": {status: count},
                "conformant": count if status == "conformant" else 0,
                "first_violation": None if status == "conformant" else item,
                "violations": [] if status == "conformant" else [item],
                "raw": raw}
    return run


def _model(props: dict, complete: bool = True):
    def run(binary, cir_path, contract_path, out_dir):
        out_dir = Path(out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        payload = {"properties": [{"id": k, "outcome": v} for k, v in props.items()],
                   "complete": complete}
        path = out_dir / "explore.stdout"
        path.write_text(json.dumps(payload), encoding="utf-8")
        return {"ok": True, "path": str(path), "payload": payload}
    return run


class OnlineControlTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = Path(self.tmp.name)
        tasks = {t.id: t for t in load_gen_tasks(REPO)}
        self.task = tasks["lock-order/abba_2lock"]
        self.cir = self.task.reference_cir_path
        self.patches = [
            mock.patch.object(rust_oracle, "cargo_build", return_value=(True, "ok")),
            mock.patch.object(rust_oracle, "instrument_wrappers", return_value=_wrapped()),
            mock.patch.object(rust_oracle, "run_native", side_effect=_write_trace),
            mock.patch.object(generation, "_conform_all_op_resource",
                              side_effect=_conform("conformant")),
            mock.patch.object(bounded_monitor, "run_monitor",
                              return_value={"status": "ok", "properties": [
                                  {"id": "no-deadlock", "status": "PASS_bounded"}]}),
            mock.patch("cir_workflow.candidate_eval.run_model_check",
                       side_effect=_model({"no-deadlock": "PASS"})),
        ]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.tmp.cleanup()

    def _assert_every_round_evaluated(self, record):
        self.assertTrue(record["rounds"])
        for rnd in record["rounds"]:
            self.assertIsNotNone(rnd.get("evaluation"), rnd)

    def test_binding_tool_failure_stops_generation(self):
        client = _FakeClient()
        with mock.patch("cir_workflow.binding.bind",
                        side_effect=BindingUnavailable("boom")):
            record = run_llmcode_from_cir(client, Path("/bin/true"), self.task,
                                          self.cir, self.out, k_code=3)
        self.assertEqual(client.calls, 1)
        self.assertEqual(record["llm_calls"], 1)
        self.assertEqual(record["status"], "binding_tool_error")
        self.assertEqual(record["action"], "tool_failure")
        self.assertNotEqual(record["status"], "generation_failed")
        self.assertEqual(record["tool_failures"], 1)
        self.assertEqual(record["semantic_repairs"], 0)
        self._assert_every_round_evaluated(record)

    def test_instrument_failure_stops_generation(self):
        client = _FakeClient()
        with mock.patch.object(rust_oracle, "instrument_wrappers",
                               side_effect=RuntimeError("instrument blew up")):
            record = run_llmcode_from_cir(client, Path("/bin/true"), self.task,
                                          self.cir, self.out, k_code=3)
        self.assertEqual(client.calls, 1)
        self.assertEqual(record["action"], "tool_failure")
        self.assertNotEqual(record["status"], "generation_failed")
        self.assertEqual(record["semantic_repairs"], 0)
        self._assert_every_round_evaluated(record)

    def test_repairable_error_retries_with_feedback(self):
        client = _FakeClient()
        with mock.patch("cir_workflow.binding.bind",
                        return_value={"verified": {"m#1": {"cir": "main::m"}},
                                      "unresolved": {}, "violated": {}}), \
                mock.patch.object(generation, "_conform_all_op_resource",
                                  side_effect=_conform("violation", "mutex_lock:main::m")):
            record = run_llmcode_from_cir(client, Path("/bin/true"), self.task, self.cir,
                                          self.out, k_code=2)
        self.assertEqual(client.calls, 2)
        self.assertIn("mutex_lock:main::m", client.users[1])
        self.assertIn("Feedback on that Rust program:", client.users[1])
        self.assertGreaterEqual(record["semantic_repairs"], 1)
        self.assertEqual(record["tool_failures"], 0)
        self._assert_every_round_evaluated(record)
        self.assertTrue(record["rounds"][0]["evaluation"]["feedback"].strip())

    def test_format_error_is_protocol_repair_not_semantic(self):
        client = _FakeClient("not rust")
        record = run_llmcode_from_cir(client, Path("/bin/true"), self.task,
                                      self.cir, self.out, k_code=2)
        self.assertEqual(client.calls, 2)
        self.assertEqual(record["protocol_repairs"], 2)
        self.assertEqual(record["semantic_repairs"], 0)
        self.assertIn("```rust", client.users[1])
        self._assert_every_round_evaluated(record)

    def test_capability_gap_does_not_blind_repair(self):
        task = SimpleNamespace(id="compute", requirements_md="print DONE done=6",
                               contract_path=FIX / "compute.contract.json",
                               contract=json.loads((FIX / "compute.contract.json").read_text()),
                               requirements=["R1", "R2"], unverifiable=[],
                               reference_cir_path=FIX / "compute.cir.json")
        client = _FakeClient("```rust\n" + (FIX / "compute_ok.rs").read_text() + "\n```")
        props = {"no-deadlock": "PASS", "c==6": "PASS", "main::w completes": "PASS"}
        monitor = {"status": "ok", "properties": [
            {"id": "no-deadlock", "status": "PASS_bounded"},
            {"id": "c==6", "status": "unsupported"},
            {"id": "main::w completes", "status": "unsupported"},
        ]}
        with mock.patch("cir_workflow.binding.bind",
                        return_value={"verified": {"m": {"cir": "main::m"}},
                                      "unresolved": {}, "violated": {}}), \
                mock.patch("cir_workflow.candidate_eval.run_model_check",
                           side_effect=_model(props)), \
                mock.patch.object(bounded_monitor, "run_monitor", return_value=monitor), \
                mock.patch("cir_workflow.candidate_eval.capture_program_stdout",
                           return_value={"stdout": "DONE done=6\n", "stderr": "",
                                         "returncode": 0, "timed_out": False,
                                         "elapsed_s": 0.01, "error": None,
                                         "kind": "completed"}):
            record = run_llmcode_from_cir(
                client, Path("/bin/true"), task, FIX / "compute.cir.json", self.out,
                k_code=2, functional_spec={"test_id": "stdout_eq", "kind": "stdout_eq",
                                           "expected": "DONE done=6"})
        self.assertEqual(client.calls, 1)
        self.assertEqual(record["action"], "capability_gap")
        self.assertEqual(record["semantic_repairs"], 0)
        self.assertNotEqual(record["status"], "generation_failed")
        ledger = record["ledger"]
        self.assertEqual(ledger["functional"]["status"], "pass")
        self.assertEqual(ledger["functional"]["supports"], ["external_stdout"])
        internal = next(p for p in ledger["properties"] if p["property_id"] == "c==6")
        self.assertEqual(internal["obligation_state"], "unresolved")
        self.assertNotEqual(ledger["current_evaluation"], "satisfied_bounded")
        self.assertEqual(ledger["evidence_grade"], "partial_external")
        self.assertEqual(ledger["delivery_status"], "withhold_capability")
        self.assertTrue(ledger["needs_human_review"])
        self.assertTrue(ledger["needs_extra_check"])
        self._assert_every_round_evaluated(record)
        self.assertEqual(record["rounds"][0]["evaluation"]["feedback"], "")


if __name__ == "__main__":
    unittest.main()
