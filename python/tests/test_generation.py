"""§2 generation: name alignment and exhaustive coverage (pure)."""

from __future__ import annotations

import copy
import json
import unittest

from cir_workflow.generation import align_resource_names, model_coverage
from tests._helpers import REPO

CYCLE = REPO / "benchmarks/families/lock-order/cycle_3lock"


class AlignmentTests(unittest.TestCase):
    def test_resources_and_functions_realign(self):
        reference = json.loads((CYCLE / "fixed.cir.json").read_text(encoding="utf-8"))
        candidate = copy.deepcopy(reference)
        for module in candidate["modules"]:
            for fn in module["functions"]:
                fn["name"] = fn["name"].replace("t", "w")
            module["provides"]["functions"] = [
                x.replace("t", "w") for x in module["provides"]["functions"]]
            rename = {"a": "m1", "b": "m2", "c": "m3"}
            for res in module["resources"]:
                res["name"] = rename.get(res["name"], res["name"])
            module["provides"]["resources"] = [
                rename.get(x, x) for x in module["provides"]["resources"]]
            for fn in module["functions"]:
                for st in fn.get("body", []):
                    if st.get("kind") == "scope":
                        st["funcs"] = [x.replace("t", "w") for x in st["funcs"]]
                    for key in ("resource", "condvar", "channel", "lock"):
                        if isinstance(st.get(key), str):
                            st[key] = rename.get(st[key], st[key])
        changes = align_resource_names(candidate, reference)
        got = {(c["old"], c["new"], c.get("kind")) for c in changes}
        self.assertIn(("m1", "a", None), got)
        self.assertIn(("w1", "t1", "function"), got)
        names = {r["name"] for m in candidate["modules"] for r in m["resources"]}
        self.assertEqual(names, {"a", "b", "c"})
        fns = {f["name"] for m in candidate["modules"] for f in m["functions"]}
        self.assertEqual(fns, {"main", "t1", "t2", "t3"})


class CoverageTests(unittest.TestCase):
    def test_model_coverage_maps_properties_to_requirements(self):
        contract = {"properties": [{"kind": "deadlock_free", "id": "d", "req": ["R1"]}],
                    "preserved": [{"kind": "reachable", "req": ["R2"]}]}
        properties = [{"id": "d", "outcome": "PASS"},
                      {"id": "preserved: x", "outcome": "FAIL"}]
        cov = model_coverage(contract, properties, ["a", "b"], [])
        self.assertEqual(cov["statuses"]["R1"], "PASS")
        self.assertEqual(cov["statuses"]["R2"], "FAIL")
        self.assertEqual(cov["rf"], 0.5)
        self.assertEqual(cov["rc"], 1.0)


if __name__ == "__main__":
    unittest.main()
