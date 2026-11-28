"""Hole-fill reply parsing and Stage 3 / Loop B prompt builders."""

from __future__ import annotations

import json
from pathlib import Path

from ..code_verify.conformance import holes_of, type_holes_of
from ..code_verify.static_verify import StaticViolation
from ..core.json_utils import extract_json


def parse_hole_reply(text: str) -> tuple[dict[str, str] | None, dict[str, object] | None]:
    """Parse a hole-provider JSON reply into ``(fills, escalation)``."""
    extracted = extract_json(text)
    try:
        data = json.loads(extracted)
    except json.JSONDecodeError:
        return None, None
    if not isinstance(data, dict):
        return None, None

    if "escalate" in data and data["escalate"]:
        esc = data["escalate"]
        if isinstance(esc, dict):
            return None, {
                "reason": str(esc.get("reason") or "hole provider requested CIR escalation"),
                "module": esc.get("module"),
                "details": esc,
            }
        return None, {
            "reason": str(data.get("reason") or esc if isinstance(esc, str) else "hole provider requested CIR escalation"),
            "module": data.get("module"),
            "details": data,
        }

    return {str(k): str(v) for k, v in data.items()}, None


def _render_hole_snippets(
    parts: list[str],
    skeleton_dir: Path,
    entries: list[dict],
    *,
    marker_prefix: str,
    header_fn,
    default_rel: str,
    context_lines: int,
) -> None:
    source_cache: dict[str, list[str]] = {}

    def _read_lines(rel: str) -> list[str]:
        if rel not in source_cache:
            p = skeleton_dir / rel
            source_cache[rel] = p.read_text(encoding="utf-8").splitlines() if p.is_file() else []
        return source_cache[rel]

    for entry in entries:
        hid = entry["id"]
        rel = str(entry.get("file") or default_rel)
        src_lines = _read_lines(rel)
        idx = next((i for i, line in enumerate(src_lines) if f"{marker_prefix}({hid})" in line), None)
        if idx is None and isinstance(entry.get("line"), int):
            idx = max(0, min(len(src_lines) - 1, int(entry["line"]) - 1)) if src_lines else None
        parts.append(header_fn(entry, rel))
        if idx is not None and src_lines:
            lo = max(0, idx - context_lines)
            hi = min(len(src_lines), idx + context_lines + 1)
            parts.append("```rust")
            parts.extend(f"{n + 1:>4}: {src_lines[n]}" for n in range(lo, hi))
            parts.append("```")
        parts.append("")


def build_hole_fill_prompt(
    skeleton_dir: Path,
    requirements: str,
    *,
    hole_ids: list[str] | None = None,
    context_lines: int = 8,
) -> str:
    """Build a prompt to fill all (or selected) SeqHoles and TypeHoles, with escalation protocol."""
    all_holes = holes_of(skeleton_dir)
    all_type_holes = type_holes_of(skeleton_dir)
    selected_set = set(hole_ids) if hole_ids is not None else None

    selected_holes = (
        [h for h in all_holes if h.get("id") in selected_set]
        if selected_set is not None
        else all_holes
    )
    selected_type_holes = (
        [th for th in all_type_holes if th.get("id") in selected_set]
        if selected_set is not None
        else all_type_holes
    )

    parts = [
        "Fill ONLY the specified hole(s) (SeqHole in src/main.rs / src/<mod>.rs and/or TypeHole in src/domain.rs) in the generated Rust skeleton.",
        "Rules:",
        "- Return a JSON object mapping each hole id (`h*` / `th*`) to valid pure Rust code.",
        "- For SeqHoles: do NOT introduce synchronization, threads, channels, atomics, unsafe, `loop`/`while`, or `cir_trace`.",
        "- For TypeHoles (`src/domain.rs`): define a pure `pub struct <Name>` (with `Send + Sync + Clone + Default + 'static`, e.g. `#[derive(Clone, Debug, Default, PartialEq)]`) and pure `impl` methods ONLY; forbid Mutex/RwLock/Condvar/Semaphore/mpsc/Atomic*/thread::spawn/unsafe/cir_trace/unbounded loops.",
        "- If a hole CANNOT be implemented purely without changing the concurrency architecture "
        "(e.g. missing shared variable, lock, or channel in CIR), return an escalation JSON instead:",
        '  {"escalate": {"reason": "<why CIR must change>", "module": "<module_name>"}}',
        "",
        "<requirements>",
        requirements.strip(),
        "</requirements>",
        "",
    ]

    _render_hole_snippets(
        parts,
        skeleton_dir,
        selected_type_holes,
        marker_prefix="TYPE_HOLE",
        default_rel="src/domain.rs",
        context_lines=context_lines,
        header_fn=lambda th, rel: (
            f"### type_hole `{th['id']}` (type `{th.get('type_name', '')}`, file `{rel}`, "
            f"desc `{th.get('description', '')}`, req `{th.get('req', [])}`)"
        ),
    )
    _render_hole_snippets(
        parts,
        skeleton_dir,
        selected_holes,
        marker_prefix="HOLE",
        default_rel="src/main.rs",
        context_lines=context_lines,
        header_fn=lambda h, rel: (
            f"### hole `{h['id']}` (expected `{h.get('expected', '()')}`, file `{rel}`, "
            f"sid `{h.get('sid')}`, req `{h.get('req', [])}`)"
        ),
    )
    parts.append("Reply with ONLY the JSON object.")
    return "\n".join(parts)


def build_hole_repair_prompt(
    skeleton_dir: Path,
    requirements: str,
    offending_holes: list[str],
    current_fills: dict[str, str],
    violations: list[StaticViolation],
    *,
    context_lines: int = 8,
) -> str:
    """Build a Loop B repair prompt targeting ONLY the offending hole(s)."""
    base = build_hole_fill_prompt(
        skeleton_dir, requirements, hole_ids=offending_holes, context_lines=context_lines
    )
    relevant_violations = [
        v.as_dict() for v in violations if v.hole_id in offending_holes or v.hole_id is None
    ]
    current_subset = {hid: current_fills.get(hid, "") for hid in offending_holes}
    repair_block = [
        "",
        "<offending_holes>",
        json.dumps(offending_holes, ensure_ascii=False),
        "</offending_holes>",
        "",
        "<previous_hole_fills>",
        json.dumps(current_subset, ensure_ascii=False, indent=2),
        "</previous_hole_fills>",
        "",
        "<static_verification_violations>",
        json.dumps(relevant_violations, ensure_ascii=False, indent=2),
        "</static_verification_violations>",
        "",
        "Repair ONLY the listed offending hole(s). Return a JSON object containing ONLY "
        "those hole id(s), OR return "
        '{"escalate": {"reason": "...", "module": "..."}} if the CIR design itself must change.',
    ]
    return base + "\n" + "\n".join(repair_block)
