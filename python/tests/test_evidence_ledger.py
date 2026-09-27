"""Strong-link evidence ledger: no upgrade of failures/limits into sufficient."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.evidence import contract_properties, evaluate_cell


def _mk_cell(root: Path, *, accepted: bool, round_no: int | None,
             monitor_props: list[dict], events: int, cir_props: list[dict],
             cir_complete: bool = True, kind: str = "reachable",
             behavior_ok: bool | None = True) -> tuple[Path, dict, dict]:
    cell = root / "cell"
    (cell / "cir" / "calls").mkdir(parents=True)
    cir = {"modules": [{"functions": [{"body": [{"kind": "mutex_lock"},
                                                 {"kind": "mutex_unlock"}]}]}]}
    cir_path = cell / "cir" / "revision-1.cir.json"
    cir_path.write_text(json.dumps(cir))
    explore = cell / "cir" / "calls" / "x-explore"
    explore.mkdir()
    (explore / "program.json").write_text(json.dumps(cir))
    (explore / "stdout.json").write_text(json.dumps(
        {"complete": cir_complete, "outcome": "PASS", "properties": cir_props}))
    if round_no:
        rnd = cell / "code" / f"round-{round_no}"
        rnd.mkdir(parents=True)
        (cell / "code" / f"round-{round_no}.rs").write_text("fn main() {}\n")
        (rnd / "monitor.json").write_text(json.dumps(
            {"events": events, "properties": monitor_props}))
        (rnd / "conform-traces").mkdir()
        (rnd / "conform-traces" / "t.jsonl").write_text("\n".join(
            ['{"op":"mutex_lock","r":"main::m","sid":"s1"}'] * events) + "\n")
    contract = {"properties": [{"id": "p1", "kind": kind, "req": ["R1"]}]}
    row = {"accepted": accepted, "cir_accepted": True, "cell": "m/t",
           "first_code_accept_round": round_no, "behavior_ok": behavior_ok}
    return cell, row, contract


class LedgerRegressionTests(unittest.TestCase):
    def test_not_accepted_does_not_fall_back(self):
        with tempfile.TemporaryDirectory() as td:
            cell, row, contract = _mk_cell(
                Path(td), accepted=False, round_no=3,
                monitor_props=[{"id": "p1", "status": "PASS_bounded"}],
                events=10, cir_props=[{"id": "p1", "outcome": "PASS"}])
            ev = evaluate_cell(cell, row, contract)
            self.assertEqual(ev.verdict, "not_accepted")
            self.assertFalse(ev.evidence_sufficient)

    def test_behavior_failure_not_overridden_by_cir(self):
        with tempfile.TemporaryDirectory() as td:
            cell, row, contract = _mk_cell(
                Path(td), accepted=True, round_no=1, kind="deadlock_free",
                monitor_props=[{"id": "p1", "status": "deferred"}],
                events=5, cir_props=[{"id": "p1", "outcome": "PASS"}],
                behavior_ok=False)
            ev = evaluate_cell(cell, row, contract)
            self.assertEqual(ev.verdict, "explicit_failure")
            self.assertEqual(ev.properties[0].final_claim, "violated")

    def test_unsupported_not_upgraded_by_cir_and_events(self):
        with tempfile.TemporaryDirectory() as td:
            cell, row, contract = _mk_cell(
                Path(td), accepted=True, round_no=1, kind="var_eq",
                monitor_props=[{"id": "p1", "status": "unsupported"}],
                events=200, cir_props=[{"id": "p1", "outcome": "PASS"}])
            ev = evaluate_cell(cell, row, contract)
            self.assertEqual(ev.properties[0].final_claim, "inconclusive")
            self.assertEqual(ev.verdict, "insufficient")

    def test_empty_properties_insufficient(self):
        with tempfile.TemporaryDirectory() as td:
            cell, row, _ = _mk_cell(
                Path(td), accepted=True, round_no=1,
                monitor_props=[], events=3, cir_props=[])
            ev = evaluate_cell(cell, row, {"properties": [], "preserved": []})
            self.assertEqual(ev.verdict, "insufficient")

    def test_events_but_missing_property_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as td:
            cell, row, contract = _mk_cell(
                Path(td), accepted=True, round_no=1, kind="reachable",
                monitor_props=[{"id": "other", "status": "PASS_bounded"}],
                events=10, cir_props=[{"id": "p1", "outcome": "PASS"}])
            ev = evaluate_cell(cell, row, contract)
            self.assertEqual(ev.properties[0].observed_trace_result, "not_observed")
            self.assertEqual(ev.properties[0].final_claim, "inconclusive")
            self.assertEqual(ev.verdict, "insufficient")


class ContractEnumerationTests(unittest.TestCase):
    def test_properties_and_preserved_are_both_enumerated(self):
        contract = {"properties": [{"id": "a", "kind": "deadlock_free", "req": ["R1"]}],
                    "preserved": [{"description": "b", "kind": "reachable", "req": ["R2"]}]}
        ids = [p["id"] for p in contract_properties(contract)]
        self.assertEqual(ids, ["a", "b"])


if __name__ == "__main__":
    unittest.main()
