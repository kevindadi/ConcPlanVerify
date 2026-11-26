"""Unit & CLI tests for Stage 0 Autoformalization (Entry A, Entry B, Step 0.3)."""

from __future__ import annotations

import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from concplan.__main__ import main as cli_main
from concplan.core.prompts import (
    ALL_PROMPT_ASSETS,
    clauses_to_contract_system_prompt,
    nl_to_clauses_system_prompt,
    prompt_asset_record,
)
from concplan.core.providers import ScriptedProvider
from concplan.formalize import (
    StructuredRequirements,
    clauses_to_contract,
    ensure_implication_non_vacuous,
    explain_contract_data,
    explain_ctl_formula,
    explain_fairness,
    explain_ltl_formula,
    explain_predicate,
    formalize_from_nl,
    formalize_from_structured,
    formula_contains_implication,
    nl_to_clauses,
    parse_structured_markdown,
    validate_contract_sanity,
)
from tests._helpers import make_stub_binary


SAMPLE_CLAUSES_JSON = {
    "title": "order_book_rwlock",
    "summary": "Concurrent order book guarded by an RwLock.",
    "entities": {
        "roles": ["reader", "writer"],
        "resources": ["book_lock", "pending"],
        "domain_types": [
            {
                "name": "OrderBook",
                "description": "Pure in-memory bid/ask book",
                "kind": "hole",
                "req": ["R4"],
            }
        ],
    },
    "clauses": [
        {
            "id": "R1",
            "text": "The system must never deadlock.",
            "category": "safety",
            "verifiable": True,
            "entities": ["book_lock"],
        },
        {
            "id": "R2",
            "text": "Both reader and writer must be able to complete.",
            "category": "reachability",
            "verifiable": True,
            "entities": ["reader", "writer"],
        },
        {
            "id": "R3",
            "text": "Whenever pending > 0, eventually pending == 0.",
            "category": "liveness",
            "verifiable": True,
            "entities": ["pending", "writer"],
        },
        {
            "id": "R4",
            "text": "OrderBook computes best-bid price matching purely in memory.",
            "category": "sequential_hole",
            "verifiable": False,
            "entities": ["OrderBook"],
        },
    ],
}

SAMPLE_TEMPORAL_CONTRACT = {
    "name": "order-book-rwlock",
    "properties": [
        {"kind": "deadlock_free", "id": "no-deadlock", "req": ["R1"]},
        {
            "kind": "safety",
            "id": "pending-non-negative",
            "invariant": {"kind": "var_cmp", "resource": "main::pending", "op": ">=", "value": 0},
            "req": ["R1"],
        },
        {
            "kind": "ctl",
            "id": "pending-drains-ctl",
            "req": ["R3"],
            # Deliberately omit non_vacuous to verify ensure_implication_non_vacuous defaults it to True!
            "formula": {
                "op": "ag",
                "arg": {
                    "op": "implies",
                    "lhs": {
                        "op": "atom",
                        "atom": {"kind": "var_cmp", "resource": "main::pending", "op": ">", "value": 0},
                    },
                    "rhs": {
                        "op": "af",
                        "arg": {
                            "op": "atom",
                            "atom": {"kind": "var_eq", "resource": "main::pending", "value": 0},
                        },
                    },
                },
            },
        },
        {
            "kind": "ltl",
            "id": "pending-drains-ltl",
            "non_vacuous": True,
            "req": ["R3"],
            "fairness": [
                {"kind": "justice", "atom": {"kind": "function_completed", "function": "main::writer"}},
            ],
            "formula": {
                "op": "always",
                "arg": {
                    "op": "implies",
                    "lhs": {
                        "op": "atom",
                        "atom": {"kind": "var_cmp", "resource": "main::pending", "op": ">", "value": 0},
                    },
                    "rhs": {
                        "op": "eventually",
                        "arg": {
                            "op": "atom",
                            "atom": {"kind": "var_eq", "resource": "main::pending", "value": 0},
                        },
                    },
                },
            },
        },
    ],
    "preserved": [
        {
            "kind": "reachable",
            "description": "reader completes",
            "goal": {"kind": "function_completed", "function": "main::reader"},
            "req": ["R2"],
        },
        {
            "kind": "reachable",
            "description": "writer completes",
            "goal": {"kind": "function_completed", "function": "main::writer"},
            "req": ["R2"],
        },
    ],
}


