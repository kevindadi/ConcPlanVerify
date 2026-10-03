"""§1 harness: `concir-backend monitor` + requirement coverage."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow import bounded_monitor
from tests._helpers import REPO

ABBA = REPO / "benchmarks/families/lock-order/abba_2lock"
FIXED_TRACE = "\n".join([
    '{"t":"t1","sid":"L1","op":"mutex_lock","r":"a"}',
    '{"t":"t1","sid":"L2","op":"mutex_lock","r":"b"}',
    '{"t":"t1","sid":"L3","op":"mutex_unlock","r":"b"}',
    '{"t":"t1","sid":"L4","op":"mutex_unlock","r":"a"}',
    '{"t":"t2","sid":"L1","op":"mutex_lock","r":"a"}',
    '{"t":"t2","sid":"L2","op":"mutex_lock","r":"b"}',
    '{"t":"t2","sid":"L3","op":"mutex_unlock","r":"b"}',
    '{"t":"t2","sid":"L4","op":"mutex_unlock","r":"a"}',
    "",
])


@unittest.skipUnless(bounded_monitor.resolve_binary() is not None,
                     "concir-backend not built")
class BoundedMonitorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads((ABBA / "contract.json").read_text(encoding="utf-8"))
        cls.reqjson = json.loads(
            (ABBA / "generation_input/requirements.json").read_text(encoding="utf-8"))

    def _run(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        (root / "trace-0.jsonl").write_text(FIXED_TRACE, encoding="utf-8")
        resources = root / "resources.json"
        resources.write_text(json.dumps({"resources": ["a", "b"]}), encoding="utf-8")
        return bounded_monitor.run_monitor(
            ABBA / "contract.json", root, resources=resources)

    def test_report_is_bounded_and_binds_clauses(self):
        report = self._run()
        self.assertTrue(report["bounded"])
        self.assertEqual(report["traces"], 1)
        self.assertEqual(report["unmapped_resources"], [])
        kinds = {p["id"]: p["status"] for p in report["properties"]}
        self.assertEqual(kinds["no-deadlock"], "deferred")
        self.assertEqual(kinds["main::t1 holds [main::a, main::b] at once"], "PASS_bounded")

    def test_requirement_coverage(self):
        report = self._run()
        cov = bounded_monitor.coverage(
            self.contract, report, self.reqjson["requirements"],
            self.reqjson["unverifiable"], behavior_ok=True)
        self.assertEqual(cov.total, 9)
        self.assertEqual(cov.statuses["R2"], "PASS_bounded")
        self.assertEqual(cov.statuses["R4"], "PASS_bounded")  # deadlock resolved
        self.assertEqual(cov.statuses["R1"], "not_observed")  # completion unwitnessed
        self.assertEqual(cov.statuses["R9"], "unverifiable")
        self.assertGreater(cov.rf, 0.0)
        self.assertLessEqual(cov.rf, 1.0)
        self.assertGreaterEqual(cov.rc, cov.rf)

    def test_behavior_hang_fails_deadlock_clause(self):
        report = self._run()
        cov = bounded_monitor.coverage(
            self.contract, report, self.reqjson["requirements"],
            self.reqjson["unverifiable"], behavior_ok=False)
        self.assertEqual(cov.statuses["R5"], "FAIL")


if __name__ == "__main__":
    unittest.main()
