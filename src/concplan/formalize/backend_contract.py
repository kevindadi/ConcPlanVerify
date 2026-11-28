"""Thin Python bindings for Rust SSoT contract validation and explanation."""

from __future__ import annotations

import copy
import json
import tempfile
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient
from ..core.toolchain import find_concir_backend
from .models import (
    ContractExplanation,
    SanityCheckReport,
    SanityIssue,
    StructuredRequirements,
)

# Schema vocabulary constants matching backends/concir + unipn::analysis::temporal.
VALID_PROPERTY_KINDS = frozenset({
    "safety",
    "deadlock_free",
    "reachability",
    "always_reachable",
    "unreachable",
    "ctl",
    "ltl",
    "pattern",
    "health",
})
VALID_PATTERN_KINDS = frozenset({
    "always",
    "exclude",
    "at_most",
    "precedence",
    "triggers",
    "can_reach",
    "always_can_reach",
})
VALID_HEALTH_CHECKS = frozenset({
    "deadlock",
    "partial_deadlock",
    "lost_wakeup",
    "wake_starvation",
    "acquire_starvation",
    "resource_leak",
    "double_lock",
    "data_race",
})
VALID_PRESERVED_KINDS = frozenset({"reachable", "always"})
VALID_PREDICATE_KINDS = frozenset({
    "true",
    "false",
    "var_eq",
    "var_cmp",
    "function_completed",
    "function_completed_at_least",
    "scope_completed",
    "statement_reached",
    "milestone_reached",
    "mutex_free",
    "mutex_held",
    "holds_all",
    "mutex_exclusive",
    "never_holds_all",
    "channel_empty",
    "channel_at_least",
    "not",
    "and",
    "or",
})
VALID_CTL_OPS = frozenset({
    "true",
    "false",
    "atom",
    "not",
    "and",
    "or",
    "implies",
    "ex",
    "ax",
    "ef",
    "af",
    "eg",
    "ag",
    "eu",
    "au",
})
VALID_LTL_OPS = frozenset({
    "true",
    "false",
    "atom",
    "not",
    "and",
    "or",
    "implies",
    "next",
    "eventually",
    "always",
    "until",
    "release",
})
VALID_FAIRNESS_KINDS = frozenset({"justice", "compassion"})
VALID_CMP_OPS = frozenset({"==", "!=", "<", "<=", ">", ">="})

_DEFAULT_WORKDIR = Path(tempfile.gettempdir()) / "concplan_contract_ssot"
_CLIENT_CACHE: dict[str, ConcirClient] = {}


def _resolve_client(client: ConcirClient | None = None) -> ConcirClient:
    if client is not None:
        return client
    binary = find_concir_backend()
    if binary is None:
        raise FileNotFoundError(
            "concir-backend binary not found; build backends/concir "
            "(`cargo build --release --bin concir-backend`) or set CONCIR_BACKEND"
        )
    key = str(binary)
    cached = _CLIENT_CACHE.get(key)
    if cached is None:
        cached = ConcirClient(binary, workdir=_DEFAULT_WORKDIR, timeout=30.0)
        _CLIENT_CACHE[key] = cached
    return cached


def _req_payload(requirements: StructuredRequirements | dict[str, Any] | None) -> dict[str, Any] | None:
    if requirements is None:
        return None
    if isinstance(requirements, StructuredRequirements):
        return requirements.as_dict()
    return requirements


def _normalize_contract_for_explain(contract: dict[str, Any]) -> dict[str, Any]:
    out = copy.deepcopy(contract)
    df_flag = out.pop("deadlock_free", None)
    props = out.get("properties")
    if not isinstance(props, list):
        props = []
        out["properties"] = props
    if df_flag is True and not any(isinstance(p, dict) and p.get("kind") == "deadlock_free" for p in props):
        props.insert(0, {"id": "deadlock_free", "kind": "deadlock_free", "req": []})
    return out


def validate_contract_sanity(
    contract: dict[str, Any],
    requirements: StructuredRequirements | dict[str, Any] | None = None,
    *,
    strict_entities: bool = True,
    require_non_vacuous: bool = True,
    client: ConcirClient | None = None,
) -> SanityCheckReport:
    """Delegate schema, grounding, clause verifiability, and vacuity validation to ``concir-backend``."""
    resolved = _resolve_client(client)
    res = resolved.validate_contract(
        contract,
        requirements=_req_payload(requirements),
        strict_entities=strict_entities,
        require_non_vacuous=require_non_vacuous,
        normalize=False,
    )
    if not res.is_semantic or not isinstance(res.payload, dict):
        raise RuntimeError(res.error or res.stderr or f"validate-contract failed: {res.status}")
    payload = res.payload
    issues = [
        SanityIssue(
            severity=str(item.get("severity") or "error"),
            code=str(item.get("code") or "validation_error"),
            message=str(item.get("message") or ""),
            property_id=item.get("property_id"),
            clause_id=item.get("clause_id"),
        )
        for item in (payload.get("issues") or [])
        if isinstance(item, dict)
    ]
    return SanityCheckReport(
        ok=bool(payload.get("ok")),
        issues=issues,
        checked_properties=int(payload.get("checked_properties", 0)),
        checked_preserved=int(payload.get("checked_preserved", 0)),
    )


def ensure_implication_non_vacuous(
    contract: dict[str, Any],
    *,
    client: ConcirClient | None = None,
) -> dict[str, Any]:
    """Normalize CTL/LTL implications and trigger/precedence patterns via ``concir-backend``."""
    resolved = _resolve_client(client)
    res = resolved.validate_contract(
        contract,
        requirements=None,
        strict_entities=False,
        require_non_vacuous=False,
        normalize=True,
    )
    if not res.is_semantic or not isinstance(res.payload, dict):
        raise RuntimeError(res.error or res.stderr or f"validate-contract --normalize failed: {res.status}")
    normalized = res.payload.get("normalized_contract")
    return normalized if isinstance(normalized, dict) else copy.deepcopy(contract)


