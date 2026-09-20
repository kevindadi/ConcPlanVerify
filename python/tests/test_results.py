"""RESULTS generator: false_accept rule, expert/extract columns, aggregation."""

from __future__ import annotations

import unittest

from cir_workflow.results import aggregate, _bug


def _summary(cells):
    tasks = {}
    for (task, arm), rec in cells.items():
        tasks.setdefault(task, {"task": task, "arms": {}})
        tasks[task]["arms"][arm] = rec
    return {"tasks": list(tasks.values()), "binary_sha256": "x", "protocol_sha256": "y"}


class BugRuleTests(unittest.TestCase):
    def test_behavior_hang(self):
        bug, sources = _bug({"oracle": {"behavior_status": "hang"}}, None)
        self.assertTrue(bug)
        self.assertIn("behavior", sources)

    def test_by_construction_note(self):
        bug, sources = _bug({"notes": {"bug_present_reason": "by_construction"}}, None)
        self.assertTrue(bug)
        self.assertIn("by_construction", sources)

    def test_expert_yes(self):
        bug, sources = _bug({}, {"bug_present": "yes"})
        self.assertTrue(bug)
        self.assertIn("expert", sources)

    def test_clean(self):
        bug, sources = _bug({"oracle": {"behavior_status": "terminated_ok"}},
                            {"bug_present": "no"})
        self.assertFalse(bug)
        self.assertEqual(sources, [])


class AggregateTests(unittest.TestCase):
    def test_false_accept_and_sources(self):
        summaries = [_summary({
            ("t", "A0_direct"): {"accepted": True,
                                 "oracle": {"behavior_status": "hang"},
                                 "consumption": {"total_tokens": 10}},
            ("t", "A1_self_iter"): {"accepted": False,
                                    "oracle": {"behavior_status": "no_build"},
                                    "consumption": {}},
        })]
        rows = aggregate(summaries, {}, {}, {})
        a0 = next(r for r in rows if r["arm"] == "A0_direct")
        self.assertEqual(a0["accepted_display"], "1/1")
        self.assertEqual(a0["false_accept"], 1)
        self.assertEqual(a0["fa_sources"]["behavior"], 1)
        a1 = next(r for r in rows if r["arm"] == "A1_self_iter")
        self.assertEqual(a1["false_accept"], 0)

    def test_reclass_overlay(self):
        summaries = [_summary({
            ("t", "A0_direct"): {"accepted": False,
                                 "oracle": {"behavior_status": "no_build"},
                                 "consumption": {"total_tokens": 5}},
        })]
        reclass = {"cells": {"t|A0_direct": {"accepted": True, "accepted_round": 1,
                                             "by_construction": True}}}
        rows = aggregate(summaries, {}, {}, {}, reclass)
        row = rows[0]
        self.assertEqual(row["accepted_display"], "1/1")
        self.assertEqual(row["false_accept"], 1)
        self.assertEqual(row["fa_sources"]["by_construction"], 1)


if __name__ == "__main__":
    unittest.main()
