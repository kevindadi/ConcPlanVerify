"""Single candidate-evaluation entry: online and offline share the rule."""

from __future__ import annotations

import unittest
from pathlib import Path

from cir_workflow import candidate_eval, generation


class CandidateEvalEntryTests(unittest.TestCase):
    def test_acceptance_policy_versioned(self):
        self.assertTrue(candidate_eval.ACCEPTANCE_POLICY.startswith("ledger-"))

    def test_interpret_is_deterministic(self):
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            (tmp / "s.rs").write_text("fn main(){}")
            (tmp / "c.json").write_text("{}")
            (tmp / "ct.json").write_text("{}")
            for role in ("model_check", "binding", "monitor", "execution", "conform"):
                (tmp / f"{role}.json").write_text("{}")
            import hashlib
            arts = [{"role": p.stem, "path": str(p),
                     "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                    for p in tmp.glob("*") if p.is_file()]
            r = {"cell": "t", "source_path": str(tmp / "s.rs"), "cir_path": str(tmp / "c.json"),
                 "contract_path": str(tmp / "ct.json"), "artifacts": arts,
                 "stages": {"source_build": "ok", "instrument": "ok", "instrumented_build": "ok"},
                 "runs_started": 32, "runs_completed": 32, "hang": False,
                 "projected_events": 5,
                 "conform": {"traces": 1, "statuses": {"conformant": 1}, "violations": []},
                 "monitor": {"status": "ok", "properties": [["p1", "PASS_bounded"]]},
                 "binding": {"mapping": {"m": "main::m"}, "ambiguous": [], "violated": {}}}
            contract = {"properties": [{"id": "p1", "kind": "safety", "req": ["R1"]}]}
            a = candidate_eval.interpret(r, contract, accepted=True,
                                         cir_props={"p1": "PASS"}, cir_complete=True)
            b = candidate_eval.interpret(r, contract, accepted=True,
                                         cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertEqual(a.to_dict(), b.to_dict())

    def test_generation_uses_shared_entry(self):
        # The online decision imports the shared entry, not a private rule.
        src = Path(generation.__file__).read_text()
        self.assertIn("candidate_eval.interpret", src)
        self.assertIn("all_obligations_satisfied", src)


if __name__ == "__main__":
    unittest.main()
