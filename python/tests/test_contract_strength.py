"""Contract-strength recompute tests (offline, frozen contracts)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.contract_strength import recompute, write_report
from tests._helpers import REPO, real_binary


class ContractStrengthTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def _batch(self, task: str, cir: Path) -> Path:
        run = self.root / f"batch/{task.replace('/', '__')}/A3_ours_revision/run-1"
        run.mkdir(parents=True)
        (run / "result.json").write_text(json.dumps({
            "repair_mode": "llm_revision", "status": "accepted",
            "versions": [{"version": 3, "accepted": True, "artifact_path": str(cir)}],
        }), encoding="utf-8")
        (run / "contract.json").write_text(json.dumps({
            "name": "old", "properties": [{"kind": "deadlock_free", "id": "d"}]}),
            encoding="utf-8")
        return run

    def test_fields_and_rejected_match(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend not found")
        task = "lock-order/partial_deadlock_bystander"
        buggy = REPO / "benchmarks/families" / task / "buggy.cir.json"
        self._batch(task, buggy)
        records = recompute([self.root / "batch"], REPO, binary=binary)
        self.assertEqual(len(records), 1)
        r = records[0]
        for field in ("run", "task", "accepted_version", "cir_sha256",
                      "old_contract_sha256", "old_outcome", "frozen_contract_sha256",
                      "new_outcome", "rejected", "binary_sha256", "git_rev"):
            self.assertTrue(getattr(r, field) is not None, field)
        self.assertEqual(r.new_outcome, "FAIL")
        self.assertTrue(any("holds" in x for x in r.rejected))
        self.assertNotEqual(r.old_contract_sha256, r.frozen_contract_sha256)

    def test_report_written(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend not found")
        task = "lock-order/abba_2lock"
        fixed = REPO / "benchmarks/families" / task / "fixed.cir.json"
        self._batch(task, fixed)
        records = recompute([self.root / "batch"], REPO, binary=binary)
        payload = write_report(records, self.root / "out")
        self.assertTrue((self.root / "out/CONTRACT_STRENGTH.json").is_file())
        self.assertTrue((self.root / "out/CONTRACT_STRENGTH.md").is_file())
        self.assertEqual(payload["records"][0]["new_outcome"], "PASS")


if __name__ == "__main__":
    unittest.main()