class Stage0FormalizationTests(unittest.TestCase):
    def test_stage0_prompts_bundled_and_hashed(self):
        rec = prompt_asset_record(include_stage0=True)
        self.assertEqual(set(rec.keys()), set(ALL_PROMPT_ASSETS))
        self.assertEqual(len(rec), 6)
        self.assertIn("domain_types", nl_to_clauses_system_prompt())
        self.assertIn("sequential_hole", nl_to_clauses_system_prompt())
        self.assertIn("non_vacuous", clauses_to_contract_system_prompt())
        self.assertIn("fairness", clauses_to_contract_system_prompt())

    def test_entry_a_formalize_from_nl_isolated_and_non_vacuous(self):
        clause_prov = ScriptedProvider([{"text": json.dumps(SAMPLE_CLAUSES_JSON)}])
        contract_prov = ScriptedProvider([{"text": json.dumps(SAMPLE_TEMPORAL_CONTRACT)}])

        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp) / "stage0"
            res = formalize_from_nl(
                "Build a deadlock-free RwLock order book where readers and writers finish, "
                "pending orders eventually drain to 0, and OrderBook computes pure best-bid matching.",
                clause_provider=clause_prov,
                contract_provider=contract_prov,
                out_dir=out_dir,
            )

            self.assertEqual(res.entry, "nl")
            self.assertTrue(res.ok, res.sanity.as_dict())
            self.assertTrue(Path(res.requirements_md_path).is_file())
            self.assertTrue(Path(res.requirements_json_path).is_file())
            self.assertTrue(Path(res.contract_path).is_file())
            self.assertTrue(Path(res.explanation_path).is_file())

            # Verify R4 is marked [U] (unverifiable) in REQUIREMENTS.md and entities are grounded.
            self.assertIn("[U] (sequential_hole)", res.requirements_md)
            self.assertIn("OrderBook", res.requirements_md)
            self.assertEqual(res.requirements.verifiable_clause_ids(), {"R1", "R2", "R3"})
            self.assertEqual(res.requirements.unverifiable_clause_ids(), {"R4"})

            # Verify Step 0.2 isolation: contract_provider never saw any candidate SCIR program!
            self.assertEqual(len(contract_prov.calls), 1)
            self.assertIsNone(contract_prov.calls[0].current_program)

            # Verify non_vacuous defaulted to True on CTL implication property!
            ctl_prop = next(p for p in res.contract["properties"] if p["id"] == "pending-drains-ctl")
            self.assertTrue(ctl_prop["non_vacuous"])

            # Verify deterministic NL back-translation populated for all properties and preserved.
            self.assertEqual(len(res.explanation.properties), 4)
            self.assertEqual(len(res.explanation.preserved), 2)
            ctl_exp = next(p for p in res.explanation.properties if p["id"] == "pending-drains-ctl")
            self.assertIn("in all reachable states", ctl_exp["explanation"].lower())
            self.assertIn("non-vacuous", ctl_exp["explanation"].lower())

    def test_entry_b_from_structured_markdown_synthesizes_and_validates(self):
        md = (
            "# OrderBook Spec\n\n"
            "## Entities\n"
            "- **Roles**: `reader`, `writer`\n"
            "- **Resources**: `book_lock`, `pending`\n"
            "- **Domain Types**: `OrderBook`\n\n"
            "## Requirements\n"
            "- **R1.** [V] (safety) No deadlocks.\n"
            "- **R2.** [V] (reachability) Both reader and writer complete.\n"
            "- **R3.** [V] (liveness) Pending orders eventually reach 0.\n"
            "- **R4.** [U] (sequential_hole) Pure OrderBook matching logic.\n"
        )
        # Case 1: Contract omitted -> Step 0.2 synthesizes it.
        contract_prov = ScriptedProvider([{"text": json.dumps(SAMPLE_TEMPORAL_CONTRACT)}])
        res = formalize_from_structured(md, contract_provider=contract_prov)
        self.assertEqual(res.entry, "structured")
        self.assertTrue(res.ok, res.sanity.as_dict())
        self.assertEqual(len(contract_prov.calls), 1)

        # Case 2: Contract provided -> validates & explains without any LLM call!
        res2 = formalize_from_structured(SAMPLE_CLAUSES_JSON, contract=SAMPLE_TEMPORAL_CONTRACT)
        self.assertEqual(res2.entry, "structured")
        self.assertTrue(res2.ok, res2.sanity.as_dict())
        self.assertEqual(res2.explanation.schema, "concir-contract-explanation/1")

    def test_step02_retries_on_sanity_error_and_recovers(self):
        structured = StructuredRequirements.from_dict(SAMPLE_CLAUSES_JSON)
        # Round 1 references unverifiable clause R4 & unknown resource ghost_var -> fails sanity!
        bad_contract = {
            "name": "bad",
            "properties": [
                {
                    "kind": "safety",
                    "id": "bad-prop",
                    "invariant": {"kind": "var_eq", "resource": "main::ghost_var", "value": 0},
                    "req": ["R4"],
                }
            ],
        }
        contract_prov = ScriptedProvider([
            {"text": json.dumps(bad_contract)},
            {"text": json.dumps(SAMPLE_TEMPORAL_CONTRACT)},
        ])
        contract, sanity = clauses_to_contract(structured, contract_prov, max_rounds=2)
        self.assertTrue(sanity.ok, sanity.as_dict())
        self.assertEqual(len(contract_prov.calls), 2)
        self.assertIn("unverifiable_req_clause", contract_prov.calls[1].feedback)
        self.assertIn("ungrounded_resource", contract_prov.calls[1].feedback)
        self.assertEqual(contract["name"], "order-book-rwlock")

    def test_sanity_check_catches_duplicates_unverifiable_ungrounded_and_vacuity(self):
        structured = StructuredRequirements.from_dict(SAMPLE_CLAUSES_JSON)
        bad_contract = {
            "name": "broken",
            "properties": [
                {"kind": "deadlock_free", "id": "dup", "req": ["R1"]},
                {"kind": "deadlock_free", "id": "dup", "req": ["R99"]},
                {
                    "kind": "ctl",
                    "id": "vac-ctl",
                    "non_vacuous": False,
                    "req": ["R4"],
                    "formula": {
                        "op": "ag",
                        "arg": {
                            "op": "implies",
                            "lhs": {"op": "atom", "atom": {"kind": "function_completed", "function": "main::ghost_fn"}},
                            "rhs": {"op": "af", "arg": {"op": "atom", "atom": {"kind": "mutex_free", "resource": "main::ghost_lock"}}},
                        },
                    },
                },
            ],
        }
        report = validate_contract_sanity(bad_contract, structured)
        self.assertFalse(report.ok)
        codes = {i.code for i in report.errors}
        self.assertIn("duplicate_property_id", codes)
        self.assertIn("unknown_req_clause", codes)
        self.assertIn("unverifiable_req_clause", codes)
        self.assertIn("ungrounded_function", codes)
        self.assertIn("ungrounded_resource", codes)
        self.assertIn("missing_non_vacuous", codes)

    def test_deterministic_back_translation_covers_all_predicate_ctl_ltl_shapes(self):
        # All PredicateSpec variants
        preds = [
            {"kind": "true"},
            {"kind": "false"},
            {"kind": "var_eq", "resource": "main::c", "value": 1},
            {"kind": "var_cmp", "resource": "main::c", "op": "<=", "value": 2},
            {"kind": "function_completed", "function": "main::w1"},
            {"kind": "function_completed_at_least", "function": "main::w1", "n": 2},
            {"kind": "scope_completed", "function": "main::main", "sid": "s1"},
            {"kind": "statement_reached", "function": "main::w1", "sid": "s2"},
            {"kind": "mutex_free", "resource": "main::m"},
            {"kind": "mutex_held", "resource": "main::m"},
            {"kind": "holds_all", "function": "main::w1", "resources": ["main::m1", "main::m2"]},
            {"kind": "mutex_exclusive", "resource": "main::m"},
            {"kind": "never_holds_all", "function": "main::w1", "resources": ["main::m1", "main::m2"]},
            {"kind": "channel_empty", "resource": "main::ch"},
            {"kind": "channel_at_least", "resource": "main::ch", "len": 1},
            {"kind": "not", "predicate": {"kind": "mutex_held", "resource": "main::m"}},
            {"kind": "and", "predicates": [{"kind": "true"}, {"kind": "false"}]},
            {"kind": "or", "predicates": [{"kind": "true"}, {"kind": "false"}]},
        ]
        for p in preds:
            rendered = explain_predicate(p)
            self.assertNotIn("<unknown", rendered)
            self.assertTrue(len(rendered) > 0)

        # CTL operators
        atom_a = {"op": "atom", "atom": {"kind": "mutex_held", "resource": "main::m"}}
        atom_b = {"op": "atom", "atom": {"kind": "mutex_free", "resource": "main::m"}}
        ctl_forms = [
            {"op": "true"},
            {"op": "false"},
            {"op": "not", "arg": atom_a},
            {"op": "and", "args": [atom_a, atom_b]},
            {"op": "or", "args": [atom_a, atom_b]},
            {"op": "ex", "arg": atom_b},
            {"op": "ax", "arg": atom_b},
            {"op": "ef", "arg": atom_b},
            {"op": "af", "arg": atom_b},
            {"op": "eg", "arg": atom_b},
            {"op": "ag", "arg": {"op": "implies", "lhs": atom_a, "rhs": {"op": "af", "arg": atom_b}}},
            {"op": "eu", "lhs": atom_a, "rhs": atom_b},
            {"op": "au", "lhs": atom_a, "rhs": atom_b},
        ]
        for f in ctl_forms:
            s = explain_ctl_formula(f)
            self.assertNotIn("<unknown", s)
        self.assertTrue(formula_contains_implication(ctl_forms[10]))
        self.assertFalse(formula_contains_implication(ctl_forms[0]))

        # LTL operators & fairness
        ltl_forms = [
            {"op": "true"},
            {"op": "false"},
            {"op": "not", "arg": atom_a},
            {"op": "and", "args": [atom_a, atom_b]},
            {"op": "or", "args": [atom_a, atom_b]},
            {"op": "next", "arg": atom_b},
            {"op": "eventually", "arg": atom_b},
            {"op": "always", "arg": {"op": "implies", "lhs": atom_a, "rhs": {"op": "eventually", "arg": atom_b}}},
            {"op": "until", "lhs": atom_a, "rhs": atom_b},
            {"op": "release", "lhs": atom_a, "rhs": atom_b},
        ]
        for f in ltl_forms:
            s = explain_ltl_formula(f)
            self.assertNotIn("<unknown", s)

        self.assertIn("infinitely often", explain_fairness({"kind": "justice", "atom": {"kind": "mutex_free", "resource": "main::m"}}))
        self.assertIn("then infinitely often", explain_fairness({
            "kind": "compassion",
            "enabled": {"kind": "mutex_free", "resource": "main::m"},
            "taken": {"kind": "mutex_held", "resource": "main::m"},
        }))

        # Full contract explanation & markdown rendering
        exp = explain_contract_data(ensure_implication_non_vacuous(SAMPLE_TEMPORAL_CONTRACT))
        md = exp.render_markdown()
        self.assertIn("pending-drains-ctl", md)
        self.assertIn("pending-drains-ltl", md)

    def test_cli_formalize_explain_contract_and_export_cvn(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            clause_script = root / "clauses.json"
            contract_script = root / "contract_resp.json"
            clause_script.write_text(json.dumps([{"text": json.dumps(SAMPLE_CLAUSES_JSON)}]), encoding="utf-8")
            contract_script.write_text(json.dumps([{"text": json.dumps(SAMPLE_TEMPORAL_CONTRACT)}]), encoding="utf-8")

            # 1. concplan formalize (Entry A)
            out_dir = root / "form-out"
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = cli_main([
                    "--out", str(out_dir),
                    "formalize",
                    "--nl-spec", "Build an order book guarded by RwLock.",
                    "--provider", "scripted",
                    "--script", str(clause_script),
                    "--contract-script", str(contract_script),
                ])
            self.assertEqual(rc, 0, buf.getvalue())
            f_out = json.loads(buf.getvalue())
            self.assertTrue(f_out["ok"])
            self.assertTrue((out_dir / "contract.json").is_file())
            self.assertTrue((out_dir / "REQUIREMENTS.md").is_file())

            # 2. concplan explain-contract --python-only
            exp_file = root / "explained.json"
            buf2 = io.StringIO()
            with contextlib.redirect_stdout(buf2):
                rc2 = cli_main([
                    "--out", str(root / "exp-work"),
                    "explain-contract", str(out_dir / "contract.json"),
                    "--python-only",
                    "--output", str(exp_file),
                ])
            self.assertEqual(rc2, 0)
            exp_payload = json.loads(buf2.getvalue())
            self.assertEqual(exp_payload["schema"], "concir-contract-explanation/1")
            self.assertTrue(exp_file.is_file())

            # 3. concplan export-cvn via stub binary
            stub = make_stub_binary(root / "bin")
            prog = root / "prog.json"
            prog.write_text('{"program": "demo", "modules": []}', encoding="utf-8")
            buf3 = io.StringIO()
            with mock.patch.dict(
                os.environ,
                {"STUB_STDOUT": '{"schema": "unipn-cvn/1", "net": {"places": [], "transitions": []}}', "STUB_EXIT": "0"},
            ):
                with contextlib.redirect_stdout(buf3):
                    rc3 = cli_main([
                        "--binary", str(stub),
                        "--out", str(root / "cvn-work"),
                        "export-cvn", str(prog),
                    ])
            self.assertEqual(rc3, 0)
            self.assertIn("unipn-cvn/1", buf3.getvalue())


if __name__ == "__main__":
    unittest.main()
