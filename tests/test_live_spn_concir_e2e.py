"""Live end-to-end integration test across real concir-backend + real spn (MIR PetriNet) + e2e_pipeline.

Skips cleanly if either `concir-backend` or `spn` binary is absent; when both are built
(as in this repository), exercises:
1. SCIR v4 (`lock_guard` + `seq_hole` + `req` tags) -> exhaustive Petri exploration -> MIR-clean codegen
   -> Loop B hole repair (round 1 injects forbidden sync in hole, round 2 fills pure code)
   -> REAL `spn` HIR/MIR Petri-net extraction (`concir-impl-net/1`)
   -> REAL `concir-backend verify-refinement` stuttering refinement PASS
   -> full `traceability_manifest.json` verification!
2. Branch-pruning stuttering refinement: specification permits a non-deterministic choice
   between two valid lock-protected paths; a concrete Rust implementation that prunes one
   branch still refines the specification (`outcome == "pass"`), whereas an implementation
   that locks resources in reverse/unauthorized order is rejected (`outcome == "fail"`).
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from concplan.code_verify.static_verify import SpnMirRefinementPlugin
from concplan.core.concir_client import ConcirClient
from concplan.core.providers import ScriptedProvider
from concplan.core.toolchain import find_concir_backend, find_spn
from concplan.pipeline.e2e_pipeline import run_e2e_pipeline


SCIR_BOUNDED_COUNTER = {
    "program": "scir_bounded_counter",
    "version": "4.0.0",
    "entry": "main::main",
    "modules": [
        {
            "name": "main",
            "req": ["R1", "R2", "R3"],
            "provides": {
                "resources": ["m", "c"],
                "functions": ["main", "w1", "w2"],
            },
            "requires": {"resources": [], "functions": []},
            "resources": [
                {"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R3"]},
                {
                    "name": "c",
                    "kind": "var",
                    "type": "Var",
                    "base": {"Int": [0, 2]},
                    "init": 0,
                    "req": ["R2"],
                },
            ],
            "protection": [{"var": "c", "lock": "m"}],
            "functions": [
                {
                    "name": "main",
                    "kind": "normal",
                    "structured_body": [
                        {"id": "b_scope", "kind": "scope", "funcs": ["main::w1", "main::w2"], "req": ["R1"]},
                        {"id": "b_ret", "kind": "return"},
                    ],
                },
                {
                    "name": "w1",
                    "kind": "normal",
                    "form": "closure",
                    "structured_body": [
                        {
                            "id": "b_lock1",
                            "kind": "lock_guard",
                            "resource": "main::m",
                            "req": ["R3"],
                            "body": [
                                {
                                    "id": "b_h1",
                                    "kind": "seq_hole",
                                    "id_hole": "h_w1",
                                    "desc": "local step for w1",
                                    "reads": ["c"],
                                    "writes": ["c"],
                                    "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                    "expected_type": "i64",
                                    "req": ["R2"],
                                }
                            ],
                        },
                        {"id": "b_ret1", "kind": "return"},
                    ],
                },
                {
                    "name": "w2",
                    "kind": "normal",
                    "form": "closure",
                    "structured_body": [
                        {
                            "id": "b_lock2",
                            "kind": "lock_guard",
                            "resource": "main::m",
                            "req": ["R3"],
                            "body": [
                                {
                                    "id": "b_h2",
                                    "kind": "seq_hole",
                                    "id_hole": "h_w2",
                                    "desc": "local step for w2",
                                    "reads": ["c"],
                                    "writes": ["c"],
                                    "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                    "expected_type": "i64",
                                    "req": ["R2"],
                                }
                            ],
                        },
                        {"id": "b_ret2", "kind": "return"},
                    ],
                },
            ],
        }
    ],
}


# Note: check exact SCIR `seq_hole` field name (`id` vs `hole_id`) in `backends/concir/src/scir.rs`!
SCIR_BOUNDED_COUNTER_V4 = {
    "program": "scir_bounded_counter",
    "version": "4.0.0",
    "entry": "main::main",
    "modules": [
        {
            "name": "main",
            "req": ["R1", "R2", "R3"],
            "provides": {
                "resources": ["m", "c"],
                "functions": ["main", "w1", "w2"],
            },
            "requires": {"resources": [], "functions": []},
            "resources": [
                {"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R3"]},
                {
                    "name": "c",
                    "kind": "var",
                    "type": "Var",
                    "base": {"Int": [0, 2]},
                    "init": 0,
                    "req": ["R2"],
                },
            ],
            "protection": [{"var": "c", "lock": "m"}],
            "functions": [
                {
                    "name": "main",
                    "kind": "normal",
                    "structured_body": [
                        {"id": "b_scope", "kind": "scope", "funcs": ["main::w1", "main::w2"], "req": ["R1"]},
                        {"id": "b_ret", "kind": "return"},
                    ],
                },
                {
                    "name": "w1",
                    "kind": "normal",
                    "form": "closure",
                    "structured_body": [
                        {
                            "id": "b_lock1",
                            "kind": "lock_guard",
                            "resource": "main::m",
                            "req": ["R3"],
                            "body": [
                                {
                                    "id": "h_w1",
                                    "kind": "seq_hole",
                                    "desc": "local step for w1",
                                    "reads": ["c"],
                                    "writes": ["c"],
                                    "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                    "expected_type": "i64",
                                    "req": ["R2"],
                                }
                            ],
                        },
                        {"id": "b_ret1", "kind": "return"},
                    ],
                },
                {
                    "name": "w2",
                    "kind": "normal",
                    "form": "closure",
                    "structured_body": [
                        {
                            "id": "b_lock2",
                            "kind": "lock_guard",
                            "resource": "main::m",
                            "req": ["R3"],
                            "body": [
                                {
                                    "id": "h_w2",
                                    "kind": "seq_hole",
                                    "desc": "local step for w2",
                                    "reads": ["c"],
                                    "writes": ["c"],
                                    "updates_modeled": [{"target": "c", "expr": "c + 1"}],
                                    "expected_type": "i64",
                                    "req": ["R2"],
                                }
                            ],
                        },
                        {"id": "b_ret2", "kind": "return"},
                    ],
                },
            ],
        }
    ],
}


CONTRACT = {
    "name": "bounded-counter",
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R3"]},
        {
            "kind": "safety",
            "id": "counter-bound",
            "invariant": {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 2},
            "req": ["R2"],
        },
    ],
    "preserved": [
        {
            "kind": "reachable",
            "description": "w1 completes",
            "goal": {"kind": "function_completed", "function": "main::w1"},
            "req": ["R1"],
        },
        {
            "kind": "reachable",
            "description": "w2 completes",
            "goal": {"kind": "function_completed", "function": "main::w2"},
            "req": ["R1"],
        },
    ],
}


class LiveSpnConcirE2ETests(unittest.TestCase):
    def setUp(self):
        self.backend = find_concir_backend()
        self.spn = find_spn()
        if self.backend is None or self.spn is None:
            self.skipTest("live concir-backend or spn binary not built")
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.client = ConcirClient(self.backend, workdir=self.root / "calls", timeout=120.0)

    def tearDown(self):
        if hasattr(self, "_tmp"):
            self._tmp.cleanup()

    def test_live_e2e_scir_spn_mir_refinement_and_loop_b_repair(self):
        requirements = (
            "R1: Spawn two worker tasks w1 and w2 that both complete.\n"
            "R2: Increment shared counter c once per worker so c <= 2.\n"
            "R3: Protect c with mutex m without deadlocks.\n"
        )
        cir_provider = ScriptedProvider([
            {"text": json.dumps(SCIR_BOUNDED_COUNTER_V4)},
        ])
        # Round 1 hole fill puts forbidden Mutex::new inside h_w2 -> Loop B repairs ONLY h_w2!
        hole_provider = ScriptedProvider([
            {"text": json.dumps({"h_w1": "10 + 5", "h_w2": "{ std::sync::Mutex::new(0); 1 }"})},
            {"text": json.dumps({"h_w2": "20 + 2"})},
        ])

        res = run_e2e_pipeline(
            requirements=requirements,
            contract=CONTRACT,
            client=self.client,
            cir_provider=cir_provider,
            hole_provider=hole_provider,
            out_dir=self.root / "e2e_out",
            run_summarize=True,
            run_dynamic_conform=True,
            native_runs=3,
        )

        self.assertEqual(res.status, "verified", f"expected verified, got {res.status}: {res.error} / {res.static_report}")
        self.assertTrue(res.ok)
        self.assertEqual(len(res.hole_rounds), 2)
        self.assertEqual(res.fills["h_w1"], "10 + 5")
        self.assertEqual(res.fills["h_w2"], "20 + 2")

        # Verify real SPN MIR refinement ran and passed!
        static_rep = res.static_report
        self.assertIsNotNone(static_rep)
        self.assertEqual(static_rep["status"], "pass")
        plugin_names = [p["plugin"] for p in static_rep["plugin_reports"]]
        self.assertIn("spn_mir_refinement", plugin_names)
        spn_rep = next(p for p in static_rep["plugin_reports"] if p["plugin"] == "spn_mir_refinement")
        self.assertEqual(spn_rep["status"], "pass")
        self.assertTrue(spn_rep["checks"]["weak_simulation"])
        self.assertTrue(spn_rep["checks"]["deadlock_free"])
        self.assertTrue(spn_rep["checks"]["preserved_goals"])

        # Verify traceability manifest links R1, R2, R3 end-to-end!
        manifest = res.manifest
        self.assertIsNotNone(manifest)
        self.assertEqual(manifest["schema"], "concir-traceability-manifest/1")
        clause_ids = {c["clause_id"] for c in manifest["clauses"]}
        self.assertEqual(clause_ids, {"R1", "R2", "R3"})

    def test_branch_pruning_refinement_pass_and_wrong_lock_order_fail(self):
        # Spec allows non-deterministically choosing either lock m1 OR lock m2 via a branch on modeled local!
        spec = {
            "program": "branch_choice",
            "version": "4.0.0",
            "entry": "main::main",
            "modules": [
                {
                    "name": "main",
                    "provides": {"resources": ["m1", "m2"], "functions": ["main"]},
                    "requires": {"resources": [], "functions": []},
                    "resources": [
                        {"name": "m1", "kind": "sync", "type": "Mutex", "mode": "Sync"},
                        {"name": "m2", "kind": "sync", "type": "Mutex", "mode": "Sync"},
                    ],
                    "protection": [],
                    "functions": [
                        {
                            "name": "main",
                            "kind": "normal",
                            "locals": [{"name": "pick", "type": "Int", "modeled": True, "init": 0}],
                            "body": [
                                {"sid": "s1", "kind": "branch", "cond": "pick == 0", "then": "s2", "else": "s5"},
                                {"sid": "s2", "kind": "mutex_lock", "resource": "m1"},
                                {"sid": "s3", "kind": "mutex_unlock", "resource": "m1"},
                                {"sid": "s4", "kind": "return"},
                                {"sid": "s5", "kind": "mutex_lock", "resource": "m2"},
                                {"sid": "s6", "kind": "mutex_unlock", "resource": "m2"},
                                {"sid": "s7", "kind": "return"},
                            ],
                        }
                    ],
                }
            ],
        }
        spec_path = self.root / "branch_spec.json"
        spec_path.write_text(json.dumps(spec), encoding="utf-8")

        # Impl 1: Prunes branch to ONLY lock/unlock m1 (weighted ImplArc objects like real spn!) -> PASS
        pruned_impl = {
            "schema": "concir-impl-net/1",
            "crate": "demo",
            "truncated": False,
            "places": [
                {"id": 0, "name": "start", "class": "control", "initial_tokens": 1},
                {"id": 1, "name": "mid", "class": "control", "initial_tokens": 0},
                {"id": 2, "name": "end", "class": "control", "initial_tokens": 0},
                {"id": 3, "name": "main::m1", "class": "resource", "resource_kind": "mutex", "initial_tokens": 1},
            ],
            "transitions": [
                {
                    "id": 0,
                    "name": "acq_m1",
                    "observable": True,
                    "op": "mutex_lock",
                    "resource": "main::m1",
                    "inputs": [{"place": 0, "weight": 1}, {"place": 3, "weight": 1}],
                    "outputs": [{"place": 1, "weight": 1}],
                },
                {
                    "id": 1,
                    "name": "rel_m1",
                    "observable": True,
                    "op": "mutex_unlock",
                    "resource": "main::m1",
                    "inputs": [{"place": 1, "weight": 1}],
                    "outputs": [{"place": 2, "weight": 1}, {"place": 3, "weight": 1}],
                },
            ],
            "states": [],
            "edges": [],
            "purity_violations": [],
            "completed_functions": ["demo::main::cf_main"],
        }
        pruned_path = self.root / "pruned_impl.json"
        pruned_path.write_text(json.dumps(pruned_impl), encoding="utf-8")

        r_pass = self.client.verify_refinement(spec_path, pruned_path)
        self.assertEqual(r_pass.outcome, "pass", f"expected pass, got {r_pass.payload}")

        # Impl 2: Acquires m1 AND nested m2 (unauthorized by spec!) -> FAIL with witness trace!
        unauthorized_impl = {
            "schema": "concir-impl-net/1",
            "crate": "demo",
            "truncated": False,
            "states": [
                {"id": 0, "initial": True, "terminal": False, "deadlock": False, "held_locks": []},
                {"id": 1, "initial": False, "terminal": False, "deadlock": False, "held_locks": ["main::m1"]},
                {"id": 2, "initial": False, "terminal": True, "deadlock": False, "held_locks": []},
            ],
            "transitions": [
                {"id": 0, "observable": True, "op": "mutex_lock", "resource": "main::m1"},
                {"id": 1, "observable": True, "op": "mutex_lock", "resource": "main::m2"},
            ],
            "edges": [
                {"from": 0, "to": 1, "transition": 0},
                {"from": 1, "to": 2, "transition": 1},
            ],
            "purity_violations": [],
            "completed_functions": ["demo::main::cf_main"],
        }
        bad_path = self.root / "bad_impl.json"
        bad_path.write_text(json.dumps(unauthorized_impl), encoding="utf-8")

        r_fail = self.client.verify_refinement(spec_path, bad_path)
        self.assertEqual(r_fail.outcome, "fail")
        self.assertFalse(r_fail.payload["checks"]["weak_simulation"])
        self.assertGreaterEqual(len(r_fail.payload["violations"]), 1)


if __name__ == "__main__":
    unittest.main()
