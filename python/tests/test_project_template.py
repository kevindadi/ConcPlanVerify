"""Q-1: every arm's project template links `concir_sync`."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from cir_workflow.project_template import CONCIR_SYNC_CRATE, cargo_toml

PROGRAM = """use concir_sync::Semaphore;
fn main() {
    let s = Semaphore::new(1);
    let _p = s.acquire();
}
"""


@unittest.skipUnless(shutil.which("cargo") and CONCIR_SYNC_CRATE.is_dir(),
                     "cargo and the concir_sync crate are required")
class ProjectTemplateTests(unittest.TestCase):
    def test_each_arm_template_compiles_semaphore(self):
        for name in ("probe", "cir_arm_probe"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as td:
                root = Path(td)
                (root / "src").mkdir()
                (root / "Cargo.toml").write_text(cargo_toml(name), encoding="utf-8")
                (root / "src/main.rs").write_text(PROGRAM, encoding="utf-8")
                proc = subprocess.run(["cargo", "build", "--offline", "--quiet"],
                                      cwd=root, capture_output=True, text=True)
                self.assertEqual(proc.returncode, 0,
                                 f"{name}: {proc.stderr[-800:]}")


if __name__ == "__main__":
    unittest.main()
