"""Tests for Stage 0 macro examples (`examples/`), loader, typed sanity checks, and CLI."""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from concplan.__main__ import main as cli_main
from concplan.code_verify.static_verify import SpnMirRefinementPlugin
from concplan.core.concir_client import ConcirClient
from concplan.formalize import (
    GroundedEntities,
    StructuredRequirements,
    validate_contract_sanity,
)
from concplan.pipeline.examples import (
    examples_summary,
    list_examples,
    load_example,
)
from tests._helpers import real_binary

EXPECTED_EXAMPLES = ("account_transfer", "config_cache", "task_handoff", "visit_counter")
FORBIDDEN_LOWLEVEL_TOKENS = ("Mutex", "RwLock", "Condvar", "Channel", "TypeHole", "SeqHole", "main::")


class ExamplesLoaderAndArtifactsTests(unittest.TestCase):
    def test_discovers_all_four_macro_examples(self):
        examples = list_examples()
        self.assertEqual([e.example_id for e in examples], list(EXPECTED_EXAMPLES))

        summary = examples_summary()
        self.assertEqual(summary["schema"], "concir-examples-summary/1")
        self.assertEqual(summary["total"], 4)
        self.assertEqual(
            [e["example_id"] for e in summary["examples"]],
            list(EXPECTED_EXAMPLES),
        )

    def test_each_example_has_macro_spec_and_valid_stage0_and_scir(self):
        binary = real_binary()
        for name in EXPECTED_EXAMPLES:
            ex = load_example(name)
            spec_text = ex.load_spec_text()
            self.assertTrue(spec_text.strip(), f"{name} spec.md must not be empty")
            for tok in FORBIDDEN_LOWLEVEL_TOKENS:
                self.assertNotIn(
                    tok,
                    spec_text,
                    f"{name}/spec.md must stay macro-level (found '{tok}')",
                )

            self.assertTrue(ex.requirements_md_path and ex.requirements_md_path.is_file())
            req_json = ex.load_requirements_json()
            self.assertIsNotNone(req_json)
            structured = StructuredRequirements.from_dict(req_json)
            self.assertGreater(len(structured.clauses), 0)

            clauses = ex.load_requirements_clauses()
            self.assertEqual(set(clauses.keys()), {c.id for c in structured.clauses})

            contract = ex.load_contract()
            sanity = validate_contract_sanity(contract, structured)
            self.assertTrue(sanity.ok, f"{name} sanity errors: {sanity.as_dict()}")

            self.assertTrue(ex.contract_explanation_path and ex.contract_explanation_path.is_file())
            self.assertTrue(ex.traceability_manifest_path and ex.traceability_manifest_path.is_file())
            vsum = ex.metadata.get("verification_summary") or {}
            self.assertEqual(vsum.get("e2e_status"), "verified", f"{name} summary: {vsum}")

            scir = ex.load_model_scir()
            self.assertIn("modules", scir)

            if binary is not None:
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    client = ConcirClient(binary, workdir=root / "calls", timeout=60.0)
                    scir_path = ex.model_scir_path
                    lowered_path = root / "lowered.cir.json"
                    low_res = client.lower(scir_path, out_path=lowered_path)
                    self.assertEqual(low_res.status, "pass", f"{name} lower: {low_res.payload}")

                    chk_res = client.check(lowered_path)
                    self.assertEqual(chk_res.status, "valid", f"{name} check: {chk_res.payload}")

                    sup_res = client.support(lowered_path)
                    self.assertEqual(sup_res.status, "supported", f"{name} support: {sup_res.payload}")

                    sum_res = client.summarize(lowered_path)
                    self.assertEqual(sum_res.status, "pass", f"{name} summarize: {sum_res.payload}")

                    exp_res = client.explore(lowered_path, ex.contract_path, engine="petri")
                    self.assertEqual(exp_res.outcome, "PASS", f"{name} explore: {exp_res.payload}")
                    self.assertTrue(exp_res.complete, f"{name} incomplete: {exp_res.payload}")
                    for prop in (exp_res.payload or {}).get("properties", []):
                        self.assertFalse(prop.get("vacuous"), f"{name} vacuous prop: {prop}")


