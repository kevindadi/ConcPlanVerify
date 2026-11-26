"""Prompt assets and structured verification feedback builders.

Active v4 prompt assets are bundled in the ``concplan.prompts`` package and
loaded via ``importlib.resources.files("concplan.prompts")``:
- ``scir_generation.md``      — SCIR v4.0 / CIR generation prompt
- ``hole_repair.md``          — sequential hole-filling, Loop B repair, and Loop C escalation
- ``cir_patch.md``            — constrained single-patch repair prompt
- ``cir_local_revision.md``   — function-scoped local CIR revision prompt
"""

from __future__ import annotations

from importlib import resources
import json
from typing import Any

from .json_utils import sha256_bytes

SCIR_GENERATION_ASSET = "scir_generation.md"
HOLE_REPAIR_ASSET = "hole_repair.md"
PATCH_ASSET = "cir_patch.md"
LOCAL_REVISION_ASSET = "cir_local_revision.md"

ACTIVE_PROMPT_ASSETS = (
    SCIR_GENERATION_ASSET,
    HOLE_REPAIR_ASSET,
    PATCH_ASSET,
    LOCAL_REVISION_ASSET,
)


def _prompt_files():
    return resources.files("concplan.prompts")


def _read(name: str) -> str:
    return _prompt_files().joinpath(name).read_text(encoding="utf-8")


def prompt_asset_record() -> dict[str, str]:
    """Return ``{filename: sha256}`` for all 4 active v4 prompt assets."""
    files = _prompt_files()
    out: dict[str, str] = {}
    for name in ACTIVE_PROMPT_ASSETS:
        data = files.joinpath(name).read_bytes()
        out[name] = sha256_bytes(data)
    return out


def scir_generation_system_prompt() -> str:
    """SCIR v4.0 generation prompt with lock_guard, while_wait, seq_hole, and req tags."""
    return _read(SCIR_GENERATION_ASSET)


def hole_repair_system_prompt() -> str:
    """Hole-filling, Loop B hole-local repair, and Loop C escalation prompt."""
    return _read(HOLE_REPAIR_ASSET)


def patch_system_prompt() -> str:
    """Constrained single-patch repair prompt."""
    return _read(PATCH_ASSET)


def local_revision_system_prompt() -> str:
    """Function-scoped local CIR revision prompt."""
    return _read(LOCAL_REVISION_ASSET)


# Clean aliases for convenience
generation_system_prompt = scir_generation_system_prompt


def requirements_only_user_prompt(
    requirements: str,
    *,
    previous_candidate: str | None = None,
    feedback: str | None = None,
) -> str:
    """Generation user prompt that never contains the verification contract."""
    parts = [
        "Produce one ConcIR / SCIR program for the requirements below. No verification "
        "contract is provided; the requirements are the whole input.",
        "",
        "<domain_requirements>",
        requirements.strip(),
        "</domain_requirements>",
    ]
    if previous_candidate:
        parts += ["", "<previous_candidate>", previous_candidate, "</previous_candidate>"]
    if feedback:
        parts += [
            "",
            "<verification_feedback>",
            feedback,
            "</verification_feedback>",
            "Output the complete corrected JSON object.",
        ]
    parts += ["", "Output only the JSON object."]
    return "\n".join(parts)


def patch_user_prompt(
    context: dict[str, Any],
    *,
    feedback: str | None = None,
    previous_candidate: str | None = None,
) -> str:
    sections = [
        "Propose exactly one constrained patch for the frozen model below.",
        "The repair context (fingerprints, allowed scope, root verification and "
        "diagnostics, functions with lock sids and original hashes) is authoritative.",
        "<repair_context>",
        json.dumps(context, ensure_ascii=False),
        "</repair_context>",
    ]
    if previous_candidate is not None:
        sections += [
            "<previous_candidate>",
            previous_candidate,
            "</previous_candidate>",
        ]
    if feedback is not None:
        sections += [
            "<rejection_feedback>",
            feedback,
            "</rejection_feedback>",
            "Propose a different single adjacent mutex_lock swap that fixes the "
            "full contract without changing the contract or the program structure.",
        ]
    sections.append("Output only the JSON object.")
    return "\n".join(sections)


def render_patch_feedback(payload: dict[str, Any]) -> str:
    """Compact, structured feedback from an evaluate-patch artifact."""
    verification = payload.get("verification") or {}
    properties = verification.get("properties") or []
    failed = [
        {"id": p.get("id"), "outcome": p.get("outcome")}
        for p in properties
        if p.get("outcome") not in (None, "PASS")
    ]
    diagnostics = []
    for d in verification.get("diagnostics") or []:
        diagnostics.append({
            "property": d.get("property"),
            "outcome": d.get("outcome"),
            "message": d.get("message"),
            "blocked": d.get("blocked"),
            "counterexample": d.get("counterexample"),
        })
    return json.dumps({
        "stage": "evaluate-patch",
        "status": payload.get("status"),
        "reject": payload.get("reject_reason"),
        "static_valid": payload.get("static_valid"),
        "supported": payload.get("supported"),
        "verification_outcome": verification.get("outcome"),
        "verification_complete": verification.get("complete"),
        "failed_properties": failed,
        "preserved_unmet": [
            f for f in failed if str(f.get("id", "")).startswith("preserved:")
        ],
        "diagnostics": diagnostics,
    }, ensure_ascii=False)


