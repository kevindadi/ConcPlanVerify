"""Multi-file SeqHole & TypeHole discovery, alignment, marked filling, and line-range tracking."""

from __future__ import annotations

import json
import re
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ..core.json_utils import extract_json

PLACEHOLDER_RE = re.compile(
    r"/\* HOLE\((?P<id>[A-Za-z0-9_]+)\) expected: (?P<expected>.*?) \*/ Default::default\(\)"
)
HOLE_MARKER_RE = re.compile(
    r"/\* (?:@hole_begin|HOLE_BEGIN)\((?P<id>[A-Za-z0-9_]+)\) \*/"
    r"(?P<body>.*?)"
    r"/\* (?:@hole_end|HOLE_END)\((?P=id)\) \*/",
    re.DOTALL,
)
TYPE_HOLE_PLACEHOLDER_RE = re.compile(
    r"/\* TYPE_HOLE\((?P<id>[A-Za-z0-9_]+)\)(?P<meta>.*?) \*/"
    r"\s*(?:#\[[^\]]*\]\s*)*"
    r"pub\s+struct\s+(?P<type_name>[A-Za-z_][A-Za-z0-9_]*)\s*\{\s*\}",
    re.DOTALL,
)
TYPE_HOLE_MARKER_RE = re.compile(
    r"/\* (?:TYPE_HOLE_BEGIN|@type_hole_begin)\((?P<id>[A-Za-z0-9_]+)\) \*/"
    r"(?P<body>.*?)"
    r"/\* (?:TYPE_HOLE_END|@type_hole_end)\((?P=id)\) \*/",
    re.DOTALL,
)
BOUNDED_LOOP_HINT_RE = re.compile(r"(?://|/\*)\s*@bound(?:ed)?\b", re.IGNORECASE)
SID_ANNOTATION_RE = re.compile(r"// @cir (\S+)")
SPAN_FILE_LINE_RE = re.compile(r"(?:(?P<file>[^\s:][^:]*\.rs):)?(?P<line>\d+)(?::\d+)?")
SPAN_LINE_RE = re.compile(r":(\d+)(?::\d+)?")
RUSTC_ERROR_SPAN_RE = re.compile(r"-->\s+(?:[^\n]*?[/\\])?(src[/\\][^\s:]+\.rs):(\d+):(\d+)")


@dataclass
class HoleFill:
    hole_id: str
    expected: str
    code: str


def holes_of(skeleton_dir: Path | str) -> list[dict[str, Any]]:
    map_path = Path(skeleton_dir) / "codegen.json"
    if not map_path.is_file():
        return []
    data = json.loads(map_path.read_text(encoding="utf-8"))
    return list(data.get("holes", []))


def type_holes_of(skeleton_dir: Path | str) -> list[dict[str, Any]]:
    map_path = Path(skeleton_dir) / "codegen.json"
    if not map_path.is_file():
        return []
    data = json.loads(map_path.read_text(encoding="utf-8"))
    return list(data.get("type_holes", []))


