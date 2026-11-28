"""AST-level SeqHole & TypeHole purity and skeleton integrity verifier plugin."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from .holes import (
    BOUNDED_LOOP_HINT_RE,
    HOLE_MARKER_RE,
    PLACEHOLDER_RE,
    SID_ANNOTATION_RE,
    TYPE_HOLE_MARKER_RE,
    TYPE_HOLE_PLACEHOLDER_RE,
    align_filled_to_skeleton,
    align_filled_to_type_hole_skeleton,
    augment_codegen_map_with_hole_ranges,
    first_diff_line,
    hole_meta_index,
    list_src_rs_files,
    offset_to_line_range,
    sid_multiset,
    type_hole_meta_index,
    unwrap_hole_markers,
    unwrap_type_hole_markers,
)
from .rust_lexer import strip_rust_comments_and_literals
from .verifier import StaticVerificationReport, StaticViolation

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

_TYPE_HOLE_FORBIDDEN_RULES: tuple[tuple[str, str, re.Pattern[str]], ...] = (
    (
        "sync_in_type_hole",
        "forbidden synchronization construct inside domain TypeHole",
        re.compile(
            r"\b(?:std::sync|Mutex|Condvar|RwLock|Semaphore|Barrier|OnceLock|Once|mpsc|channel)\b"
            r"|\.(?:lock|try_lock|wait|wait_while|wait_timeout|"
            r"notify_one|notify_all|send|recv|try_recv)\s*\("
        ),
    ),
    (
        "thread_in_type_hole",
        "forbidden thread or blocking construct inside domain TypeHole",
        re.compile(r"\b(?:std::thread|thread::spawn|thread::scope|thread::sleep)\b"),
    ),
    (
        "atomic_in_type_hole",
        "forbidden atomic construct inside domain TypeHole",
        re.compile(r"\b(?:Atomic\w*|atomic::|Ordering::)\b"),
    ),
    (
        "unsafe_in_type_hole",
        "forbidden unsafe/trace/extern construct inside domain TypeHole",
        re.compile(r"\b(?:unsafe|cir_trace|extern)\b"),
    ),
    (
        "loop_in_type_hole",
        "forbidden unbounded loop/while without explicit bound inside domain TypeHole",
        re.compile(r"\b(?:loop|while|spin_loop)\b"),
    ),
)


def _is_explicitly_bounded_loop(raw_inner: str, rel_line: int) -> bool:
    lines = raw_inner.splitlines()
    for idx in (rel_line, rel_line - 1):
        if 0 <= idx < len(lines) and BOUNDED_LOOP_HINT_RE.search(lines[idx]):
            return True
    return False


def scan_filled_crate(
    skeleton: Path,
    filled: Path,
    *,
    fills: dict[str, str] | None = None,
    hole_meta: dict[str, dict[str, Any]] | None = None,
    type_hole_meta: dict[str, dict[str, Any]] | None = None,
    plugin_name: str = "ast_hole_purity",
    check_sid_on_all_files: bool = False,
) -> tuple[bool, bool, list[StaticViolation], list[str], list[str], list[str]]:
    """Pure multi-file integrity and O1 purity scanner (no disk writes)."""
    hole_meta = hole_meta or {}
    type_hole_meta = type_hole_meta or {}
    skel_files = list_src_rs_files(skeleton)
    fill_files = list_src_rs_files(filled)

    violations: list[StaticViolation] = []
    skeleton_failed = False
    hole_failed = False
    holes_checked: list[str] = []
    type_holes_checked: list[str] = []

    for extra in sorted(set(fill_files) - set(skel_files)):
        skeleton_failed = True
        violations.append(
            StaticViolation(
                plugin=plugin_name,
                obligation="O_skeleton_integrity",
                kind="unexpected_source_file",
                message=f"unexpected Rust file added outside skeleton: {extra}",
                rust_span=f"{extra}:1:1",
            )
        )
    for missing_f in sorted(set(skel_files) - set(fill_files)):
        skeleton_failed = True
        violations.append(
            StaticViolation(
                plugin=plugin_name,
                obligation="O_skeleton_integrity",
                kind="missing_source_file",
                message=f"skeleton Rust file removed in filled output: {missing_f}",
                rust_span=f"{missing_f}:1:1",
            )
        )

    def _check_sid_multiset(rel: str, skel_text: str, fill_text: str) -> None:
        nonlocal skeleton_failed
        s_sids = sid_multiset(skel_text)
        f_sids = sid_multiset(fill_text)
        if s_sids != f_sids:
            skeleton_failed = True
            violations.append(
                StaticViolation(
                    plugin=plugin_name,
                    obligation="O_skeleton_integrity",
                    kind="sid_count",
                    message=(
                        f"multiset of `// @cir <sid>` annotations changed between "
                        f"skeleton and filled source in {rel}"
                    ),
                    rust_span=f"{rel}:1:1",
                    witness_trace=[{"file": rel, "skeleton": s_sids, "filled": f_sids}],
                )
            )

    for rel in skel_files:
        if rel not in fill_files:
            continue
        skel_text = (skeleton / rel).read_text(encoding="utf-8")
        fill_text = (filled / rel).read_text(encoding="utf-8")

        if check_sid_on_all_files:
            _check_sid_multiset(rel, skel_text, fill_text)

        has_seq_holes = bool(PLACEHOLDER_RE.search(skel_text))
        has_type_holes = bool(TYPE_HOLE_PLACEHOLDER_RE.search(skel_text))

        if has_type_holes:
            _th_ok, extracted_th, th_errs = align_filled_to_type_hole_skeleton(
                skel_text, fill_text, fills
            )
            for err in th_errs:
                skeleton_failed = True
                violations.append(
                    StaticViolation(
                        plugin=plugin_name,
                        obligation="O_skeleton_integrity",
                        kind=err["kind"],
                        message=f"{rel}: {err['message']}",
                        rust_span=f"{rel}:{err['line']}:1",
                    )
                )
            th_to_scan = list(extracted_th)
            if not th_to_scan and TYPE_HOLE_MARKER_RE.search(fill_text):
                for m in TYPE_HOLE_MARKER_RE.finditer(fill_text):
                    s, e = offset_to_line_range(fill_text, m.start(), m.end())
                    th_to_scan.append({
                        "id": m.group("id"),
                        "body": m.group(0),
                        "start_line": s,
                        "end_line": e,
                    })
            for item in th_to_scan:
                th_id = item["id"]
                type_holes_checked.append(th_id)
                raw_body = item["body"]
                start_line = int(item["start_line"])
                meta = type_hole_meta.get(th_id, {})
                reqs = [str(r) for r in (meta.get("req") or [])]

                inner_body = unwrap_type_hole_markers(th_id, raw_body)
                leading_offset = 0
                begin_prefix = f"/* TYPE_HOLE_BEGIN({th_id}) */"
                if raw_body.startswith(begin_prefix):
                    leading_offset = raw_body[:len(begin_prefix)].count("\n")

                if SID_ANNOTATION_RE.search(inner_body):
                    hole_failed = True
                    violations.append(
                        StaticViolation(
                            plugin=plugin_name,
                            obligation="O1_purity",
                            kind="cir_annotation_in_type_hole",
                            message=f"type hole '{th_id}' must not contain `// @cir` annotations",
                            hole_id=th_id,
                            req_clauses=reqs,
                            rust_span=f"{rel}:{start_line}:1",
                        )
                    )

                stripped = strip_rust_comments_and_literals(inner_body)
                for kind, desc, pattern in _TYPE_HOLE_FORBIDDEN_RULES:
                    for m in pattern.finditer(stripped):
                        rel_line = stripped.count("\n", 0, m.start())
                        if kind == "loop_in_type_hole" and _is_explicitly_bounded_loop(inner_body, rel_line):
                            continue
                        hole_failed = True
                        line_no = start_line + leading_offset + rel_line
                        last_nl = stripped.rfind("\n", 0, m.start())
                        col_no = (m.start() - last_nl) if last_nl >= 0 else (m.start() + 1)
                        token = m.group(0).strip()
                        violations.append(
                            StaticViolation(
                                plugin=plugin_name,
                                obligation="O1_purity",
                                kind=kind,
                                message=f"{desc}: found `{token}` in type hole '{th_id}' ({rel})",
                                hole_id=th_id,
                                req_clauses=reqs,
                                rust_span=f"{rel}:{line_no}:{col_no}",
                                witness_trace=[{"inner_body": inner_body.strip()}],
                            )
                        )
            continue

        if has_seq_holes:
            _skel_ok, extracted_holes, structural_errors = align_filled_to_skeleton(
                skel_text, fill_text, fills
            )
            for err in structural_errors:
                skeleton_failed = True
                violations.append(
                    StaticViolation(
                        plugin=plugin_name,
                        obligation="O_skeleton_integrity",
                        kind=err["kind"],
                        message=err["message"],
                        rust_span=f"{rel}:{err['line']}:1",
                    )
                )
            if not check_sid_on_all_files:
                _check_sid_multiset(rel, skel_text, fill_text)

            holes_to_scan = list(extracted_holes)
            if not holes_to_scan and HOLE_MARKER_RE.search(fill_text):
                for m in HOLE_MARKER_RE.finditer(fill_text):
                    s, e = offset_to_line_range(fill_text, m.start(), m.end())
                    holes_to_scan.append({
                        "id": m.group("id"),
                        "body": m.group(0),
                        "start_line": s,
                        "end_line": e,
                    })

            for item in holes_to_scan:
                hid = item["id"]
                holes_checked.append(hid)
                raw_body = item["body"]
                start_line = int(item["start_line"])
                meta = hole_meta.get(hid, {})
                sid = str(meta["sid"]) if meta.get("sid") is not None else None
                reqs = [str(r) for r in (meta.get("req") or [])]

                inner_body = unwrap_hole_markers(hid, raw_body)

                if SID_ANNOTATION_RE.search(inner_body):
                    hole_failed = True
                    violations.append(
                        StaticViolation(
                            plugin=plugin_name,
                            obligation="O1_purity",
                            kind="cir_annotation_in_hole",
                            message=f"hole '{hid}' must not contain `// @cir` annotations",
                            hole_id=hid,
                            sid=sid,
                            req_clauses=reqs,
                            rust_span=f"{rel}:{start_line}:1",
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
                                plugin=plugin_name,
                                obligation="O1_purity",
                                kind=kind,
                                message=f"{desc}: found `{token}` in hole '{hid}'",
                                hole_id=hid,
                                sid=sid,
                                req_clauses=reqs,
                                rust_span=f"{rel}:{line_no}:{col_no}",
                                witness_trace=[{"inner_body": inner_body.strip()}],
                            )
                        )
            continue

        if fill_text != skel_text:
            skeleton_failed = True
            diff_line = first_diff_line(skel_text, fill_text)
            violations.append(
                StaticViolation(
                    plugin=plugin_name,
                    obligation="O_skeleton_integrity",
                    kind="changed_outside_hole",
                    message=f"{rel} modified outside holes near line {diff_line}",
                    rust_span=f"{rel}:{diff_line}:1",
                )
            )

    return skeleton_failed, hole_failed, violations, holes_checked, type_holes_checked, skel_files


def lint_filled(skeleton_dir: Path | str, filled_dir: Path | str) -> dict[str, Any]:
    """Multi-file, multi-line structural + purity diff across ``src/*.rs`` and ``TypeHole``s."""
    skel_root = Path(skeleton_dir).expanduser().resolve()
    fill_root = Path(filled_dir).expanduser().resolve()
    _skel_fail, _hole_fail, raw_violations, _hc, _thc, _sf = scan_filled_crate(
        skel_root,
        fill_root,
        check_sid_on_all_files=True,
    )

    violations: list[dict[str, Any]] = []
    seen_forbidden_holes: set[tuple[str, str]] = set()
    for v in raw_violations:
        rel_file = (v.rust_span or "").split(":", 1)[0]
        line_no: int | None = None
        if v.rust_span and ":" in v.rust_span:
            parts = v.rust_span.split(":")
            if len(parts) >= 2 and parts[1].isdigit():
                line_no = int(parts[1])

        if v.kind == "sid_count":
            trace = v.witness_trace[0] if v.witness_trace else {}
            violations.append({
                "kind": "sid_count",
                "file": trace.get("file", rel_file),
                "detail": {
                    "skeleton": trace.get("skeleton", {}),
                    "filled": trace.get("filled", {}),
                },
            })
        elif v.obligation == "O1_purity":
            key = (rel_file, v.hole_id or "")
            if key in seen_forbidden_holes and v.kind not in (
                "cir_annotation_in_hole",
                "cir_annotation_in_type_hole",
            ):
                continue
            seen_forbidden_holes.add(key)
            detail = (
                v.witness_trace[0].get("inner_body", v.message)
                if v.witness_trace
                else v.message
            )
            violations.append({
                "kind": "forbidden_in_hole",
                "file": rel_file,
                "hole_id": v.hole_id,
                "line": line_no,
                "detail": detail,
            })
        else:
            violations.append({
                "kind": "changed_outside_hole",
                "file": rel_file,
                "line": line_no or 1,
                "detail": v.message,
            })

    return {"ok": not violations, "violations": violations}


class AstHolePurityPlugin:
    """Verify skeleton integrity and comment/string-safe purity across all ``src/*.rs`` files."""

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
        del spec_path, contract_path
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

        out_map = (
            Path(codegen_map_path).expanduser().resolve()
            if codegen_map_path is not None
            else filled / "codegen_filled.json"
        )
        augmented = augment_codegen_map_with_hole_ranges(
            skeleton, filled, fills, out_path=out_map
        )
        hole_ranges: dict[str, dict[str, Any]] = augmented.get("hole_ranges", {}) or {}
        type_hole_ranges: dict[str, dict[str, Any]] = augmented.get("type_hole_ranges", {}) or {}
        hole_meta = hole_meta_index(augmented)
        type_hole_meta = type_hole_meta_index(augmented)

        skeleton_failed, hole_failed, violations, holes_checked, type_holes_checked, skel_files = (
            scan_filled_crate(
                skeleton,
                filled,
                fills=fills,
                hole_meta=hole_meta,
                type_hole_meta=type_hole_meta,
                plugin_name=self.name,
                check_sid_on_all_files=False,
            )
        )
        for v in violations:
            if v.kind == "sid_count":
                v.rust_span = None
            v.witness_trace = []

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
                "type_hole_ranges": type_hole_ranges,
                "holes_checked": holes_checked,
                "type_holes_checked": type_holes_checked,
                "files_checked": skel_files,
            },
        )
