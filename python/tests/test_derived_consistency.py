"""Derived SUMMARY must agree with the per-cell ledgers (single source)."""

from __future__ import annotations

import json
import unittest
from collections import Counter
from pathlib import Path

QWEN = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v4/reexec-qwen")
FOUR = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v5")
FOUR6 = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v6")


class DerivedConsistencyTests(unittest.TestCase):
    def test_summary_matches_cells(self):
        if not (QWEN / "SUMMARY.json").is_file():
            self.skipTest("derived summary not present")
        summary = json.loads((QWEN / "SUMMARY.json").read_text())
        results = sorted(QWEN.glob("**/result.json"))
        # cell set matches
        self.assertEqual(summary["cells_total"], len(results))
        # verdict totals sum and match per-cell ledgers
        per_cell = Counter()
        for p in results:
            led = json.loads(p.read_text())["ledger"]
            per_cell[led["current_evaluation"]] += 1
        self.assertEqual(sum(summary["verdict_totals"].values()), summary["cells_total"])
        self.assertEqual(dict(per_cell), summary["verdict_totals"])
        # each summary cell's terminal matches its result
        for c in summary["cells"]:
            led = json.loads(Path(c["evidence_path"]).read_text())["ledger"]
            self.assertEqual(c["current_evaluation"], led["current_evaluation"])
            self.assertEqual(c["trace_state"], led["trace"]["state"])


class FourModelConsistencyTests(unittest.TestCase):
    def test_four_model_summary_matches_per_model(self):
        if not (FOUR / "SUMMARY.json").is_file():
            self.skipTest("four-model summary not present")
        combined = json.loads((FOUR / "SUMMARY.json").read_text())
        total = 0
        for row in combined["models"]:
            s = json.loads((FOUR / f"reexec-{row['model']}" / "SUMMARY.json").read_text())
            self.assertEqual(row["candidates"], s["cells_total"])
            self.assertEqual(sum(row["verdicts"].values()), s["cells_total"])
            total += row["candidates"]
        self.assertEqual(combined["total_candidates"], total)
        self.assertEqual(sum(combined["verdicts"].values()), total)


class FullMatrixTests(unittest.TestCase):
    def test_full_matrix_has_96_cells(self):
        if not (FOUR6 / "FULL_MATRIX.jsonl").is_file():
            self.skipTest("v6 matrix not present")
        rows = [json.loads(l) for l in (FOUR6 / "FULL_MATRIX.jsonl").read_text().splitlines() if l.strip()]
        self.assertEqual(len(rows), 96)
        results = sorted(FOUR6.glob("reexec-*/**/result.json"))
        self.assertEqual(len(results), 96)
        # every matrix row maps to a real result
        paths = {str(p) for p in results}
        self.assertTrue(all(r["evidence_path"] in paths for r in rows))
        # missing-candidate reasons are preserved, not dropped
        reasons = {r["current_evaluation"] for r in rows}
        self.assertTrue({"cir_not_accepted", "cir_accepted_no_rust",
                         "cir_unknown_or_raw_error"} & reasons)

    def test_binding_sufficient_excludes_not_run(self):
        if not (FOUR6 / "FULL_MATRIX.jsonl").is_file():
            self.skipTest("v6 matrix not present")
        rows = [json.loads(l) for l in (FOUR6 / "FULL_MATRIX.jsonl").read_text().splitlines() if l.strip()]
        for r in rows:
            if r["binding_sufficient"]:
                self.assertTrue(r["binding_ran"], "binding sufficient requires the stage to have run")


if __name__ == "__main__":
    unittest.main()
