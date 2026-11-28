"""Thin compatibility facade re-exporting ``manifest``, ``fill_prompts``, and ``e2e``."""

from .e2e import run_e2e_pipeline
from .fill_prompts import build_hole_fill_prompt, build_hole_repair_prompt, parse_hole_reply
from .manifest import CLAUSE_RE, E2EResult, EscalationEvent, build_traceability_manifest

__all__ = [
    "CLAUSE_RE",
    "E2EResult",
    "EscalationEvent",
    "build_hole_fill_prompt",
    "build_hole_repair_prompt",
    "build_traceability_manifest",
    "parse_hole_reply",
    "run_e2e_pipeline",
]