def formula_contains_implication(
    formula: Any,
    *,
    client: ConcirClient | None = None,
) -> bool:
    """Return True iff a CTL or LTL AST contains an ``implies`` node (Rust SSoT)."""
    if not isinstance(formula, dict):
        return False
    probe = {
        "name": "_probe_implication",
        "properties": [{"id": "probe", "kind": "ctl", "formula": formula}],
    }
    normalized = ensure_implication_non_vacuous(probe, client=client)
    props = normalized.get("properties") or []
    return bool(props and isinstance(props[0], dict) and props[0].get("non_vacuous") is True)


def explain_contract_data(
    contract: dict[str, Any],
    *,
    client: ConcirClient | None = None,
    prefer_backend: bool = True,
) -> ContractExplanation:
    """Deterministically back-translate a contract JSON via ``concir-backend explain-contract``."""
    del prefer_backend  # Rust SSoT is always used cleanly via ConcirClient.
    resolved = _resolve_client(client)
    norm_contract = _normalize_contract_for_explain(contract)
    res = resolved.explain_contract(norm_contract)
    if res.is_semantic and res.status == "pass" and isinstance(res.payload, dict):
        payload = res.payload
        return ContractExplanation(
            name=str(payload.get("name") or contract.get("name") or "contract"),
            properties=list(payload.get("properties") or []),
            preserved=list(payload.get("preserved") or []),
            schema=str(payload.get("schema") or "concir-contract-explanation/1"),
            source="concir-backend",
        )
    raise RuntimeError(res.error or res.stderr or f"explain-contract failed: {res.status}")


def _explain_single_property_safe(prop: dict[str, Any], *, client: ConcirClient | None = None) -> dict[str, Any]:
    resolved = _resolve_client(client)
    res = resolved.explain_contract({"name": "probe", "properties": [prop]})
    if res.is_semantic and res.status == "pass" and isinstance(res.payload, dict):
        props = res.payload.get("properties") or []
        if props and isinstance(props[0], dict):
            return props[0]
    detail = res.stderr or res.error or res.status
    return {
        "id": str(prop.get("id") or ""),
        "kind": str(prop.get("kind") or ""),
        "req": list(prop.get("req") or []),
        "non_vacuous": bool(prop.get("non_vacuous", False)),
        "explanation": f"<unknown or invalid property: {detail}>",
    }


def explain_property(prop: dict[str, Any], *, client: ConcirClient | None = None) -> dict[str, Any]:
    """Render one ``PropertySpec`` dict via ``concir-backend explain-contract``."""
    return _explain_single_property_safe(prop, client=client)


def explain_preserved(pres: dict[str, Any], *, client: ConcirClient | None = None) -> dict[str, Any]:
    """Render one ``PreservedSpec`` dict via ``concir-backend explain-contract``."""
    resolved = _resolve_client(client)
    res = resolved.explain_contract({"name": "probe", "properties": [], "preserved": [pres]})
    if res.is_semantic and res.status == "pass" and isinstance(res.payload, dict):
        items = res.payload.get("preserved") or []
        if items and isinstance(items[0], dict):
            return items[0]
    detail = res.stderr or res.error or res.status
    return {
        "description": str(pres.get("description") or pres.get("id") or pres.get("kind") or "preserved"),
        "req": list(pres.get("req") or []),
        "explanation": f"<unknown or invalid preserved: {detail}>",
    }


def explain_predicate(pred: Any, *, client: ConcirClient | None = None) -> str:
    """Deterministically render any ``PredicateSpec`` JSON via ``concir-backend``."""
    if not isinstance(pred, dict):
        return f"<invalid predicate: {pred!r}>"
    entry = _explain_single_property_safe({"id": "p", "kind": "safety", "invariant": pred}, client=client)
    text = str(entry.get("explanation") or "")
    prefix = "In every reachable state, "
    if text.startswith(prefix):
        text = text[len(prefix):].removesuffix(".")
    return text


def explain_ctl_formula(formula: Any, *, client: ConcirClient | None = None) -> str:
    """Deterministically render any ``CtlFormula`` JSON via ``concir-backend``."""
    if not isinstance(formula, dict):
        return f"<invalid CTL formula: {formula!r}>"
    entry = _explain_single_property_safe({"id": "p", "kind": "ctl", "formula": formula}, client=client)
    return str(entry.get("explanation") or "")


def explain_ltl_formula(formula: Any, *, client: ConcirClient | None = None) -> str:
    """Deterministically render any ``LtlFormula`` JSON via ``concir-backend``."""
    if not isinstance(formula, dict):
        return f"<invalid LTL formula: {formula!r}>"
    entry = _explain_single_property_safe({"id": "p", "kind": "ltl", "formula": formula}, client=client)
    return str(entry.get("explanation") or "")


def explain_fairness(fc: Any, *, client: ConcirClient | None = None) -> str:
    """Deterministically render a ``FairnessConstraint`` JSON via ``concir-backend``."""
    if not isinstance(fc, dict):
        return f"<invalid fairness constraint: {fc!r}>"
    entry = _explain_single_property_safe(
        {"id": "p", "kind": "ltl", "formula": {"op": "true"}, "fairness": [fc]},
        client=client,
    )
    text = str(entry.get("explanation") or "")
    prefix = "true (under fairness: "
    if text.startswith(prefix) and text.endswith(")"):
        return text[len(prefix):-1]
    return text or json.dumps(fc, ensure_ascii=False)