def fill_prompt(skeleton_dir: Path | str, requirements: str, *, context_lines: int = 8) -> str:
    """Build the hole-filling prompt (multi-file SeqHoles + TypeHoles with context)."""
    root = Path(skeleton_dir)
    file_cache: dict[str, list[str]] = {}

    def _read_lines(rel_file: str) -> list[str]:
        if rel_file not in file_cache:
            path = root / rel_file
            file_cache[rel_file] = (
                path.read_text(encoding="utf-8").splitlines() if path.is_file() else []
            )
        return file_cache[rel_file]

    parts = [
        "You are filling only the marked holes in a Rust skeleton.",
        "Rules:",
        "- Replace ONLY the text inside a `/* HOLE(id) expected: T */` marker's",
        "  placeholder (or `/* TYPE_HOLE(id) ... */` struct in `src/domain.rs`);",
        "  do not touch any other line, statement, or `// @cir` sid comment.",
        "- SeqHole code is sequential local computation only. It must not introduce",
        "  any synchronization, thread, channel, atomic, unsafe, `cir_trace`,",
        "  unbounded loop, or new `use`.",
        "- TypeHole code in `src/domain.rs` must define a pure `pub struct <Name>`",
        "  implementing `Send + Sync + Clone + Default + 'static` (pure `impl`",
        "  methods and bounded `for` loops allowed; no sync, threads, atomics,",
        "  unsafe, `cir_trace`, or unbounded `loop`/`while`).",
        "- Return a JSON object mapping each hole / type_hole id to the exact Rust",
        "  code to substitute.",
        "",
        f"Specification:\n{requirements}",
        "",
    ]
    for th in type_holes_of(root):
        th_id = th["id"]
        rel_file = th.get("file") or "src/domain.rs"
        source = _read_lines(rel_file)
        idx = next((i for i, l in enumerate(source) if f"TYPE_HOLE({th_id})" in l), None)
        if idx is None:
            continue
        lo, hi = max(0, idx - context_lines), min(len(source), idx + context_lines + 1)
        type_name = th.get("type_name") or ""
        parts.append(f"### type_hole {th_id} (type {type_name} in {rel_file})")
        parts.append("```rust")
        parts.extend(f"{n + 1:>4}: {source[n]}" for n in range(lo, hi))
        parts.append("```")
        parts.append("")
    for hole in holes_of(root):
        hid = hole["id"]
        rel_file = hole.get("file") or "src/main.rs"
        source = _read_lines(rel_file)
        idx = next((i for i, l in enumerate(source) if f"HOLE({hid})" in l), None)
        if idx is None:
            continue
        lo, hi = max(0, idx - context_lines), min(len(source), idx + context_lines + 1)
        parts.append(f"### hole {hid} (expected {hole['expected']} in {rel_file})")
        parts.append("```rust")
        parts.extend(f"{n + 1:>4}: {source[n]}" for n in range(lo, hi))
        parts.append("```")
        parts.append("")
    parts.append(
        "Reply with only the JSON object, e.g. "
        '{"h1": "x + 1", "h2": "if flag { 1 } else { 0 }"}.'
    )
    return "\n".join(parts)


def parse_fill(text: str) -> dict[str, str]:
    """Extract ``{hole_id: code}`` from a model reply (JSON object, fenced ok)."""
    try:
        data = json.loads(extract_json(text))
    except json.JSONDecodeError:
        return {}
    if not isinstance(data, dict):
        return {}
    return {str(k): str(v) for k, v in data.items()}


def list_src_rs_files(crate_dir: Path) -> list[str]:
    src_dir = crate_dir / "src"
    if not src_dir.is_dir():
        return []
    return sorted(
        f"src/{p.relative_to(src_dir).as_posix()}"
        for p in src_dir.rglob("*.rs")
        if p.is_file()
    )


def _discover_type_holes(skeleton: Path, filled: Path) -> list[dict[str, Any]]:
    known = type_holes_of(skeleton)
    if known:
        return known
    domain_rs = filled / "src/domain.rs"
    if domain_rs.is_file():
        return [
            {
                "id": m.group("id"),
                "type_name": m.group("type_name"),
                "file": "src/domain.rs",
            }
            for m in TYPE_HOLE_PLACEHOLDER_RE.finditer(domain_rs.read_text(encoding="utf-8"))
        ]
    return []


def _apply_seq_hole_fills_in_place(
    skeleton: Path,
    filled: Path,
    fills: dict[str, str],
    *,
    marked: bool = True,
) -> tuple[list[str], list[str]]:
    applied: list[str] = []
    missing: list[str] = []
    rs_files = list_src_rs_files(filled)

    for hole in holes_of(skeleton):
        hid = hole["id"]
        if hid not in fills:
            missing.append(hid)
            continue
        preferred = str(hole.get("file") or "src/main.rs")
        candidate_files = [preferred] + [f for f in rs_files if f != preferred]
        placeholder = f"/* HOLE({hid}) expected: {hole['expected']} */ Default::default()"
        replacement = (
            f"/* @hole_begin({hid}) */ {fills[hid]} /* @hole_end({hid}) */"
            if marked
            else fills[hid]
        )
        pattern = re.compile(
            rf"/\* HOLE\({re.escape(hid)}\) expected: .*? \*/ Default::default\(\)"
        )
        matched = False
        for rel in candidate_files:
            fpath = filled / rel
            if not fpath.is_file():
                continue
            text = fpath.read_text(encoding="utf-8")
            if placeholder in text:
                fpath.write_text(text.replace(placeholder, replacement, 1), encoding="utf-8")
                applied.append(hid)
                matched = True
                break
            new_text, count = pattern.subn(lambda _m: replacement, text, count=1)
            if count:
                fpath.write_text(new_text, encoding="utf-8")
                applied.append(hid)
                matched = True
                break
        if not matched:
            missing.append(hid)
    return applied, missing


