"""Offline tests for the conformance orchestration layer."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from concplan.code_verify.conformance import (
    TraceRun, conform_all, fill_holes, fill_prompt, lint_filled, parse_fill,
)

SKELETON = """fn cf_t1() {
    cir_trace::ev(tag, "s1"); // @cir s1
    let x = /* HOLE(h1) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s2"); // @cir s2
}
"""


def _write(root: Path, skeleton: str, filled: str) -> tuple[Path, Path]:
    skel = root / "skeleton"
    fill = root / "filled"
    for base, text in ((skel, skeleton), (fill, filled)):
        (base / "src").mkdir(parents=True, exist_ok=True)
        (base / "src/main.rs").write_text(text)
    (skel / "codegen.json").write_text(json.dumps({
        "holes": [{"id": "h1", "function": "t1", "sid": "s1", "expected": "i64"}]}))
    (fill / "codegen.json").write_text((skel / "codegen.json").read_text())
    return skel, fill


class LintTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def test_valid_fill_passes(self):
        filled = SKELETON.replace(
            "/* HOLE(h1) expected: i64 */ Default::default()", "2 + 3")
        skel, fill = _write(self.root, SKELETON, filled)
        self.assertTrue(lint_filled(skel, fill)["ok"])

    def test_forbidden_in_hole(self):
        filled = SKELETON.replace(
            "/* HOLE(h1) expected: i64 */ Default::default()",
            "x.lock().unwrap() as i64")
        skel, fill = _write(self.root, SKELETON, filled)
        out = lint_filled(skel, fill)
        self.assertFalse(out["ok"])
        self.assertIn("forbidden_in_hole", [v["kind"] for v in out["violations"]])

    def test_change_outside_hole(self):
        filled = SKELETON.replace('cir_trace::ev(tag, "s2")', 'cir_trace::ev(tag, "sX")')
        skel, fill = _write(self.root, SKELETON, filled)
        out = lint_filled(skel, fill)
        self.assertIn("changed_outside_hole", [v["kind"] for v in out["violations"]])

    def test_duplicate_sid_flagged(self):
        filled = SKELETON.replace("// @cir s2", "// @cir s1")
        skel, fill = _write(self.root, SKELETON, filled)
        out = lint_filled(skel, fill)
        kinds = [v["kind"] for v in out["violations"]]
        self.assertIn("sid_count", kinds)


class FillTests(unittest.TestCase):
    def test_parse_fill_fenced_json(self):
        self.assertEqual(parse_fill('```json\n{"h1": "1"}\n```'), {"h1": "1"})
        self.assertEqual(parse_fill('{"h1": "1", "h2": "2"}'), {"h1": "1", "h2": "2"})
        self.assertEqual(parse_fill("not json"), {})

    def test_multifile_and_type_hole_fill_prompt_fill_holes_and_lint(self):
        main_skel = """mod book;
mod domain;
fn cf_main() {
    cir_trace::ev(tag, "s1"); // @cir s1
}
"""
        book_skel = """fn cf_reader() {
    cir_trace::ev(tag, "s1"); // @cir s1
    let v = /* HOLE(h_book) expected: i64 */ Default::default();
    cir_trace::ev(tag, "s2"); // @cir s2
}
"""
        domain_skel = """/* TYPE_HOLE(th1) type: OrderBook */
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrderBook {}
const _: fn() = || { fn assert_domain_bounds<T: Send + Sync + Clone + Default + 'static>() {} assert_domain_bounds::<OrderBook>(); };
"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            skel = root / "skeleton"
            (skel / "src").mkdir(parents=True)
            (skel / "src/main.rs").write_text(main_skel, encoding="utf-8")
            (skel / "src/book.rs").write_text(book_skel, encoding="utf-8")
            (skel / "src/domain.rs").write_text(domain_skel, encoding="utf-8")
            (skel / "codegen.json").write_text(
                json.dumps({
                    "holes": [
                        {
                            "id": "h_book",
                            "function": "book::reader",
                            "sid": "s1",
                            "expected": "i64",
                            "file": "src/book.rs",
                        }
                    ],
                    "type_holes": [
                        {
                            "id": "th1",
                            "type_name": "OrderBook",
                            "module": "book",
                            "file": "src/domain.rs",
                            "line": 1,
                        }
                    ],
                }),
                encoding="utf-8",
            )

            prompt = fill_prompt(skel, "R1: pure OrderBook and reader.")
            self.assertIn("### type_hole th1 (type OrderBook in src/domain.rs)", prompt)
            self.assertIn("### hole h_book (expected i64 in src/book.rs)", prompt)
            self.assertIn("TYPE_HOLE(th1)", prompt)
            self.assertIn("HOLE(h_book)", prompt)

            # Multi-line SeqHole + TypeHole fills pass lint_filled cleanly.
            filled_ok = root / "filled_ok"
            good_fills = {
                "h_book": "{\n        let a = 20;\n        a + 22\n    }",
                "th1": (
                    "#[derive(Clone, Debug, Default, PartialEq)]\n"
                    "pub struct OrderBook { pub best_bid: i64 }\n"
                    "impl OrderBook { pub fn best(&self) -> i64 { self.best_bid } }"
                ),
            }
            res = fill_holes(skel, filled_ok, good_fills)
            self.assertEqual(res["applied"], ["h_book"])
            self.assertEqual(res["missing"], [])
            self.assertEqual(res["applied_type_holes"], ["th1"])
            self.assertEqual(res["missing_type_holes"], [])
            lint_ok = lint_filled(skel, filled_ok)
            self.assertTrue(lint_ok["ok"], lint_ok)

            # Forbidden Mutex inside src/domain.rs TypeHole is caught.
            filled_bad_domain = root / "filled_bad_domain"
            fill_holes(
                skel,
                filled_bad_domain,
                {
                    "h_book": "1",
                    "th1": "pub struct OrderBook { pub m: std::sync::Mutex<i64> }",
                },
            )
            lint_bad_domain = lint_filled(skel, filled_bad_domain)
            self.assertFalse(lint_bad_domain["ok"])
            self.assertIn(
                "forbidden_in_hole",
                [v["kind"] for v in lint_bad_domain["violations"]],
            )

            # Forbidden Mutex inside src/book.rs SeqHole is caught.
            filled_bad_book = root / "filled_bad_book"
            fill_holes(
                skel,
                filled_bad_book,
                {
                    "h_book": "{ let _ = std::sync::Mutex::new(0); 1 }",
                    "th1": good_fills["th1"],
                },
            )
            lint_bad_book = lint_filled(skel, filled_bad_book)
            self.assertFalse(lint_bad_book["ok"])
            self.assertIn(
                "forbidden_in_hole",
                [v["kind"] for v in lint_bad_book["violations"]],
            )


class ConformAllTests(unittest.TestCase):
    def test_counts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            traces = [
                TraceRun("native", 0, str(root / "a.jsonl"), False, 0, 1),
                TraceRun("native", 1, str(root / "b.jsonl"), False, 0, 1),
                TraceRun("native", 2, None, True, None, 5000, hang_suspect=True),
            ]
            for name in ("a.jsonl", "b.jsonl"):
                (root / name).write_text("", encoding="utf-8")

            def runner(program, trace):
                if trace.name == "a.jsonl":
                    return {"status": "conformant",
                            "coverage": {"sids_seen": 4, "sids_total": 4}}
                return {"status": "violation", "event_index": 2}

            out = conform_all(root / "prog.json", traces, conform_runner=runner)
            self.assertEqual(out["conformant"], 1)
            self.assertEqual(out["violation"], 1)
            self.assertEqual(out["timeout"], 1)
            self.assertEqual(out["hang_suspect"], 1)
            self.assertEqual(out["coverage"]["sids_seen"], 4)


if __name__ == "__main__":
    unittest.main()
