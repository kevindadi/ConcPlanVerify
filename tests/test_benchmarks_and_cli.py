"""Tests for pipeline.benchmarks loader and unified v4 CLI (__main__.py)."""

from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from concplan.__main__ import main as cli_main
from concplan.pipeline.benchmarks import (
    benchmarks_summary,
    list_benchmarks,
    load_benchmark,
)
from tests._helpers import real_binary


class BenchmarksLoaderTests(unittest.TestCase):
    def test_discovers_families_and_real_cases(self):
        tasks = list_benchmarks()
        families = [t for t in tasks if t.suite == "families"]
        real_cases = [t for t in tasks if t.suite == "real-cases"]

        self.assertGreaterEqual(len(families), 20)
        self.assertGreaterEqual(len(real_cases), 2)

        family_names = {t.family for t in families}
        self.assertTrue(
            {"lock-order", "condvar", "channel", "semaphore", "atomic-data", "structure"}
            .issubset(family_names)
        )

    def test_load_family_task_reads_requirements_contract_and_reference_cirs(self):
        task = load_benchmark("lock-order/abba_2lock")
        self.assertEqual(task.suite, "families")
        self.assertEqual(task.family, "lock-order")
        self.assertEqual(task.case, "abba_2lock")
        self.assertEqual(task.tier, "Medium")

        req_md = task.load_requirements_text()
        self.assertIn("R1.", req_md)
        self.assertIn("Entities", req_md)

        req_json = task.load_requirements_json()
        self.assertIsNotNone(req_json)
        self.assertEqual(len(req_json["requirements"]), 9)

        clauses = task.load_requirements_clauses()
        self.assertEqual(len(clauses), 9)
        self.assertIn("R1", clauses)

        contract = task.load_contract()
        self.assertIn("properties", contract)

        fixed = task.load_reference_cir("fixed")
        buggy = task.load_reference_cir("buggy")
        self.assertIn("modules", fixed)
        self.assertIn("modules", buggy)

    def test_load_real_case_task(self):
        task = load_benchmark("real-cases/rmw-zenoh-998")
        self.assertEqual(task.suite, "real-cases")
        contract = task.load_contract()
        self.assertIn("properties", contract)
        buggy = task.load_reference_cir("buggy")
        fixed = task.load_reference_cir("fixed")
        self.assertIn("modules", buggy)
        self.assertIn("modules", fixed)
        self.assertTrue(task.load_requirements_text())

    def test_benchmarks_summary_schema(self):
        summary = benchmarks_summary()
        self.assertEqual(summary["schema"], "concir-benchmarks-summary/1")
        self.assertEqual(summary["total"], summary["families_count"] + summary["real_cases_count"])
        self.assertGreaterEqual(summary["families_count"], 20)
        self.assertGreaterEqual(summary["real_cases_count"], 2)


class UnifiedCliTests(unittest.TestCase):
    def test_cli_list_benchmarks(self):
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = cli_main(["list-benchmarks"])
        self.assertEqual(rc, 0)
        payload = json.loads(buf.getvalue())
        self.assertEqual(payload["schema"], "concir-benchmarks-summary/1")
        ids = {t["task_id"] for t in payload["tasks"]}
        self.assertIn("lock-order/abba_2lock", ids)
        self.assertIn("real-cases/rmw-zenoh-998", ids)

        # Suite filter
        buf2 = io.StringIO()
        with contextlib.redirect_stdout(buf2):
            rc2 = cli_main(["list-benchmarks", "--suite", "real-cases"])
        self.assertEqual(rc2, 0)
        payload2 = json.loads(buf2.getvalue())
        self.assertTrue(all(t["suite"] == "real-cases" for t in payload2["tasks"]))

    def test_cli_e2e_scripted_with_benchmark_task(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")

        task = load_benchmark("lock-order/abba_2lock")
        fixed_cir = task.load_reference_cir("fixed")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script_path = root / "script.json"
            script_path.write_text(
                json.dumps([{"text": json.dumps(fixed_cir)}]), encoding="utf-8"
            )
            report_path = root / "e2e_report.json"
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = cli_main([
                    "--binary", str(binary),
                    "--out", str(root / "out"),
                    "e2e",
                    "--task", "lock-order/abba_2lock",
                    "--provider", "scripted",
                    "--script", str(script_path),
                    "--ast-only",
                    "--report", str(report_path),
                ])
            self.assertEqual(rc, 0, buf.getvalue())
            out = json.loads(buf.getvalue())
            self.assertEqual(out["status"], "verified")
            self.assertTrue(out["ok"])
            self.assertTrue(report_path.is_file())
            self.assertTrue(Path(out["manifest_path"]).is_file())


if __name__ == "__main__":
    unittest.main()