def _apply_type_hole_fills_in_place(
    skeleton: Path,
    filled: Path,
    type_fills: dict[str, str],
    *,
    marked: bool = True,
) -> tuple[list[str], list[str]]:
    applied: list[str] = []
    missing: list[str] = []

    for th in _discover_type_holes(skeleton, filled):
        th_id = str(th.get("id") or "")
        if not th_id:
            continue
        if th_id not in type_fills:
            missing.append(th_id)
            continue
        rel_file = str(th.get("file") or "src/domain.rs")
        target_path = filled / rel_file
        if not target_path.is_file():
            missing.append(th_id)
            continue
        text = target_path.read_text(encoding="utf-8")
        body_code = type_fills[th_id].strip("\r\n")
        replacement = (
            f"/* TYPE_HOLE_BEGIN({th_id}) */\n{body_code}\n/* TYPE_HOLE_END({th_id}) */"
            if marked
            else body_code
        )
        pattern = re.compile(
            rf"/\* TYPE_HOLE\({re.escape(th_id)}\).*? \*/"
            r"\s*(?:#\[[^\]]*\]\s*)*"
            r"pub\s+struct\s+[A-Za-z_][A-Za-z0-9_]*\s*\{\s*\}",
            re.DOTALL,
        )
        new_text, count = pattern.subn(lambda _m: replacement, text, count=1)
        if count:
            target_path.write_text(new_text, encoding="utf-8")
            applied.append(th_id)
        else:
            missing.append(th_id)
    return applied, missing


def fill_holes(skeleton_dir: Path | str, filled_dir: Path | str, fills: dict[str, str]) -> dict[str, Any]:
    """Substitute SeqHole and TypeHole code into a copy of the skeleton without markers."""
    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if filled.exists():
        shutil.rmtree(filled)
    shutil.copytree(skeleton, filled)

    applied, missing = _apply_seq_hole_fills_in_place(skeleton, filled, fills, marked=False)
    applied_th, missing_th = _apply_type_hole_fills_in_place(skeleton, filled, fills, marked=False)
    return {
        "applied": applied,
        "missing": missing,
        "applied_type_holes": applied_th,
        "missing_type_holes": missing_th,
    }


def fill_type_holes_marked(
    skeleton_dir: Path | str,
    filled_dir: Path | str,
    type_fills: dict[str, str],
    *,
    copy_skeleton: bool = True,
) -> dict[str, Any]:
    """Substitute ``TYPE_HOLE(id)`` placeholders wrapped in BEGIN/END markers and record ranges."""
    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if copy_skeleton and filled != skeleton:
        if filled.exists():
            shutil.rmtree(filled)
        shutil.copytree(skeleton, filled)

    applied, missing = _apply_type_hole_fills_in_place(skeleton, filled, type_fills)
    out_map_path = filled / "codegen_filled.json"
    augmented = augment_codegen_map_with_hole_ranges(
        skeleton, filled, type_fills, out_path=out_map_path
    )
    return {
        "applied": applied,
        "missing": missing,
        "type_hole_ranges": augmented.get("type_hole_ranges", {}),
        "hole_ranges": augmented.get("hole_ranges", {}),
        "codegen_filled_path": str(out_map_path),
    }


