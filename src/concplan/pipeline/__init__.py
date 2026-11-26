"""End-to-end bi-level verification pipeline and benchmark discovery."""

from .benchmarks import (
    DEFAULT_BENCHMARKS_DIR,
    BenchmarkTask,
    benchmarks_summary,
    list_benchmarks,
    load_benchmark,
)
from .e2e_pipeline import (
    E2EResult,
    EscalationEvent,
    build_hole_fill_prompt,
    build_hole_repair_prompt,
    build_traceability_manifest,
    parse_hole_reply,
    run_e2e_pipeline,
)

__all__ = [
    "BenchmarkTask",
    "DEFAULT_BENCHMARKS_DIR",
    "E2EResult",
    "EscalationEvent",
    "benchmarks_summary",
    "build_hole_fill_prompt",
    "build_hole_repair_prompt",
    "build_traceability_manifest",
    "list_benchmarks",
    "load_benchmark",
    "parse_hole_reply",
    "run_e2e_pipeline",
]
