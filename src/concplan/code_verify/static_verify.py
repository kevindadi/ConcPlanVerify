"""Thin compatibility facade re-exporting static verification submodules."""

from .ast_purity import AstHolePurityPlugin, scan_filled_crate
from .holes import (
    BOUNDED_LOOP_HINT_RE,
    HOLE_MARKER_RE,
    RUSTC_ERROR_SPAN_RE,
    SID_ANNOTATION_RE,
    SPAN_FILE_LINE_RE,
    SPAN_LINE_RE,
    TYPE_HOLE_MARKER_RE,
    TYPE_HOLE_PLACEHOLDER_RE,
    attribute_span_to_hole,
    augment_codegen_map_with_hole_ranges,
    fill_holes_marked,
    fill_type_holes_marked,
    list_src_rs_files,
)
from .lockbud import (
    LOCKBUD_BUG_KIND_RE,
    LOCKBUD_DEADLOCK_KINDS,
    LOCKBUD_TOOLCHAIN,
    LockbudPlugin,
    classify_lockbud,
)
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
    "BOUNDED_LOOP_HINT_RE",
    "CompositeStaticVerifier",
    "HOLE_MARKER_RE",
    "LOCKBUD_BUG_KIND_RE",
    "LOCKBUD_DEADLOCK_KINDS",
    "LOCKBUD_TOOLCHAIN",
    "LockbudPlugin",
    "RUSTC_ERROR_SPAN_RE",
    "SID_ANNOTATION_RE",
    "SPAN_FILE_LINE_RE",
    "SPAN_LINE_RE",
    "SpnMirRefinementPlugin",
    "StaticVerificationReport",
    "StaticVerifierPlugin",
    "StaticViolation",
    "TYPE_HOLE_MARKER_RE",
    "TYPE_HOLE_PLACEHOLDER_RE",
    "attribute_span_to_hole",
    "augment_codegen_map_with_hole_ranges",
    "classify_lockbud",
    "fill_holes_marked",
    "fill_type_holes_marked",
    "list_src_rs_files",
    "scan_filled_crate",
    "strip_rust_comments_and_literals",
]
