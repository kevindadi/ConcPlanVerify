"""Thin compatibility facade re-exporting dynamic conformance and hole-filling APIs."""

from .ast_purity import lint_filled
from .dynamic import (
    TraceRun,
    codegen,
    collect_traces,
    conform_all,
    render_conformance_md,
    run_conformance_smoke,
)
from .holes import PLACEHOLDER_RE, HoleFill, fill_holes, fill_prompt, holes_of, parse_fill, type_holes_of

__all__ = [
    "HoleFill",
    "PLACEHOLDER_RE",
    "TraceRun",
    "codegen",
    "collect_traces",
    "conform_all",
    "fill_holes",
    "fill_prompt",
    "holes_of",
    "lint_filled",
    "parse_fill",
    "render_conformance_md",
    "run_conformance_smoke",
    "type_holes_of",
]
