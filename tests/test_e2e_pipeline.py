"""Unit tests for the end-to-end bi-level pipeline (Loops A, B, C & traceability manifest)."""

from __future__ import annotations

import json
import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path

from concplan.code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    StaticVerificationReport,
    StaticViolation,
)
from concplan.core.concir_client import ConcirClient
from concplan.core.prompts import (
    hole_repair_system_prompt,
    scir_generation_system_prompt,
)
from concplan.core.providers import ScriptedProvider
from concplan.pipeline.e2e_pipeline import parse_hole_reply, run_e2e_pipeline

SCIR_PROGRAM = {
    "program": "demo_counter",
    "version": "4.0.0",
    "entry": "main::main",
    "modules": [
        {
            "name": "main",
            "req": ["R1", "R2"],
            "provides": {"resources": ["m", "c"], "functions": ["main", "w1"]},
            "requires": {"resources": [], "functions": []},
            "resources": [
                {"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R1"]},
                {"name": "c", "kind": "var", "type": "Var", "base": "Int", "init": 0, "req": ["R2"]},
            ],
            "protection": [{"var": "c", "lock": "m"}],
            "functions": [
                {
                    "name": "w1",
                    "kind": "normal",
                    "form": "closure",
                    "req": ["R2"],
                    "structured_body": [
                        {
                            "id": "b1",
                            "kind": "lock_guard",
                            "resource": "main::m",
                            "req": ["R1"],
                            "body": [
                                {
                                    "id": "b2",
                                    "kind": "stmt",
                                    "req": ["R2"],
                                    "op": {
                                        "kind": "seq_hole",
                                        "id": "h_inc",
                                        "desc": "increment counter",
                                        "reads": ["main::c"],
                                        "writes": ["main::c"],
                                        "updates_modeled": [{"target": "main::c", "expr": "c + 1"}],
                                        "expected_type": "i64",
                                        "req": ["R2"],
                                    },
                                }
                            ],
                        },
                        {"id": "b3", "kind": "stmt", "op": {"kind": "return"}},
                    ],
                },
                {
                    "name": "main",
                    "kind": "normal",
                    "structured_body": [
                        {"id": "b1", "kind": "stmt", "op": {"kind": "scope", "funcs": ["main::w1"]}},
                        {"id": "b2", "kind": "stmt", "op": {"kind": "return"}},
                    ],
                },
            ],
        }
    ],
}

FLAT_CIR_LOWERED = {
    "program": "demo_counter",
    "version": "3.5.0",
    "entry": "main::main",
    "modules": [
        {
            "name": "main",
            "req": ["R1", "R2"],
            "provides": {"resources": ["m", "c"], "functions": ["main", "w1"]},
            "requires": {"resources": [], "functions": []},
            "resources": [
                {"name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R1"]},
                {"name": "c", "kind": "var", "type": "Var", "base": "Int", "init": 0, "req": ["R2"]},
            ],
            "protection": [{"var": "c", "lock": "m"}],
            "functions": [
                {
                    "name": "w1",
                    "kind": "normal",
                    "form": "closure",
                    "req": ["R2"],
                    "body": [
                        {"sid": "s1", "kind": "mutex_lock", "resource": "main::m", "req": ["R1"]},
                        {
                            "sid": "s2",
                            "kind": "seq_hole",
                            "id": "h_inc",
                            "reads": ["main::c"],
                            "writes": ["main::c"],
                            "expected_type": "i64",
                            "req": ["R2"],
                        },
                        {"sid": "s3", "kind": "mutex_unlock", "resource": "main::m", "req": ["R1"]},
                        {"sid": "s4", "kind": "return"},
                    ],
                },
                {
                    "name": "main",
                    "kind": "normal",
                    "body": [
                        {"sid": "s1", "kind": "scope", "funcs": ["main::w1"]},
                        {"sid": "s2", "kind": "return"},
                    ],
                },
            ],
        }
    ],
}

