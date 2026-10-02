"""v25 value-observation wiring: the monitor receives the CIR program so a
protected var's value can be attributed to its lock. Fake binary, no cargo."""

from __future__ import annotations

import json
import os
import stat
import tempfile
import unittest
from pathlib import Path

from cir_workflow import bounded_monitor


class ValueWiringTests(unittest.TestCase):
    def test_run_monitor_passes_program_flag(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            fake = root / "fake-backend"
            fake.write_text("#!/bin/sh\nprintf '{\"status\":\"ok\",\"properties\":[]}'\n")
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            contract = root / "contract.json"; contract.write_text("{}")
            traces = root / "traces"; traces.mkdir()
            program = root / "program.json"; program.write_text("{}")
            report = bounded_monitor.run_monitor(
                contract, traces, program=program, binary=fake, timeout=10)
        self.assertIn("--program", report["_argv"])
        self.assertIn(str(program), report["_argv"])

    def test_run_monitor_omits_program_when_absent(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            fake = root / "fake-backend"
            fake.write_text("#!/bin/sh\nprintf '{\"status\":\"ok\",\"properties\":[]}'\n")
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            contract = root / "contract.json"; contract.write_text("{}")
            traces = root / "traces"; traces.mkdir()
            report = bounded_monitor.run_monitor(contract, traces, binary=fake, timeout=10)
        self.assertNotIn("--program", report["_argv"])


if __name__ == "__main__":
    unittest.main()
