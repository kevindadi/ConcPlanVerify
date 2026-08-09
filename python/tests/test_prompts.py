import unittest

from cir_workflow.prompts import (
    generation_system_prompt,
    generation_user_prompt,
    verification_feedback,
)


class PromptFeedbackTests(unittest.TestCase):
    def test_generation_prompt_contains_user_requirements_and_output_contract(self):
        requirements = "Two workers coordinate through a mutex."

        prompt = generation_user_prompt(requirements)

        self.assertIn("<domain_requirements>", prompt)
        self.assertIn(requirements, prompt)
        self.assertIn("complete ConcIR JSON object", prompt)
        self.assertIn("Output only the JSON object", prompt)

    def test_generation_system_prompt_requires_complete_top_level_shape(self):
        prompt = generation_system_prompt()

        for key in (
            "program",
            "resources",
            "protection",
            "functions",
            "entry",
            "goals",
        ):
            self.assertIn(f'"{key}"', prompt)
        self.assertIn("Every function body must contain at least one statement", prompt)
        self.assertIn("Do not add unknown top-level or nested fields", prompt)

    def test_equivalent_bugs_are_grouped_and_constraints_rendered_once(self):
        def deadlock(trace_len):
            return {
                "kind": {"Deadlock": {"participants": [
                    {"function": "w1", "waiting_for": "m2"},
                    {"function": "w2", "waiting_for": "m1"},
                ]}},
                "summary": "Deadlock detected involving w1, w2",
                "trace": [{"description": f"step {i}"} for i in range(trace_len)],
                "involved_resources": ["m1", "m2"],
                "involved_functions": ["w1", "w2"],
                "preservation_constraints": ["Resource 'm1' must remain in the artifact"],
            }

        feedback = verification_feedback(
            {"status": "verified_unsafe", "bugs": [deadlock(9), deadlock(3), deadlock(6)]}
        )

        self.assertIn("3 counterexamples, 1 distinct groups", feedback)
        self.assertIn("(3 equivalent counterexamples, one shown)", feedback)
        # The shortest trace is kept as the representative witness.
        self.assertEqual(feedback.count("step 0"), 1)
        self.assertIn("step 2", feedback)
        self.assertNotIn("step 5", feedback)
        self.assertEqual(feedback.count("must remain in the artifact"), 1)
        self.assertIn("Preservation constraints (apply to every fix)", feedback)

    def test_long_witness_traces_are_compressed(self):
        bug = {
            "kind": {"Deadlock": {"participants": []}},
            "summary": "Deadlock detected",
            "trace": [{"description": f"step {i}"} for i in range(100)],
        }

        feedback = verification_feedback(
            {"status": "verified_unsafe", "bugs": [bug]}, max_trace_steps=40
        )

        self.assertIn("intermediate steps omitted", feedback)
        self.assertIn("step 0", feedback)
        self.assertIn("step 99", feedback)
        self.assertNotIn("step 50", feedback)

    def test_reads_top_level_diagnostics_from_rust_cli_protocol(self):
        feedback = verification_feedback(
            {
                "status": "invalid_model",
                "diagnostics": [
                    {
                        "code": "E301",
                        "path": "functions[0].body[0].op",
                        "message": "unknown resource",
                        "fix_hint": "declare the resource first",
                    }
                ],
            }
        )

        self.assertIn("E301 [functions[0].body[0].op]", feedback)
        self.assertIn("unknown resource", feedback)
        self.assertIn("Fix: declare the resource first", feedback)


if __name__ == "__main__":
    unittest.main()
