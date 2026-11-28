"""Code-level static & dynamic verification (hole purity, SPN MIR refinement, Lockbud, dynamic conformance)."""

from .ast_purity import AstHolePurityPlugin, lint_filled
from .dynamic import (
    TraceRun,
    codegen,
    collect_traces,
    conform_all,
    run_conformance_smoke,
)
from .holes import (
    HoleFill,
    augment_codegen_map_with_hole_ranges,
    fill_holes,
    fill_holes_marked,
    fill_prompt,
    fill_type_holes_marked,
    holes_of,
    parse_fill,
    type_holes_of,
)
from .lockbud import LOCKBUD_TOOLCHAIN, LockbudPlugin, classify_lockbud
from .rust_lexer import strip_rust_comments_and_literals
from .spn_mir import SpnMirRefinementPlugin
from .verifier import (
    CompositeStaticVerifier,
    StaticVerificationReport,
    StaticVerifierPlugin,
    StaticViolation,
)

__all__ = [
    "AstHolePurityPlugin",
    "CompositeStaticVerifier",
    "HoleFill",
    "LOCKBUD_TOOLCHAIN",
    "LockbudPlugin",
    "SpnMirRefinementPlugin",
    "StaticVerificationReport",
    "StaticVerifierPlugin",
    "StaticViolation",
    "TraceRun",
    "augment_codegen_map_with_hole_ranges",
    "classify_lockbud",
    "codegen",
    "collect_traces",
    "conform_all",
    "fill_holes",
    "fill_holes_marked",
    "fill_prompt",
    "fill_type_holes_marked",
    "holes_of",
    "lint_filled",
    "parse_fill",
    "run_conformance_smoke",
    "strip_rust_comments_and_literals",
    "type_holes_of",
]
