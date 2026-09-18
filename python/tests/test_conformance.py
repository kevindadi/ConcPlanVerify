"""Offline tests for the conformance orchestration layer."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.conformance import (
    TraceRun, conform_all, fill_holes, lint_filled, parse_fill,
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
