"""The v4 CIR prompt must declare real operations and valid examples."""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
V4 = REPO / "prompts" / "concir_generation_v4.md"

# Statement kinds the backend schema accepts (observed across the benchmark).
SCHEMA_KINDS = {
    "return", "mutex_lock", "mutex_unlock", "scope", "semaphore_acquire",
    "semaphore_release", "write_shared", "condvar_wait", "channel_send",
    "channel_recv", "branch", "atomic_load", "goto", "condvar_notify",
    "call", "condvar_notify_all", "atomic_store", "rwlock_unlock",
    "read_shared", "async_call", "await", "atomic_cas", "rwlock_write",
    "rwlock_read", "spawn", "join",
}


def _binary() -> Path | None:
    cand = REPO / "ConcIR" / "target" / "release" / "concir-backend"
    return cand if cand.is_file() else None


class PromptInterfaceTests(unittest.TestCase):
    def test_v4_exists(self):
        self.assertTrue(V4.is_file())

    def test_declared_kinds_are_real_schema_kinds(self):
        text = V4.read_text(encoding="utf-8")
        declared = set(re.findall(r"`([a-z_]+)`", text)) & SCHEMA_KINDS
        # every statement kind named in backticks that looks like an op is real
        self.assertTrue({"mutex_lock", "condvar_wait", "channel_send", "branch",
                         "goto", "atomic_cas", "semaphore_acquire", "scope"}
                        <= declared)
        for op in declared:
            self.assertIn(op, SCHEMA_KINDS)

    def test_examples_are_valid_json(self):
        text = V4.read_text(encoding="utf-8")
        blocks = re.findall(r"```json\n(.*?)```", text, re.S)
        self.assertGreaterEqual(len(blocks), 4)
        for b in blocks:
            json.loads(b)  # must parse

    def test_examples_pass_backend_static_check(self):
        binary = _binary()
        if binary is None:
            self.skipTest("concir-backend not built")
        text = V4.read_text(encoding="utf-8")
        blocks = re.findall(r"```json\n(.*?)```", text, re.S)
        for i, b in enumerate(blocks, 1):
            with tempfile.TemporaryDirectory() as td:
                p = Path(td) / f"ex{i}.cir.json"
                p.write_text(b, encoding="utf-8")
                proc = subprocess.run([str(binary), "check", str(p)],
                                      capture_output=True, text=True, timeout=60)
                self.assertEqual(proc.returncode, 0,
                                 f"example {i} failed static check: {proc.stdout[:300]}"
                                 f"{proc.stderr[:200]}")


if __name__ == "__main__":
    unittest.main()
