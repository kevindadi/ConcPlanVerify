"""G3 feedback must carry schema and static-check detail into the next round."""

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient
from cir_workflow.prompts import concir_generation_v3_system_prompt
from cir_workflow.revision_workflow import build_schema_feedback, parse_schema_stderr

REPO = Path(__file__).resolve().parents[2]
BINARY = REPO.parent / "ConcIR/target/release/concir-backend"
EXAMPLES = REPO / "prompts/examples"


class SchemaStderrTests(unittest.TestCase):
    def test_unknown_base_field_is_specific(self):
        raw = ("JSON parse error in '/tmp/abs/program.json': unknown field `base`, "
               "expected one of `name`, `type`, `modeled` at line 4 column 8")
        parsed = parse_schema_stderr(raw)
        self.assertEqual(parsed["error_class"], "schema_parse")
        self.assertIsNone(parsed["json_path"])
        self.assertIn("base", parsed["message"])
        self.assertNotIn("/tmp/abs", parsed["message"])
        self.assertEqual(parsed["allowed_fields"], ["name", "type", "modeled"])
        self.assertIn("params and locals", parsed["fix_hint"])

    def test_args_string_expects_a_sequence(self):
        raw = 'JSON parse error in \'/x/program.json\': invalid type: string "1", expected a sequence at line 8 column 10'
        parsed = parse_schema_stderr(raw)
        self.assertEqual(parsed["expected_type"], "a sequence")
        self.assertIn("array", parsed["fix_hint"])
        self.assertIsNone(parsed["json_path"])


class BackendFeedbackTests(unittest.TestCase):
    def setUp(self):
        if not BINARY.is_file():
            self.skipTest("concir-backend not built")
        self.tmp = tempfile.TemporaryDirectory()
        self.client = ConcirClient(BINARY, workdir=self.tmp.name, timeout=30)

    def tearDown(self):
        self.tmp.cleanup()

    def test_examples_pass_check(self):
        for path in sorted(EXAMPLES.glob("v3_*.json")):
            result = self.client.check(path)
            self.assertEqual(result.status, "valid", path.name)

    def test_param_base_reaches_feedback(self):
        program = json.loads((EXAMPLES / "v3_call_args.json").read_text())
        program["modules"][0]["functions"][0]["params"][0]["base"] = "Int"
        path = Path(self.tmp.name) / "bad.json"
        path.write_text(json.dumps(program))
        check = self.client.check(path)
        fb = build_schema_feedback(check, [], program)
        self.assertEqual(fb["error_class"], "schema_parse")
        self.assertTrue(fb["diagnostics"])
        self.assertIn("base", fb["diagnostics"][0]["message"])
        self.assertIn("params and locals", fb["diagnostics"][0]["fix_hint"])

    def test_args_string_reaches_feedback(self):
        program = json.loads((EXAMPLES / "v3_call_args.json").read_text())
        program["modules"][0]["functions"][1]["body"][0]["args"] = "1"
        path = Path(self.tmp.name) / "args.json"
        path.write_text(json.dumps(program))
        check = self.client.check(path)
        fb = build_schema_feedback(check, [], program)
        self.assertEqual(fb["error_class"], "schema_parse")
        self.assertIn("sequence", fb["message"])

    def test_undefined_print_has_concrete_feedback(self):
        program = json.loads((EXAMPLES / "v3_two_workers.json").read_text())
        program["modules"][0]["functions"][0]["body"].insert(1, {
            "sid": "s9", "kind": "call", "func": "main::println", "args": ["\"DONE\""],
        })
        path = Path(self.tmp.name) / "print.json"
        path.write_text(json.dumps(program))
        check = self.client.check(path)
        self.assertEqual(check.status, "invalid")
        fb = build_schema_feedback(check, [], program)
        self.assertEqual(fb["error_class"], "static_check")
        blob = json.dumps(fb)
        self.assertIn("println", blob)
        self.assertTrue(fb["diagnostics"][0].get("fix_hint") or fb.get("expected_shapes"))

    def test_prompt_examples_match_files_and_prompt_has_no_ellipsis_program(self):
        text = concir_generation_v3_system_prompt()
        self.assertNotIn("...", text)
        self.assertIn("two_workers_one_lock", text)
        self.assertIn("call_with_args", text)


if __name__ == "__main__":
    unittest.main()
