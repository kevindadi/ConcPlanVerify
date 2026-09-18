"""Unit tests for the CLI protocol mapping using a controllable stub binary."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.concir_client import ConcirClient
from tests._helpers import make_stub_binary, stub_env


class ConcirClientProtocolTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.stub = make_stub_binary(self.root / "bin")
        self.program = self.root / "program.json"
        self.program.write_text('{"program":"x"}')

    def tearDown(self):
        self.tmp.cleanup()

    def client(self, **env):
        return ConcirClient(self.stub, workdir=self.root / "work",
                            timeout=5.0, env=stub_env(**env))

    def test_check_valid_and_invalid_mapping(self):
        r = self.client(STUB_STDOUT='{"valid": true}', STUB_EXIT="0").check(self.program)
        self.assertEqual((r.kind, r.status), ("semantic", "valid"))
        r = self.client(STUB_STDOUT='{"valid": false, "diagnostics": []}',
                        STUB_EXIT="4").check(self.program)
        self.assertEqual((r.kind, r.status), ("semantic", "invalid"))

    def test_explore_and_repair_outcome_mapping(self):
        r = self.client(STUB_STDOUT='{"outcome": "FAIL", "complete": true}',
                        STUB_EXIT="1").explore(self.program, self.program)
        self.assertEqual((r.kind, r.status, r.outcome), ("semantic", "fail", "FAIL"))
        r = self.client(STUB_STDOUT='{"outcome": "analysis_unknown"}',
                        STUB_EXIT="3").repair(self.program, self.program)
        self.assertEqual((r.kind, r.status), ("semantic", "analysis_unknown"))

    def test_exit_outcome_mismatch_is_protocol_error(self):
        r = self.client(STUB_STDOUT='{"outcome": "FAIL", "complete": true}',
                        STUB_EXIT="0").explore(self.program, self.program)
        self.assertEqual(r.kind, "protocol_error")

    def test_empty_and_non_json_output(self):
        r = self.client(STUB_STDOUT="", STUB_EXIT="0").check(self.program)
        self.assertEqual((r.kind, r.status), ("protocol_error", "empty_output"))
        r = self.client(STUB_STDOUT="not json", STUB_EXIT="0").check(self.program)
        self.assertEqual((r.kind, r.status), ("protocol_error", "non_json_output"))

    def test_usage_exit_is_not_a_crash(self):
        r = self.client(STUB_STDOUT="", STUB_STDERR="usage", STUB_EXIT="2").check(self.program)
        self.assertEqual((r.kind, r.status), ("usage_error", "usage_error"))

    def test_timeout_kills_process(self):
        r = self.client(STUB_STDOUT="{}", STUB_SLEEP="5").check(self.program)
        self.assertEqual((r.kind, r.status), ("process_error", "timeout"))
        self.assertTrue(r.timed_out)

    def test_paths_with_spaces(self):
        spaced = self.root / "with space" / "out dir"
        client = ConcirClient(self.stub, workdir=spaced, timeout=5.0,
                              env=stub_env(STUB_STDOUT='{"valid": true}', STUB_EXIT="0"))
        r = client.check(self.program)
        self.assertEqual((r.kind, r.status), ("semantic", "valid"))
        self.assertIn("with space", r.call_dir)

    def test_missing_binary_is_actionable(self):
        client = ConcirClient(self.root / "nope" / "concir-backend", workdir=self.root / "w")
        with self.assertRaises(FileNotFoundError):
            client.check(self.program)

    def test_raw_output_is_preserved(self):
        r = self.client(STUB_STDOUT='{"valid": true}', STUB_STDERR="note",
                        STUB_EXIT="0").check(self.program)
        self.assertTrue(Path(r.stdout_path).read_text().strip())
        self.assertTrue(Path(r.stderr_path).read_text().strip())
        self.assertEqual(Path(r.exit_path).read_text().strip(), "0")


if __name__ == "__main__":
    unittest.main()
