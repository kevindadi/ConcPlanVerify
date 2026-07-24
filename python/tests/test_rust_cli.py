import json
import os
import stat
import tempfile
import unittest
from pathlib import Path

from cir_workflow.rust_cli import RustCli


class RustCliTests(unittest.TestCase):
    def test_decodes_structured_protocol_and_exit_code(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "fake-cir2cvn"
            binary.write_text(
                "#!/bin/sh\n"
                "if [ \"$1\" = \"--validate\" ]; then\n"
                "  printf '%s' '{\"status\":\"invalid_json\",\"valid\":false,\"diagnostics\":[]}'\n"
                "  exit 2\n"
                "fi\n"
                "printf '%s' '{\"status\":\"verified_safe\"}'\n"
            )
            binary.chmod(binary.stat().st_mode | stat.S_IXUSR)
            cli = RustCli(repo_root=directory, binary=binary, build_if_missing=False)

            invalid = cli.validate("not-json")
            self.assertEqual(invalid.exit_code, 2)
            self.assertEqual(invalid.status, "invalid_json")
            self.assertFalse(invalid.valid)

            safe = cli.analyze("{}")
            self.assertEqual(safe.exit_code, 0)
            self.assertEqual(safe.status, "verified_safe")


if __name__ == "__main__":
    unittest.main()
