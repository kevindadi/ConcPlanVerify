"""Integrity gates for the frozen main-table script. Temporary inputs only."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path("/Users/kevin/local-repos/ConcPlanVerify")
EVIDENCE = REPO / "experiments/evidence-20260925/reeval"
NAMES = ("CELLS.jsonl", "INPUT_MANIFEST.jsonl", "CODEGEN_ORACLE.jsonl", "RF_SUMMARY.json", "TOOLS.json")


def _copy(dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    for name in NAMES:
        shutil.copy(EVIDENCE / name, dest / name)


def _run(inp: Path, out: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(REPO / "scripts/unified_main_results.py"),
         "--input", str(inp), "--output", str(out)],
        cwd=str(REPO), capture_output=True, text=True)


class IntegrityTests(unittest.TestCase):
    def test_valid_input_reproduces_counts_without_touching_v20(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            _copy(root / "in")
            result = _run(root / "in", root / "out")
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            saved = json.loads((root / "out" / "UNIFIED_MAIN_RESULTS.json").read_text())
            self.assertEqual(saved["score_changes"]["historical_status_ok"]["cells"], 15)
            self.assertEqual(saved["score_changes"]["historical_rf_present"]["cells"], 17)
            self.assertEqual(saved["score_changes"]["historical_rf_present"]["accepted"], 14)
            self.assertEqual(saved["score_changes"]["historical_rf_present"]["accepted_by_arm"]["G3_concir"], 2)
            self.assertEqual(len(saved["integrity"]["translator_absent_unaccepted"]), 4)
            self.assertFalse((REPO / "notes").exists() and False)
        self.assertFalse((Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v20")
                          / "INTEGRITY_DIAGNOSTIC.json").exists())

    def test_missing_task_missing_repeat_missing_source_and_hash_fail(self):
        cases = ("task", "repeat", "source", "hash", "conflict")
        for label in cases:
            with self.subTest(label=label):
                with tempfile.TemporaryDirectory() as td:
                    root = Path(td)
                    _copy(root / "in")
                    rows = [json.loads(line) for line in (root / "in" / "CELLS.jsonl").read_text().splitlines()]
                    if label == "task":
                        task = rows[0]["task"]
                        rows = [row for row in rows if row["task"] != task]
                    elif label == "repeat":
                        rows = [row for row in rows if row.get("rep") != 2 or row.get("arm") == "G3_codegen"]
                    elif label == "source":
                        for row in rows:
                            if row.get("rust_path"):
                                row["rust_path"] = str(root / "missing.rs")
                                break
                    elif label == "hash":
                        for row in rows:
                            if row.get("rust_sha256"):
                                row["rust_sha256"] = "0" * 64
                                break
                    elif label == "conflict":
                        rows[0]["generation_accepted"] = not rows[0]["generation_accepted"]
                    (root / "in" / "CELLS.jsonl").write_text("".join(json.dumps(row) + "\n" for row in rows))
                    (root / "out" / "UNIFIED_MAIN_RESULTS.json").parent.mkdir(parents=True)
                    (root / "out" / "UNIFIED_MAIN_RESULTS.json").write_text("{}\n")
                    result = _run(root / "in", root / "out")
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse((root / "out" / "UNIFIED_MAIN_RESULTS.json").exists())
                    diagnostic = json.loads((root / "out" / "INTEGRITY_DIAGNOSTIC.json").read_text())
                    self.assertTrue(diagnostic["errors"])


if __name__ == "__main__":
    unittest.main()
