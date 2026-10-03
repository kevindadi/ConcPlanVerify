"""R-1 de-leak lint: repair_input must not carry the answer or qualitative words."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path

from tests._helpers import REPO

MANIFEST = REPO / "benchmarks/MANIFEST.json"
FORBIDDEN = ("deadlock", "lost", "leak", "bug", "fix", "wrong", "order")
PROGRAM_RE = re.compile(r"^case_[a-z][0-9]+$")


@unittest.skipUnless(MANIFEST.is_file(), "benchmarks/MANIFEST.json not built")
class SanitizeLintTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(MANIFEST.read_text(encoding="utf-8"))
        cls.root = MANIFEST.parent

    def _buggy_tasks(self):
        for task in self.data["tasks"]:
            if task.get("status") == "ready" and task.get("buggy_cir"):
                yield task

    def test_forbidden_words_absent(self):
        checked = 0
        for task in self._buggy_tasks():
            rin = self.root / task["directory"] / "repair_input"
            self.assertTrue(rin.is_dir(), f"missing repair_input for {task['id']}")
            for name in ("requirements.txt", "input.cir.json", "input.rs"):
                path = rin / name
                if not path.is_file():
                    continue
                text = path.read_text(encoding="utf-8").lower()
                for word in FORBIDDEN:
                    self.assertNotIn(word, text,
                                     f"{task['id']}/{name} contains {word!r}")
                checked += 1
        self.assertGreater(checked, 0)

    def test_program_name_is_anonymised(self):
        for task in self._buggy_tasks():
            program = json.loads(
                (self.root / task["directory"] / "repair_input/input.cir.json")
                .read_text(encoding="utf-8"))
            self.assertRegex(program["program"], PROGRAM_RE, task["id"])

    def test_no_description_or_note_fields(self):
        for task in self._buggy_tasks():
            program = json.loads(
                (self.root / task["directory"] / "repair_input/input.cir.json")
                .read_text(encoding="utf-8"))
            blob = json.dumps(program)
            self.assertNotIn('"description"', blob)
            self.assertNotIn('"note"', blob)

    def test_rust_comments_removed(self):
        for task in self._buggy_tasks():
            rust = self.root / task["directory"] / "repair_input/input.rs"
            if not rust.is_file():
                continue
            text = rust.read_text(encoding="utf-8")
            self.assertNotIn("//", text, task["id"])
            self.assertNotIn("/*", text, task["id"])

    def test_repair_task_points_at_inputs(self):
        for task in self._buggy_tasks():
            rt = json.loads((self.root / task["directory"] / "repair_task.json")
                            .read_text(encoding="utf-8"))
            self.assertIn("repair_input/", rt["input_cir"])
            self.assertIn("repair_input/requirements.txt", rt["requirements_file"])


if __name__ == "__main__":
    unittest.main()
