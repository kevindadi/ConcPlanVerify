"""End-to-end bi-level verification pipeline and Stage 0 example discovery."""

from .e2e_pipeline import (
    E2EResult,
    EscalationEvent,
    build_hole_fill_prompt,
    build_hole_repair_prompt,
    build_traceability_manifest,
    parse_hole_reply,
    run_e2e_pipeline,
)
from .examples import (
    DEFAULT_EXAMPLES_DIR,
    ExampleTask,
    examples_summary,
    list_examples,
    load_example,
)

__all__ = [
    "DEFAULT_EXAMPLES_DIR",
    "E2EResult",
    "EscalationEvent",
    "ExampleTask",
    "build_hole_fill_prompt",
    "build_hole_repair_prompt",
    "build_traceability_manifest",
    "examples_summary",
    "list_examples",
    "load_example",
    "parse_hole_reply",
    "run_e2e_pipeline",
]
