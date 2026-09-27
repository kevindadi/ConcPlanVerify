"""Common-eval bookkeeping: accepted_missing is only for accepted-but-unevaluated."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

PAPER = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v1/common-eval")


class CommonEvalBookkeepingTests(unittest.TestCase):
    def test_every_accepted_cell_has_a_coverage(self):
        if not PAPER.is_dir():
            self.skipTest("common-eval artifacts not present")
        missing = []
        seen = 0
        for p in PAPER.glob("*/*/*/*/accepted/eval.json"):
            row = json.loads(p.read_text())
            if row.get("provenance", {}).get("accepted"):
                seen += 1
                if not row.get("coverage"):
                    missing.append(str(p))
        self.assertGreater(seen, 0)
        self.assertEqual(missing, [], f"accepted cells without coverage: {missing}")


if __name__ == "__main__":
    unittest.main()