def generation_user_prompt(requirements: str, contract: dict[str, Any]) -> str:
    return (
        "Produce one ConcIR / SCIR program for the requirements below. The contract is "
        "frozen and supplied by the caller; do not modify it.\n\n"
        "<domain_requirements>\n"
        f"{requirements.strip()}\n"
        "</domain_requirements>\n\n"
        "<contract>\n"
        f"{json.dumps(contract, ensure_ascii=False, indent=2)}\n"
        "</contract>\n\n"
        "Output only the JSON object."
    )


def retry_user_prompt(
    requirements: str,
    contract: dict[str, Any],
    *,
    previous_candidate: str,
    feedback: dict[str, Any],
) -> str:
    return (
        "Revise the ConcIR / SCIR candidate for the same requirements. The requirements "
        "and the frozen contract are authoritative; the feedback is repair "
        "context.\n\n"
        "<domain_requirements>\n"
        f"{requirements.strip()}\n"
        "</domain_requirements>\n\n"
        "<contract>\n"
        f"{json.dumps(contract, ensure_ascii=False, indent=2)}\n"
        "</contract>\n\n"
        "<previous_candidate>\n"
        f"{previous_candidate}\n"
        "</previous_candidate>\n\n"
        "<verification_feedback>\n"
        f"{json.dumps(feedback, ensure_ascii=False, indent=2)}\n"
        "</verification_feedback>\n\n"
        "Output only the revised JSON object."
    )


def llm_user_prompt_builder(request) -> str:
    """Adapter used by :class:`~concplan.core.providers.LlmCandidateProvider`."""
    if request.feedback is None:
        return generation_user_prompt(request.requirements, request.contract)
    return retry_user_prompt(
        request.requirements,
        request.contract,
        previous_candidate=request.previous_candidate or "",
        feedback={"rendered": request.feedback},
    )


def hole_user_prompt_builder(request) -> str:
    """Adapter for hole filling / repair where ``request.requirements`` already carries the hole prompt."""
    return request.requirements


def build_check_feedback(result) -> dict[str, Any]:
    payload = result.payload or {}
    diagnostics = payload.get("diagnostics", []) or []
    return {
        "stage": "check",
        "status": result.status,
        "valid": payload.get("valid"),
        "validation_diagnostics": diagnostics,
        "process_error": result.error if result.kind != "semantic" else None,
    }


def build_explore_feedback(
    result, *, preserved_ids: list[str] | None = None
) -> dict[str, Any]:
    del preserved_ids
    payload = result.payload or {}
    properties = payload.get("properties", []) or []
    failed_properties = [
        {"id": p.get("id"), "outcome": p.get("outcome"), "detail": p.get("detail")}
        for p in properties
        if p.get("outcome") not in (None, "PASS")
    ]
    preserved_unmet = [
        p
        for p in failed_properties
        if (p.get("id") or "").startswith("preserved:")
    ]
    diagnostics = []
    for d in payload.get("diagnostics", []) or []:
        diagnostics.append({
            "property": d.get("property"),
            "outcome": d.get("outcome"),
            "message": d.get("message"),
            "complete": d.get("complete"),
            "counterexample_names": d.get("counterexample_names"),
            "counterexample": d.get("counterexample"),
            "cir_statements": _named_cir_statements(d.get("cir_statements")),
            "blocked": _named_blocked(d.get("blocked")),
            "doom_state": d.get("doom_state"),
            "proven_facts": d.get("proven_facts"),
            "boundary_events": payload.get("boundary_events"),
            "repair_hints": d.get("repair_hints"),
        })
    return {
        "stage": "explore",
        "outcome": result.outcome,
        "complete": result.complete,
        "failed_properties": failed_properties,
        "preserved_unmet": preserved_unmet,
        "diagnostics": diagnostics,
        "unsupported": payload.get("unsupported"),
        "invalid": payload.get("invalid"),
        "note": (
            "UNKNOWN means the analysis did not complete; it is not a proof of "
            "safety. FAIL with complete=false may already contain a counterexample."
        ),
    }


def _named_cir_statements(statements: Any) -> list[dict[str, Any]]:
    """Keep the backend's named statements; a missing sid is unknown, not None."""
    out: list[dict[str, Any]] = []
    if isinstance(statements, list):
        for stmt in statements:
            if not isinstance(stmt, dict):
                continue
            sid = stmt.get("sid")
            out.append({
                "module": stmt.get("module"),
                "function": stmt.get("function"),
                "sid": sid if sid not in (None, "") else "<unknown>",
            })
    return out


def _named_blocked(blocked: Any) -> list[dict[str, Any]]:
    """Keep the backend's blocked entries with their resolved resource names."""
    out: list[dict[str, Any]] = []
    if isinstance(blocked, list):
        for b in blocked:
            if not isinstance(b, dict):
                continue
            out.append({
                "thread": b.get("thread"),
                "kind": b.get("kind"),
                "resource": b.get("resource_name") or b.get("resource"),
                "detail": b.get("detail"),
                "waiting_on": b.get("waiting"),
            })
    return out


def render_feedback(feedback: dict[str, Any]) -> str:
    return json.dumps(feedback, ensure_ascii=False, indent=2)
