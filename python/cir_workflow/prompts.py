"""Prompt loading and structured Rust verification feedback."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
GENERATION_PROMPT_PATH = ROOT_DIR / "src" / "generation_nl_prompt.md"
REPAIR_PROMPT_PATH = ROOT_DIR / "src" / "repair" / "cir_schema_prompt.md"


def generation_system_prompt() -> str:
    return GENERATION_PROMPT_PATH.read_text()


def repair_system_prompt() -> str:
    return REPAIR_PROMPT_PATH.read_text()


def generation_user_prompt(requirements: str) -> str:
    return (
        "Model the following concurrent system as a complete CIR JSON object.\n\n"
        "## Requirements\n\n"
        f"{requirements}\n"
        "Output only the JSON object."
    )


def verification_feedback(payload: dict[str, Any] | None, fallback: str = "") -> str:
    """Convert the Rust result into stable, useful repair feedback."""

    if not payload:
        return fallback or "Rust verification did not return a structured result."

    sections: list[str] = [f"Verification status: {payload.get('status', 'unknown')}"]
    validation = payload.get("validation")
    if isinstance(validation, dict) and "diagnostics" in validation:
        diagnostics = validation["diagnostics"]
    else:
        diagnostics = payload.get("diagnostics", [])
    if diagnostics:
        sections.append("Static diagnostics:\n" + _diagnostics(diagnostics))

    for title, key in (
        ("Translation errors", "translation_errors"),
        ("Translation warnings", "translation_warnings"),
        ("Goal warnings", "goal_warnings"),
    ):
        values = payload.get(key, [])
        if values:
            sections.append(title + ":\n" + "\n".join(f"- {value}" for value in values))

    if payload.get("analysis_error"):
        sections.append(f"Analysis error: {payload['analysis_error']}")

    bugs = payload.get("bugs", [])
    if bugs:
        rendered = []
        for index, bug in enumerate(bugs, 1):
            rendered.append(f"### Bug {index}\n{_render_bug(bug)}")
        sections.append("Detected bugs:\n" + "\n\n".join(rendered))

    unmet = payload.get("unmet_goals", [])
    if unmet:
        lines = []
        for item in unmet:
            goal = item.get("goal", {})
            lines.append(
                f"- {goal.get('id', '?')}: {goal.get('desc', '')}"
                f" ({item.get('reason', 'unreachable')})"
            )
        sections.append("Unmet business goals:\n" + "\n".join(lines))

    if fallback:
        sections.append(fallback)
    return "\n\n".join(sections)


def _diagnostics(diagnostics: list[dict[str, Any]]) -> str:
    lines = []
    for diagnostic in diagnostics:
        path = f" [{diagnostic['path']}]" if diagnostic.get("path") else ""
        hint = f" Fix: {diagnostic['fix_hint']}" if diagnostic.get("fix_hint") else ""
        lines.append(
            f"- {diagnostic.get('code', '?')}{path}: {diagnostic.get('message', '')}{hint}"
        )
    return "\n".join(lines)


def _render_bug(bug: dict[str, Any]) -> str:
    kind = bug.get("kind", {})
    if isinstance(kind, dict) and kind:
        bug_kind = next(iter(kind))
    else:
        bug_kind = str(kind or "Unknown")
    lines = [f"Kind: {bug_kind}", f"Summary: {bug.get('summary', '')}"]
    if bug.get("final_marking_summary"):
        lines.append(f"Final marking: {bug['final_marking_summary']}")
    if bug.get("involved_resources"):
        lines.append("Resources: " + ", ".join(bug["involved_resources"]))
    if bug.get("involved_functions"):
        lines.append("Functions: " + ", ".join(bug["involved_functions"]))
    if bug.get("trace"):
        lines.append("Witness trace:\n" + "\n".join(
            f"  {step.get('description', step.get('transition_id', '?'))}"
            for step in bug["trace"]
        ))
    if bug.get("cir_slice"):
        lines.append("Relevant CIR slice:\n" + "\n".join(
            f"  {item.get('function', '?')}.{item.get('sid', '?')}: {item.get('op', '')}"
            for item in bug["cir_slice"]
        ))
    if bug.get("preservation_constraints"):
        lines.append("Preservation constraints:\n" + "\n".join(
            f"  - {item}" for item in bug["preservation_constraints"]
        ))
    if bug.get("repair_hint"):
        lines.append(f"Repair hint: {bug['repair_hint']}")
    return "\n".join(lines)


def repair_user_prompt(cir_json: str, feedback: str) -> str:
    return (
        "# CIR Verification Repair Request\n\n"
        "Repair the complete CIR JSON using the Rust verification feedback below.\n\n"
        "## Verification Feedback\n\n"
        f"{feedback}\n\n"
        "## Current CIR\n\n"
        f"```json\n{cir_json}\n```\n\n"
        "Output the complete revised CIR JSON only. Preserve resources, functions, "
        "protection entries, and business goal ids unless a change is required."
    )