TRACE_MAP_DATA = {
    "schema": "concir-trace-map/1",
    "program": "demo_counter",
    "entries": {
        "main::w1::s1": {
            "module": "main",
            "function": "w1",
            "sid": "s1",
            "structured_id": "b1",
            "hole_id": None,
            "req": ["R1"],
        },
        "main::w1::s2": {
            "module": "main",
            "function": "w1",
            "sid": "s2",
            "structured_id": "b2",
            "hole_id": "h_inc",
            "req": ["R2"],
        },
    },
}

CONTRACT = {
    "deadlock_free": True,
    "preserved": [{"id": "w1_done", "kind": "function_completed", "function": "main::w1"}],
}

TWO_HOLE_SKELETON = """\
fn cf_w1() {
    cir_trace::ev(tag, "s1"); // @cir s1
    let _v1: i64 = /* HOLE(h1) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s2"); // @cir s2
    let _v2: i64 = /* HOLE(h2) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s3"); // @cir s3
}
"""

TWO_HOLE_CODEGEN = {
    "resources": [{"fqn": "main::m", "field": "r0_main_m", "kind": "Mutex"}],
    "holes": [
        {"id": "h1", "module": "main", "function": "main::w1", "sid": "s1", "expected": "i64", "line": 3, "req": ["R1"]},
        {"id": "h2", "module": "main", "function": "main::w1", "sid": "s2", "expected": "i64", "line": 5, "req": ["R2"]},
    ],
}

ONE_HOLE_SKELETON = """\
fn cf_w1() {
    cir_trace::ev(tag, "s1"); // @cir s1
    let _v: i64 = /* HOLE(h_inc) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s3"); // @cir s3
}
"""

ONE_HOLE_CODEGEN = {
    "resources": [{"fqn": "main::m", "field": "r0_main_m", "kind": "Mutex"}],
    "holes": [
        {"id": "h_inc", "module": "main", "function": "main::w1", "sid": "s2", "expected": "i64", "line": 3, "req": ["R2"]},
    ],
}


