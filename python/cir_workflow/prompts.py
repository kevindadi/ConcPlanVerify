"""Prompt assets and structured verification feedback.

Assets are versioned files in ``prompt_assets/``; their sha256 is recorded so an
experiment knows exactly which prompt text produced a candidate. Feedback keeps
the backend's actual property ids, statement ids, counterexamples and preserved
failures; unknown outcomes and tool errors are never rewritten as "no defect".
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

PROMPT_ASSET_DIR = Path(__file__).resolve().parent / "prompt_assets"
GENERATION_ASSET = "concir_generation_v1.md"
FEEDBACK_ASSET = "concir_feedback_v1.md"
PATCH_ASSET = "concir_patch_v1.md"
GENERATION_PROMPT_VERSION = "concir-generation-v1"
FEEDBACK_PROMPT_VERSION = "concir-feedback-v1"
PATCH_PROMPT_VERSION = "concir-patch-v1"


def _read(name: str) -> str:
    return (PROMPT_ASSET_DIR / name).read_text(encoding="utf-8")


def prompt_asset_record() -> dict[str, str]:
    out = {}
    for name in (GENERATION_ASSET, FEEDBACK_ASSET, PATCH_ASSET):
        data = (PROMPT_ASSET_DIR / name).read_bytes()
        out[name] = hashlib.sha256(data).hexdigest()
    return out


def generation_system_prompt() -> str:
    return _read(GENERATION_ASSET)


def feedback_system_prompt() -> str:
    return _read(FEEDBACK_ASSET)


def patch_system_prompt() -> str:
    return _read(PATCH_ASSET)


def patch_user_prompt(context: dict[str, Any], *,
                      feedback: str | None = None,
                      previous_candidate: str | None = None) -> str:
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
    failed = [{"id": p.get("id"), "outcome": p.get("outcome")} for p in properties
              if p.get("outcome") not in (None, "PASS")]
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
        "preserved_unmet": [f for f in failed if str(f.get("id", "")).startswith("preserved:")],
        "diagnostics": diagnostics,
    }, ensure_ascii=False)


def generation_user_prompt(requirements: str, contract: dict[str, Any]) -> str:
    return (
        "Produce one ConcIR program for the requirements below. The contract is "
        "frozen and supplied by the caller; do not modify it.\n\n"
        "<domain_requirements>\n"
        f"{requirements.strip()}\n"
        "</domain_requirements>\n\n"
        "<contract>\n"
        f"{json.dumps(contract, ensure_ascii=False, indent=2)}\n"
        "</contract>\n\n"
        "Output only the JSON object."
    )


def retry_user_prompt(requirements: str, contract: dict[str, Any], *,
                      previous_candidate: str, feedback: dict[str, Any]) -> str:
    return (
        "Revise the ConcIR candidate for the same requirements. The requirements "
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
    """Adapter used by :class:`~cir_workflow.providers.LlmCandidateProvider`."""
    if request.feedback is None:
        return generation_user_prompt(request.requirements, request.contract)
    return retry_user_prompt(
        request.requirements,
        request.contract,
        previous_candidate=request.previous_candidate or "",
        feedback={"rendered": request.feedback},
    )


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


def build_explore_feedback(result, *, preserved_ids: list[str] | None = None) -> dict[str, Any]:
    payload = result.payload or {}
    properties = payload.get("properties", []) or []
    failed_properties = [
        {"id": p.get("id"), "outcome": p.get("outcome"), "detail": p.get("detail")}
        for p in properties
        if p.get("outcome") not in (None, "PASS")
    ]
    preserved_unmet = [
        p for p in failed_properties
        if (p.get("id") or "").startswith("preserved:")
    ]
    diagnostics = []
    for d in payload.get("diagnostics", []) or []:
        diagnostics.append({
            "property": d.get("property"),
            "outcome": d.get("outcome"),
            "message": d.get("message"),
            "complete": d.get("complete"),
            "related_functions": _functions_from_counterexample(d.get("counterexample")),
            "related_sids": _sids_from_cir_statements(d.get("cir_statements")),
            "counterexample": d.get("counterexample"),
            "blocked": d.get("blocked"),
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


def _functions_from_counterexample(counterexample: Any) -> list[str]:
    funcs: list[str] = []
    if isinstance(counterexample, list):
        for step in counterexample:
            origin = (step or {}).get("origin") if isinstance(step, dict) else None
            if isinstance(origin, dict):
                mod = origin.get("module")
                fun = origin.get("function")
                token = f"{mod}::{fun}"
                if token not in funcs:
                    funcs.append(token)
    return funcs


def _sids_from_cir_statements(statements: Any) -> list[str]:
    out: list[str] = []
    if isinstance(statements, list):
        for stmt in statements:
            if isinstance(stmt, dict):
                token = f"{stmt.get('module')}::{stmt.get('function')}.{stmt.get('sid')}"
                if token not in out:
                    out.append(token)
    return out


def render_feedback(feedback: dict[str, Any]) -> str:
    return json.dumps(feedback, ensure_ascii=False, indent=2)
