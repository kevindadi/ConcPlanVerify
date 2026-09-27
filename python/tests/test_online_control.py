"""Online entry control flow: tool failures stop generation retries."""

from __future__ import annotations

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
RUST = "```rust\nfn main() {}\n```"


class _FakeClient:
    def __init__(self, text: str = RUST) -> None:
        self.calls = 0
        self.text = text

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(text=self.text, usage=None, wall_ms=1,
                               prompt_sha256="x", sent=None)


def _wrapped():
    return {"annotated": "fn main() {}", "runtime": "", "resources": [],
            "limitations": []}


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
            mock.patch.object(rust_oracle, "run_native",
                              return_value=[{"completed": True, "timed_out": False}]),
            mock.patch.object(generation, "_conform_all_op_resource",
                              return_value={"traces": 1, "statuses": {"conformant": 1},
                                            "conformant": 1, "first_violation": None,
                                            "violations": []}),
            mock.patch.object(bounded_monitor, "run_monitor",
                              return_value={"status": "ok", "properties": []}),
            mock.patch.object(generation, "_cir_props_for",
                              return_value=({"no-deadlock": "PASS"}, True)),
        ]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.tmp.cleanup()

    def test_binding_tool_failure_stops_generation(self):
        client = _FakeClient()
        with mock.patch("cir_workflow.binding.bind",
                        side_effect=BindingUnavailable("boom")):
            record = run_llmcode_from_cir(client, Path("/bin/true"), self.task,
                                          self.cir, self.out, k_code=3)
        self.assertEqual(client.calls, 1, "tool failure must not retry the model")
        self.assertEqual(record["status"], "binding_tool_error")
        self.assertEqual(record["action"], "tool_failure")
        # every round has an evaluation record
        self.assertTrue(record["rounds"])

    def test_instrument_failure_stops_generation(self):
        client = _FakeClient()
        with mock.patch.object(rust_oracle, "instrument_wrappers",
                               side_effect=RuntimeError("instrument blew up")):
            record = run_llmcode_from_cir(client, Path("/bin/true"), self.task,
                                          self.cir, self.out, k_code=3)
        self.assertEqual(client.calls, 1)
        self.assertEqual(record["action"], "tool_failure")

    def test_repairable_error_retries_with_feedback(self):
        # A conformance violation is repairable: the next round gets feedback.
        client = _FakeClient()
        seen_feedback = []
        orig = client.complete

        def complete(system, user):
            seen_feedback.append("Feedback on that Rust program:" in user)
            return orig(system, user)

        client.complete = complete
        with mock.patch("cir_workflow.binding.bind",
                        return_value={"verified": {"m#1": {"cir": "main::m"}},
                                      "unresolved": {}, "violated": {}}), \
                mock.patch.object(generation, "_conform_all_op_resource",
                                  return_value={"traces": 1,
                                                "statuses": {"violation": 1},
                                                "conformant": 0,
                                                "first_violation": {
                                                    "status": "violation",
                                                    "got": "mutex_lock:main::m"},
                                                "violations": [{"status": "violation",
                                                                "got": "mutex_lock:main::m",
                                                                "resource": "main::m"}]}):
            run_llmcode_from_cir(client, Path("/bin/true"), self.task, self.cir,
                                 self.out, k_code=2)
        self.assertGreaterEqual(client.calls, 2)
        # the second round received non-empty feedback
        self.assertTrue(any(seen_feedback[1:]), "repair round must carry feedback")


    def test_capability_gap_does_not_blind_repair(self):
        # A correct program whose value property has no checker must stop, not
        # retry with empty feedback.
        fix = REPO / "python/tests/fixtures/stronglink"
        task = SimpleNamespace(id="compute", requirements_md="print DONE done=6",
                               contract_path=fix / "compute.contract.json",
                               contract=json.loads((fix / "compute.contract.json").read_text()),
                               requirements=["R1", "R2"], unverifiable=[],
                               reference_cir_path=fix / "compute.cir.json")
        client = _FakeClient("```rust\n" + (fix / "compute_ok.rs").read_text() + "\n```")
        with mock.patch("cir_workflow.binding.bind",
                        return_value={"verified": {}, "unresolved": {}, "violated": {}}):
            record = run_llmcode_from_cir(client, Path("/bin/true"), task,
                                          fix / "compute.cir.json", self.out, k_code=2)
        self.assertEqual(client.calls, 1, "capability gap must not retry the model")
        self.assertEqual(record["action"], "capability_gap")


if __name__ == "__main__":
    unittest.main()