def _make_smart_backend_stub(directory: Path) -> Path:
    """Create a cross-platform stub CLI implementing lower/check/support/explore/summarize."""
    directory.mkdir(parents=True, exist_ok=True)
    script = f"""\
import json, sys
from pathlib import Path

args = sys.argv[1:]
cmd = args[0] if args else ""
lowered = {json.dumps(FLAT_CIR_LOWERED)!r}
trace_map = {json.dumps(TRACE_MAP_DATA)!r}

if cmd == "lower":
    out_p = None
    tm_p = None
    for i, a in enumerate(args):
        if a == "--out" and i + 1 < len(args):
            out_p = Path(args[i + 1])
        if a == "--trace-map" and i + 1 < len(args):
            tm_p = Path(args[i + 1])
    if out_p is not None:
        out_p.write_text(lowered, encoding="utf-8")
    if tm_p is not None:
        tm_p.write_text(trace_map, encoding="utf-8")
    sys.stdout.write(lowered)
    sys.exit(0)

if cmd == "check":
    sys.stdout.write(json.dumps({{"valid": True, "diagnostics": []}}))
    sys.exit(0)

if cmd == "support":
    sys.stdout.write(json.dumps({{"supported": True, "unsupported": []}}))
    sys.exit(0)

if cmd == "summarize":
    sys.stdout.write(json.dumps({{
        "schema": "concir-module-summary/1",
        "cross_module_lock_cycle": False,
        "modules": [{{"name": "main", "cross_module_lock_cycle": False}}],
    }}))
    sys.exit(0)

if cmd == "validate-contract":
    c = json.loads(Path(args[1]).read_text(encoding="utf-8"))
    out = {{
        "schema": "concir-contract-validation/1",
        "ok": True,
        "checked_properties": len(c.get("properties") or []),
        "checked_preserved": len(c.get("preserved") or []),
        "issues": [],
    }}
    if "--normalize" in args:
        out["normalized_contract"] = c
    sys.stdout.write(json.dumps(out))
    sys.exit(0)

if cmd == "explain-contract":
    c = json.loads(Path(args[1]).read_text(encoding="utf-8"))
    sys.stdout.write(json.dumps({{
        "schema": "concir-contract-explanation/1",
        "name": c.get("name", "contract"),
        "properties": [
            {{
                "id": p.get("id", ""),
                "kind": p.get("kind", ""),
                "req": p.get("req", []),
                "non_vacuous": bool(p.get("non_vacuous", False)),
                "explanation": f"Property {{p.get('id', '')}} ({{p.get('kind', '')}})",
            }}
            for p in (c.get("properties") or [])
            if isinstance(p, dict)
        ],
        "preserved": [
            {{
                "description": pr.get("description", ""),
                "req": pr.get("req", []),
                "explanation": f"Preserved {{pr.get('description', '')}}",
            }}
            for pr in (c.get("preserved") or [])
            if isinstance(pr, dict)
        ],
    }}))
    sys.exit(0)

if cmd == "explore":
    prog = json.loads(Path(args[1]).read_text(encoding="utf-8"))
    if prog.get("program") == "fail_first":
        sys.stdout.write(json.dumps({{
            "outcome": "FAIL",
            "complete": True,
            "properties": [{{"id": "deadlock_free", "outcome": "FAIL"}}],
            "diagnostics": [{{"property": "deadlock_free", "outcome": "FAIL", "message": "lock cycle"}}],
        }}))
        sys.exit(1)
    if prog.get("program") == "vacuous_first":
        sys.stdout.write(json.dumps({{
            "outcome": "FAIL",
            "complete": True,
            "properties": [{{
                "id": "liveness-implication",
                "outcome": "FAIL",
                "vacuous": True,
                "vacuous_antecedents": ["main::c > 0"],
                "explanation": "CTL implication antecedent never reached",
            }}],
            "diagnostics": [{{
                "property": "liveness-implication",
                "outcome": "FAIL",
                "message": "vacuous implication: antecedent main::c > 0 never holds",
            }}],
        }}))
        sys.exit(1)
    sys.stdout.write(json.dumps({{
        "outcome": "PASS",
        "complete": True,
        "properties": [
            {{"id": "deadlock_free", "outcome": "PASS"}},
            {{"id": "preserved:w1_done", "outcome": "PASS"}},
        ],
        "diagnostics": [],
    }}))
    sys.exit(0)

sys.stderr.write(f"unknown cmd: {{cmd}}\\n")
sys.exit(2)
"""
    if os.name == "nt":
        py_file = directory / "smart-backend.py"
        py_file.write_text(script, encoding="utf-8")
        cmd_file = directory / "smart-backend.cmd"
        cmd_file.write_text(
            f'@echo off\r\n"{sys.executable}" "{py_file}" %*\r\n', encoding="utf-8"
        )
        return cmd_file
    bin_file = directory / "smart-backend"
    bin_file.write_text("#!/usr/bin/env python3\n" + script, encoding="utf-8")
    bin_file.chmod(bin_file.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)
    return bin_file


class _AlwaysPassPlugin:
    name = "stub_refinement_pass"

    def verify(self, **kwargs) -> StaticVerificationReport:
        return StaticVerificationReport(
            status="pass",
            plugin=self.name,
            checks={"weak_simulation": True, "deadlock_free": True},
        )


def _make_codegen_fn(skeleton_src: str, codegen_json: dict):
    def _fn(_prog: Path, out_dir: Path) -> dict:
        (out_dir / "src").mkdir(parents=True, exist_ok=True)
        (out_dir / "Cargo.toml").write_text(
            '[package]\nname = "cir_generated"\nversion = "0.1.0"\nedition = "2021"\n',
            encoding="utf-8",
        )
        (out_dir / "src/main.rs").write_text(skeleton_src, encoding="utf-8")
        (out_dir / "codegen.json").write_text(
            json.dumps(codegen_json, indent=2), encoding="utf-8"
        )
        return codegen_json

    return _fn


