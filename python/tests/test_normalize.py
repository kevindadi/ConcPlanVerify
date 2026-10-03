"""Tier-1 normalisation tests (deterministic, recorded rewrites)."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient
from cir_workflow.normalize import SID_RE, normalize
from tests._helpers import REPO, real_binary


def _bare_wait() -> dict:
    # lost_wakeup's buggy variant has a `ready` Var and a `write_shared`.
    return json.loads((REPO / "benchmarks/families/condvar/"
                       "lost_wakeup_notify_before_wait/buggy.cir.json")
                      .read_text(encoding="utf-8"))


class NormalizeUnitTests(unittest.TestCase):
    def test_expr_object_to_string(self):
        program = _bare_wait()
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared" and "expr" in stmt:
                    stmt["expr"] = {"kind": "bool", "value": True}
        out, records, _ = normalize(program)
        exprs = [s["expr"] for m in out["modules"] for f in m["functions"]
                 for s in f.get("body", []) if s.get("kind") == "write_shared"]
        self.assertIn("true", exprs)
        self.assertTrue(any(r["rule"] == "expr_object_to_string" for r in records))

    def test_field_alias_var_to_resource(self):
        program = _bare_wait()
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared" and "resource" in stmt:
                    stmt["var"] = stmt.pop("resource")
        out, records, _ = normalize(program)
        for fn in out["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared":
                    self.assertIn("resource", stmt)
                    self.assertNotIn("var", stmt)
        self.assertTrue(any(r["rule"] == "field_alias" for r in records))

    def test_missing_base_inferred(self):
        program = _bare_wait()
        for res in program["modules"][0]["resources"]:
            if res.get("type") == "Var":
                res.pop("base", None)
                res["init"] = True
        out, records, _ = normalize(program)
        var = [r for r in out["modules"][0]["resources"] if r.get("type") == "Var"][0]
        self.assertEqual(var["base"], "Bool")
        self.assertTrue(any(r["rule"] == "infer_base" for r in records))

    def test_bad_sid_is_renamed_and_recorded(self):
        program = _bare_wait()
        stmt = program["modules"][0]["functions"][-1]["body"][0]
        stmt["sid"] = "b6"
        out, records, issues = normalize(program)
        self.assertNotEqual(out["modules"][0]["functions"][-1]["body"][0]["sid"], "b6")
        self.assertEqual(issues, [])
        self.assertTrue(any(r["rule"] == "sid_rename" and r.get("from") == "b6"
                            for r in records))

    def test_missing_sid_filled_and_goto_rewritten(self):
        program = _bare_wait()
        fn = program["modules"][0]["functions"][-1]
        fn["body"] = [
            {"sid": "b6", "kind": "goto", "target": "b6"},
            {"kind": "return"},
        ]
        out, records, issues = normalize(program)
        body = out["modules"][0]["functions"][-1]["body"]
        self.assertTrue(all(SID_RE.match(s["sid"]) for s in body))
        self.assertIn(body[0]["target"], [s["sid"] for s in body])
        self.assertEqual(issues, [])


SMOKE_D = (REPO / "experiments/flash-repair-smoke-v1/run-20260918T161254-8755-bf0e2a/"
           "condvar__bare_wait_no_predicate/A3_ours_revision/"
           "run-20260918T161854-8755-c04819/revision-3.cir.json")


@unittest.skipUnless(SMOKE_D.is_file(), "smoke-d bare_wait v3 raw text not present")
class SmokeDRegressionTests(unittest.TestCase):
    def test_bare_wait_v3_becomes_valid_and_passes(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend binary not found")
        raw = json.loads(SMOKE_D.read_text(encoding="utf-8"))
        out, records, issues = normalize(raw)
        self.assertTrue(records, "expected Tier-1 rewrites on the raw v3 text")
        self.assertEqual(issues, [])
        contract = (REPO / "benchmarks/families/condvar/bare_wait_no_predicate/"
                    "contract.json")
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "normalized.json"
            path.write_text(json.dumps(out), encoding="utf-8")
            client = ConcirClient(binary, workdir=Path(tmp) / "calls", timeout=30.0)
            self.assertEqual(client.check(path).status, "valid")
            self.assertEqual(client.explore(path, contract, "petri").outcome, "PASS")


class NormalizeIntegrationTests(unittest.TestCase):
    def test_normalized_program_is_statically_valid(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend binary not found")
        program = _bare_wait()
        for res in program["modules"][0]["resources"]:
            if res.get("type") == "Var":
                res.pop("base", None)
        for fn in program["modules"][0]["functions"]:
            for stmt in fn.get("body", []):
                if stmt.get("kind") == "write_shared":
                    stmt["var"] = stmt.pop("resource")
                    stmt["expr"] = {"kind": "bool", "value": True}
        out, records, _ = normalize(copy.deepcopy(program))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "normalized.json"
            path.write_text(json.dumps(out), encoding="utf-8")
            client = ConcirClient(binary, workdir=Path(tmp) / "calls", timeout=30.0)
            result = client.check(path)
            self.assertEqual(result.status, "valid", result.payload)
        self.assertTrue(records)


if __name__ == "__main__":
    unittest.main()
