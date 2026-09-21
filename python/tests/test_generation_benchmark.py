"""§0 benchmark v3: generation requirements, req-tagged contracts, tiers."""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

from tests._helpers import REPO

BUILD = REPO / "benchmarks/build_families.py"
MANIFEST = REPO / "benchmarks/GENERATION_MANIFEST.json"


def _load_build_families():
    spec = importlib.util.spec_from_file_location("build_families_under_test", BUILD)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


@unittest.skipUnless(MANIFEST.is_file(), "GENERATION_MANIFEST.json not built")
class GenerationBenchmarkTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.bf = _load_build_families()
        cls.manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.by_task = {t["task"]: t for t in cls.manifest["tasks"]}

    def test_manifest_covers_every_generation_case(self):
        self.assertEqual(set(self.by_task), set(self.bf.GENERATION))
        self.assertEqual(len(self.by_task), 24)

    def test_tiers_are_balanced(self):
        counts = {}
        for task in self.by_task.values():
            counts[task["tier"]] = counts.get(task["tier"], 0) + 1
        self.assertEqual(counts, {"Simple": 8, "Medium": 8, "Complex": 8})

    def test_tier_classification_thresholds(self):
        classify = self.bf.classify_tier
        self.assertEqual(classify(5, 3, 9), "Simple")
        self.assertEqual(classify(9, 4, 39), "Medium")
        self.assertEqual(classify(10, 5, 43), "Complex")
        self.assertEqual(classify(7, 4, 92), "Medium")
        self.assertEqual(classify(7, 4, 116), "Complex")
        self.assertEqual(classify(7, 7, 20), "Complex")

    def test_contracts_carry_req_tags_and_clause_lengths(self):
        for key in self.bf.GENERATION:
            with self.subTest(task=key):
                task_dir = REPO / "benchmarks/families" / key
                contract = json.loads((task_dir / "contract.json").read_text(encoding="utf-8"))
                reqjson = json.loads(
                    (task_dir / "generation_input/requirements.json").read_text(encoding="utf-8"))
                reqs = reqjson["requirements"]
                ids = {f"R{i}" for i in range(1, len(reqs) + 1)}
                self.assertEqual(len(reqs), len(set(reqs)))
                for clause in contract["properties"] + contract.get("preserved", []):
                    self.assertIn("req", clause)
                    self.assertTrue(set(clause["req"]) <= ids)
                covered = {r for clause in contract["properties"] + contract.get("preserved", [])
                           for r in clause["req"]}
                for i in range(1, len(reqs) + 1):
                    rid = f"R{i}"
                    if i in reqjson["unverifiable"]:
                        self.assertNotIn(rid, covered)
                    else:
                        self.assertIn(rid, covered)

    def test_requirement_text_has_no_leak_tokens(self):
        errors = []
        for key in self.bf.GENERATION:
            meta = {"task": key, "requirements": self.bf.GENERATION[key]["requirements"]}
            errors.extend(self.bf.check_requirement_text(meta))
        self.assertEqual(errors, [])

    def test_offline_check_passes(self):
        errors = self.bf.validate_generation(self.manifest["tasks"])
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
