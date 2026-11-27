"""Integration tests for Module 3 & 4: ConstraintPattern, Health, ReqIR, and Dual-Net Design Refinement."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from concplan.__main__ import main as cli_main
from concplan.code_verify.static_verify import AstHolePurityPlugin, CompositeStaticVerifier
from concplan.core.concir_client import ConcirClient
from concplan.core.providers import ScriptedProvider
from concplan.core.toolchain import find_concir_backend
from concplan.formalize import (
    ensure_implication_non_vacuous,
    explain_contract_data,
    formalize_from_structured,
    validate_contract_sanity,
)
from concplan.formalize.models import GroundedEntities, RequirementClause, StructuredRequirements
from concplan.mcp import create_server
from concplan.pipeline.e2e_pipeline import run_e2e_pipeline


def _pc_req_ir() -> dict:
    return {
        "schema": "concir-req-ir/1",
        "name": "pc_req",
        "roles": [
            {
                "name": "producer",
                "instances": 1,
                "init_state": "ready",
                "states": ["ready", "done"],
                "terminal_states": ["done"],
                "req": ["R1"],
            },
            {
                "name": "consumer",
                "instances": 1,
                "init_state": "waiting",
                "states": ["waiting", "done"],
                "terminal_states": ["done"],
                "req": ["R2"],
            },
        ],
        "state_vars": [
            {"name": "items", "type": {"Int": [0, 1]}, "init": 0, "req": ["R1", "R2"]},
        ],
        "milestones": [
            {"name": "item_produced", "description": "Producer emits item", "req": ["R1"]},
            {"name": "item_consumed", "description": "Consumer handles item", "req": ["R2"]},
        ],
        "steps": [
            {
                "id": "p_produce",
                "role": "producer",
                "from_state": "ready",
                "to_state": "done",
                "guard": "items == 0",
                "updates": [{"target": "items", "expr": "items + 1"}],
                "milestone": "item_produced",
                "req": ["R1"],
            },
            {
                "id": "c_consume",
                "role": "consumer",
                "from_state": "waiting",
                "to_state": "done",
                "guard": "items == 1",
                "updates": [{"target": "items", "expr": "items - 1"}],
                "milestone": "item_consumed",
                "req": ["R2"],
            },
        ],
    }


def _pc_scir_design(*, include_consumed_ms: bool = True, wrong_order: bool = False) -> dict:
    c_extra = (
        [{"id": "c_bad", "kind": "nop", "milestone": "item_consumed"}]
        if wrong_order
        else []
    )
    consume_stmt: dict = {
        "id": "c_hole",
        "kind": "stmt",
        "req": ["R2"],
        "op": {
            "kind": "seq_hole",
            "id": "h_consume",
            "desc": "consume item",
            "reads": ["items"],
            "writes": ["items"],
            "updates_modeled": [{"target": "items", "expr": "items - 1"}],
            "expected_type": "()",
        },
    }
    if include_consumed_ms and not wrong_order:
        consume_stmt["milestone"] = "item_consumed"

    return {
        "program": "pc_design",
        "version": "4.0.0",
        "entry": "main::main",
        "modules": [
            {
                "name": "main",
                "resources": [
                    {"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"},
                    {"name": "cv", "kind": "sync", "type": "Condvar", "mode": "Sync"},
                    {
                        "name": "items",
                        "kind": "var",
                        "type": "Var",
                        "base": {"Int": [0, 1]},
                        "init": 0,
                    },
                ],
                "protection": [{"var": "items", "lock": "mtx"}],
                "functions": [
                    {
                        "name": "producer",
                        "kind": "normal",
                        "form": "closure",
                        "req": ["R1"],
                        "structured_body": [
                            {
                                "id": "p_lg",
                                "kind": "lock_guard",
                                "resource": "mtx",
                                "body": [
                                    {
                                        "id": "p_hole",
                                        "kind": "stmt",
                                        "milestone": "item_produced",
                                        "req": ["R1"],
                                        "op": {
                                            "kind": "seq_hole",
                                            "id": "h_produce",
                                            "desc": "produce item",
                                            "reads": ["items"],
                                            "writes": ["items"],
                                            "updates_modeled": [
                                                {"target": "items", "expr": "items + 1"}
                                            ],
                                            "expected_type": "()",
                                        },
                                    },
                                    {"id": "p_sig", "kind": "condvar_notify_all", "condvar": "cv"},
                                ],
                            }
                        ],
                    },
                    {
                        "name": "consumer",
                        "kind": "normal",
                        "form": "closure",
                        "req": ["R2"],
                        "structured_body": c_extra
                        + [
                            {
                                "id": "c_lg",
                                "kind": "lock_guard",
                                "resource": "mtx",
                                "body": [
                                    {
                                        "id": "c_wait",
                                        "kind": "while_wait",
                                        "cond": "items == 0",
                                        "condvar": "cv",
                                        "lock": "mtx",
                                        "body": [],
                                    },
                                    consume_stmt,
                                ],
                            }
                        ],
                    },
                    {
                        "name": "main",
                        "kind": "normal",
                        "structured_body": [
                            {"id": "m_sc", "kind": "scope", "funcs": ["producer", "consumer"]}
                        ],
                    },
                ],
            }
        ],
    }


def _pc_contract() -> dict:
    return {
        "name": "pc-contract",
        "properties": [
            {
                "kind": "health",
                "id": "p_health",
                "checks": ["deadlock", "lost_wakeup", "data_race"],
                "req": ["R1", "R2"],
            },
            {
                "kind": "pattern",
                "id": "p_order",
                "pattern": "precedence",
                "effect": {"kind": "milestone_reached", "milestone": "item_consumed"},
                "cause": {"kind": "milestone_reached", "milestone": "item_produced"},
                "req": ["R2"],
            },
        ],
        "preserved": [
            {
                "kind": "reachable",
                "description": "consumer reaches item_consumed",
                "goal": {"kind": "milestone_reached", "milestone": "item_consumed"},
                "req": ["R2"],
            }
        ],
    }


class PatternAndMilestoneOfflineTests(unittest.TestCase):
    def test_patterns_health_and_milestone_sanity_and_explain(self):
        structured = StructuredRequirements(
            title="pc",
            entities=GroundedEntities(
                roles=["producer", "consumer"],
                resources=["mtx", "cv", "items"],
                milestones=["item_produced", "item_consumed"],
            ),
            clauses=[
                RequirementClause(id="R1", text="Producer produces safely", category="safety", verifiable=True),
                RequirementClause(id="R2", text="Consumer consumes after produce", category="liveness", verifiable=True),
            ],
        )
        contract = ensure_implication_non_vacuous({
            "name": "all-patterns",
            "properties": [
                {"kind": "health", "id": "p_h", "checks": ["deadlock", "lost_wakeup", "data_race"], "req": ["R1"]},
                {"kind": "pattern", "id": "p_alw", "pattern": "always", "cond": {"kind": "var_cmp", "resource": "items", "op": "<=", "value": 1}, "req": ["R1"]},
                {"kind": "pattern", "id": "p_exc", "pattern": "exclude", "preds": [{"kind": "mutex_free", "resource": "mtx"}, {"kind": "mutex_held", "resource": "mtx"}], "req": ["R1"]},
                {"kind": "pattern", "id": "p_max", "pattern": "at_most", "conds": [{"kind": "var_eq", "resource": "items", "value": 1}], "max": 1, "req": ["R1"]},
                {"kind": "pattern", "id": "p_pre", "pattern": "precedence", "effect": {"kind": "milestone_reached", "milestone": "item_consumed"}, "cause": {"kind": "milestone_reached", "milestone": "item_produced"}, "req": ["R2"]},
                {"kind": "pattern", "id": "p_trg", "pattern": "triggers", "cause": {"kind": "milestone_reached", "milestone": "item_produced"}, "effect": {"kind": "milestone_reached", "milestone": "item_consumed"}, "req": ["R2"]},
                {"kind": "pattern", "id": "p_rch", "pattern": "can_reach", "goal": {"kind": "milestone_reached", "milestone": "item_produced"}, "req": ["R1"]},
                {"kind": "pattern", "id": "p_acr", "pattern": "always_can_reach", "goal": {"kind": "var_eq", "resource": "items", "value": 0}, "req": ["R2"]},
            ],
        })
        self.assertTrue(contract["properties"][4]["non_vacuous"])
        self.assertTrue(contract["properties"][5]["non_vacuous"])

        sanity = validate_contract_sanity(contract, structured)
        self.assertTrue(sanity.ok, sanity.as_dict())

        exp = explain_contract_data(contract)
        rendered = exp.render_markdown()
        self.assertIn("Pattern(precedence)", rendered)
        self.assertIn("Concurrency health obligations", rendered)
        self.assertIn("item_produced", rendered)

        # Ungrounded milestone fails sanity when entities.milestones is declared.
        bad_contract = {
            "name": "bad-ms",
            "properties": [
                {
                    "kind": "pattern",
                    "id": "p_bad",
                    "pattern": "can_reach",
                    "goal": {"kind": "milestone_reached", "milestone": "unknown_ms"},
                }
            ],
        }
        bad_sanity = validate_contract_sanity(bad_contract, structured)
        self.assertFalse(bad_sanity.ok)
        self.assertTrue(any(i.code == "ungrounded_milestone" for i in bad_sanity.errors))


class LiveTwoNetRefinementTests(unittest.TestCase):
    def setUp(self):
        self.binary = find_concir_backend()
        if self.binary is None:
            self.skipTest("concir-backend binary not found")
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.client = ConcirClient(self.binary, workdir=self.root / "calls", timeout=60.0)

    def tearDown(self):
        self._tmp.cleanup()

    def test_client_check_req_and_verify_design(self):
        req_ir = _pc_req_ir()
        res_ok = self.client.check_req(req_ir)
        self.assertEqual(res_ok.status, "pass", res_ok)

        bad_req = _pc_req_ir()
        bad_req["milestones"].append({"name": "dead_ms", "description": "unused", "req": ["R1"]})
        res_bad = self.client.check_req(bad_req)
        self.assertEqual(res_bad.status, "fail", res_bad)
        self.assertIn("dead_ms", (res_bad.payload or {}).get("missing_milestones", []))

        vd_ok = self.client.verify_design(req_ir, _pc_scir_design())
        self.assertEqual(vd_ok.status, "pass", vd_ok)

        vd_wrong = self.client.verify_design(req_ir, _pc_scir_design(wrong_order=True))
        self.assertEqual(vd_wrong.status, "fail", vd_wrong)
        self.assertFalse((vd_wrong.payload or {})["checks"]["weak_simulation"])

        vd_miss = self.client.verify_design(req_ir, _pc_scir_design(include_consumed_ms=False))
        self.assertEqual(vd_miss.status, "fail", vd_miss)
        self.assertIn("item_consumed", (vd_miss.payload or {}).get("missing_milestones", []))

    def test_stage0_checks_req_ir_via_client(self):
        structured = StructuredRequirements(
            title="pc",
            entities=GroundedEntities(
                roles=["producer", "consumer"],
                resources=["mtx", "cv", "items"],
                milestones=["item_produced", "item_consumed"],
            ),
            clauses=[
                RequirementClause(id="R1", text="Produce", category="safety", verifiable=True),
                RequirementClause(id="R2", text="Consume", category="liveness", verifiable=True),
            ],
        )
        fres = formalize_from_structured(
            structured,
            contract=_pc_contract(),
            client=self.client,
            out_dir=self.root / "stage0-ok",
            req_ir=_pc_req_ir(),
        )
        self.assertTrue(fres.ok, fres.sanity.as_dict())
        self.assertIsNotNone(fres.req_check)
        self.assertEqual(fres.req_check["outcome"], "pass")
        self.assertTrue(Path(fres.req_ir_path).is_file())

    def test_e2e_loop_a_verify_design_feedback_and_manifest(self):
        cir_prov = ScriptedProvider([
            # Attempt 1: valid explore, but missing 'item_consumed' milestone -> caught by verify-design
            {"text": json.dumps(_pc_scir_design(include_consumed_ms=False))},
            # Attempt 2: both milestones present -> passes explore + verify-design
            {"text": json.dumps(_pc_scir_design(include_consumed_ms=True))},
        ])
        hole_prov = ScriptedProvider([
            {"text": json.dumps({"h_produce": "let _ = 1;", "h_consume": "let _ = 1;"})},
        ])
        res = run_e2e_pipeline(
            requirements="- R1 [V]: Produce\n- R2 [V]: Consume\n",
            contract={
                "name": "basic-health",
                "properties": [
                    {"kind": "health", "id": "p_h", "checks": ["deadlock", "lost_wakeup", "data_race"], "req": ["R1", "R2"]},
                ],
                "preserved": [
                    {"kind": "reachable", "description": "produced", "goal": {"kind": "milestone_reached", "milestone": "item_produced"}, "req": ["R1"]},
                ],
            },
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "e2e",
            static_verifier=CompositeStaticVerifier([AstHolePurityPlugin()]),
            req_ir=_pc_req_ir(),
        )
        # Note: attempt 1 fails milestone_reached contract resolve (`item_produced` present, not missing `item_consumed`, so health/preserved PASS on attempt 1 unless verify-design catches missing `item_consumed`!)
        self.assertEqual(res.status, "verified", res.error)
        self.assertEqual(len(res.cir_rounds), 2, f"expected Loop A retry on verify-design fail: {res.cir_rounds}")
        self.assertEqual(res.cir_rounds[0]["result"], "design_refinement_fail")
        self.assertEqual(res.cir_rounds[1]["result"], "accepted")
        self.assertIsNotNone(res.req_net_check)
        self.assertEqual(res.req_net_check["outcome"], "pass")
        self.assertIsNotNone(res.design_refinement)
        self.assertEqual(res.design_refinement["outcome"], "pass")
        self.assertIn("req_net_check", res.manifest)
        self.assertIn("design_refinement", res.manifest)

    def test_cli_and_mcp_check_req_and_verify_design(self):
        req_path = self.root / "req_ir.json"
        design_path = self.root / "design.scir.json"
        req_path.write_text(json.dumps(_pc_req_ir()), encoding="utf-8")
        design_path.write_text(json.dumps(_pc_scir_design()), encoding="utf-8")

        rc_req = cli_main(["--out", str(self.root / "cli"), "check-req", str(req_path)])
        self.assertEqual(rc_req, 0)
        rc_vd = cli_main([
            "--out", str(self.root / "cli"),
            "verify-design",
            "--req", str(req_path),
            "--design", str(design_path),
        ])
        self.assertEqual(rc_vd, 0)

        server = create_server(workdir=self.root / "mcp")
        resp1 = server.handle_message({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "concplan_check_req", "arguments": {"req_ir": _pc_req_ir()}},
        })
        p1 = resp1["result"]["structuredContent"]
        self.assertEqual(p1["outcome"], "pass", p1)

        resp2 = server.handle_message({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "concplan_verify_design",
                "arguments": {"req_ir": _pc_req_ir(), "program": _pc_scir_design()},
            },
        })
        p2 = resp2["result"]["structuredContent"]
        self.assertEqual(p2["outcome"], "pass", p2)
        self.assertTrue(p2["checks"]["weak_simulation"])
