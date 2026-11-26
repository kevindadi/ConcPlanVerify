"""Pluggable post-codegen static verification (Section 3 of REFINEMENT_SPEC.md).

Provides:
- :class:`StaticViolation`, :class:`StaticVerificationReport`,
  :class:`StaticVerifierPlugin` (Protocol), and :class:`CompositeStaticVerifier`.
- :func:`fill_holes_marked` and :func:`augment_codegen_map_with_hole_ranges`
  for exact multi-line ``hole_ranges`` tracking in ``codegen_filled.json``.
- :class:`AstHolePurityPlugin`: skeleton integrity + comment/string-safe AST
  purity checking for forbidden sync/thread/atomic/unsafe/loop constructs.
- :class:`SpnMirRefinementPlugin`: invokes ``cargo check --offline
  --no-default-features`` with ``RUSTC_WRAPPER=<spn>`` to export
  ``concir-impl-net/1`` and checks stuttering refinement via
  ``ConcirClient.verify_refinement``.
- :class:`LockbudPlugin`: optional adapter for ``backends/third_party/lockbud``.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Protocol, runtime_checkable

from ..core.concir_client import ConcirClient
from ..core.toolchain import find_concir_backend, find_lockbud, find_spn
from .conformance import PLACEHOLDER_RE, holes_of

LOCKBUD_TOOLCHAIN = "nightly-2026-02-07"
LOCKBUD_DEADLOCK_KINDS = {"ConflictLock", "DoubleLock", "CondvarDeadlock"}
LOCKBUD_BUG_KIND_RE = re.compile(r'"bug_kind"\s*:\s*"([^"]+)"')


def classify_lockbud(text: str) -> dict[str, Any]:
    """Classify Lockbud output by parsed ``bug_kind`` records only."""
    kinds = sorted({m.group(1) for m in LOCKBUD_BUG_KIND_RE.finditer(text)})
    deadlocks = [k for k in kinds if k in LOCKBUD_DEADLOCK_KINDS]
    return {
        "status": "detected" if deadlocks else "clean",
        "detected": deadlocks,
        "bug_kinds": kinds,
    }

HOLE_MARKER_RE = re.compile(
    r"/\* @hole_begin\((?P<id>[A-Za-z0-9_]+)\) \*/"
    r"(?P<body>.*?)"
    r"/\* @hole_end\((?P=id)\) \*/",
    re.DOTALL,
)
SID_ANNOTATION_RE = re.compile(r"// @cir (\S+)")
SPAN_LINE_RE = re.compile(r":(\d+)(?::\d+)?")

# Categorized forbidden patterns scanned ONLY after stripping Rust comments and literals.
_FORBIDDEN_RULES: tuple[tuple[str, str, re.Pattern[str]], ...] = (
    (
        "sync_in_hole",
        "forbidden synchronization construct inside sequential hole",
        re.compile(
            r"\b(?:std::sync|Mutex|Condvar|RwLock|Semaphore|Barrier|OnceLock|Once|mpsc|channel)\b"
            r"|\.(?:lock|try_lock|read|write|wait|wait_while|wait_timeout|"
            r"notify_one|notify_all|send|recv|try_recv)\s*\("
        ),
    ),
    (
        "thread_in_hole",
        "forbidden thread or blocking construct inside sequential hole",
        re.compile(
            r"\b(?:std::thread|thread::spawn|thread::scope|thread::sleep)\b"
            r"|\bscope\s*\("
        ),
    ),
    (
        "atomic_in_hole",
        "forbidden atomic construct inside sequential hole",
        re.compile(r"\b(?:Atomic\w*|atomic::|Ordering::)\b"),
    ),
    (
        "unsafe_in_hole",
        "forbidden unsafe/trace/import construct inside sequential hole",
        re.compile(r"\b(?:unsafe|cir_trace|extern|use)\b"),
    ),
    (
        "loop_in_hole",
        "forbidden potentially diverging loop construct inside sequential hole",
        re.compile(r"\b(?:loop|while|spin_loop)\b"),
    ),
)


@dataclass
class StaticViolation:
    plugin: str
    obligation: str
    kind: str
    message: str
    hole_id: str | None = None
    sid: str | None = None
    req_clauses: list[str] = field(default_factory=list)
    rust_span: str | None = None
    witness_trace: list[dict[str, Any]] = field(default_factory=list)
    expected_spec_ops: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "plugin": self.plugin,
            "obligation": self.obligation,
            "kind": self.kind,
            "message": self.message,
            "hole_id": self.hole_id,
            "sid": self.sid,
            "req_clauses": list(self.req_clauses),
            "rust_span": self.rust_span,
            "witness_trace": list(self.witness_trace),
            "expected_spec_ops": list(self.expected_spec_ops),
        }


@dataclass
class StaticVerificationReport:
    """Outcome of one plugin or a composite verification pass.

    ``status`` is one of:
    - ``"pass"``: all enabled plugins verified without violation or unknown.
    - ``"fail_hole"``: hole-level purity or hole-attributable violation.
    - ``"fail_refinement"``: MIR/refinement/deadlock/liveness check failed.
    - ``"fail_skeleton"``: code outside holes or ``// @cir`` annotations changed.
    - ``"unknown"``: at least one plugin could not conclude (e.g. missing ``spn``
      or truncated state graph); never treated as a pass.
    - ``"error"``: build/process/protocol error.
    """

    status: str
    plugin: str
    violations: list[StaticViolation] = field(default_factory=list)
    checks: dict[str, bool] = field(default_factory=dict)
    plugin_reports: list[dict[str, Any]] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)
    error: str | None = None

    @property
    def ok(self) -> bool:
        return self.status == "pass"

    @property
    def offending_holes(self) -> list[str]:
        seen: list[str] = []
        for v in self.violations:
            if v.hole_id and v.hole_id not in seen:
                seen.append(v.hole_id)
        return seen

    def as_dict(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "plugin": self.plugin,
            "ok": self.ok,
            "offending_holes": self.offending_holes,
            "violations": [v.as_dict() for v in self.violations],
            "checks": dict(self.checks),
            "plugin_reports": list(self.plugin_reports),
            "metadata": dict(self.metadata),
            "error": self.error,
        }


@runtime_checkable
class StaticVerifierPlugin(Protocol):
    name: str

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport: ...


# ---------------------------------------------------------------------------
# Rust comment & string literal stripping (line- and column-preserving)
# ---------------------------------------------------------------------------

def strip_rust_comments_and_literals(code: str) -> str:
    """Replace Rust comments and string/char literals with spaces, preserving newlines.

    Handles:
    - ``// ...`` line comments
    - nested ``/* ... */`` block comments
    - normal, byte, and C string literals (``"..."``, ``b"..."``, ``c"..."``)
    - raw string literals (``r"..."``, ``r#"..."#``, ``br#"..."#``, ``cr#"..."#``)
    - character and byte literals (``'a'``, ``'\\n'``, ``b'x'``) without consuming
      Rust lifetimes such as ``'a`` or ``'static``
    """

    n = len(code)
    out: list[str] = []
    i = 0

    def _emit_spaces(start: int, end: int) -> None:
        for ch in code[start:end]:
            out.append("\n" if ch == "\n" else " ")

    while i < n:
        ch = code[i]

        # Line comment //
        if ch == "/" and i + 1 < n and code[i + 1] == "/":
            j = i + 2
            while j < n and code[j] != "\n":
                j += 1
            _emit_spaces(i, j)
            i = j
            continue

        # Nested block comment /* ... */
        if ch == "/" and i + 1 < n and code[i + 1] == "*":
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if code[j] == "/" and j + 1 < n and code[j + 1] == "*":
                    depth += 1
                    j += 2
                elif code[j] == "*" and j + 1 < n and code[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            _emit_spaces(i, j)
            i = j
            continue

        # Raw string literal: [bc]?r#*"..."#*
        raw_match = _match_raw_string_prefix(code, i)
        if raw_match is not None:
            hashes, content_start = raw_match
            closing = '"' + ("#" * hashes)
            end_idx = code.find(closing, content_start)
            j = n if end_idx == -1 else end_idx + len(closing)
            _emit_spaces(i, j)
            i = j
            continue

        # Byte/C string prefix: b"..." or c"..."
        if ch in ("b", "c") and i + 1 < n and code[i + 1] == '"' and (
            i == 0 or not (code[i - 1].isalnum() or code[i - 1] == "_")
        ):
            j = _scan_quoted_string(code, i + 1)
            _emit_spaces(i, j)
            i = j
            continue

        # Normal string literal: "..."
        if ch == '"':
            j = _scan_quoted_string(code, i)
            _emit_spaces(i, j)
            i = j
            continue

        # Byte char literal b'x' or char literal 'x' (NOT lifetimes like 'a)
        if ch == "b" and i + 1 < n and code[i + 1] == "'" and (
            i == 0 or not (code[i - 1].isalnum() or code[i - 1] == "_")
        ):
            end_char = _match_char_literal(code, i + 1)
            if end_char is not None:
                _emit_spaces(i, end_char)
                i = end_char
                continue

        if ch == "'":
            end_char = _match_char_literal(code, i)
            if end_char is not None:
                _emit_spaces(i, end_char)
                i = end_char
                continue

        out.append(ch)
        i += 1

    return "".join(out)


def _match_raw_string_prefix(code: str, i: int) -> tuple[int, int] | None:
    n = len(code)
    if i > 0 and (code[i - 1].isalnum() or code[i - 1] == "_"):
        return None
    j = i
    if j < n and code[j] in ("b", "c"):
        j += 1
    if j >= n or code[j] != "r":
        return None
    j += 1
    hashes = 0
    while j < n and code[j] == "#":
        hashes += 1
        j += 1
    if j < n and code[j] == '"':
        return hashes, j + 1
    return None


def _scan_quoted_string(code: str, quote_idx: int) -> int:
    n = len(code)
    j = quote_idx + 1
    while j < n:
        c = code[j]
        if c == "\\":
            j += 2
            continue
        if c == '"':
            return j + 1
        j += 1
    return n


def _match_char_literal(code: str, quote_idx: int) -> int | None:
    """Match a Rust char literal starting at ``quote_idx`` without matching lifetimes."""
    n = len(code)
    if quote_idx + 2 >= n:
        return None
    if code[quote_idx + 1] == "\\":
        # Escaped char literal: '\n', '\'', '\x7f', '\u{1234}'
        j = quote_idx + 2
        if j >= n:
            return None
        if code[j] == "u" and j + 1 < n and code[j + 1] == "{":
            close_brace = code.find("}", j + 2)
            if close_brace != -1 and close_brace + 1 < n and code[close_brace + 1] == "'":
                return close_brace + 2
            return None
        if code[j] == "x":
            if j + 3 < n and code[j + 3] == "'":
                return j + 4
            return None
        if j + 1 < n and code[j + 1] == "'":
            return j + 2
        return None
    # Simple single-character literal: 'x' (not newline, not empty '')
    if code[quote_idx + 1] not in ("'", "\n") and code[quote_idx + 2] == "'":
        return quote_idx + 3
    return None


# ---------------------------------------------------------------------------
# Hole alignment, marked filling, and codegen_filled.json augmentation
# ---------------------------------------------------------------------------

def _split_skeleton_by_holes(skeleton_source: str) -> tuple[list[str], list[dict[str, Any]]]:
    """Split skeleton Rust source into alternating static segments and hole descriptors."""
    segments: list[str] = []
    holes: list[dict[str, Any]] = []
    cursor = 0
    for m in PLACEHOLDER_RE.finditer(skeleton_source):
        segments.append(skeleton_source[cursor:m.start()])
        holes.append({
            "id": m.group("id"),
            "expected": m.group("expected"),
            "placeholder": m.group(0),
            "start": m.start(),
            "end": m.end(),
        })
        cursor = m.end()
    segments.append(skeleton_source[cursor:])
    return segments, holes


def _offset_to_line_range(text: str, start: int, end: int) -> tuple[int, int]:
    start_line = text.count("\n", 0, start) + 1
    if end <= start:
        return start_line, start_line
    end_line = text.count("\n", 0, end - 1) + 1
    return start_line, end_line


def _align_filled_to_skeleton(
    skeleton_source: str,
    filled_source: str,
    fills: dict[str, str] | None = None,
) -> tuple[bool, list[dict[str, Any]], list[dict[str, Any]]]:
    """Align ``filled_source`` against ``skeleton_source`` across all hole placeholders.

    Returns ``(skeleton_intact, extracted_holes, structural_errors)`` where each
    entry in ``extracted_holes`` has ``id``, ``body``, ``start``, ``end``,
    ``start_line``, and ``end_line``.
    """

    segments, skel_holes = _split_skeleton_by_holes(skeleton_source)
    structural_errors: list[dict[str, Any]] = []
    extracted: list[dict[str, Any]] = []

    if not skel_holes:
        if filled_source == skeleton_source:
            return True, [], []
        diff_line = _first_diff_line(skeleton_source, filled_source)
        structural_errors.append({
            "kind": "changed_outside_hole",
            "line": diff_line,
            "message": f"source modified outside holes near line {diff_line}",
        })
        return False, [], structural_errors

    if not filled_source.startswith(segments[0]):
        diff_line = _first_diff_line(segments[0], filled_source)
        structural_errors.append({
            "kind": "changed_outside_hole",
            "line": diff_line,
            "message": f"code before first hole ({skel_holes[0]['id']}) was modified near line {diff_line}",
        })
        return False, [], structural_errors

    pos = len(segments[0])
    fills = fills or {}

    for idx, hole in enumerate(skel_holes):
        hid = hole["id"]
        next_seg = segments[idx + 1]
        is_last = idx == len(skel_holes) - 1

        # Prefer exact marker boundaries if present right at pos.
        marker_prefix = f"/* @hole_begin({hid}) */"
        marker_suffix = f"/* @hole_end({hid}) */"
        next_pos = -1

        if filled_source.startswith(marker_prefix, pos):
            end_marker = filled_source.find(marker_suffix, pos + len(marker_prefix))
            if end_marker != -1:
                candidate_next = end_marker + len(marker_suffix)
                if (is_last and filled_source[candidate_next:] == next_seg) or (
                    not is_last and filled_source.startswith(next_seg, candidate_next)
                ):
                    next_pos = candidate_next

        # Next try known fill text if provided.
        if next_pos == -1 and hid in fills:
            candidate_body = fills[hid]
            for variant in (
                f"{marker_prefix} {candidate_body} {marker_suffix}",
                f"{marker_prefix}{candidate_body}{marker_suffix}",
                candidate_body,
            ):
                if filled_source.startswith(variant, pos):
                    candidate_next = pos + len(variant)
                    if (is_last and filled_source[candidate_next:] == next_seg) or (
                        not is_last and filled_source.startswith(next_seg, candidate_next)
                    ):
                        next_pos = candidate_next
                        break

        # Next try exact placeholder (unfilled hole).
        if next_pos == -1 and filled_source.startswith(hole["placeholder"], pos):
            candidate_next = pos + len(hole["placeholder"])
            if (is_last and filled_source[candidate_next:] == next_seg) or (
                not is_last and filled_source.startswith(next_seg, candidate_next)
            ):
                next_pos = candidate_next

        # General structural match: locate next static segment.
        if next_pos == -1:
            if is_last:
                if next_seg == "":
                    next_pos = len(filled_source)
                elif filled_source.endswith(next_seg) and len(filled_source) - len(next_seg) >= pos:
                    next_pos = len(filled_source) - len(next_seg)
            else:
                if next_seg == "":
                    next_pos = pos
                else:
                    found = filled_source.find(next_seg, pos)
                    if found != -1:
                        next_pos = found

        if next_pos == -1:
            line_no = filled_source.count("\n", 0, pos) + 1
            structural_errors.append({
                "kind": "changed_outside_hole",
                "line": line_no,
                "message": (
                    f"skeleton segment after hole '{hid}' was modified or removed "
                    f"(expected static code starting near line {line_no})"
                ),
            })
            return False, extracted, structural_errors

        body = filled_source[pos:next_pos]
        start_line, end_line = _offset_to_line_range(filled_source, pos, next_pos)
        extracted.append({
            "id": hid,
            "expected": hole["expected"],
            "body": body,
            "start": pos,
            "end": next_pos,
            "start_line": start_line,
            "end_line": end_line,
        })
        pos = next_pos + len(next_seg)

    if pos != len(filled_source):
        line_no = filled_source.count("\n", 0, min(pos, len(filled_source))) + 1
        structural_errors.append({
            "kind": "changed_outside_hole",
            "line": line_no,
            "message": f"trailing code after last hole was modified near line {line_no}",
        })
        return False, extracted, structural_errors

    return True, extracted, []


def _first_diff_line(expected: str, actual: str) -> int:
    e_lines = expected.splitlines()
    a_lines = actual.splitlines()
    for i in range(min(len(e_lines), len(a_lines))):
        if e_lines[i] != a_lines[i]:
            return i + 1
    return min(len(e_lines), len(a_lines)) + 1


def _unwrap_hole_markers(hid: str, body: str) -> str:
    m = HOLE_MARKER_RE.fullmatch(body.strip())
    if m and m.group("id") == hid:
        return m.group("body")
    prefix = f"/* @hole_begin({hid}) */"
    suffix = f"/* @hole_end({hid}) */"
    if body.startswith(prefix) and body.endswith(suffix):
        return body[len(prefix):len(body) - len(suffix)]
    return body


def fill_holes_marked(
    skeleton_dir: Path | str,
    filled_dir: Path | str,
    fills: dict[str, str],
) -> dict[str, Any]:
    """Copy skeleton, substitute holes wrapped in ``@hole_begin``/``@hole_end`` markers,
    and write ``codegen_filled.json`` with exact multi-line ``hole_ranges``.
    """

    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if filled.exists():
        shutil.rmtree(filled)
    shutil.copytree(skeleton, filled)

    main_rs = filled / "src/main.rs"
    text = main_rs.read_text(encoding="utf-8")
    applied: list[str] = []
    missing: list[str] = []
    for hole in holes_of(skeleton):
        hid = hole["id"]
        if hid not in fills:
            missing.append(hid)
            continue
        placeholder = f"/* HOLE({hid}) expected: {hole['expected']} */ Default::default()"
        replacement = f"/* @hole_begin({hid}) */ {fills[hid]} /* @hole_end({hid}) */"
        if placeholder in text:
            text = text.replace(placeholder, replacement, 1)
            applied.append(hid)
        else:
            # Fallback to regex matching that specific hole id.
            pattern = re.compile(
                rf"/\* HOLE\({re.escape(hid)}\) expected: .*? \*/ Default::default\(\)"
            )
            new_text, count = pattern.subn(lambda _m: replacement, text, count=1)
            if count:
                text = new_text
                applied.append(hid)
            else:
                missing.append(hid)
    main_rs.write_text(text, encoding="utf-8")

    out_map_path = filled / "codegen_filled.json"
    augmented = augment_codegen_map_with_hole_ranges(
        skeleton, text, fills, out_path=out_map_path
    )
    return {
        "applied": applied,
        "missing": missing,
        "hole_ranges": augmented.get("hole_ranges", {}),
        "codegen_filled_path": str(out_map_path),
    }


def augment_codegen_map_with_hole_ranges(
    skeleton_dir: Path | str,
    filled_source: Path | str,
    fills: dict[str, str] | None = None,
    *,
    out_path: Path | str | None = None,
) -> dict[str, Any]:
    """Locate every filled hole in ``src/main.rs`` (including multi-line fills)
    and write ``codegen_filled.json`` with ``hole_ranges``.

    ``hole_ranges`` maps each ``hole_id`` to
    ``{"file": "src/main.rs", "start_line": s, "end_line": e}`` (1-based inclusive).
    """

    skeleton = Path(skeleton_dir).expanduser().resolve()
    codegen_path = skeleton / "codegen.json"
    if codegen_path.is_file():
        data = json.loads(codegen_path.read_text(encoding="utf-8"))
    else:
        data = {"holes": [], "resources": []}

    if isinstance(filled_source, Path):
        filled_text = filled_source.read_text(encoding="utf-8")
    elif isinstance(filled_source, str) and "\n" not in filled_source and Path(filled_source).is_file():
        filled_text = Path(filled_source).read_text(encoding="utf-8")
    else:
        filled_text = str(filled_source)

    hole_ranges: dict[str, dict[str, Any]] = {}

    # 1. Explicit @hole_begin / @hole_end markers (exact even if skeleton was edited).
    for m in HOLE_MARKER_RE.finditer(filled_text):
        hid = m.group("id")
        start_line, end_line = _offset_to_line_range(filled_text, m.start(), m.end())
        hole_ranges[hid] = {
            "file": "src/main.rs",
            "start_line": start_line,
            "end_line": end_line,
        }

    # 2. Structural skeleton alignment (handles unmarked single- and multi-line fills).
    skel_main = skeleton / "src/main.rs"
    if skel_main.is_file():
        skel_text = skel_main.read_text(encoding="utf-8")
        _ok, extracted, _errs = _align_filled_to_skeleton(skel_text, filled_text, fills)
        for item in extracted:
            hid = item["id"]
            if hid not in hole_ranges:
                hole_ranges[hid] = {
                    "file": "src/main.rs",
                    "start_line": item["start_line"],
                    "end_line": item["end_line"],
                }

    # 3. Fallback for any hole still unlocated (e.g., unfilled or modified skeleton).
    fills = fills or {}
    for hole in data.get("holes", []) or []:
        hid = str(hole.get("id", ""))
        if not hid or hid in hole_ranges:
            continue
        if hid in fills and fills[hid]:
            idx = filled_text.find(fills[hid])
            if idx != -1:
                s, e = _offset_to_line_range(filled_text, idx, idx + len(fills[hid]))
                hole_ranges[hid] = {"file": "src/main.rs", "start_line": s, "end_line": e}
                continue
        if isinstance(hole.get("line"), int):
            ln = int(hole["line"])
            hole_ranges[hid] = {"file": "src/main.rs", "start_line": ln, "end_line": ln}

    augmented = dict(data)
    augmented["hole_ranges"] = hole_ranges

    dest = Path(out_path).expanduser().resolve() if out_path else (skeleton / "codegen_filled.json")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(augmented, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return augmented


def _sid_multiset(text: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for m in SID_ANNOTATION_RE.finditer(text):
        sid = m.group(1)
        counts[sid] = counts.get(sid, 0) + 1
    return counts


def _hole_meta_index(codegen_data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    out: dict[str, dict[str, Any]] = {}
    for h in codegen_data.get("holes", []) or []:
        if isinstance(h, dict) and h.get("id"):
            out[str(h["id"])] = h
    return out


def _attribute_span_to_hole(
    rust_span: str | None,
    hole_ranges: dict[str, dict[str, Any]],
    hole_meta: dict[str, dict[str, Any]],
) -> tuple[str | None, str | None, list[str]]:
    """Map a Rust source span (e.g. ``src/main.rs:42:5``) to ``(hole_id, sid, req_clauses)``."""
    if not rust_span:
        return None, None, []
    m = SPAN_LINE_RE.search(rust_span)
    if not m:
        return None, None, []
    line_no = int(m.group(1))
    for hid, rng in hole_ranges.items():
        s = int(rng.get("start_line", -1))
        e = int(rng.get("end_line", -1))
        if s <= line_no <= e:
            meta = hole_meta.get(hid, {})
            sid = meta.get("sid")
            reqs = [str(r) for r in (meta.get("req") or [])]
            return hid, (str(sid) if sid is not None else None), reqs
    return None, None, []


# ---------------------------------------------------------------------------
# Plugin 1: AstHolePurityPlugin
# ---------------------------------------------------------------------------

class AstHolePurityPlugin:
    """Verify skeleton integrity and comment/string-safe sequential hole purity."""

    name = "ast_hole_purity"

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        del spec_path, contract_path  # unused by syntactic purity plugin
        skeleton = Path(skeleton_dir).expanduser().resolve()
        filled = Path(filled_dir).expanduser().resolve()
        skel_main = skeleton / "src/main.rs"
        fill_main = filled / "src/main.rs"

        if not skel_main.is_file() or not fill_main.is_file():
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                error=f"missing src/main.rs in skeleton ({skel_main}) or filled ({fill_main})",
            )

        skel_text = skel_main.read_text(encoding="utf-8")
        fill_text = fill_main.read_text(encoding="utf-8")

        out_map = (
            Path(codegen_map_path).expanduser().resolve()
            if codegen_map_path is not None
            else filled / "codegen_filled.json"
        )
        augmented = augment_codegen_map_with_hole_ranges(
            skeleton, fill_text, fills, out_path=out_map
        )
        hole_ranges: dict[str, dict[str, Any]] = augmented.get("hole_ranges", {})
        hole_meta = _hole_meta_index(augmented)

        skeleton_ok, extracted_holes, structural_errors = _align_filled_to_skeleton(
            skel_text, fill_text, fills
        )
        violations: list[StaticViolation] = []
        skeleton_failed = False

        for err in structural_errors:
            skeleton_failed = True
            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation="O_skeleton_integrity",
                    kind=err["kind"],
                    message=err["message"],
                    rust_span=f"src/main.rs:{err['line']}:1",
                )
            )

        # Check @cir sid multiset across the entire file.
        if _sid_multiset(skel_text) != _sid_multiset(fill_text):
            skeleton_failed = True
            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation="O_skeleton_integrity",
                    kind="sid_count",
                    message=(
                        "multiset of `// @cir <sid>` annotations changed between "
                        "skeleton and filled source"
                    ),
                )
            )

        # Inspect every hole body (using extracted holes, or markers if skeleton failed).
        holes_to_scan = list(extracted_holes)
        if not holes_to_scan and HOLE_MARKER_RE.search(fill_text):
            for m in HOLE_MARKER_RE.finditer(fill_text):
                s, e = _offset_to_line_range(fill_text, m.start(), m.end())
                holes_to_scan.append({
                    "id": m.group("id"),
                    "body": m.group(0),
                    "start_line": s,
                    "end_line": e,
                })

        hole_failed = False
        for item in holes_to_scan:
            hid = item["id"]
            raw_body = item["body"]
            start_line = int(item["start_line"])
            meta = hole_meta.get(hid, {})
            sid = str(meta["sid"]) if meta.get("sid") is not None else None
            reqs = [str(r) for r in (meta.get("req") or [])]

            inner_body = _unwrap_hole_markers(hid, raw_body)

            # Disallow injecting // @cir markers inside a hole.
            if SID_ANNOTATION_RE.search(inner_body):
                hole_failed = True
                violations.append(
                    StaticViolation(
                        plugin=self.name,
                        obligation="O1_purity",
                        kind="cir_annotation_in_hole",
                        message=f"hole '{hid}' must not contain `// @cir` annotations",
                        hole_id=hid,
                        sid=sid,
                        req_clauses=reqs,
                        rust_span=f"src/main.rs:{start_line}:1",
                    )
                )

            stripped = strip_rust_comments_and_literals(inner_body)
            for kind, desc, pattern in _FORBIDDEN_RULES:
                for m in pattern.finditer(stripped):
                    hole_failed = True
                    rel_line = stripped.count("\n", 0, m.start())
                    line_no = start_line + rel_line
                    last_nl = stripped.rfind("\n", 0, m.start())
                    col_no = (m.start() - last_nl) if last_nl >= 0 else (m.start() + 1)
                    token = m.group(0).strip()
                    violations.append(
                        StaticViolation(
                            plugin=self.name,
                            obligation="O1_purity",
                            kind=kind,
                            message=f"{desc}: found `{token}` in hole '{hid}'",
                            hole_id=hid,
                            sid=sid,
                            req_clauses=reqs,
                            rust_span=f"src/main.rs:{line_no}:{col_no}",
                        )
                    )

        if skeleton_failed:
            status = "fail_skeleton"
        elif hole_failed:
            status = "fail_hole"
        else:
            status = "pass"

        return StaticVerificationReport(
            status=status,
            plugin=self.name,
            violations=violations,
            checks={
                "skeleton_integrity": not skeleton_failed,
                "hole_purity": not hole_failed,
            },
            metadata={
                "codegen_filled_path": str(out_map),
                "hole_ranges": hole_ranges,
                "holes_checked": [h["id"] for h in holes_to_scan],
            },
        )


# ---------------------------------------------------------------------------
# Plugin 2: SpnMirRefinementPlugin
# ---------------------------------------------------------------------------

def _format_spn_path(path: Path) -> str:
    """Format a path safely for ``SPN_FLAGS`` (parsed with Rust ``shellwords``)."""
    posix = path.as_posix()
    if any(ch.isspace() for ch in posix):
        return f'"{posix}"'
    return posix


def _detect_spn_toolchain(spn_path: Path, explicit: str | None = None) -> str | None:
    if explicit:
        return explicit
    env_tc = os.environ.get("CONCIR_SPN_TOOLCHAIN")
    if env_tc:
        return env_tc
    # Walk up from spn_path (or repo backends/spn) to find rust-toolchain.toml
    candidates = [
        spn_path.parent.parent.parent / "rust-toolchain.toml",
        Path(__file__).resolve().parents[3] / "backends/spn/rust-toolchain.toml",
    ]
    for tc_file in candidates:
        if tc_file.is_file():
            m = re.search(r'channel\s*=\s*"([^"]+)"', tc_file.read_text(encoding="utf-8"))
            if m:
                return m.group(1)
    return None


def _inject_toolchain_runtime_env(env: dict[str, str], toolchain: str | None) -> None:
    if not toolchain:
        return
    env.setdefault("RUSTUP_TOOLCHAIN", toolchain)
    rustup_home = Path(env.get("RUSTUP_HOME") or (Path.home() / ".rustup")) / "toolchains"
    if not rustup_home.is_dir():
        return
    matches = sorted(
        d for d in rustup_home.iterdir()
        if d.is_dir() and (d.name == toolchain or d.name.startswith(f"{toolchain}-"))
    )
    if not matches:
        return
    tc_dir = matches[0]
    bin_dir = tc_dir / "bin"
    lib_dir = tc_dir / "lib"
    path_sep = os.pathsep
    if bin_dir.is_dir():
        env["PATH"] = f"{bin_dir}{path_sep}{env.get('PATH', '')}"
    if os.name != "nt" and lib_dir.is_dir():
        ld_key = "DYLD_LIBRARY_PATH" if os.uname().sysname == "Darwin" else "LD_LIBRARY_PATH"
        env[ld_key] = f"{lib_dir}{path_sep}{env.get(ld_key, '')}"


class SpnMirRefinementPlugin:
    """Extract MIR Petri net with ``spn`` and verify stuttering refinement with ``concir-backend``."""

    name = "spn_mir_refinement"

    def __init__(
        self,
        *,
        client: ConcirClient | None = None,
        concir_binary: Path | str | None = None,
        spn_binary: Path | str | None = None,
        timeout: float = 180.0,
        cargo_offline: bool = True,
        spn_toolchain: str | None = None,
    ) -> None:
        self.client = client
        self.concir_binary = concir_binary
        self.spn_binary = spn_binary
        self.timeout = float(timeout)
        self.cargo_offline = cargo_offline
        self.spn_toolchain = spn_toolchain

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        skeleton = Path(skeleton_dir).expanduser().resolve()
        filled = Path(filled_dir).expanduser().resolve()

        spn_path = find_spn(self.spn_binary)
        if spn_path is None:
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks={"spn_available": False},
                error="spn binary not found (set CONCIR_SPN or build backends/spn)",
            )

        if spec_path is None or not Path(spec_path).is_file():
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True},
                error="SpnMirRefinementPlugin requires an existing spec_path",
            )
        spec = Path(spec_path).expanduser().resolve()

        # Ensure codegen_filled.json exists with hole_ranges.
        if codegen_map_path is not None and Path(codegen_map_path).is_file():
            cmap_path = Path(codegen_map_path).expanduser().resolve()
            cmap_data = json.loads(cmap_path.read_text(encoding="utf-8"))
        elif (filled / "codegen_filled.json").is_file():
            cmap_path = filled / "codegen_filled.json"
            cmap_data = json.loads(cmap_path.read_text(encoding="utf-8"))
        else:
            cmap_path = filled / "codegen_filled.json"
            fill_main = filled / "src/main.rs"
            if fill_main.is_file():
                cmap_data = augment_codegen_map_with_hole_ranges(
                    skeleton, fill_main, fills, out_path=cmap_path
                )
            else:
                cmap_data = {"holes": [], "resources": [], "hole_ranges": {}}
                cmap_path.write_text(json.dumps(cmap_data, indent=2) + "\n", encoding="utf-8")

        hole_ranges: dict[str, dict[str, Any]] = cmap_data.get("hole_ranges", {}) or {}
        hole_meta = _hole_meta_index(cmap_data)

        spn_work = filled / ".spn-verify"
        spn_work.mkdir(parents=True, exist_ok=True)
        impl_net_path = spn_work / "impl_net.json"
        if impl_net_path.exists():
            impl_net_path.unlink()
        refinement_out_path = spn_work / "refinement_report.json"

        # Use an isolated target dir and clear cached fingerprint for cir_generated so
        # cargo always invokes RUSTC_WRAPPER=spn on the current filled crate.
        target_dir = spn_work / "target-spn"
        if target_dir.exists():
            shutil.rmtree(target_dir, ignore_errors=True)

        argv = ["cargo"]
        if self.spn_toolchain:
            argv.append(f"+{self.spn_toolchain}")
        argv.append("check")
        if self.cargo_offline:
            argv.append("--offline")
        argv.append("--no-default-features")

        spn_flags = (
            f"--export-concir-net {_format_spn_path(impl_net_path)} "
            f"--codegen-map {_format_spn_path(cmap_path)}"
        )
        env = dict(os.environ)
        env["RUSTC_WRAPPER"] = str(spn_path)
        env["SPN_FLAGS"] = spn_flags
        env["CARGO_TARGET_DIR"] = str(target_dir)
        if self.cargo_offline:
            env["CARGO_NET_OFFLINE"] = "true"
        _inject_toolchain_runtime_env(env, _detect_spn_toolchain(spn_path, self.spn_toolchain))

        try:
            proc = subprocess.run(
                argv,
                cwd=str(filled),
                env=env,
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_completed": False},
                error=f"cargo/spn timed out after {self.timeout:g}s",
                metadata={"argv": argv, "spn_flags": spn_flags},
            )
        except OSError as exc:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_completed": False},
                error=f"failed to invoke cargo/spn: {exc}",
                metadata={"argv": argv, "spn_flags": spn_flags},
            )

        if not impl_net_path.is_file():
            stderr_tail = (proc.stderr or proc.stdout or "").strip()[-1500:]
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_exported": False},
                violations=[
                    StaticViolation(
                        plugin=self.name,
                        obligation="O2_refinement",
                        kind="build_failed" if proc.returncode != 0 else "missing_impl_net",
                        message=(
                            f"spn did not produce impl_net.json (cargo exit {proc.returncode}): "
                            f"{stderr_tail}"
                        ),
                    )
                ],
                error=f"spn did not export {impl_net_path.name} (exit {proc.returncode})",
                metadata={
                    "argv": argv,
                    "spn_flags": spn_flags,
                    "cargo_exit_code": proc.returncode,
                    "cargo_stderr": stderr_tail,
                },
            )

        # Resolve ConcirClient for verify-refinement.
        client = self.client
        if client is None:
            backend = find_concir_backend(self.concir_binary)
            if backend is None:
                return StaticVerificationReport(
                    status="unknown",
                    plugin=self.name,
                    checks={"spn_available": True, "spn_exported": True},
                    error="concir-backend not found for verify-refinement",
                    metadata={"impl_net_path": str(impl_net_path)},
                )
            client = ConcirClient(backend, workdir=spn_work / "concir-calls", timeout=self.timeout)

        ref_res = client.verify_refinement(
            spec,
            impl_net_path,
            contract_path=contract_path,
            codegen_map_path=cmap_path,
            out_path=refinement_out_path,
        )

        if ref_res.kind != "semantic":
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"spn_available": True, "spn_exported": True},
                error=ref_res.error or f"verify-refinement {ref_res.kind}: {ref_res.status}",
                metadata={
                    "impl_net_path": str(impl_net_path),
                    "codegen_filled_path": str(cmap_path),
                },
            )

        payload = ref_res.payload or {}
        checks = {k: bool(v) for k, v in (payload.get("checks") or {}).items()}
        checks["spn_available"] = True
        checks["spn_exported"] = True

        metadata = {
            "impl_net_path": str(impl_net_path),
            "codegen_filled_path": str(cmap_path),
            "refinement_report_path": str(refinement_out_path)
            if refinement_out_path.is_file()
            else ref_res.stdout_path,
            "spec_states": payload.get("spec_states"),
            "impl_states": payload.get("impl_states"),
            "relation": payload.get("relation"),
            "outcome": payload.get("outcome"),
        }

        if ref_res.status == "pass":
            return StaticVerificationReport(
                status="pass",
                plugin=self.name,
                checks=checks,
                metadata=metadata,
            )

        if ref_res.status == "unknown":
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks=checks,
                metadata=metadata,
                error=payload.get("message") or "refinement analysis returned unknown (e.g. truncated)",
            )

        # Failure: parse violations and enrich hole attribution via hole_ranges.
        violations: list[StaticViolation] = []
        for raw in payload.get("violations", []) or []:
            if not isinstance(raw, dict):
                continue
            rust_span = raw.get("rust_span")
            hole_id = raw.get("hole_id")
            sid = raw.get("sid")
            req_clauses = [str(r) for r in (raw.get("req_clauses") or [])]

            if not hole_id and rust_span:
                attr_hid, attr_sid, attr_reqs = _attribute_span_to_hole(
                    str(rust_span), hole_ranges, hole_meta
                )
                if attr_hid:
                    hole_id = attr_hid
                if not sid and attr_sid:
                    sid = attr_sid
                if not req_clauses and attr_reqs:
                    req_clauses = attr_reqs
            elif hole_id and (not sid or not req_clauses):
                meta = hole_meta.get(str(hole_id), {})
                if not sid and meta.get("sid"):
                    sid = str(meta["sid"])
                if not req_clauses and meta.get("req"):
                    req_clauses = [str(r) for r in meta["req"]]

            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation=str(raw.get("obligation") or "O2_refinement"),
                    kind=str(raw.get("kind") or "refinement_violation"),
                    message=str(raw.get("message") or "refinement check failed"),
                    hole_id=str(hole_id) if hole_id else None,
                    sid=str(sid) if sid else None,
                    req_clauses=req_clauses,
                    rust_span=str(rust_span) if rust_span else None,
                    witness_trace=list(raw.get("witness_trace") or []),
                    expected_spec_ops=[str(op) for op in (raw.get("expected_spec_ops") or [])],
                )
            )

        if not violations:
            violations.append(
                StaticViolation(
                    plugin=self.name,
                    obligation="O2_refinement",
                    kind=f"refinement_{ref_res.status}",
                    message=str(payload.get("message") or f"verify-refinement returned {ref_res.status}"),
                )
            )

        all_hole_purity = all(
            v.obligation == "O1_purity" or v.kind == "sync_in_hole"
            for v in violations
        )
        status = "fail_hole" if all_hole_purity else "fail_refinement"

        return StaticVerificationReport(
            status=status,
            plugin=self.name,
            violations=violations,
            checks=checks,
            metadata=metadata,
        )


# ---------------------------------------------------------------------------
# Plugin 3: LockbudPlugin (optional adapter for backends/third_party/lockbud)
# ---------------------------------------------------------------------------

class LockbudPlugin:
    """Optional static deadlock detector wrapping ``backends/third_party/lockbud``."""

    name = "lockbud"

    def __init__(
        self,
        *,
        lockbud_binary: Path | str | None = None,
        toolchain: str = LOCKBUD_TOOLCHAIN,
        crate_name: str = "cir_generated",
        timeout: float = 180.0,
        cargo_offline: bool = True,
    ) -> None:
        self.lockbud_binary = lockbud_binary
        self.toolchain = toolchain
        self.crate_name = crate_name
        self.timeout = float(timeout)
        self.cargo_offline = cargo_offline

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        del skeleton_dir, spec_path, contract_path, codegen_map_path, fills
        binary = find_lockbud(self.lockbud_binary)
        if binary is None:
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks={"lockbud_available": False},
                error="lockbud binary not found (set CONCIR_LOCKBUD or build backends/third_party/lockbud)",
            )

        filled = Path(filled_dir).expanduser().resolve()
        target_dir = filled / ".lockbud-verify" / "target"
        target_dir.parent.mkdir(parents=True, exist_ok=True)
        if target_dir.exists():
            shutil.rmtree(target_dir, ignore_errors=True)

        argv = ["cargo", f"+{self.toolchain}", "check"]
        if self.cargo_offline:
            argv.append("--offline")
        argv.append("--no-default-features")

        env = dict(os.environ)
        env.update({
            "RUSTC_WRAPPER": str(binary),
            "LOCKBUD_FLAGS": f"-k deadlock -l {self.crate_name}",
            "LOCKBUD_LOG": "info",
            "RUSTUP_TOOLCHAIN": self.toolchain,
            "CARGO_TARGET_DIR": str(target_dir),
        })
        if self.cargo_offline:
            env["CARGO_NET_OFFLINE"] = "true"

        try:
            proc = subprocess.run(
                argv,
                cwd=str(filled),
                env=env,
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"lockbud timed out after {self.timeout:g}s",
            )
        except OSError as exc:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"failed to run lockbud: {exc}",
            )

        combined = (proc.stdout or "") + "\n" + (proc.stderr or "")
        classified = classify_lockbud(combined)
        detected = classified.get("detected") or []

        if detected:
            violations = [
                StaticViolation(
                    plugin=self.name,
                    obligation="O_deadlock",
                    kind=f"lockbud_{bug_kind}",
                    message=f"lockbud reported potential deadlock ({bug_kind})",
                )
                for bug_kind in detected
            ]
            return StaticVerificationReport(
                status="fail_refinement",
                plugin=self.name,
                violations=violations,
                checks={"lockbud_available": True, "lockbud_deadlock_free": False},
                metadata={"bug_kinds": classified.get("bug_kinds", [])},
            )

        if proc.returncode != 0:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"lockbud failed with exit {proc.returncode}: {(proc.stderr or '').strip()[-500:]}",
            )

        return StaticVerificationReport(
            status="pass",
            plugin=self.name,
            checks={"lockbud_available": True, "lockbud_deadlock_free": True},
            metadata={"bug_kinds": classified.get("bug_kinds", [])},
        )


# ---------------------------------------------------------------------------
# CompositeStaticVerifier
# ---------------------------------------------------------------------------

class CompositeStaticVerifier:
    """Run an ordered sequence of :class:`StaticVerifierPlugin` instances and aggregate verdicts."""

    def __init__(
        self,
        plugins: list[StaticVerifierPlugin],
        *,
        stop_on_skeleton_fail: bool = True,
    ) -> None:
        self.plugins = list(plugins)
        self.stop_on_skeleton_fail = stop_on_skeleton_fail

    def verify(
        self,
        *,
        skeleton_dir: Path | str,
        filled_dir: Path | str,
        spec_path: Path | str | None = None,
        contract_path: Path | str | None = None,
        codegen_map_path: Path | str | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        if not self.plugins:
            return StaticVerificationReport(
                status="unknown",
                plugin="composite",
                error="no static verifier plugins configured",
            )

        skeleton = Path(skeleton_dir).expanduser().resolve()
        filled = Path(filled_dir).expanduser().resolve()
        spec = Path(spec_path).expanduser().resolve() if spec_path is not None else None
        contract = Path(contract_path).expanduser().resolve() if contract_path is not None else None
        cmap = Path(codegen_map_path).expanduser().resolve() if codegen_map_path is not None else None

        all_violations: list[StaticViolation] = []
        merged_checks: dict[str, bool] = {}
        plugin_reports: list[dict[str, Any]] = []
        merged_meta: dict[str, Any] = {}
        statuses: list[str] = []
        errors: list[str] = []

        for plugin in self.plugins:
            rep = plugin.verify(
                skeleton_dir=skeleton,
                filled_dir=filled,
                spec_path=spec,
                contract_path=contract,
                codegen_map_path=cmap,
                fills=fills,
            )
            statuses.append(rep.status)
            plugin_reports.append(rep.as_dict())
            all_violations.extend(rep.violations)
            merged_checks.update(rep.checks)
            merged_meta.update(rep.metadata)
            if rep.error:
                errors.append(f"{plugin.name}: {rep.error}")

            # Thread generated codegen_filled.json to downstream plugins.
            if cmap is None and rep.metadata.get("codegen_filled_path"):
                candidate = Path(rep.metadata["codegen_filled_path"])
                if candidate.is_file():
                    cmap = candidate

            if self.stop_on_skeleton_fail and rep.status == "fail_skeleton":
                break

        # Determine aggregate status (never downgrade unknown/fail/error to pass).
        if "fail_skeleton" in statuses:
            agg_status = "fail_skeleton"
        elif "fail_hole" in statuses:
            agg_status = "fail_hole"
        elif "fail_refinement" in statuses:
            agg_status = "fail_refinement"
        elif "error" in statuses:
            agg_status = "error"
        elif "unknown" in statuses:
            agg_status = "unknown"
        else:
            agg_status = "pass"

        return StaticVerificationReport(
            status=agg_status,
            plugin="composite",
            violations=all_violations,
            checks=merged_checks,
            plugin_reports=plugin_reports,
            metadata=merged_meta,
            error="; ".join(errors) if errors else None,
        )
