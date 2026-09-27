"""CLI integration test for common_eval_v2: real main() path, full cell matrix."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("common_eval_v2", REPO / "scripts" / "common_eval_v2.py")
ce = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ce)

TASKS = ["lock-order/abba_2lock", "lock-order/cycle_3lock"]


def _fake_evaluator(rust, task, work, binary, instrument):
    rf = float(Path(rust).read_text().strip() or "0")
    return {"status": "ok", "failure": None, "coverage": {"rf": rf, "total": 1,
            "satisfied": int(rf)}, "behavior_ok": True, "hang": False,
            "monitor_status": "ok", "limitations": []}


def _write_batch(root: Path, arm: str, cells: list[dict]) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "SUMMARY.json").write_text(json.dumps({"cells": cells}) + "\n")
    for c in cells:
        if c.get("_skip_cell"):
            continue
        cdir = root / "M" / c["task"].replace("/", "__") / f"rep{c['replicate']}"
        cdir.mkdir(parents=True, exist_ok=True)
        rf = c.get("_rf", 0.0)
        if arm == "G2":
            (cdir / "run-1" / "round-1").mkdir(parents=True, exist_ok=True)
            cand = cdir / "run-1" / "round-1" / "candidate.rs"
            cand.write_text(str(rf))
            (cdir / "CELL.json").write_text(json.dumps(
                {"accepted": c.get("accepted"), "final_artifact_path": str(cand)}))
        else:
            (cdir / "code").mkdir(parents=True, exist_ok=True)
            (cdir / "code" / "round-1.rs").write_text(str(rf))


class CommonEvalCliTests(unittest.TestCase):
    def _run(self, tmp: Path) -> dict:
        g2 = tmp / "g2"; g3 = tmp / "g3"
        # G2: t1@0 acc(eval .5); t1@1 not delivered; t2@0 acc(eval 0.0); t2@1 accepted but no eval
        _write_batch(g2, "G2", [
            {"task": TASKS[0], "replicate": 0, "accepted": True, "_rf": 0.5},
            {"task": TASKS[0], "replicate": 1, "accepted": False, "_rf": 0.0},
            {"task": TASKS[1], "replicate": 0, "accepted": True, "_rf": 0.0},
            {"task": TASKS[1], "replicate": 1, "accepted": True, "_rf": 0.0,
             "_missing_eval": True},
        ])
        # G3: t1@0 acc(1.0); t1@1 MISSING cell; t2@0 acc(.5); t2@1 not delivered
        _write_batch(g3, "G3", [
            {"task": TASKS[0], "replicate": 0, "accepted": True, "_rf": 1.0,
             "first_code_accept_round": 1},
            {"task": TASKS[1], "replicate": 0, "accepted": True, "_rf": 0.5,
             "first_code_accept_round": 1},
            {"task": TASKS[1], "replicate": 1, "accepted": False, "_rf": 0.0},
        ])
        pairs = tmp / "pairs.json"
        pairs.write_text(json.dumps({"M": {"g2": str(g2), "g3": str(g3)}}))
        out = tmp / "out"
        old_eval = ce._EVALUATOR
        old_argv = sys.argv

        def fake(rust, task, work, binary, instrument):
            # simulate an unevaluated accepted cell
            if "rep1" in str(rust) and "cycle_3lock" in str(rust):
                return {"status": "build_failed", "failure": "build_failed_after_instrument",
                        "coverage": None, "behavior_ok": None, "hang": None,
                        "monitor_status": None, "limitations": []}
            return _fake_evaluator(rust, task, work, binary, instrument)

        ce._EVALUATOR = fake
        sys.argv = ["common_eval_v2", "--pairs", str(pairs), "--out", str(out)]
        try:
            ce.main()
        finally:
            ce._EVALUATOR = old_eval
            sys.argv = old_argv
        return json.loads((out / "COMMON_EVAL_V2.json").read_text())

    def test_full_matrix_metrics(self):
        with tempfile.TemporaryDirectory() as td:
            rep = self._run(Path(td))["M"]
        g2, g3 = rep["G2"], rep["G3"]
        self.assertEqual(g2["cells_expected"], 4)
        self.assertEqual(g2["cells_present"], 4)
        self.assertEqual(g2["cells_missing"], 0)
        self.assertEqual(g2["not_delivered"], 1)
        self.assertEqual(g2["accepted_missing"], 1)
        self.assertAlmostEqual(g2["delivered_rf_all"], 0.125, places=4)
        self.assertAlmostEqual(g2["accepted_quality_rf"], 0.25, places=4)
        # G3 is missing t1@rep1 entirely (not silently intersected away)
        self.assertEqual(g3["cells_expected"], 4)
        self.assertEqual(g3["cells_present"], 3)
        self.assertEqual(g3["cells_missing"], 1)
        self.assertAlmostEqual(g3["delivered_rf_all"], 0.375, places=4)
        self.assertAlmostEqual(g3["accepted_quality_rf"], 0.75, places=4)

    def test_summarize_empty_is_empty(self):
        self.assertEqual(ce.summarize([], {}), {})


if __name__ == "__main__":
    unittest.main()
