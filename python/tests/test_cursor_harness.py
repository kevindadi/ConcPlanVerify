"""Adapter tests: the payload that is actually sent, not the intended prompt."""

import unittest
from pathlib import Path

from cir_workflow.cursor_harness import StagedCursorClient


class _Budget:
    def __init__(self):
        self.used = 0

    def exhausted(self):
        return None

    def reserve(self):
        self.used += 1


class _Result:
    def __init__(self, text, model):
        self.result = text
        self.id = "run"
        self.model = model
        self.usage = None


class _Model:
    def __init__(self, ident):
        self.id = ident


class _Agent:
    def __init__(self, sink):
        self.sink = sink

    def send(self, message):
        self.sink.append(message)
        return self

    def wait(self):
        return _Result("ok", _Model("composer-2.5"))

    def close(self):
        return None


class _Factory:
    def __init__(self):
        self.created = 0
        self.sink = []

    def create(self, _options):
        self.created += 1
        return _Agent(self.sink)


class _Options:
    def __init__(self, **kwargs):
        self.kwargs = kwargs


class _Local:
    def __init__(self, **kwargs):
        self.kwargs = kwargs


class PayloadTests(unittest.TestCase):
    def client(self):
        factory = _Factory()
        client = StagedCursorClient(
            "composer-2.5", "not-a-key", _Budget(), Path("/tmp/inbox"),
            agent_cls=factory, options_cls=_Options, local_cls=_Local)
        return client, factory

    def test_cir_first_round_contains_cir_rules(self):
        client, factory = self.client()
        client.set_stage("cir")
        client.complete("CIR RULES v3", "requirements only")
        self.assertEqual(factory.created, 1)
        self.assertIn("CIR RULES v3", factory.sink[0])
        self.assertIn("requirements only", factory.sink[0])

    def test_code_stage_receives_rust_rules(self):
        client, factory = self.client()
        client.set_stage("cir")
        client.complete("CIR RULES v3", "make a CIR")
        client.set_stage("code")
        client.complete("RUST RULES semaphore", "implement this CIR")
        self.assertIn("RUST RULES semaphore", factory.sink[-1])
        self.assertNotIn("CIR RULES v3", factory.sink[-1])

    def test_code_retry_contains_previous_rust_and_feedback(self):
        client, factory = self.client()
        client.set_stage("code")
        previous = "fn main() { let _ = a.lock(); }"
        feedback = "source_build_failed: error[E0425]: cannot find value `a`"
        user = f"Previous Rust:\n{previous}\n\nFeedback:\n{feedback}"
        client.complete("RUST RULES semaphore", user)
        sent = factory.sink[-1]
        self.assertIn("RUST RULES semaphore", sent)
        self.assertIn(previous, sent)
        self.assertIn("error[E0425]", sent)

    def test_code_rounds_do_not_reuse_a_session_that_drops_rules(self):
        client, factory = self.client()
        client.set_stage("code")
        client.complete("RUST RULES one", "round 1")
        client.complete("RUST RULES two", "round 2")
        self.assertEqual(factory.created, 2)
        self.assertIn("RUST RULES two", factory.sink[-1])
        self.assertNotIn("RUST RULES one", factory.sink[-1])


if __name__ == "__main__":
    unittest.main()
