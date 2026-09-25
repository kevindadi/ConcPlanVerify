import unittest

from cir_workflow.mutation_protocol import classify


def _row(**kwargs):
    base = {
        "source_build": True,
        "instrumentation": "ok",
        "instrumented_build": True,
        "n_traces": 1,
        "conform_statuses": {"conformant": 1},
        "hang": False,
        "monitor_fail": [],
        "design_deviation": "no",
        "role": "negative",
    }
    base.update(kwargs)
    return base


class ClassifyTests(unittest.TestCase):
    def test_error_status_is_not_a_true_positive(self):
        out = classify(_row(
            role="mutant", design_deviation="yes",
            conform_statuses={"error": 2}, n_traces=2,
        ))
        self.assertEqual(out["call"], "no_detection_evidence")
        self.assertFalse(out["detected"])

    def test_zero_traces_are_not_a_true_negative(self):
        out = classify(_row(n_traces=0, conform_statuses={}))
        self.assertEqual(out["pipeline"], "no_trace")
        self.assertEqual(out["call"], "no_detection_evidence")

    def test_timeout_does_not_hide_an_earlier_violation(self):
        out = classify(_row(
            role="mutant", design_deviation="yes", hang=True,
            conform_statuses={"violation": 1, "conformant": 2}, n_traces=3,
        ))
        self.assertEqual(out["pipeline"], "violation_observed")
        self.assertEqual(out["call"], "tp")

    def test_timeout_without_violation_is_not_a_miss(self):
        out = classify(_row(
            role="mutant", design_deviation="yes", hang=True,
            conform_statuses={}, n_traces=0,
        ))
        self.assertNotEqual(out["call"], "fn")

    def test_source_compile_failure_is_not_detection(self):
        out = classify(_row(source_build=False, role="invalid", design_deviation="uncertain"))
        self.assertEqual(out["pipeline"], "source_compile_failure")
        self.assertEqual(out["call"], "no_detection_evidence")

    def test_clean_negative_is_tn(self):
        out = classify(_row())
        self.assertEqual(out["call"], "tn")

    def test_known_deviation_that_passes_is_fn(self):
        out = classify(_row(role="mutant", design_deviation="yes"))
        self.assertEqual(out["call"], "fn")


if __name__ == "__main__":
    unittest.main()