class E2EPipelineTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.backend = _make_smart_backend_stub(self.root / "bin")
        self.client = ConcirClient(self.backend, workdir=self.root / "calls", timeout=10.0)

    def tearDown(self):
        self._tmp.cleanup()

    def test_prompts_load_cleanly(self):
        scir_p = scir_generation_system_prompt()
        self.assertIn("lock_guard", scir_p)
        self.assertIn("while_wait", scir_p)
        self.assertIn("seq_hole", scir_p)
        hole_p = hole_repair_system_prompt()
        self.assertIn("escalate", hole_p)
        self.assertIn("O1", hole_p)

    def test_parse_hole_reply_fills_and_escalation(self):
        fills, esc = parse_hole_reply('```json\n{"h1": "1 + 2"}\n```')
        self.assertEqual(fills, {"h1": "1 + 2"})
        self.assertIsNone(esc)

        fills2, esc2 = parse_hole_reply(
            '{"escalate": {"reason": "needs extra lock", "module": "main"}}'
        )
        self.assertIsNone(fills2)
        self.assertEqual(esc2["reason"], "needs extra lock")
        self.assertEqual(esc2["module"], "main")

    def test_scir_loop_a_and_traceability_manifest(self):
        # Attempt 1 fails explore (Loop A), Attempt 2 sends SCIR v4.0 -> lowers -> PASS!
        bad_cir = dict(FLAT_CIR_LOWERED, program="fail_first")
        cir_prov = ScriptedProvider([
            {"text": json.dumps(bad_cir)},
            {"text": json.dumps(SCIR_PROGRAM)},
        ])
        hole_prov = ScriptedProvider([
            {"text": json.dumps({"h_inc": "{\n    let x = 1;\n    x + 1\n}"})},
        ])
        verifier = CompositeStaticVerifier([AstHolePurityPlugin(), _AlwaysPassPlugin()])

        res = run_e2e_pipeline(
            requirements="R1: protect c with mutex m. R2: increment counter c.",
            contract=CONTRACT,
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "runs",
            static_verifier=verifier,
            run_summarize=True,
            requirements_clauses={"R1": "Mutex protection", "R2": "Counter increment"},
            codegen_fn=_make_codegen_fn(ONE_HOLE_SKELETON, ONE_HOLE_CODEGEN),
        )
        self.assertEqual(res.status, "verified")
        self.assertTrue(res.ok)
        self.assertEqual(len(res.cir_rounds), 2)
        self.assertEqual(res.cir_rounds[0]["result"], "explore_fail")
        self.assertEqual(res.cir_rounds[1]["lower_status"], "pass")
        self.assertEqual(res.cir_rounds[1]["summarize_status"], "pass")
        self.assertIsNotNone(res.manifest_path)
        self.assertTrue(Path(res.manifest_path).is_file())

        manifest = json.loads(Path(res.manifest_path).read_text(encoding="utf-8"))
        self.assertEqual(manifest["schema"], "concir-traceability-manifest/1")
        self.assertEqual(manifest["status"], "verified")
        clauses_by_id = {c["clause_id"]: c for c in manifest["clauses"]}
        self.assertIn("R1", clauses_by_id)
        self.assertIn("R2", clauses_by_id)
        r2 = clauses_by_id["R2"]
        self.assertIn("deadlock_free", r2["contract_properties"])
        self.assertIn("preserved:w1_done", r2["contract_properties"])
        self.assertEqual(r2["holes"][0]["hole_id"], "h_inc")
        # Multi-line fill spans lines 3..6 in src/main.rs
        self.assertEqual(r2["holes"][0]["rust_span"], "src/main.rs:3-6")
        # Verify flat link preserves structured_id from TraceMap!
        r2_links = [lk for lk in manifest["links"] if lk["clause_id"] == "R2" and lk["hole_id"] == "h_inc"]
        self.assertEqual(len(r2_links), 1)
        self.assertEqual(r2_links[0]["cir_node"], "main::w1::s2")
        self.assertEqual(r2_links[0]["structured_id"], "b2")
        self.assertEqual(r2_links[0]["rust_location"], "src/main.rs:3-6")
        self.assertEqual(r2_links[0]["model_verdict"], "PASS")
        self.assertEqual(r2_links[0]["static_verdict"], "pass")

    def test_loop_b_single_hole_repair_preserves_passing_hole(self):
        cir_prov = ScriptedProvider([{"text": json.dumps(FLAT_CIR_LOWERED)}])
        # Initial fill: h1 is pure ("10 + 5"), h2 violates O1 purity ("m.lock().unwrap(); 2").
        # Repair reply: ONLY repairs h2 (and even tries to inject a change to h1, which must be ignored!).
        hole_prov = ScriptedProvider([
            {"text": json.dumps({"h1": "10 + 5", "h2": "m.lock().unwrap(); 2"})},
            {"text": json.dumps({"h1": "9999", "h2": "{\n    let y = 2;\n    y + 1\n}"})},
        ])
        verifier = CompositeStaticVerifier([AstHolePurityPlugin(), _AlwaysPassPlugin()])

        res = run_e2e_pipeline(
            requirements="R1: compute v1. R2: compute v2.",
            contract=CONTRACT,
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "runs",
            static_verifier=verifier,
            max_hole_rounds=3,
            codegen_fn=_make_codegen_fn(TWO_HOLE_SKELETON, TWO_HOLE_CODEGEN),
        )
        self.assertEqual(res.status, "verified")
        self.assertEqual(len(res.escalations), 0)
        # Exactly 1 initial fill + 1 Loop B repair round targeting ONLY h2.
        self.assertEqual(len(res.hole_rounds), 2)
        self.assertEqual(res.hole_rounds[1]["kind"], "hole_repair")
        self.assertEqual(res.hole_rounds[1]["target_holes"], ["h2"])
        self.assertEqual(res.hole_rounds[1]["result"], "repaired")
        # Passing hole h1 remained frozen at "10 + 5" (ignored rogue "9999").
        self.assertEqual(res.fills["h1"], "10 + 5")
        self.assertIn("y + 1", res.fills["h2"])
        # Verify the repair prompt sent to hole_prov mentioned h2 and violation feedback.
        repair_call = hole_prov.calls[1]
        self.assertIn('"h2"', repair_call.requirements)
        self.assertIn("sync_in_hole", repair_call.feedback)

    def test_loop_c_explicit_escalation_revises_cir(self):
        cir_prov = ScriptedProvider([
            {"text": json.dumps(FLAT_CIR_LOWERED)},
            {"text": json.dumps(SCIR_PROGRAM)},
        ])
        # Epoch 0 hole fill emits explicit escalation; Epoch 1 fills cleanly.
        hole_prov = ScriptedProvider([
            {"text": json.dumps({"escalate": {"reason": "need mutex-guarded accumulator", "module": "main"}})},
            {"text": json.dumps({"h_inc": "42"})},
        ])
        verifier = CompositeStaticVerifier([AstHolePurityPlugin(), _AlwaysPassPlugin()])

        res = run_e2e_pipeline(
            requirements="R1: protect c. R2: compute increment.",
            contract=CONTRACT,
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "runs",
            static_verifier=verifier,
            max_escalations=2,
            codegen_fn=_make_codegen_fn(ONE_HOLE_SKELETON, ONE_HOLE_CODEGEN),
        )
        self.assertEqual(res.status, "verified")
        self.assertEqual(len(res.escalations), 1)
        self.assertEqual(res.escalations[0]["source"], "hole_provider_explicit")
        self.assertEqual(res.escalations[0]["module"], "main")
        # Second CIR call must have received the escalation feedback!
        self.assertEqual(len(cir_prov.calls), 2)
        self.assertIn("cross_layer_escalation", cir_prov.calls[1].feedback)
        self.assertIn("need mutex-guarded accumulator", cir_prov.calls[1].feedback)

    def test_loop_c_escalation_after_hole_repair_exhausted(self):
        cir_prov = ScriptedProvider([
            {"text": json.dumps(FLAT_CIR_LOWERED)},
            {"text": json.dumps(FLAT_CIR_LOWERED)},
        ])
        # Epoch 0: initial fill + 1 repair round both fail with forbidden loop -> exhausts max_hole_rounds=1 -> Loop C!
        # Epoch 1: clean fill -> verified!
        hole_prov = ScriptedProvider([
            {"text": json.dumps({"h_inc": "loop {}"})},
            {"text": json.dumps({"h_inc": "while true {}"})},
            {"text": json.dumps({"h_inc": "1 + 1"})},
        ])
        verifier = CompositeStaticVerifier([AstHolePurityPlugin(), _AlwaysPassPlugin()])

        res = run_e2e_pipeline(
            requirements="R2: increment counter.",
            contract=CONTRACT,
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "runs",
            static_verifier=verifier,
            max_hole_rounds=1,
            max_escalations=1,
            codegen_fn=_make_codegen_fn(ONE_HOLE_SKELETON, ONE_HOLE_CODEGEN),
        )
        self.assertEqual(res.status, "verified")
        self.assertEqual(len(res.escalations), 1)
        self.assertEqual(res.escalations[0]["source"], "hole_repair_exhausted")
        self.assertEqual(res.escalations[0]["offending_holes"], ["h_inc"])
        self.assertEqual(len(cir_prov.calls), 2)
        self.assertIn("hole_repair_exhausted", cir_prov.calls[1].feedback)

    def test_stage0_nl_spec_vacuity_loop_a_and_type_hole_loop_b_repair(self):
        # Stage 0 Entry A: raw NL -> clauses -> contract -> Loop A (vacuous fail -> fixed)
        # -> Stage 3 (TypeHole th1 + SeqHole h_inc) -> Loop B repairs ONLY th1 (h_inc stays frozen!)
        clauses_reply = {
            "title": "counter_domain",
            "entities": {
                "roles": ["w1"],
                "resources": ["m", "c"],
                "domain_types": [{"name": "CounterStats", "description": "Pure stats", "kind": "hole", "req": ["R3"]}],
            },
            "clauses": [
                {"id": "R1", "text": "No deadlocks.", "category": "safety", "verifiable": True},
                {"id": "R2", "text": "Whenever c > 0, eventually w1 completes.", "category": "liveness", "verifiable": True},
                {"id": "R3", "text": "CounterStats holds pure statistics.", "category": "sequential_hole", "verifiable": False},
            ],
        }
        contract_reply = {
            "name": "counter-domain",
            "properties": [
                {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R1"]},
                {
                    "kind": "ctl",
                    "id": "liveness-implication",
                    "non_vacuous": True,
                    "req": ["R2"],
                    "formula": {
                        "op": "ag",
                        "arg": {
                            "op": "implies",
                            "lhs": {"op": "atom", "atom": {"kind": "var_cmp", "resource": "main::c", "op": ">", "value": 0}},
                            "rhs": {"op": "af", "arg": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::w1"}}},
                        },
                    },
                },
            ],
            "preserved": [
                {"kind": "reachable", "description": "w1 completes", "goal": {"kind": "function_completed", "function": "main::w1"}, "req": ["R2"]},
            ],
        }

        vac_cir = dict(FLAT_CIR_LOWERED, program="vacuous_first")
        clause_prov = ScriptedProvider([{"text": json.dumps(clauses_reply)}])
        contract_prov = ScriptedProvider([{"text": json.dumps(contract_reply)}])
        cir_prov = ScriptedProvider([
            {"text": json.dumps(vac_cir)},
            {"text": json.dumps(SCIR_PROGRAM)},
        ])
        hole_prov = ScriptedProvider([
            # Initial fill: h_inc is clean, th1 sneaks Mutex::new!
            {"text": json.dumps({
                "h_inc": "1 + 1",
                "th1": "#[derive(Default)]\npub struct CounterStats { pub lock: std::sync::Mutex<i64> }",
            })},
            # Loop B repair: fixes ONLY th1 (rogue change to h_inc must be ignored!)
            {"text": json.dumps({
                "h_inc": "9999",
                "th1": "#[derive(Clone, Debug, Default, PartialEq)]\npub struct CounterStats { pub hits: i64 }",
            })},
        ])

        domain_skel = (
            "/* TYPE_HOLE(th1) type: CounterStats */\n"
            "#[derive(Clone, Debug, Default, PartialEq)]\n"
            "pub struct CounterStats {}\n"
        )
        cg_with_th = {
            "resources": [{"fqn": "main::m", "field": "r0_main_m", "kind": "Mutex"}],
            "holes": [
                {"id": "h_inc", "module": "main", "function": "main::w1", "sid": "s2", "expected": "i64", "line": 3, "req": ["R2"]},
            ],
            "type_holes": [
                {"id": "th1", "type_name": "CounterStats", "module": "main", "file": "src/domain.rs", "line": 1, "req": ["R3"]},
            ],
        }

        def _codegen_with_domain(_prog: Path, out_dir: Path) -> dict:
            (out_dir / "src").mkdir(parents=True, exist_ok=True)
            (out_dir / "Cargo.toml").write_text('[package]\nname = "cir_generated"\nversion = "0.1.0"\nedition = "2021"\n', encoding="utf-8")
            (out_dir / "src/main.rs").write_text(ONE_HOLE_SKELETON, encoding="utf-8")
            (out_dir / "src/domain.rs").write_text(domain_skel, encoding="utf-8")
            (out_dir / "codegen.json").write_text(json.dumps(cg_with_th, indent=2), encoding="utf-8")
            return cg_with_th

        verifier = CompositeStaticVerifier([AstHolePurityPlugin(), _AlwaysPassPlugin()])
        res = run_e2e_pipeline(
            nl_spec="Protect counter c with mutex m, ensure w1 completes whenever c > 0, and store pure CounterStats.",
            clause_provider=clause_prov,
            contract_provider=contract_prov,
            client=self.client,
            cir_provider=cir_prov,
            hole_provider=hole_prov,
            out_dir=self.root / "runs_v5",
            static_verifier=verifier,
            codegen_fn=_codegen_with_domain,
        )

        self.assertEqual(res.status, "verified", res.error)
        # Loop A surfaced vacuity feedback on round 1 -> round 2 passed!
        self.assertEqual(len(cir_prov.calls), 2)
        self.assertIn("vacuous_properties", cir_prov.calls[1].feedback)
        self.assertIn("VACUOUS FAIL", cir_prov.calls[1].feedback)
        # Loop B repaired ONLY th1 and kept h_inc frozen at "1 + 1"!
        self.assertEqual(len(res.hole_rounds), 2)
        self.assertEqual(res.hole_rounds[1]["target_holes"], ["th1"])
        self.assertEqual(res.fills["h_inc"], "1 + 1")
        self.assertIn("hits: i64", res.fills["th1"])
        # Upgraded traceability_manifest.json contains all v5 sections!
        manifest = res.manifest
        self.assertIsNotNone(manifest["formalization"])
        self.assertIsNotNone(manifest["nl_explanations"])
        self.assertEqual(manifest["vacuity_status"]["has_vacuous"], False)
        self.assertEqual(len(manifest["type_holes"]), 1)
        self.assertEqual(manifest["type_holes"][0]["type_hole_id"], "th1")
        self.assertTrue(manifest["type_holes"][0]["rust_span"].startswith("src/domain.rs:"))


if __name__ == "__main__":
    unittest.main()
