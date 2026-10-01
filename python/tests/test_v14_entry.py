"""The batch entry itself, with AuditedClient and a fake transport."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")
CIR = (REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash/DeepSeek Flash"
       / "channel__send_while_holding_mutex/rep0/cir/revision-1.cir.json")
CONTRACT = REPO / "benchmarks/families/channel/send_while_holding_mutex/contract.json"
CONTROL = (Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v11/derived_control.rs")
           .read_text(encoding="utf-8"))


def _load_entry():
    spec = importlib.util.spec_from_file_location("v12_entry", REPO / "scripts/v12_three_arm.py")
    entry = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(entry)
    return entry


def _score(source, _work):
    if "sync_channel" in source:
        return {"status": "bounded_covered_satisfied", "runs": []}
    return {"status": "fail", "runs": [{"kind": "completed", "returncode": 0,
                                        "stdout": "DONE done=1\n", "timed_out": False}]}


class EntryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.entry = _load_entry()
        req = self.root / "requirements.txt"
        req.write_text("audit fixture", encoding="utf-8")
        defect = self.root / "defect.rs"
        defect.write_text("fn main() {}\n", encoding="utf-8")
        self.selected = self.root / "selected.json"
        self.selected.write_text(json.dumps({"selected_records": [{
            "task": "channel/send_while_holding_mutex",
            "source_path": str(defect),
            "requirements_path": str(req),
            "cir_path": str(CIR),
            "contract_path": str(CONTRACT),
        }]}), encoding="utf-8")
        self.calls = []
        self.freeze_args = []

    def tearDown(self):
        self.tmp.cleanup()

    def _patches(self, models):
        entry = self.entry
        original = entry.run_repairs

        def runner(*args, **kwargs):
            self.freeze_args.append(kwargs.get("freeze_files"))
            return original(*args, **kwargs)

        def build_client(spec, *, budget, **kwargs):
            parent = self

            class Fake:
                def complete(self, system, user):
                    parent.calls.append(spec.model_id)
                    budget.reserve()
                    if spec.model_id == "deepseek-flash":
                        return SimpleNamespace(text="```rust\n" + CONTROL + "\n```",
                                               response_model=None, usage={"prompt_tokens": 3, "completion_tokens": 1},
                                               wall_ms=1, finish_reason="stop")
                    return SimpleNamespace(text="```rust\n" + CONTROL + "\n```",
                                           response_model=spec.model_id,
                                           usage={"prompt_tokens": 4, "completion_tokens": 2},
                                           wall_ms=1, finish_reason="stop")

            return Fake()

        return patch.object(entry, "MODELS", models), patch.object(entry, "load_dotenv"), \
            patch.object(entry, "key_for", return_value="fake-no-secret"), \
            patch.object(entry, "build_client", side_effect=build_client), \
            patch.object(entry, "_eval", return_value={}), \
            patch.object(entry, "evaluate_requirements", side_effect=_score), \
            patch.object(entry, "run_repairs", side_effect=runner)

    def _run(self, out: Path, confirm: bool) -> int:
        argv = ["v12_three_arm.py", "--selected", str(self.selected), "--out", str(out)]
        if confirm:
            argv.append("--confirm-paid-run")
        with patch.object(sys, "argv", argv):
            return self.entry.main()

    def test_tampered_protocol_is_not_overwritten_and_sends_nothing(self):
        out = self.root / "tamper"
        patches = self._patches(["DeepSeek Flash"])
        with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5], patches[6]:
            self.assertEqual(self._run(out, False), 0)
            protocol = json.loads((out / "PROTOCOL.json").read_text(encoding="utf-8"))
            protocol["max_repairs"] = 99
            (out / "PROTOCOL.json").write_text(json.dumps(protocol), encoding="utf-8")
            frozen_rows = (out / "RESULTS.json").read_text(encoding="utf-8")
            self.calls.clear()
            code = self._run(out, True)
        protocol_text = (out / "PROTOCOL.json").read_text(encoding="utf-8")
        self.assertEqual(code, 2)
        self.assertEqual(json.loads(protocol_text)["max_repairs"], 99)
        self.assertEqual((out / "RESULTS.json").read_text(encoding="utf-8"), frozen_rows)
        self.assertEqual(self.calls, [])
        self.assertFalse((out / "budget.json").exists())
        rejection = json.loads((out / "RUN_REJECTION.json").read_text(encoding="utf-8"))
        self.assertEqual(rejection[-1]["reason"], "frozen_protocol_mismatch")
        rows = json.loads(frozen_rows)
        self.assertTrue(all(row["status"] == "not_started" for row in rows))

    def test_missing_identity_blocks_only_that_model_and_restart_does_not_resend(self):
        out = self.root / "identity"
        patches = self._patches(["DeepSeek Flash", "Qwen"])
        with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5], patches[6]:
            self.assertEqual(self._run(out, False), 0)
            self.assertEqual(self._run(out, True), 0)
            first = list(self.calls)
            before = json.loads((out / "RESULTS.json").read_text(encoding="utf-8"))
            self.calls.clear()
            self.assertEqual(self._run(out, True), 0)
            second = list(self.calls)
            after = json.loads((out / "RESULTS.json").read_text(encoding="utf-8"))
            self.assertEqual(before, after)
        self.assertTrue(self.freeze_args)
        self.assertTrue(all(item for item in self.freeze_args))
        self.assertEqual(first.count("deepseek-flash"), 1)
        self.assertGreater(first.count("qwen3.8-flash"), 0)
        self.assertEqual(second.count("deepseek-flash"), 0)
        self.assertEqual(second.count("qwen3.8-flash"), 0)
        rows = json.loads((out / "RESULTS.json").read_text())
        summary = json.loads((out / "SUMMARY.json").read_text())["results"]
        self.assertEqual(len(rows), 6)
        self.assertEqual([row["status"] for row in rows], [row["status"] for row in summary])
        deepseek = [row for row in rows if row["model_id"] == "deepseek-flash"]
        self.assertEqual(deepseek[0]["status"], "executed")
        self.assertEqual(deepseek[0]["stop"], "identity_unconfirmed")
        self.assertTrue(all(row["status"] == "blocked" for row in deepseek[1:]))
        saved = {name: (out / name).read_bytes() for name in
                 ("PROTOCOL.json", "RESULTS.json", "SUMMARY.json", "REQUEST_EVENTS.jsonl", "budget.json")}
        with patches[0], patches[1], patches[2], patches[3], patches[4], patches[5], patches[6]:
            self.assertEqual(self._run(out, False), 0)
        for name, data in saved.items():
            self.assertEqual((out / name).read_bytes(), data, name)
        self.assertTrue(all(row["status"] == "executed" for row in rows if row["model_id"] == "qwen3.8-flash"))
        events = (out / "REQUEST_EVENTS.jsonl").read_text(encoding="utf-8").strip().splitlines()
        self.assertEqual(sum(1 for line in events if '"deepseek-flash/' in line), 1)
