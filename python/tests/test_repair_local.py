import json
import unittest

from cir_workflow.repair_local import (
    build_slice_prompt,
    function_sync_summary,
    implicated_functions,
    splice_functions,
)

PROGRAM = {
    "program": "demo",
    "resources": [{"name": "m1", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
    "protection": [],
    "functions": [
        {
            "name": "main",
            "kind": "normal",
            "body": [
                {"sid": "s1", "op": ["spawn", "w1"], "transfer": ["next", "s2"]},
                {"sid": "s2", "op": ["join", "w1"], "transfer": ["next", "s3"]},
                {"sid": "s3", "op": "return", "transfer": "return"},
            ],
        },
        {
            "name": "w1",
            "kind": "closure",
            "body": [
                {"sid": "s1", "op": ["res_op", "m1", "lock"], "transfer": ["next", "s2"]},
                {"sid": "s2", "op": ["res_op", "m1", "drop"], "transfer": ["next", "s3"]},
                {"sid": "s3", "op": "return", "transfer": "return"},
            ],
        },
    ],
    "entry": "main",
    "goals": [],
}


class ImplicatedFunctionsTests(unittest.TestCase):
    def test_collects_functions_from_all_bug_report_fields(self):
        payload = {
            "bugs": [
                {
                    "involved_functions": ["w1"],
                    "cir_slice": [{"function": "main", "sid": "s1", "op": "spawn"}],
                    "kind": {"Deadlock": {"participants": [
                        {"function": "w2", "waiting_for": "m1"},
                    ]}},
                }
            ]
        }
        self.assertEqual(implicated_functions(payload), ["w1", "main", "w2"])

    def test_empty_payload_yields_empty_slice(self):
        self.assertEqual(implicated_functions(None), [])
        self.assertEqual(implicated_functions({"bugs": []}), [])


class SpliceTests(unittest.TestCase):
    def test_splice_replaces_only_allowed_functions(self):
        replacement = {
            "name": "w1",
            "kind": "closure",
            "body": [{"sid": "s1", "op": "return", "transfer": "return"}],
        }
        frozen_edit = {
            "name": "main",
            "kind": "normal",
            "body": [{"sid": "s1", "op": "return", "transfer": "return"}],
        }

        result, applied, rejected = splice_functions(
            PROGRAM, [replacement, frozen_edit], allowed=["w1"]
        )

        self.assertEqual(applied, ["w1"])
        self.assertEqual(rejected, ["main"])
        functions = {fn["name"]: fn for fn in result["functions"]}
        self.assertEqual(len(functions["w1"]["body"]), 1)
        # Frozen function is byte-identical to the original.
        self.assertEqual(functions["main"], PROGRAM["functions"][0])
        # The original program object is untouched.
        self.assertEqual(len(PROGRAM["functions"][1]["body"]), 3)

    def test_function_order_is_preserved(self):
        replacement = {
            "name": "w1",
            "kind": "closure",
            "body": [{"sid": "s1", "op": "return", "transfer": "return"}],
        }
        result, _, _ = splice_functions(PROGRAM, [replacement], allowed=["w1"])
        self.assertEqual([fn["name"] for fn in result["functions"]], ["main", "w1"])


class PromptTests(unittest.TestCase):
    def test_sync_summary_lists_sync_ops_in_order(self):
        summary = function_sync_summary(PROGRAM["functions"][1])
        self.assertIn("w1 (closure): lock(m1) -> drop(m1)", summary)

    def test_slice_prompt_freezes_other_functions(self):
        prompt = build_slice_prompt(PROGRAM, ["w1"], feedback="Deadlock detected")

        self.assertIn("Functions you may modify", prompt)
        self.assertIn("Deadlock detected", prompt)
        # main appears only as a frozen summary, not as an editable body.
        self.assertIn("- main (normal): spawn(w1) -> join(w1)", prompt)
        editable = prompt.split("## Functions you may modify")[1].split("## Other functions")[0]
        self.assertIn('"w1"', editable)
        self.assertNotIn('"name": "main"', editable)
        # Global declarations travel verbatim.
        self.assertIn('"entry": "main"', prompt)

    def test_slice_prompt_output_contract_requests_fragment(self):
        prompt = build_slice_prompt(PROGRAM, ["w1"], feedback="x")
        self.assertIn('{"functions": [{"name": ..., "kind": ..., "body": [...]}', prompt)
        self.assertIn("Do not output the full ConcIR", prompt)


if __name__ == "__main__":
    unittest.main()
