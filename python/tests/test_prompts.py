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
        self.assertIn("complete CIR JSON object", prompt)
        self.assertIn("Output only the JSON object", prompt)

    def test_generation_system_prompt_requires_complete_top_level_shape(self):
        prompt = generation_system_prompt()

        for key in (
            "program",
            "resources",
            "protection",
            "functions",
            "fn_summaries",
            "entry",
            "goals",
        ):
            self.assertIn(f'"{key}"', prompt)
        self.assertIn("Every function body must contain at least one statement", prompt)
        self.assertIn("Do not add unknown top-level or nested fields", prompt)

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
