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


CTL_VACUOUS_CONTRACT = {
    "name": "ctl-vacuity",
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock"},
        {
            "kind": "ctl",
            "id": "never-true-antecedent",
            "non_vacuous": True,
            "formula": {
                "op": "ag",
                "arg": {
                    "op": "implies",
                    "lhs": {"op": "atom", "atom": {"kind": "var_cmp", "resource": "main::c", "op": ">", "value": 2}},
                    "rhs": {"op": "atom", "atom": {"kind": "var_eq", "resource": "main::c", "value": 0}},
                },
            },
        },
        {
            "kind": "ltl",
            "id": "never-true-antecedent-ltl",
            "non_vacuous": True,
            "formula": {
                "op": "always",
                "arg": {
                    "op": "implies",
                    "lhs": {"op": "atom", "atom": {"kind": "var_cmp", "resource": "main::c", "op": ">", "value": 2}},
                    "rhs": {"op": "eventually", "arg": {"op": "atom", "atom": {"kind": "var_eq", "resource": "main::c", "value": 0}}},
                },
            },
        },
    ],
}

CTL_LTL_PASS_CONTRACT = {
    "name": "ctl-ltl-ok",
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock"},
        {
            "kind": "ctl",
            "id": "ef-done",
            "formula": {"op": "ef", "arg": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::w1"}}},
        },
        {
            "kind": "ctl",
            "id": "nonvac-implication",
            "non_vacuous": True,
            "formula": {
                "op": "ag",
                "arg": {
                    "op": "implies",
                    "lhs": {"op": "atom", "atom": {"kind": "var_eq", "resource": "main::c", "value": 1}},
                    "rhs": {"op": "af", "arg": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::w1"}}},
                },
            },
        },
        {
            "kind": "ltl",
            "id": "eventually-done",
            "formula": {"op": "eventually", "arg": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::w1"}}},
        },
    ],
}

MULTI_MODULE_RWLOCK_PROGRAM = {
    "program": "multi_mod_rwlock_e2e",
    "version": "4.0.0",
    "entry": "app::main",
    "modules": [
        {
            "name": "app",
            "req": ["R1"],
            "provides": {"resources": [], "functions": ["main"]},
            "requires": {"resources": [], "functions": ["book::reader", "book::writer"]},
            "resources": [],
            "protection": [],
            "functions": [
                {
                    "name": "main",
                    "kind": "normal",
                    "req": ["R1"],
                    "body": [
                        {"sid": "s1", "kind": "scope", "funcs": ["book::reader", "book::writer"], "req": ["R1"]},
                        {"sid": "s2", "kind": "return"},
                    ],
                }
            ],
        },
        {
            "name": "book",
            "req": ["R2", "R3"],
            "provides": {"resources": ["book_lock", "pending"], "functions": ["reader", "writer"], "types": ["OrderBook"]},
            "requires": {"resources": [], "functions": []},
            "types": [
                {
                    "name": "OrderBook",
                    "type": {"Hole": {"description": "Pure in-memory order book", "req": ["R3"]}},
                }
            ],
            "resources": [
                {"name": "book_lock", "kind": "sync", "type": "RwLock", "mode": "Sync", "base": "OrderBook", "req": ["R2"]},
                {"name": "pending", "kind": "var", "type": "Var", "base": {"Int": [0, 2]}, "init": 0, "req": ["R2"]},
            ],
            "protection": [{"var": "pending", "lock": "book_lock"}],
            "functions": [
                {
                    "name": "reader",
                    "kind": "normal",
                    "req": ["R2"],
                    "structured_body": [
                        {
                            "id": "rg_read",
                            "kind": "rwlock_read_guard",
                            "resource": "book_lock",
                            "req": ["R2"],
                            "body": [
                                {
                                    "id": "h_read",
                                    "kind": "seq_hole",
                                    "reads": ["pending"],
                                    "expected_type": "i64",
                                    "req": ["R3"],
                                }
                            ],
                        },
                        {"id": "b_ret_r", "kind": "return"},
                    ],
                },
                {
                    "name": "writer",
                    "kind": "normal",
                    "req": ["R2"],
                    "structured_body": [
                        {
                            "id": "wg_write",
                            "kind": "rwlock_write_guard",
                            "resource": "book_lock",
                            "req": ["R2"],
                            "body": [
                                {
                                    "id": "h_write",
                                    "kind": "seq_hole",
                                    "reads": ["pending"],
                                    "writes": ["pending"],
                                    "updates_modeled": [{"target": "pending", "expr": "pending + 1"}],
                                    "expected_type": "()",
                                    "req": ["R3"],
                                }
                            ],
                        },
                        {"id": "b_ret_w", "kind": "return"},
                    ],
                },
            ],
        },
    ],
}

MULTI_MODULE_RWLOCK_CONTRACT = {
    "name": "multi-mod-rwlock",
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R2"]},
        {
            "kind": "safety",
            "id": "pending-bound",
            "invariant": {"kind": "var_cmp", "resource": "book::pending", "op": "<=", "value": 2},
            "req": ["R2"],
        },
    ],
    "preserved": [
        {
            "kind": "reachable",
            "description": "reader completes",
            "goal": {"kind": "function_completed", "function": "book::reader"},
            "req": ["R1"],
        },
        {
            "kind": "reachable",
            "description": "writer completes",
            "goal": {"kind": "function_completed", "function": "book::writer"},
            "req": ["R1"],
        },
    ],
}


def _backend_supports(client: ConcirClient, tmp: Path, *, contract: dict) -> bool:
    """True if the built concir-backend understands the v5 contract kinds (ctl/ltl)."""
    cpath = tmp / "probe_contract.json"
    cpath.write_text(json.dumps(contract), encoding="utf-8")
    ppath = tmp / "probe_prog.json"
    ppath.write_text(json.dumps(SCIR_BOUNDED_COUNTER_V4), encoding="utf-8")
    low = client.lower(ppath, out_path=tmp / "probe_lowered.json")
    if low.kind != "semantic" or low.status != "pass":
        return False
    res = client.explore(tmp / "probe_lowered.json", cpath, "petri")
    return res.kind == "semantic"


class LiveV5TemporalAndTypeHoleTests(unittest.TestCase):
    """Live v5 checks; each skips cleanly when the built binaries lack the v5 feature."""

    def setUp(self):
        self.backend = find_concir_backend()
        self.spn = find_spn()
        if self.backend is None:
            self.skipTest("live concir-backend binary not built")
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.client = ConcirClient(self.backend, workdir=self.root / "calls", timeout=120.0)

    def tearDown(self):
        if hasattr(self, "_tmp"):
            self._tmp.cleanup()

    def test_live_ctl_ltl_model_checking_and_non_vacuity_rejection(self):
        if not _backend_supports(self.client, self.root, contract=CTL_LTL_PASS_CONTRACT):
            self.skipTest("concir-backend does not support CTL/LTL contracts yet")
        lowered = self.root / "probe_lowered.json"
        ok_path = self.root / "ok_contract.json"
        ok_path.write_text(json.dumps(CTL_LTL_PASS_CONTRACT), encoding="utf-8")
        ok = self.client.explore(lowered, ok_path, "petri")
        if ok.outcome is None:
            self.skipTest("concir-backend did not evaluate CTL/LTL properties")
        self.assertEqual(ok.outcome, "PASS", ok.payload)

        vac_path = self.root / "vac_contract.json"
        vac_path.write_text(json.dumps(CTL_VACUOUS_CONTRACT), encoding="utf-8")
        vac = self.client.explore(lowered, vac_path, "petri")
        self.assertEqual(vac.outcome, "FAIL", vac.payload)
        vac_props = [p for p in vac.payload["properties"] if p["id"] == "never-true-antecedent"]
        self.assertTrue(vac_props and vac_props[0].get("vacuous"), vac.payload)

    def test_live_explain_contract_matches_python_renderer(self):
        cpath = self.root / "c.json"
        cpath.write_text(json.dumps(CTL_LTL_PASS_CONTRACT), encoding="utf-8")
        res = self.client.explain_contract(cpath)
        if res.kind != "semantic":
            self.skipTest("concir-backend lacks explain-contract")
        self.assertEqual(res.payload["schema"], "concir-contract-explanation/1")
        ids = {p["id"] for p in res.payload["properties"]}
        self.assertTrue({"ef-done", "eventually-done"} <= ids)

    def test_live_type_hole_domain_rs_pass_and_mutex_sneak_repaired_in_loop_b(self):
        if self.spn is None:
            self.skipTest("live spn binary not built")
        program = json.loads(json.dumps(SCIR_BOUNDED_COUNTER_V4))
        program["modules"][0]["types"] = [
            {"name": "Stats", "ty": {"Hole": {"description": "pure stats", "req": ["R2"]}}}
        ]
        cpath = self.root / "tc.json"
        cpath.write_text(json.dumps(CONTRACT), encoding="utf-8")
        ppath = self.root / "tp.json"
        ppath.write_text(json.dumps(program), encoding="utf-8")
        low = self.client.lower(ppath, out_path=self.root / "tp_lowered.json")
        if low.kind != "semantic" or low.status != "pass":
            self.skipTest("concir-backend does not support module types / TypeHole")
        from concplan.code_verify.conformance import codegen as _codegen, type_holes_of
        skel = self.root / "probe_skel"
        _codegen(self.root / "tp_lowered.json", skel, binary=self.backend)
        if not type_holes_of(skel):
            self.skipTest("concir-backend codegen does not emit TypeHoles yet")

        good = "#[derive(Clone, Debug, Default, PartialEq)]\npub struct Stats { pub hits: i64 }"
        bad = "#[derive(Default)]\npub struct Stats { pub m: std::sync::Mutex<i64> }"
        hole_provider = ScriptedProvider([
            {"text": json.dumps({"h_w1": "1", "h_w2": "2", "th1": bad})},
            {"text": json.dumps({"th1": good})},
        ])
        res = run_e2e_pipeline(
            requirements="R1: spawn workers. R2: count with pure Stats. R3: protect c with m.",
            contract=CONTRACT,
            client=self.client,
            cir_provider=ScriptedProvider([{"text": json.dumps(program)}]),
            hole_provider=hole_provider,
            out_dir=self.root / "e2e_th",
        )
        self.assertEqual(res.status, "verified", f"{res.error} / {res.static_report}")
        self.assertEqual(res.hole_rounds[1]["target_holes"], ["th1"])
        self.assertEqual(res.fills["th1"], good)
        self.assertEqual(res.manifest["type_holes"][0]["rust_file"], "src/domain.rs")

    def test_live_rwlock_multimodule_modular_refinement_and_export_cvn(self):
        if self.spn is None:
            self.skipTest("live spn binary not built")

        # Exercise live export-cvn on SCIR_BOUNDED_COUNTER_V4 (auto-lowers SCIR -> UniPN CVN!)
        scir_path = self.root / "scir_cvn.json"
        scir_path.write_text(json.dumps(SCIR_BOUNDED_COUNTER_V4), encoding="utf-8")
        cvn_res = self.client.export_cvn(scir_path, out_path=self.root / "scir_cvn.out.json")
        self.assertEqual((cvn_res.kind, cvn_res.status), ("semantic", "pass"), cvn_res.error)
        self.assertEqual(cvn_res.payload["schema"], "concir-cvn-export/1")
        self.assertGreaterEqual(len(cvn_res.payload["net"]["places"]), 2)
        self.assertGreaterEqual(len(cvn_res.payload["net"]["transitions"]), 2)

        pure_order_book = (
            "#[derive(Clone, Debug, Default, PartialEq)]\n"
            "pub struct OrderBook { pub best_bid: i64 }\n"
            "impl OrderBook {\n"
            "    pub fn update_bid(&mut self, p: i64) { if p > self.best_bid { self.best_bid = p; } }\n"
            "}"
        )
        hole_provider = ScriptedProvider([
            {
                "text": json.dumps({
                    "th1": pure_order_book,
                    "h_read": "_rg_r_book__book_lock.best_bid",
                    "h_write": "{ _wg_r_book__book_lock.update_bid(42); }",
                })
            }
        ])
        res = run_e2e_pipeline(
            requirements=(
                "R1: Spawn book::reader and book::writer from app::main.\n"
                "R2: Guard pending with RwLock book_lock without deadlock.\n"
                "R3: Implement pure OrderBook TypeHole and read/write SeqHoles.\n"
            ),
            contract=MULTI_MODULE_RWLOCK_CONTRACT,
            client=self.client,
            cir_provider=ScriptedProvider([{"text": json.dumps(MULTI_MODULE_RWLOCK_PROGRAM)}]),
            hole_provider=hole_provider,
            out_dir=self.root / "e2e_rwlock_mod",
            run_summarize=True,
            modular_refinement=True,
            run_dynamic_conform=True,
            native_runs=2,
            miri_seeds=0,
        )
        self.assertEqual(res.status, "verified", f"{res.error} / {res.static_report}")
        self.assertTrue(res.ok)
        self.assertIsNotNone(res.dynamic_report)
        self.assertEqual(res.dynamic_report["conformant"], 2, res.dynamic_report)
        self.assertEqual(res.dynamic_report["violation"], 0, res.dynamic_report)
        self.assertIsNotNone(res.modular_reports)
        self.assertEqual(set(res.modular_reports.keys()), {"app", "book"})
        self.assertEqual(res.modular_reports["app"]["status"], "pass")
        self.assertEqual(res.modular_reports["book"]["status"], "pass")
        self.assertEqual(len(res.manifest["type_holes"]), 1)
        self.assertEqual(res.manifest["type_holes"][0]["type_name"], "OrderBook")
        self.assertEqual(res.manifest["type_holes"][0]["rust_file"], "src/domain.rs")
        # Multi-file codegen placed book module holes into src/book.rs!
        self.assertTrue((Path(res.filled_dir) / "src/book.rs").is_file())
        self.assertTrue((Path(res.filled_dir) / "src/domain.rs").is_file())

    def test_live_modular_refinement_cross_module_shared_resource(self):
        if self.spn is None:
            self.skipTest("live spn binary not built")

        prog = {
            "program": "cross_mod_shared_res",
            "version": "5.0.0",
            "entry": "app::main",
            "modules": [
                {
                    "name": "app",
                    "req": ["R1"],
                    "provides": {"resources": [], "functions": ["main"]},
                    "requires": {"resources": [], "functions": ["storage::init", "worker::run"]},
                    "resources": [],
                    "protection": [],
                    "functions": [
                        {
                            "name": "main",
                            "kind": "normal",
                            "req": ["R1"],
                            "structured_body": [
                                {
                                    "id": "b_scope",
                                    "kind": "scope",
                                    "funcs": ["storage::init", "worker::run"],
                                    "req": ["R1"],
                                },
                                {"id": "b_ret", "kind": "return"},
                            ],
                        }
                    ],
                },
                {
                    "name": "storage",
                    "req": ["R2"],
                    "provides": {"resources": ["db_lock"], "functions": ["init"]},
                    "requires": {"resources": [], "functions": []},
                    "resources": [
                        {"name": "db_lock", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R2"]},
                    ],
                    "protection": [],
                    "functions": [
                        {
                            "name": "init",
                            "kind": "normal",
                            "req": ["R2"],
                            "structured_body": [
                                {
                                    "id": "lg_init",
                                    "kind": "lock_guard",
                                    "resource": "db_lock",
                                    "req": ["R2"],
                                    "body": [
                                        {"id": "h_init", "kind": "seq_hole", "expected_type": "i64", "req": ["R2"]},
                                    ],
                                },
                                {"id": "b_ret_s", "kind": "return"},
                            ],
                        }
                    ],
                },
                {
                    "name": "worker",
                    "req": ["R2"],
                    "provides": {"resources": [], "functions": ["run"]},
                    "requires": {"resources": ["storage::db_lock"], "functions": []},
                    "resources": [],
                    "protection": [],
                    "functions": [
                        {
                            "name": "run",
                            "kind": "normal",
                            "req": ["R2"],
                            "structured_body": [
                                {
                                    "id": "lg_work",
                                    "kind": "lock_guard",
                                    "resource": "storage::db_lock",
                                    "req": ["R2"],
                                    "body": [
                                        {"id": "h_work", "kind": "seq_hole", "expected_type": "i64", "req": ["R2"]},
                                    ],
                                },
                                {"id": "b_ret_w", "kind": "return"},
                            ],
                        }
                    ],
                },
            ],
        }
        contract = {
            "name": "cross-mod-shared-res",
            "properties": [
                {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R2"]},
            ],
            "preserved": [
                {
                    "kind": "reachable",
                    "description": "storage init completes",
                    "goal": {"kind": "function_completed", "function": "storage::init"},
                    "req": ["R1"],
                },
                {
                    "kind": "reachable",
                    "description": "worker run completes",
                    "goal": {"kind": "function_completed", "function": "worker::run"},
                    "req": ["R1"],
                },
            ],
        }
        res = run_e2e_pipeline(
            requirements=(
                "R1: Spawn storage::init and worker::run from app::main.\n"
                "R2: Both storage::init and worker::run lock storage::db_lock cleanly.\n"
            ),
            contract=contract,
            client=self.client,
            cir_provider=ScriptedProvider([{"text": json.dumps(prog)}]),
            hole_provider=ScriptedProvider([{"text": json.dumps({"h_init": "1", "h_work": "2"})}]),
            out_dir=self.root / "e2e_cross_mod_shared",
            run_summarize=True,
            modular_refinement=True,
        )
        self.assertEqual(res.status, "verified", f"{res.error} / {res.static_report}")
        self.assertTrue(res.ok)
        self.assertIsNotNone(res.modular_reports)
        self.assertEqual(set(res.modular_reports.keys()), {"app", "storage", "worker"})
        for mod_name in ("app", "storage", "worker"):
            self.assertEqual(
                res.modular_reports[mod_name]["status"],
                "pass",
                f"modular report failed for {mod_name}: {res.modular_reports[mod_name]}",
            )


if __name__ == "__main__":
    unittest.main()
