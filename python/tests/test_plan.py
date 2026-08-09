"""Offline tests for the LLM modularity planner."""

from __future__ import annotations

import unittest

from cir_workflow.plan import PlanError, render_plan, run_plan


class FakeLlm:
    def __init__(self, response):
        self.response = response
        self.calls = []

    def chat(self, system_prompt, user_prompt, **kwargs):
        self.calls.append((system_prompt, user_prompt, kwargs))
        return self.response, {}


class PlanTests(unittest.TestCase):
    def test_direct_generation_when_modular_false(self):
        llm = FakeLlm('{"modular": false, "rationale": "small program"}')
        plan = run_plan(llm, "two threads and one mutex")
        self.assertIs(plan["modular"], False)
        self.assertEqual(render_plan(plan), "Direct generation (modular: false)")

    def test_modular_plan_requires_single_entry(self):
        response = (
            '{"modular": true, "rationale": "large", "modules": ['
            '{"name": "main", "entry": true, "responsibility": "wiring",'
            ' "functions": ["main"], "resources": ["mtx"]},'
            '{"name": "w", "entry": true, "responsibility": "worker",'
            ' "functions": ["worker"], "resources": []}],'
            '"shared_resources": ["mtx"]}'
        )
        llm = FakeLlm(response)
        with self.assertRaises(PlanError) as ctx:
            run_plan(llm, "requirements")
        self.assertIn("exactly one entry module", str(ctx.exception))

    def test_modular_plan_parses_and_renders(self):
        response = (
            '{"modular": true, "rationale": "two subsystems", "modules": ['
            '{"name": "main", "entry": true, "responsibility": "deployment",'
            ' "functions": ["main"], "resources": ["mtx", "ch"]},'
            '{"name": "worker", "entry": false, "responsibility": "workers",'
            ' "functions": ["worker"], "resources": []}],'
            '"shared_resources": ["mtx", "ch"]}'
        )
        llm = FakeLlm(response)
        plan = run_plan(llm, "requirements")
        self.assertIs(plan["modular"], True)
        text = render_plan(plan)
        self.assertIn("main [entry]", text)
        self.assertIn("shared resources: mtx, ch", text)
        # Planner output contract reached the LLM.
        self.assertIn("modular", llm.calls[0][0])
        self.assertEqual(llm.calls[0][1], "requirements")

    def test_non_json_plan_is_an_error(self):
        llm = FakeLlm("sorry, no json here")
        with self.assertRaises(PlanError):
            run_plan(llm, "requirements")


if __name__ == "__main__":
    unittest.main()