def fill_holes_marked(
    skeleton_dir: Path | str,
    filled_dir: Path | str,
    fills: dict[str, str],
    *,
    type_fills: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Copy skeleton, substitute SeqHoles/TypeHoles wrapped in markers, and write ``codegen_filled.json``."""
    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if filled.exists():
        shutil.rmtree(filled)
    shutil.copytree(skeleton, filled)

    applied, missing = _apply_seq_hole_fills_in_place(skeleton, filled, fills, marked=True)

    combined_type_fills: dict[str, str] = {}
    if type_fills:
        combined_type_fills.update(type_fills)
    for th in _discover_type_holes(skeleton, filled):
        th_id = str(th.get("id") or "")
        if th_id and th_id in fills and th_id not in combined_type_fills:
            combined_type_fills[th_id] = fills[th_id]

    applied_th, missing_th = _apply_type_hole_fills_in_place(
        skeleton, filled, combined_type_fills, marked=True
    )

    out_map_path = filled / "codegen_filled.json"
    all_fills = dict(fills)
    all_fills.update(combined_type_fills)
    augmented = augment_codegen_map_with_hole_ranges(
        skeleton, filled, all_fills, out_path=out_map_path
    )
    return {
        "applied": applied,
        "missing": missing,
        "applied_type_holes": applied_th,
        "missing_type_holes": missing_th,
        "hole_ranges": augmented.get("hole_ranges", {}),
        "type_hole_ranges": augmented.get("type_hole_ranges", {}),
        "codegen_filled_path": str(out_map_path),
    }


def split_skeleton_by_holes(skeleton_source: str) -> tuple[list[str], list[dict[str, Any]]]:
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


def split_skeleton_by_type_holes(skeleton_source: str) -> tuple[list[str], list[dict[str, Any]]]:
    segments: list[str] = []
    holes: list[dict[str, Any]] = []
    cursor = 0
    for m in TYPE_HOLE_PLACEHOLDER_RE.finditer(skeleton_source):
        segments.append(skeleton_source[cursor:m.start()])
        holes.append({
            "id": m.group("id"),
            "type_name": m.group("type_name"),
            "placeholder": m.group(0),
            "start": m.start(),
            "end": m.end(),
        })
        cursor = m.end()
    segments.append(skeleton_source[cursor:])
    return segments, holes


def offset_to_line_range(text: str, start: int, end: int) -> tuple[int, int]:
    start_line = text.count("\n", 0, start) + 1
    if end <= start:
        return start_line, start_line
    end_line = text.count("\n", 0, end - 1) + 1
    return start_line, end_line


def first_diff_line(expected: str, actual: str) -> int:
    e_lines = expected.splitlines()
    a_lines = actual.splitlines()
    for i in range(min(len(e_lines), len(a_lines))):
        if e_lines[i] != a_lines[i]:
            return i + 1
    return min(len(e_lines), len(a_lines)) + 1


def _align_filled_generic(
    skeleton_source: str,
    filled_source: str,
    *,
    splitter,
    marker_pairs_fn,
    hole_label: str,
    empty_message_tpl: str,
    extra_key: str,
    fills: dict[str, str] | None = None,
    try_fill_variants: bool = False,
) -> tuple[bool, list[dict[str, Any]], list[dict[str, Any]]]:
    segments, skel_holes = splitter(skeleton_source)
    structural_errors: list[dict[str, Any]] = []
    extracted: list[dict[str, Any]] = []

    if not skel_holes:
        if filled_source == skeleton_source:
            return True, [], []
        diff_line = first_diff_line(skeleton_source, filled_source)
        structural_errors.append({
            "kind": "changed_outside_hole",
            "line": diff_line,
            "message": empty_message_tpl.format(line=diff_line),
        })
        return False, [], structural_errors

    if not filled_source.startswith(segments[0]):
        diff_line = first_diff_line(segments[0], filled_source)
        structural_errors.append({
            "kind": "changed_outside_hole",
            "line": diff_line,
            "message": f"code before first {hole_label} ({skel_holes[0]['id']}) was modified near line {diff_line}",
        })
        return False, [], structural_errors

    pos = len(segments[0])
    fills = fills or {}

    def _matches_next_seg(candidate_next: int, next_seg: str, is_last: bool) -> bool:
        if is_last:
            return filled_source[candidate_next:] == next_seg
        return filled_source.startswith(next_seg, candidate_next)

    for idx, hole in enumerate(skel_holes):
        hid = hole["id"]
        next_seg = segments[idx + 1]
        is_last = idx == len(skel_holes) - 1
        next_pos = -1

        for marker_prefix, marker_suffix in marker_pairs_fn(hid):
            if filled_source.startswith(marker_prefix, pos):
                end_marker = filled_source.find(marker_suffix, pos + len(marker_prefix))
                if end_marker != -1:
                    candidate_next = end_marker + len(marker_suffix)
                    if _matches_next_seg(candidate_next, next_seg, is_last):
                        next_pos = candidate_next
                        break

        if next_pos == -1 and try_fill_variants and hid in fills:
            candidate_body = fills[hid]
            mp_prefix, mp_suffix = marker_pairs_fn(hid)[0]
            for variant in (
                f"{mp_prefix} {candidate_body} {mp_suffix}",
                f"{mp_prefix}{candidate_body}{mp_suffix}",
                candidate_body,
            ):
                if filled_source.startswith(variant, pos):
                    candidate_next = pos + len(variant)
                    if _matches_next_seg(candidate_next, next_seg, is_last):
                        next_pos = candidate_next
                        break

        if next_pos == -1 and filled_source.startswith(hole["placeholder"], pos):
            candidate_next = pos + len(hole["placeholder"])
            if _matches_next_seg(candidate_next, next_seg, is_last):
                next_pos = candidate_next

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
                    f"skeleton segment after {hole_label} '{hid}' was modified or removed "
                    f"(expected static code starting near line {line_no})"
                ),
            })
            return False, extracted, structural_errors

        body = filled_source[pos:next_pos]
        start_line, end_line = offset_to_line_range(filled_source, pos, next_pos)
        extracted.append({
            "id": hid,
            extra_key: hole[extra_key],
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
            "message": f"trailing code after last {hole_label} was modified near line {line_no}",
        })
        return False, extracted, structural_errors

    return True, extracted, []


def align_filled_to_skeleton(
    skeleton_source: str,
    filled_source: str,
    fills: dict[str, str] | None = None,
) -> tuple[bool, list[dict[str, Any]], list[dict[str, Any]]]:
    return _align_filled_generic(
        skeleton_source,
        filled_source,
        splitter=split_skeleton_by_holes,
        marker_pairs_fn=lambda hid: ((f"/* @hole_begin({hid}) */", f"/* @hole_end({hid}) */"),),
        hole_label="hole",
        empty_message_tpl="source modified outside holes near line {line}",
        extra_key="expected",
        fills=fills,
        try_fill_variants=True,
    )


def align_filled_to_type_hole_skeleton(
    skeleton_source: str,
    filled_source: str,
    fills: dict[str, str] | None = None,
) -> tuple[bool, list[dict[str, Any]], list[dict[str, Any]]]:
    return _align_filled_generic(
        skeleton_source,
        filled_source,
        splitter=split_skeleton_by_type_holes,
        marker_pairs_fn=lambda th_id: (
            (f"/* TYPE_HOLE_BEGIN({th_id}) */", f"/* TYPE_HOLE_END({th_id}) */"),
            (f"/* @type_hole_begin({th_id}) */", f"/* @type_hole_end({th_id}) */"),
        ),
        hole_label="type hole",
        empty_message_tpl="domain source modified outside type holes near line {line}",
        extra_key="type_name",
        fills=fills,
        try_fill_variants=False,
    )


def _unwrap_markers(
    hid: str,
    body: str,
    pattern: re.Pattern[str],
    pairs: tuple[tuple[str, str], ...],
) -> str:
    m = pattern.fullmatch(body.strip())
    if m and m.group("id") == hid:
        return m.group("body")
    for prefix, suffix in pairs:
        if body.startswith(prefix) and body.endswith(suffix):
            return body[len(prefix):len(body) - len(suffix)]
    return body


def unwrap_hole_markers(hid: str, body: str) -> str:
    return _unwrap_markers(
        hid,
        body,
        HOLE_MARKER_RE,
        (
            (f"/* @hole_begin({hid}) */", f"/* @hole_end({hid}) */"),
            (f"/* HOLE_BEGIN({hid}) */", f"/* HOLE_END({hid}) */"),
        ),
    )


def unwrap_type_hole_markers(th_id: str, body: str) -> str:
    return _unwrap_markers(
        th_id,
        body,
        TYPE_HOLE_MARKER_RE,
        (
            (f"/* TYPE_HOLE_BEGIN({th_id}) */", f"/* TYPE_HOLE_END({th_id}) */"),
            (f"/* @type_hole_begin({th_id}) */", f"/* @type_hole_end({th_id}) */"),
        ),
    )


def augment_codegen_map_with_hole_ranges(
    skeleton_dir: Path | str,
    filled_source: Path | str,
    fills: dict[str, str] | None = None,
    *,
    out_path: Path | str | None = None,
) -> dict[str, Any]:
    """Locate every filled SeqHole and TypeHole and write ``codegen_filled.json`` with line ranges."""
    skeleton = Path(skeleton_dir).expanduser().resolve()
    codegen_path = skeleton / "codegen.json"
    if codegen_path.is_file():
        data = json.loads(codegen_path.read_text(encoding="utf-8"))
    else:
        data = {"holes": [], "resources": []}

    filled_files: dict[str, str] = {}
    if isinstance(filled_source, Path) and filled_source.is_dir():
        for rel in list_src_rs_files(filled_source):
            filled_files[rel] = (filled_source / rel).read_text(encoding="utf-8")
    elif isinstance(filled_source, Path) and filled_source.is_file():
        crate_candidate = filled_source.parent.parent if filled_source.parent.name == "src" else None
        if crate_candidate and (crate_candidate / "src").is_dir():
            for rel in list_src_rs_files(crate_candidate):
                filled_files[rel] = (crate_candidate / rel).read_text(encoding="utf-8")
        else:
            filled_files["src/main.rs"] = filled_source.read_text(encoding="utf-8")
    elif isinstance(filled_source, str) and "\n" not in filled_source and Path(filled_source).exists():
        p = Path(filled_source)
        if p.is_dir():
            for rel in list_src_rs_files(p):
                filled_files[rel] = (p / rel).read_text(encoding="utf-8")
        else:
            filled_files["src/main.rs"] = p.read_text(encoding="utf-8")
    else:
        filled_files["src/main.rs"] = str(filled_source)

    hole_ranges: dict[str, dict[str, Any]] = {}
    type_hole_ranges: dict[str, dict[str, Any]] = {}
    fills = fills or {}

    for rel, text in filled_files.items():
        for m in HOLE_MARKER_RE.finditer(text):
            hid = m.group("id")
            s, e = offset_to_line_range(text, m.start(), m.end())
            hole_ranges[hid] = {"file": rel, "start_line": s, "end_line": e}

        for m in TYPE_HOLE_MARKER_RE.finditer(text):
            th_id = m.group("id")
            s, e = offset_to_line_range(text, m.start(), m.end())
            type_hole_ranges[th_id] = {"file": rel, "start_line": s, "end_line": e}

        skel_file = skeleton / rel
        if skel_file.is_file():
            skel_text = skel_file.read_text(encoding="utf-8")
            if PLACEHOLDER_RE.search(skel_text):
                _ok, extracted, _errs = align_filled_to_skeleton(skel_text, text, fills)
                for item in extracted:
                    hid = item["id"]
                    if hid not in hole_ranges:
                        hole_ranges[hid] = {
                            "file": rel,
                            "start_line": item["start_line"],
                            "end_line": item["end_line"],
                        }
            if TYPE_HOLE_PLACEHOLDER_RE.search(skel_text):
                _ok, th_extracted, _errs = align_filled_to_type_hole_skeleton(skel_text, text, fills)
                for item in th_extracted:
                    th_id = item["id"]
                    if th_id not in type_hole_ranges:
                        type_hole_ranges[th_id] = {
                            "file": rel,
                            "start_line": item["start_line"],
                            "end_line": item["end_line"],
                        }

    for hole in data.get("holes", []) or []:
        hid = str(hole.get("id", ""))
        if not hid or hid in hole_ranges:
            continue
        rel = str(hole.get("file") or "src/main.rs")
        text = filled_files.get(rel, filled_files.get("src/main.rs", ""))
        if hid in fills and fills[hid] and text:
            idx = text.find(fills[hid])
            if idx != -1:
                s, e = offset_to_line_range(text, idx, idx + len(fills[hid]))
                hole_ranges[hid] = {"file": rel, "start_line": s, "end_line": e}
                continue
        if isinstance(hole.get("line"), int):
            ln = int(hole["line"])
            hole_ranges[hid] = {"file": rel, "start_line": ln, "end_line": ln}

    for th in data.get("type_holes", []) or []:
        th_id = str(th.get("id", ""))
        if not th_id or th_id in type_hole_ranges:
            continue
        rel = str(th.get("file") or "src/domain.rs")
        text = filled_files.get(rel, "")
        if th_id in fills and fills[th_id] and text:
            idx = text.find(fills[th_id])
            if idx != -1:
                s, e = offset_to_line_range(text, idx, idx + len(fills[th_id]))
                type_hole_ranges[th_id] = {"file": rel, "start_line": s, "end_line": e}
                continue
        if isinstance(th.get("line"), int):
            ln = int(th["line"])
            type_hole_ranges[th_id] = {"file": rel, "start_line": ln, "end_line": ln}

    augmented = dict(data)
    augmented["hole_ranges"] = hole_ranges
    if type_hole_ranges or data.get("type_holes"):
        augmented["type_hole_ranges"] = type_hole_ranges

    dest = Path(out_path).expanduser().resolve() if out_path else (skeleton / "codegen_filled.json")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(augmented, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return augmented


def sid_multiset(text: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for m in SID_ANNOTATION_RE.finditer(text):
        sid = m.group(1)
        counts[sid] = counts.get(sid, 0) + 1
    return counts


def _index_by_id(entries: list[Any] | None) -> dict[str, dict[str, Any]]:
    out: dict[str, dict[str, Any]] = {}
    for item in entries or []:
        if isinstance(item, dict) and item.get("id"):
            out[str(item["id"])] = item
    return out


def hole_meta_index(codegen_data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return _index_by_id(codegen_data.get("holes"))


def type_hole_meta_index(codegen_data: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return _index_by_id(codegen_data.get("type_holes"))


def _span_file_matches(span_file: str | None, range_file: str | None) -> bool:
    if not span_file or not range_file:
        return True
    norm_span = span_file.replace("\\", "/")
    norm_rng = range_file.replace("\\", "/")
    return norm_span == norm_rng or norm_span.endswith("/" + norm_rng) or norm_rng.endswith("/" + norm_span)


def attribute_span_to_hole(
    rust_span: str | None,
    hole_ranges: dict[str, dict[str, Any]],
    hole_meta: dict[str, dict[str, Any]],
    *,
    type_hole_ranges: dict[str, dict[str, Any]] | None = None,
    type_hole_meta: dict[str, dict[str, Any]] | None = None,
) -> tuple[str | None, str | None, list[str]]:
    """Map a Rust source span to ``(hole_id, sid, req_clauses)`` across SeqHoles and TypeHoles."""
    if not rust_span:
        return None, None, []
    span_file: str | None = None
    line_no: int | None = None
    m_fl = SPAN_FILE_LINE_RE.search(rust_span)
    if m_fl:
        span_file = m_fl.group("file")
        line_no = int(m_fl.group("line"))
    else:
        m = SPAN_LINE_RE.search(rust_span)
        if m:
            line_no = int(m.group(1))
    if line_no is None:
        return None, None, []

    for hid, rng in (type_hole_ranges or {}).items():
        if not _span_file_matches(span_file, rng.get("file")):
            continue
        s = int(rng.get("start_line", -1))
        e = int(rng.get("end_line", -1))
        if s <= line_no <= e:
            meta = (type_hole_meta or {}).get(hid, {})
            reqs = [str(r) for r in (meta.get("req") or [])]
            return hid, None, reqs

    for hid, rng in hole_ranges.items():
        if not _span_file_matches(span_file, rng.get("file")):
            continue
        s = int(rng.get("start_line", -1))
        e = int(rng.get("end_line", -1))
        if s <= line_no <= e:
            meta = hole_meta.get(hid, {})
            sid = meta.get("sid")
            reqs = [str(r) for r in (meta.get("req") or [])]
            return hid, (str(sid) if sid is not None else None), reqs
    return None, None, []
