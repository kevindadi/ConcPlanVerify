"""§1 harness: instrument-v2 resource auto-mapping (pure, no toolchain)."""

from __future__ import annotations

import json
import unittest

from cir_workflow import rust_oracle
from tests._helpers import REPO

ABBA = REPO / "benchmarks/families/lock-order/abba_2lock"


class AutoMappingTests(unittest.TestCase):
    def test_kind_and_spawn_order(self):
        contract = json.loads((ABBA / "contract.json").read_text(encoding="utf-8"))
        cir = json.loads((ABBA / "fixed.cir.json").read_text(encoding="utf-8"))
        resources = [
            {"name": "mtx_a_mutex0", "kind": "Mutex"},
            {"name": "mtx_b_mutex0", "kind": "Mutex"},
            {"name": "w1", "kind": "Spawn"},
            {"name": "w2", "kind": "Spawn"},
        ]
        mapping, provenance = rust_oracle.auto_mapping(resources, contract, cir)
        self.assertEqual(mapping["mtx_a_mutex0"], "main::a")
        self.assertEqual(mapping["mtx_b_mutex0"], "main::b")
        self.assertEqual(mapping["w1"], "main::t1")
        self.assertEqual(mapping["w2"], "main::t2")
        self.assertIn("Mutex", provenance["by_kind"])

    def test_reference_kinds(self):
        cir = json.loads((ABBA / "fixed.cir.json").read_text(encoding="utf-8"))
        kinds = rust_oracle.reference_kinds(cir)
        self.assertEqual(kinds["main::a"], "Mutex")
        self.assertEqual(kinds["main::b"], "Mutex")


if __name__ == "__main__":
    unittest.main()
