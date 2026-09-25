"""Classification for the mutation harness.

Ground truth is an input. Conform statuses never define it. ``error`` is not a
violation, an empty trace set is not a pass, and a timeout does not erase a
violation already recorded on an earlier trace.
"""

from __future__ import annotations

from typing import Any

VIOLATION = frozenset({"violation"})
PASS = frozenset({"conformant"})
NON_EVIDENCE = frozenset({"error", "unknown_sid", "unsupported", "timeout", "tool_error"})


def classify(row: dict[str, Any]) -> dict[str, Any]:
    """Return detection fields. Does not overwrite the caller's ground truth."""
    statuses = dict((row.get("conform_statuses") or {}))
    n_traces = int(row.get("n_traces") or 0)
    n_violation = int(statuses.get("violation", 0))
    n_pass = int(statuses.get("conformant", 0))
    n_error = sum(int(statuses.get(k, 0)) for k in statuses if k in NON_EVIDENCE or k not in VIOLATION | PASS)
    # Any status outside the known pass/violation sets is non-evidence.
    known = n_violation + n_pass
    n_other = max(0, sum(int(v) for v in statuses.values()) - known)
    n_error = max(n_error, n_other)
    hang = bool(row.get("hang"))
    source_build = row.get("source_build")
    inst_build = row.get("instrumented_build")
    inst_ok = row.get("instrumentation") == "ok"
    monitor_fail = list(row.get("monitor_fail") or [])
    structural_violation = n_violation > 0
    monitor_violation = bool(monitor_fail)
    detected = structural_violation or monitor_violation
    if n_traces == 0:
        trace_validity = "no_trace"
    elif hang and n_violation == 0 and n_pass == 0:
        trace_validity = "timeout_without_trace"
    elif hang and (n_violation or n_pass):
        trace_validity = "timeout_after_traces"
    else:
        trace_validity = "traces_present"
    if source_build is False:
        pipeline = "source_compile_failure"
    elif row.get("instrumentation") == "error":
        pipeline = "instrument_failure"
    elif inst_ok and inst_build is False:
        pipeline = "instrumented_compile_failure"
    elif trace_validity == "no_trace":
        pipeline = "no_trace"
    elif hang and not detected:
        pipeline = "timeout"
    elif detected:
        pipeline = "violation_observed"
    elif n_pass > 0 and n_error == 0 and not hang:
        pipeline = "pass_observed"
    elif n_error and not detected:
        pipeline = "tool_error"
    else:
        pipeline = "incomplete"
    gt = row.get("design_deviation")  # yes / no / uncertain / not_applicable
    role = row.get("role")
    if pipeline in {"source_compile_failure", "instrument_failure", "instrumented_compile_failure", "no_trace", "tool_error"}:
        call = "no_detection_evidence"
    elif role == "invalid":
        call = "invalid_mutant"
    elif gt == "uncertain":
        call = "uncertain_detected" if detected else "uncertain_not_detected"
    elif gt == "yes" and pipeline == "timeout" and not detected:
        call = "timeout_not_a_miss"
    elif gt == "yes":
        call = "tp" if detected else ("fn" if pipeline == "pass_observed" else "no_detection_evidence")
    elif gt == "no" and role == "negative":
        if pipeline == "timeout" and not detected:
            call = "timeout_not_a_false_positive"
        elif detected:
            call = "fp"
        elif pipeline == "pass_observed":
            call = "tn"
        else:
            call = "no_detection_evidence"
    else:
        call = "not_in_denominator"
    return {
        "n_violation": n_violation,
        "n_pass": n_pass,
        "n_non_evidence": n_error,
        "trace_validity": trace_validity,
        "structural_violation_observed": structural_violation,
        "monitor_violation_observed": monitor_violation,
        "detected": detected,
        "pipeline": pipeline,
        "call": call,
        "timeout_hides_violation": False if not (hang and structural_violation) else False,
    }