class TypedSymbolAndHoleErrorAttributionTests(unittest.TestCase):
    def test_resource_entity_compatibility_rejects_mutex_free_on_rwlock(self):
        structured = StructuredRequirements.from_dict({
            "title": "cache",
            "entities": {
                "roles": ["reader", "writer"],
                "resources": [
                    {"name": "cache_lock", "kind": "RwLock", "domain": "Snapshot"},
                    {"name": "pending", "kind": "Var", "domain": "Int[0..=1]", "init": 0},
                ],
            },
            "clauses": [
                {"id": "R1", "text": "No deadlock.", "category": "safety", "verifiable": True},
            ],
        })
        self.assertIsInstance(structured.entities, GroundedEntities)
        self.assertEqual(structured.entities.resources, ["cache_lock", "pending"])
        bad_contract = {
            "name": "cache",
            "properties": [
                {
                    "kind": "safety",
                    "id": "bad-rwlock-pred",
                    "invariant": {"kind": "mutex_free", "resource": "main::cache_lock"},
                    "req": ["R1"],
                }
            ],
        }
        report = validate_contract_sanity(bad_contract, structured)
        self.assertFalse(report.ok)
        self.assertIn(
            "incompatible_resource_predicate",
            {i.code for i in report.issues},
        )

    def test_spn_plugin_attributes_rustc_compile_error_inside_hole_to_fail_hole(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            skel = root / "skel"
            filled = root / "filled"
            (skel / "src").mkdir(parents=True)
            (filled / "src").mkdir(parents=True)
            (filled / "src/main.rs").write_text("fn main() {}\n", encoding="utf-8")
            spec = root / "spec.json"
            spec.write_text("{}", encoding="utf-8")
            cmap = filled / "codegen_filled.json"
            cmap.write_text(
                json.dumps({
                    "holes": [
                        {
                            "id": "h1",
                            "sid": "s2",
                            "req": ["R2"],
                            "file": "src/main.rs",
                            "start_line": 22,
                            "end_line": 25,
                        }
                    ],
                    "hole_ranges": {
                        "h1": {"file": "src/main.rs", "start_line": 22, "end_line": 25}
                    },
                    "type_holes": [],
                    "type_hole_ranges": {},
                }),
                encoding="utf-8",
            )
            fake_proc = subprocess.CompletedProcess(
                args=["cargo", "check"],
                returncode=101,
                stdout="",
                stderr=(
                    "error[E0308]: mismatched types\n"
                    "  --> src\\main.rs:23:30\n"
                    "   |\n"
                    "23 |     let _x: i64 = true;\n"
                ),
            )
            plugin = SpnMirRefinementPlugin(spn_binary=root / "fake-spn.exe")
            with (
                mock.patch("concplan.code_verify.static_verify.find_spn", return_value=root / "fake-spn.exe"),
                mock.patch("concplan.code_verify.static_verify.find_concir_backend", return_value=root / "fake-concir.exe"),
                mock.patch("concplan.code_verify.static_verify.subprocess.run", return_value=fake_proc),
            ):
                rep = plugin.verify(
                    skeleton_dir=skel,
                    filled_dir=filled,
                    spec_path=spec,
                    codegen_map_path=cmap,
                )
            self.assertEqual(rep.status, "fail_hole")
            self.assertEqual(rep.offending_holes, ["h1"])
            self.assertEqual(rep.violations[0].kind, "compile_error_in_hole")
            self.assertEqual(rep.violations[0].sid, "s2")


class UnifiedCliTests(unittest.TestCase):
    def test_cli_list_examples(self):
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = cli_main(["list-examples"])
        self.assertEqual(rc, 0)
        payload = json.loads(buf.getvalue())
        self.assertEqual(payload["schema"], "concir-examples-summary/1")
        self.assertEqual(payload["total"], 4)
        ids = {e["example_id"] for e in payload["examples"]}
        self.assertEqual(ids, set(EXPECTED_EXAMPLES))

    def test_cli_formalize_entry_b_with_example(self):
        with tempfile.TemporaryDirectory() as tmp:
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = cli_main(["--out", tmp, "formalize", "--example", "visit_counter"])
            self.assertEqual(rc, 0, buf.getvalue())
            payload = json.loads(buf.getvalue())
            self.assertTrue(payload["ok"])
            self.assertEqual(payload["entry"], "structured")

    def test_cli_e2e_scripted_with_example(self):
        binary = real_binary()
        if binary is None:
            raise unittest.SkipTest("concir-backend binary not found (set CONCIR_BACKEND)")

        ex = load_example("visit_counter")
        scir = ex.load_model_scir()

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script_path = root / "script.json"
            script_path.write_text(
                json.dumps([{"text": json.dumps(scir)}]), encoding="utf-8"
            )
            hole_script_path = root / "hole_script.json"
            hole_script_path.write_text(
                json.dumps([{
                    "text": json.dumps({
                        "th1": "#[derive(Clone, Debug, Default, PartialEq)]\npub struct VisitSummary { pub count: i64, pub ready: bool }",
                        "h_record_a": "()",
                        "h_ready_a": "()",
                        "h_record_b": "()",
                        "h_ready_b": "()",
                    })
                }]),
                encoding="utf-8",
            )
            report_path = root / "e2e_report.json"
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = cli_main([
                    "--binary", str(binary),
                    "--out", str(root / "out"),
                    "e2e",
                    "--example", "visit_counter",
                    "--provider", "scripted",
                    "--script", str(script_path),
                    "--hole-script", str(hole_script_path),
                    "--ast-only",
                    "--report", str(report_path),
                ])
            self.assertEqual(rc, 0, buf.getvalue())
            out = json.loads(buf.getvalue())
            self.assertEqual(out["status"], "verified")
            self.assertTrue(out["ok"])
            self.assertTrue(report_path.is_file())
            self.assertTrue(Path(out["manifest_path"]).is_file())


if __name__ == "__main__":
    unittest.main()
