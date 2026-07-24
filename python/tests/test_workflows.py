import json
import unittest

from cir_workflow.generation import GenerationWorkflow
from cir_workflow.repair import RepairWorkflow
from cir_workflow.models import RustCliResult


VALID_CIR = {"program": "demo", "resources": [], "functions": []}


class FakeLlm:
    def __init__(self, responses):
        self.responses = iter(responses)
        self.calls = []

    def chat(self, system_prompt, user_prompt, **kwargs):
        self.calls.append((system_prompt, user_prompt, kwargs))
        return next(self.responses), {}


class FakeRust:
    def __init__(self, validation_results=None, analysis_results=None):
        self.validation_results = iter(validation_results or [])
        self.analysis_results = iter(analysis_results or [])

    def validate(self, cir_json):
        return next(self.validation_results)

    def analyze(self, cir_json):
        return next(self.analysis_results)


def result(mode, status, payload=None, exit_code=0):
    return RustCliResult(
        mode=mode,
        exit_code=exit_code,
        status=status,
        payload=payload or {"status": status, "valid": status == "valid"},
    )


class WorkflowTests(unittest.TestCase):
    def test_generation_retries_after_validation_failure(self):
        candidate = json.dumps(VALID_CIR)
        rust = FakeRust([
            result(
                "--validate",
                "invalid_model",
                {
                    "status": "invalid_model",
                    "valid": False,
                    "diagnostics": [
                        {
                            "code": "E101",
                            "path": "functions[0].body[0].op[1]",
                            "message": "undefined resource 'queue'",
                            "fix_hint": "add this resource to resources",
                        }
                    ],
                },
            ),
            result("--validate", "valid", {"status": "valid", "valid": True}),
        ])
        llm = FakeLlm([candidate, candidate])
        workflow = GenerationWorkflow(llm, rust, max_rounds=2)
        requirements = "A producer sends one item to a queue and a consumer receives it."
        output = workflow.run(requirements)
        self.assertTrue(output.success)
        self.assertEqual(json.loads(output.cir_json), VALID_CIR)
        self.assertEqual(len(output.rounds), 2)
        self.assertEqual(len(llm.calls), 2)
        first_prompt = llm.calls[0][1]
        second_prompt = llm.calls[1][1]
        self.assertIn(requirements, first_prompt)
        self.assertIn("complete CIR JSON object", first_prompt)
        self.assertIn(requirements, second_prompt)
        self.assertIn("E101 [functions[0].body[0].op[1]]", second_prompt)
        self.assertIn("undefined resource 'queue'", second_prompt)
        self.assertIn("Current candidate", second_prompt)

    def test_generation_retry_keeps_original_requirements_after_parse_error(self):
        llm = FakeLlm(["not json", json.dumps(VALID_CIR)])
        rust = FakeRust([
            result("--validate", "valid", {"status": "valid", "valid": True}),
        ])
        workflow = GenerationWorkflow(llm, rust, max_rounds=2)
        requirements = "A single worker increments a protected counter."

        output = workflow.run(requirements)

        self.assertTrue(output.success)
        self.assertIn(requirements, llm.calls[1][1])
        self.assertIn("not json", llm.calls[1][1])
        self.assertIn("Parse error", llm.calls[1][1])

    def test_repair_accepts_verified_safe_candidate(self):
        candidate = json.dumps(VALID_CIR)
        rust = FakeRust(analysis_results=[
            result("--analyze", "verified_unsafe", {"status": "verified_unsafe", "bugs": [{"kind": {"Deadlock": {}}}]}),
            result("--analyze", "verified_safe", {"status": "verified_safe", "bugs": []}),
        ])
        workflow = RepairWorkflow(FakeLlm([candidate]), rust, max_rounds=1)
        output = workflow.run(candidate)
        self.assertTrue(output.success)
        self.assertEqual(json.loads(output.fixed_cir_json), VALID_CIR)
        self.assertEqual(output.rounds[0].verification.status, "verified_safe")


if __name__ == "__main__":
    unittest.main()
