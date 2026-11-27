"""Stage 0 Autoformalization: Dual-Entry NL/Structured -> Contract + Deterministic Back-Translation.

Implements:
- **Entry A (`formalize_from_nl`)**:
  - Step 0.1 (`nl_to_clauses`): raw natural language -> grounded entities (`roles`,
    `resources`, `domain_types`) + atomic clauses `R1..Rn` classified into
    `safety | liveness | reachability | sequential_hole` (`verifiable: true/false`).
  - Step 0.2 (`clauses_to_contract`): **strictly isolated** contract synthesis
    (never receives any candidate SCIR/CIR program to prevent specification-model
    collusion!), supporting classic `PropertySpec` kinds as well as nested
    CTL (`kind: "ctl"`) and LTL (`kind: "ltl"`, `fairness`, `non_vacuous: true`)
    matching ``unipn::analysis::temporal``.
  - Step 0.3 (`explain_contract_data`, `validate_contract_sanity`): deterministic
    NL back-translation and structural/grounding/non-vacuity sanity checks.
- **Entry B (`formalize_from_structured`)**:
  - Accepts existing `R1..Rn` markdown, `requirements.json`, or `StructuredRequirements`,
    plus optional existing `contract.json`. Synthesizes `contract.json` via Step 0.2
    when omitted; always runs Step 0.3 explanation and sanity validation.
"""

from __future__ import annotations

import copy
import json
import re
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient
from ..core.json_utils import extract_json, write_json
from ..core.providers import CandidateProvider, CandidateRequest
from .models import (
    ContractExplanation,
    DomainTypeEntity,
    FormalizationResult,
    GroundedEntities,
    RequirementClause,
    SanityCheckReport,
    SanityIssue,
    StructuredRequirements,
)

# Valid schema vocabularies matching backends/concir + backends/unipn::analysis::temporal.
VALID_PROPERTY_KINDS = frozenset({
    "safety",
    "deadlock_free",
    "reachability",
    "always_reachable",
    "unreachable",
    "ctl",
    "ltl",
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

_MD_CLAUSE_RE = re.compile(
    r"^\s*(?:[-*]\s*)?(?:\*\*)?(R\d+)[.:]?(?:\*\*)?\s*"
    r"(?:(\[V\]|\[U\])\s*)?"
    r"(?:\((safety|liveness|reachability|sequential_hole)\)\s*)?"
    r"(.+?)\s*$",
    re.IGNORECASE,
)
_MD_TITLE_RE = re.compile(r"^\s*#\s+(.+?)\s*$")
_BACKTICK_RE = re.compile(r"`([^`]+)`")


# ---------------------------------------------------------------------------
# Parsing pre-authored structured documents (Entry B)
# ---------------------------------------------------------------------------

def parse_structured_markdown(text: str, *, default_title: str = "requirements") -> StructuredRequirements:
    """Parse a structured ``REQUIREMENTS.md`` document with ``R1..Rn`` clauses and optional Entities."""
    lines = text.splitlines()
    title = default_title
    roles: list[str] = []
    resources: list[str] = []
    domain_types: list[DomainTypeEntity] = []
    clauses: list[RequirementClause] = []

    in_entities = False
    for raw_line in lines:
        line = raw_line.strip()
        if not line:
            continue
        m_title = _MD_TITLE_RE.match(line)
        if m_title and title == default_title:
            title = m_title.group(1).strip()
            continue
        if line.startswith("##"):
            heading = line.lstrip("#").strip().lower()
            in_entities = "entit" in heading
            continue
        if in_entities:
            lower = line.lower()
            tokens = [t.strip() for t in _BACKTICK_RE.findall(line) if t.strip()]
            if any(k in lower for k in ("role", "function", "worker", "actor", "thread")):
                roles.extend(tokens)
                continue
            if any(k in lower for k in ("resource", "mutex", "lock", "channel", "semaphore", "shared", "var")):
                resources.extend(tokens)
                continue
            if any(k in lower for k in ("domain", "type", "struct", "payload")):
                for tok in tokens:
                    domain_types.append(DomainTypeEntity(name=tok, kind="hole"))
                continue

        m_clause = _MD_CLAUSE_RE.match(raw_line)
        if m_clause:
            cid = m_clause.group(1).upper()
            v_flag = (m_clause.group(2) or "").upper()
            cat = (m_clause.group(3) or "").lower()
            body = m_clause.group(4).strip()
            if v_flag == "[U]" or cat == "sequential_hole":
                verifiable = False
                cat = cat or "sequential_hole"
            else:
                verifiable = True
                if not cat:
                    cat = _infer_clause_category(body)
            clauses.append(
                RequirementClause(
                    id=cid,
                    text=body,
                    category=cat,
                    verifiable=verifiable,
                )
            )

    return StructuredRequirements(
        title=title,
        clauses=clauses,
        entities=GroundedEntities(
            roles=list(dict.fromkeys(roles)),
            resources=list(dict.fromkeys(resources)),
            domain_types=domain_types,
        ),
    )


def _infer_clause_category(text: str) -> str:
    lower = text.lower()
    if any(k in lower for k in ("eventually", "starvation", "liveness", "always responds", "fair")):
        return "liveness"
    if any(k in lower for k in ("reachable", "can complete", "completes", "terminates")):
        return "reachability"
    return "safety"


def parse_structured_requirements(
    source: StructuredRequirements | dict[str, Any] | Path | str,
    *,
    default_title: str = "requirements",
) -> StructuredRequirements:
    """Load structured requirements from an object, dict, JSON/MD file path, or raw text."""
    if isinstance(source, StructuredRequirements):
        return source
    if isinstance(source, dict):
        return StructuredRequirements.from_dict(source)
    if isinstance(source, Path):
        p = source.expanduser().resolve()
        text = p.read_text(encoding="utf-8")
        if p.suffix.lower() == ".json":
            return StructuredRequirements.from_dict(json.loads(text))
        return parse_structured_markdown(text, default_title=p.stem)
    if isinstance(source, str):
        stripped = source.strip()
        if "\n" not in stripped and len(stripped) < 4096:
            candidate = Path(stripped)
            if candidate.is_file():
                return parse_structured_requirements(candidate, default_title=default_title)
        if stripped.startswith("{"):
            return StructuredRequirements.from_dict(json.loads(stripped))
        return parse_structured_markdown(source, default_title=default_title)
    raise TypeError(f"unsupported structured requirements source: {type(source).__name__}")


# ---------------------------------------------------------------------------
# Step 0.3 — Deterministic Back-Translation (Predicate / CTL / LTL / Contract)
# ---------------------------------------------------------------------------

def explain_predicate(pred: Any) -> str:
    """Deterministically render any ConcIR ``PredicateSpec`` JSON into unambiguous English."""
    if not isinstance(pred, dict):
        return f"<invalid predicate: {pred!r}>"
    kind = str(pred.get("kind") or "").strip().lower()
    if kind == "true":
        return "true"
    if kind == "false":
        return "false"
    if kind == "var_eq":
        res = pred.get("resource", "?")
        val = json.dumps(pred.get("value"), ensure_ascii=False)
        return f"{res} == {val}"
    if kind == "var_cmp":
        res = pred.get("resource", "?")
        op = pred.get("op", "?")
        val = json.dumps(pred.get("value"), ensure_ascii=False)
        return f"{res} {op} {val}"
    if kind == "function_completed":
        return f"function `{pred.get('function', '?')}` has completed"
    if kind == "function_completed_at_least":
        return f"function `{pred.get('function', '?')}` has completed at least {pred.get('n', '?')} time(s)"
    if kind == "scope_completed":
        return f"scope `{pred.get('sid', '?')}` in `{pred.get('function', '?')}` has completed"
    if kind == "statement_reached":
        return f"statement `{pred.get('sid', '?')}` in `{pred.get('function', '?')}` is reached"
    if kind == "mutex_free":
        return f"mutex `{pred.get('resource', '?')}` is free"
    if kind == "mutex_held":
        return f"mutex `{pred.get('resource', '?')}` is held"
    if kind == "holds_all":
        fn = pred.get("function", "?")
        res_list = ", ".join(f"`{r}`" for r in (pred.get("resources") or []))
        return f"a thread in `{fn}` holds all of [{res_list}]"
    if kind == "mutex_exclusive":
        return f"at most one thread holds `{pred.get('resource', '?')}`"
    if kind == "never_holds_all":
        fn = pred.get("function", "?")
        res_list = ", ".join(f"`{r}`" for r in (pred.get("resources") or []))
        return f"no thread in `{fn}` simultaneously holds all of [{res_list}]"
    if kind == "channel_empty":
        return f"channel `{pred.get('resource', '?')}` is empty"
    if kind == "channel_at_least":
        return f"channel `{pred.get('resource', '?')}` holds at least {pred.get('len', '?')} item(s)"
    if kind == "not":
        return f"not ({explain_predicate(pred.get('predicate'))})"
    if kind == "and":
        subs = [explain_predicate(p) for p in (pred.get("predicates") or [])]
        return "(" + " and ".join(subs) + ")" if subs else "true"
    if kind == "or":
        subs = [explain_predicate(p) for p in (pred.get("predicates") or [])]
        return "(" + " or ".join(subs) + ")" if subs else "false"
    return f"<unknown predicate kind '{kind}'>"


def explain_ctl_formula(formula: Any) -> str:
    """Deterministically render any ``unipn::analysis::temporal::CtlFormula`` JSON."""
    if not isinstance(formula, dict):
        return f"<invalid CTL formula: {formula!r}>"
    op = str(formula.get("op") or "").strip().lower()
    if op == "true":
        return "true"
    if op == "false":
        return "false"
    if op == "atom":
        return explain_predicate(formula.get("atom"))
    if op == "not":
        return f"not ({explain_ctl_formula(formula.get('arg'))})"
    if op == "and":
        args = [explain_ctl_formula(a) for a in (formula.get("args") or [])]
        return "(" + " and ".join(args) + ")" if args else "true"
    if op == "or":
        args = [explain_ctl_formula(a) for a in (formula.get("args") or [])]
        return "(" + " or ".join(args) + ")" if args else "false"
    if op == "implies":
        lhs = explain_ctl_formula(formula.get("lhs"))
        rhs = explain_ctl_formula(formula.get("rhs"))
        return f"whenever ({lhs}), then ({rhs})"
    if op == "ex":
        return f"there exists a next step where ({explain_ctl_formula(formula.get('arg'))})"
    if op == "ax":
        return f"in every next step, ({explain_ctl_formula(formula.get('arg'))})"
    if op == "ef":
        return f"there exists a path that eventually satisfies ({explain_ctl_formula(formula.get('arg'))})"
    if op == "af":
        return f"on every execution path, eventually ({explain_ctl_formula(formula.get('arg'))})"
    if op == "eg":
        return f"there exists a path where always ({explain_ctl_formula(formula.get('arg'))})"
    if op == "ag":
        arg = formula.get("arg")
        if isinstance(arg, dict) and str(arg.get("op") or "").lower() == "implies":
            lhs = explain_ctl_formula(arg.get("lhs"))
            rhs = explain_ctl_formula(arg.get("rhs"))
            return f"in all reachable states, whenever ({lhs}), then ({rhs})"
        return f"in all reachable states, ({explain_ctl_formula(arg)})"
    if op == "eu":
        lhs = explain_ctl_formula(formula.get("lhs"))
        rhs = explain_ctl_formula(formula.get("rhs"))
        return f"there exists a path where ({lhs}) holds until ({rhs})"
    if op == "au":
        lhs = explain_ctl_formula(formula.get("lhs"))
        rhs = explain_ctl_formula(formula.get("rhs"))
        return f"on every execution path, ({lhs}) holds until ({rhs})"
    return f"<unknown CTL op '{op}'>"


def explain_ltl_formula(formula: Any) -> str:
    """Deterministically render any ``unipn::analysis::temporal::LtlFormula`` JSON."""
    if not isinstance(formula, dict):
        return f"<invalid LTL formula: {formula!r}>"
    op = str(formula.get("op") or "").strip().lower()
    if op == "true":
        return "true"
    if op == "false":
        return "false"
    if op == "atom":
        return explain_predicate(formula.get("atom"))
    if op == "not":
        return f"not ({explain_ltl_formula(formula.get('arg'))})"
    if op == "and":
        args = [explain_ltl_formula(a) for a in (formula.get("args") or [])]
        return "(" + " and ".join(args) + ")" if args else "true"
    if op == "or":
        args = [explain_ltl_formula(a) for a in (formula.get("args") or [])]
        return "(" + " or ".join(args) + ")" if args else "false"
    if op == "implies":
        lhs = explain_ltl_formula(formula.get("lhs"))
        rhs = explain_ltl_formula(formula.get("rhs"))
        return f"if ({lhs}), then ({rhs})"
    if op == "next":
        return f"in the next step, ({explain_ltl_formula(formula.get('arg'))})"
    if op == "eventually":
        return f"eventually ({explain_ltl_formula(formula.get('arg'))})"
    if op == "always":
        arg = formula.get("arg")
        if isinstance(arg, dict) and str(arg.get("op") or "").lower() == "implies":
            lhs = explain_ltl_formula(arg.get("lhs"))
            rhs = explain_ltl_formula(arg.get("rhs"))
            return f"always, whenever ({lhs}), then ({rhs})"
        return f"always ({explain_ltl_formula(arg)})"
    if op == "until":
        lhs = explain_ltl_formula(formula.get("lhs"))
        rhs = explain_ltl_formula(formula.get("rhs"))
        return f"({lhs}) holds until ({rhs})"
    if op == "release":
        lhs = explain_ltl_formula(formula.get("lhs"))
        rhs = explain_ltl_formula(formula.get("rhs"))
        return f"({rhs}) holds continuously until and including when ({lhs}) releases it"
    return f"<unknown LTL op '{op}'>"


def explain_fairness(fc: Any) -> str:
    """Deterministically render a ``FairnessConstraint`` JSON."""
    if not isinstance(fc, dict):
        return f"<invalid fairness constraint: {fc!r}>"
    kind = str(fc.get("kind") or "").strip().lower()
    if kind == "justice":
        return f"infinitely often ({explain_predicate(fc.get('atom'))})"
    if kind == "compassion":
        enabled = explain_predicate(fc.get("enabled"))
        taken = explain_predicate(fc.get("taken"))
        return f"if infinitely often ({enabled}), then infinitely often ({taken})"
    return f"<unknown fairness kind '{kind}'>"


def explain_property(prop: dict[str, Any]) -> dict[str, Any]:
    """Render one ``PropertySpec`` dict into a structured explanation entry."""
    pid = str(prop.get("id") or "")
    kind = str(prop.get("kind") or "").strip().lower()
    reqs = [str(r) for r in (prop.get("req") or [])]
    non_vacuous = bool(prop.get("non_vacuous", False))

    if kind == "deadlock_free":
        text = "No reachable state is a deadlock."
    elif kind == "safety":
        text = f"In every reachable state, {explain_predicate(prop.get('invariant'))}."
    elif kind == "reachability":
        text = f"Some reachable state satisfies: {explain_predicate(prop.get('goal'))}."
    elif kind == "always_reachable":
        text = f"From every reachable state, it remains possible to reach: {explain_predicate(prop.get('goal'))}."
    elif kind == "unreachable":
        text = f"No reachable state satisfies: {explain_predicate(prop.get('bad'))}."
    elif kind == "ctl":
        body = explain_ctl_formula(prop.get("formula"))
        nv = " (non-vacuous: implication antecedents must be reachable)" if non_vacuous else ""
        text = f"CTL: {body}.{nv}"
    elif kind == "ltl":
        body = explain_ltl_formula(prop.get("formula"))
        fairness = prop.get("fairness") or []
        if fairness:
            f_str = "; ".join(explain_fairness(fc) for fc in fairness)
            body = f"Under fairness [{f_str}], on every execution trace: {body}"
        else:
            body = f"On every execution trace: {body}"
        nv = " (non-vacuous: implication antecedents must be reachable)" if non_vacuous else ""
        text = f"LTL: {body}.{nv}"
    else:
        text = f"<unknown property kind '{kind}'>"

    return {
        "id": pid,
        "kind": kind,
        "req": reqs,
        "non_vacuous": non_vacuous,
        "explanation": text,
    }


def explain_preserved(pres: dict[str, Any]) -> dict[str, Any]:
    """Render one ``PreservedSpec`` dict into a structured explanation entry."""
    kind = str(pres.get("kind") or "").strip().lower()
    desc = str(pres.get("description") or pres.get("id") or kind or "preserved")
    reqs = [str(r) for r in (pres.get("req") or [])]
    if kind == "reachable":
        text = f"Preserved reachability ({desc}): {explain_predicate(pres.get('goal'))} must remain reachable."
    elif kind == "always":
        text = f"Preserved invariant ({desc}): {explain_predicate(pres.get('invariant'))} must hold in every reachable state."
    elif kind == "function_completed":
        # Legacy shorthand used in some unit fixtures.
        text = f"Preserved completion ({desc}): function `{pres.get('function', '?')}` must be able to complete."
    else:
        text = f"Preserved ({desc}): {json.dumps(pres, ensure_ascii=False)}"
    return {
        "description": desc,
        "req": reqs,
        "explanation": text,
    }


def explain_contract_data(
    contract: dict[str, Any],
    *,
    client: ConcirClient | None = None,
    prefer_backend: bool = False,
) -> ContractExplanation:
    """Deterministically back-translate a contract JSON to natural language.

    When ``prefer_backend=True`` and ``client`` is supplied, attempts
    ``client.explain_contract(...)`` first and falls back cleanly to the
    pure-Python renderer if the backend is unavailable or does not support it.
    """
    if prefer_backend and client is not None:
        try:
            res = client.explain_contract(json.dumps(contract, ensure_ascii=False))
            if res.is_semantic and res.status == "pass" and isinstance(res.payload, dict):
                payload = res.payload
                return ContractExplanation(
                    name=str(payload.get("name") or contract.get("name") or "contract"),
                    properties=list(payload.get("properties") or []),
                    preserved=list(payload.get("preserved") or []),
                    schema=str(payload.get("schema") or "concir-contract-explanation/1"),
                    source="concir-backend",
                )
        except Exception:  # noqa: BLE001
            pass

    name = str(contract.get("name") or "contract")
    props_out: list[dict[str, Any]] = []
    if contract.get("deadlock_free") is True and not any(
        isinstance(p, dict) and p.get("kind") == "deadlock_free"
        for p in (contract.get("properties") or [])
    ):
        props_out.append({
            "id": "deadlock_free",
            "kind": "deadlock_free",
            "req": [],
            "non_vacuous": False,
            "explanation": "No reachable state is a deadlock.",
        })
    for p in contract.get("properties") or []:
        if isinstance(p, dict):
            props_out.append(explain_property(p))

    pres_out: list[dict[str, Any]] = []
    for pr in contract.get("preserved") or []:
        if isinstance(pr, dict):
            pres_out.append(explain_preserved(pr))

    return ContractExplanation(
        name=name,
        properties=props_out,
        preserved=pres_out,
        schema="concir-contract-explanation/1",
        source="python",
    )


# ---------------------------------------------------------------------------
# Step 0.3 — Formula Inspection & Contract Sanity Check
# ---------------------------------------------------------------------------

def formula_contains_implication(formula: Any) -> bool:
    """Return True if a CTL or LTL formula tree contains any ``implies`` node."""
    if not isinstance(formula, dict):
        return False
    op = str(formula.get("op") or "").strip().lower()
    if op == "implies":
        return True
    if "arg" in formula and formula_contains_implication(formula["arg"]):
        return True
    if "lhs" in formula and formula_contains_implication(formula["lhs"]):
        return True
    if "rhs" in formula and formula_contains_implication(formula["rhs"]):
        return True
    for sub in formula.get("args") or []:
        if formula_contains_implication(sub):
            return True
    return False


def ensure_implication_non_vacuous(contract: dict[str, Any]) -> dict[str, Any]:
    """Return a copy of ``contract`` where CTL/LTL implication properties default to ``non_vacuous: true``."""
    out = copy.deepcopy(contract)
    for prop in out.get("properties") or []:
        if not isinstance(prop, dict):
            continue
        kind = str(prop.get("kind") or "").strip().lower()
        if kind in ("ctl", "ltl") and "non_vacuous" not in prop:
            if formula_contains_implication(prop.get("formula")):
                prop["non_vacuous"] = True
    return out


def _collect_predicate_refs(
    pred: Any,
    *,
    functions: set[str],
    resources: set[str],
    errors: list[str],
) -> None:
    if not isinstance(pred, dict):
        errors.append(f"predicate must be a JSON object, got {type(pred).__name__}")
        return
    kind = str(pred.get("kind") or "").strip().lower()
    if kind not in VALID_PREDICATE_KINDS:
        errors.append(f"unknown predicate kind '{kind}'")
        return
    if kind in ("var_eq", "var_cmp", "mutex_free", "mutex_held", "mutex_exclusive", "channel_empty", "channel_at_least"):
        if not pred.get("resource"):
            errors.append(f"predicate '{kind}' missing 'resource'")
        else:
            resources.add(str(pred["resource"]))
        if kind == "var_cmp" and str(pred.get("op") or "") not in VALID_CMP_OPS:
            errors.append(f"var_cmp has invalid op '{pred.get('op')}'")
    elif kind in ("function_completed", "function_completed_at_least", "scope_completed", "statement_reached"):
        if not pred.get("function"):
            errors.append(f"predicate '{kind}' missing 'function'")
        else:
            functions.add(str(pred["function"]))
    elif kind in ("holds_all", "never_holds_all"):
        if not pred.get("function"):
            errors.append(f"predicate '{kind}' missing 'function'")
        else:
            functions.add(str(pred["function"]))
        for r in pred.get("resources") or []:
            resources.add(str(r))
    elif kind == "not":
        _collect_predicate_refs(pred.get("predicate"), functions=functions, resources=resources, errors=errors)
    elif kind in ("and", "or"):
        subs = pred.get("predicates")
        if not isinstance(subs, list):
            errors.append(f"predicate '{kind}' requires list 'predicates'")
        else:
            for s in subs:
                _collect_predicate_refs(s, functions=functions, resources=resources, errors=errors)


def _validate_temporal_tree(
    node: Any,
    *,
    logic: str,
    functions: set[str],
    resources: set[str],
    errors: list[str],
) -> None:
    if not isinstance(node, dict):
        errors.append(f"{logic.upper()} formula node must be a JSON object")
        return
    op = str(node.get("op") or "").strip().lower()
    valid_ops = VALID_CTL_OPS if logic == "ctl" else VALID_LTL_OPS
    if op not in valid_ops:
        errors.append(f"unknown {logic.upper()} operator '{op}'")
        return
    if op in ("true", "false"):
        return
    if op == "atom":
        _collect_predicate_refs(node.get("atom"), functions=functions, resources=resources, errors=errors)
        return
    if op in ("not", "ex", "ax", "ef", "af", "eg", "ag", "next", "eventually", "always"):
        _validate_temporal_tree(node.get("arg"), logic=logic, functions=functions, resources=resources, errors=errors)
        return
    if op in ("implies", "eu", "au", "until", "release"):
        _validate_temporal_tree(node.get("lhs"), logic=logic, functions=functions, resources=resources, errors=errors)
        _validate_temporal_tree(node.get("rhs"), logic=logic, functions=functions, resources=resources, errors=errors)
        return
    if op in ("and", "or"):
        args = node.get("args")
        if not isinstance(args, list) or not args:
            errors.append(f"{logic.upper()} '{op}' requires non-empty list 'args'")
            return
        for a in args:
            _validate_temporal_tree(a, logic=logic, functions=functions, resources=resources, errors=errors)


def _lookup_resource_kind(res_name: str, res_kind_map: dict[str, str]) -> str:
    short = str(res_name).split("::")[-1]
    return res_kind_map.get(str(res_name), "") or res_kind_map.get(short, "")


def _check_predicate_resource_compat(
    pred: Any,
    res_kind_map: dict[str, str],
    *,
    property_id: str,
    issues: list[SanityIssue],
) -> None:
    if not isinstance(pred, dict) or not res_kind_map:
        return
    pkind = str(pred.get("kind") or "").strip().lower()
    if pkind in ("mutex_free", "mutex_held", "mutex_exclusive"):
        res = str(pred.get("resource") or "")
        decl_kind = _lookup_resource_kind(res, res_kind_map)
        if decl_kind and decl_kind != "mutex":
            issues.append(
                SanityIssue(
                    "error",
                    "incompatible_resource_predicate",
                    f"predicate '{pkind}' requires a Mutex resource, but '{res}' has kind '{decl_kind}'",
                    property_id=property_id,
                )
            )
    elif pkind in ("var_eq", "var_cmp"):
        res = str(pred.get("resource") or "")
        decl_kind = _lookup_resource_kind(res, res_kind_map)
        if decl_kind in {"mutex", "rwlock", "condvar", "semaphore", "channel"}:
            issues.append(
                SanityIssue(
                    "error",
                    "incompatible_resource_predicate",
                    f"predicate '{pkind}' requires a modeled Var/Atomic resource, but '{res}' has kind '{decl_kind}'",
                    property_id=property_id,
                )
            )
    elif pkind in ("channel_empty", "channel_at_least"):
        res = str(pred.get("resource") or "")
        decl_kind = _lookup_resource_kind(res, res_kind_map)
        if decl_kind and decl_kind != "channel":
            issues.append(
                SanityIssue(
                    "error",
                    "incompatible_resource_predicate",
                    f"predicate '{pkind}' requires a Channel resource, but '{res}' has kind '{decl_kind}'",
                    property_id=property_id,
                )
            )
    elif pkind == "not":
        _check_predicate_resource_compat(pred.get("predicate"), res_kind_map, property_id=property_id, issues=issues)
    elif pkind in ("and", "or"):
        for sub in pred.get("predicates") or []:
            _check_predicate_resource_compat(sub, res_kind_map, property_id=property_id, issues=issues)


def _check_temporal_resource_compat(
    node: Any,
    res_kind_map: dict[str, str],
    *,
    property_id: str,
    issues: list[SanityIssue],
) -> None:
    if not isinstance(node, dict) or not res_kind_map:
        return
    op = str(node.get("op") or "").strip().lower()
    if op == "atom":
        _check_predicate_resource_compat(node.get("atom"), res_kind_map, property_id=property_id, issues=issues)
    elif "arg" in node:
        _check_temporal_resource_compat(node.get("arg"), res_kind_map, property_id=property_id, issues=issues)
    if "lhs" in node:
        _check_temporal_resource_compat(node.get("lhs"), res_kind_map, property_id=property_id, issues=issues)
    if "rhs" in node:
        _check_temporal_resource_compat(node.get("rhs"), res_kind_map, property_id=property_id, issues=issues)
    for sub in node.get("args") or []:
        _check_temporal_resource_compat(sub, res_kind_map, property_id=property_id, issues=issues)


def _matches_grounded(name: str, grounded: set[str], *, allow_main: bool = False) -> bool:
    if not grounded:
        return True
    short = name.split("::")[-1]
    if allow_main and short == "main":
        return True
    if name in grounded or short in grounded:
        return True
    grounded_shorts = {g.split("::")[-1] for g in grounded}
    return short in grounded_shorts


def _validate_req_clauses(
    reqs: Any,
    *,
    owner_kind: str,
    owner_label: str,
    property_id: str,
    clause_index: dict[str, RequirementClause],
    issues: list[SanityIssue],
) -> None:
    if not clause_index:
        return
    for cid in reqs or []:
        cid_s = str(cid).strip()
        if cid_s not in clause_index:
            issues.append(
                SanityIssue(
                    "error",
                    "unknown_req_clause",
                    f"{owner_kind} '{owner_label}' references unknown clause '{cid_s}'",
                    property_id=property_id,
                    clause_id=cid_s,
                )
            )
        elif not clause_index[cid_s].verifiable:
            issues.append(
                SanityIssue(
                    "error",
                    "unverifiable_req_clause",
                    f"{owner_kind} '{owner_label}' references unverifiable"
                    + ("/sequential_hole" if owner_kind == "property" else "")
                    + f" clause '{cid_s}'",
                    property_id=property_id,
                    clause_id=cid_s,
                )
            )


def _validate_entity_grounding(
    funcs: set[str],
    resources: set[str],
    *,
    owner_kind: str,
    owner_label: str,
    grounded_roles: set[str],
    grounded_resources: set[str],
    strict_entities: bool,
    issues: list[SanityIssue],
) -> None:
    ent_sev = "error" if strict_entities else "warning"
    for fn in sorted(funcs):
        if not _matches_grounded(fn, grounded_roles, allow_main=True):
            issues.append(
                SanityIssue(
                    ent_sev,
                    "ungrounded_function",
                    f"{owner_kind} '{owner_label}' references function '{fn}' not in grounded roles {sorted(grounded_roles)}",
                    property_id=owner_label,
                )
            )
    for res in sorted(resources):
        if not _matches_grounded(res, grounded_resources):
            issues.append(
                SanityIssue(
                    ent_sev,
                    "ungrounded_resource",
                    f"{owner_kind} '{owner_label}' references resource '{res}' not in grounded resources {sorted(grounded_resources)}",
                    property_id=owner_label,
                )
            )


def validate_contract_sanity(
    contract: dict[str, Any],
    structured: StructuredRequirements | None = None,
    *,
    strict_entities: bool = True,
    require_non_vacuous: bool = True,
) -> SanityCheckReport:
    """Run deterministic Step 0.3 sanity checks on a verification contract.

    Checks:
    1. Every property ``id`` is non-empty and unique.
    2. Every property / CTL / LTL / fairness / predicate shape uses valid schema tags.
    3. Every ``req`` clause referenced exists in ``structured.clauses`` and is marked
       ``verifiable: true`` (catching bogus formalization of ``sequential_hole`` / ``[U]`` clauses).
    4. Referenced ``function`` and ``resource`` names align with grounded entities (when provided).
    5. CTL/LTL formulas containing ``implies`` set ``non_vacuous: true``.
    """
    issues: list[SanityIssue] = []
    if not isinstance(contract, dict):
        issues.append(SanityIssue("error", "invalid_contract", "contract must be a JSON object"))
        return SanityCheckReport(ok=False, issues=issues)

    props = contract.get("properties")
    if props is None:
        if contract.get("deadlock_free"):
            props = []
        else:
            issues.append(SanityIssue("error", "missing_properties", "contract must contain a 'properties' list"))
            return SanityCheckReport(ok=False, issues=issues)
    if not isinstance(props, list):
        issues.append(SanityIssue("error", "invalid_properties", "'properties' must be a list"))
        return SanityCheckReport(ok=False, issues=issues)

    clause_index = structured.clause_by_id() if structured is not None else {}
    grounded_roles = set(structured.entities.roles) if structured is not None else set()
    grounded_resources = set(structured.entities.resources) if structured is not None else set()
    res_kind_map = (
        {
            rs.name.split("::")[-1]: rs.kind.strip().lower()
            for rs in structured.entities.resource_specs
            if rs.kind
        }
        if structured is not None
        else {}
    )

    seen_ids: set[str] = set()

    for idx, prop in enumerate(props):
        if not isinstance(prop, dict):
            issues.append(SanityIssue("error", "invalid_property", f"property[{idx}] must be a JSON object"))
            continue
        pid = str(prop.get("id") or "").strip()
        if not pid:
            issues.append(SanityIssue("error", "empty_property_id", f"property[{idx}] has empty 'id'"))
        elif pid in seen_ids:
            issues.append(
                SanityIssue("error", "duplicate_property_id", f"duplicate property id '{pid}'", property_id=pid)
            )
        else:
            seen_ids.add(pid)

        kind = str(prop.get("kind") or "").strip().lower()
        if kind not in VALID_PROPERTY_KINDS:
            issues.append(
                SanityIssue("error", "unknown_property_kind", f"property '{pid}' has unknown kind '{kind}'", property_id=pid)
            )
            continue

        if structured is not None:
            _validate_req_clauses(
                prop.get("req"),
                owner_kind="property",
                owner_label=pid,
                property_id=pid,
                clause_index=clause_index,
                issues=issues,
            )

        tree_errors: list[str] = []
        p_funcs: set[str] = set()
        p_res: set[str] = set()

        if kind == "safety":
            _collect_predicate_refs(prop.get("invariant"), functions=p_funcs, resources=p_res, errors=tree_errors)
            _check_predicate_resource_compat(prop.get("invariant"), res_kind_map, property_id=pid, issues=issues)
        elif kind in ("reachability", "always_reachable"):
            _collect_predicate_refs(prop.get("goal"), functions=p_funcs, resources=p_res, errors=tree_errors)
            _check_predicate_resource_compat(prop.get("goal"), res_kind_map, property_id=pid, issues=issues)
        elif kind == "unreachable":
            _collect_predicate_refs(prop.get("bad"), functions=p_funcs, resources=p_res, errors=tree_errors)
            _check_predicate_resource_compat(prop.get("bad"), res_kind_map, property_id=pid, issues=issues)
        elif kind == "ctl":
            _validate_temporal_tree(prop.get("formula"), logic="ctl", functions=p_funcs, resources=p_res, errors=tree_errors)
            _check_temporal_resource_compat(prop.get("formula"), res_kind_map, property_id=pid, issues=issues)
            if require_non_vacuous and formula_contains_implication(prop.get("formula")) and not prop.get("non_vacuous"):
                issues.append(
                    SanityIssue(
                        "error",
                        "missing_non_vacuous",
                        f"CTL property '{pid}' contains an implication and must set 'non_vacuous': true",
                        property_id=pid,
                    )
                )
        elif kind == "ltl":
            _validate_temporal_tree(prop.get("formula"), logic="ltl", functions=p_funcs, resources=p_res, errors=tree_errors)
            _check_temporal_resource_compat(prop.get("formula"), res_kind_map, property_id=pid, issues=issues)
            for fc in prop.get("fairness") or []:
                if not isinstance(fc, dict):
                    tree_errors.append("fairness entry must be an object")
                    continue
                fkind = str(fc.get("kind") or "").strip().lower()
                if fkind not in VALID_FAIRNESS_KINDS:
                    tree_errors.append(f"unknown fairness kind '{fkind}'")
                elif fkind == "justice":
                    _collect_predicate_refs(fc.get("atom"), functions=p_funcs, resources=p_res, errors=tree_errors)
                elif fkind == "compassion":
                    _collect_predicate_refs(fc.get("enabled"), functions=p_funcs, resources=p_res, errors=tree_errors)
                    _collect_predicate_refs(fc.get("taken"), functions=p_funcs, resources=p_res, errors=tree_errors)
            if require_non_vacuous and formula_contains_implication(prop.get("formula")) and not prop.get("non_vacuous"):
                issues.append(
                    SanityIssue(
                        "error",
                        "missing_non_vacuous",
                        f"LTL property '{pid}' contains an implication and must set 'non_vacuous': true",
                        property_id=pid,
                    )
                )

        for err in tree_errors:
            issues.append(SanityIssue("error", "malformed_formula", f"property '{pid}': {err}", property_id=pid))

        if structured is not None:
            _validate_entity_grounding(
                p_funcs,
                p_res,
                owner_kind="property",
                owner_label=pid,
                grounded_roles=grounded_roles,
                grounded_resources=grounded_resources,
                strict_entities=strict_entities,
                issues=issues,
            )

    preserved_list = contract.get("preserved") or []
    if not isinstance(preserved_list, list):
        issues.append(SanityIssue("error", "invalid_preserved", "'preserved' must be a list"))
        preserved_list = []

    for idx, pr in enumerate(preserved_list):
        if not isinstance(pr, dict):
            issues.append(SanityIssue("error", "invalid_preserved_entry", f"preserved[{idx}] must be a JSON object"))
            continue
        pkind = str(pr.get("kind") or "").strip().lower()
        p_label = str(pr.get("description") or pr.get("id") or f"preserved[{idx}]")
        if structured is not None:
            _validate_req_clauses(
                pr.get("req"),
                owner_kind="preserved",
                owner_label=p_label,
                property_id=p_label,
                clause_index=clause_index,
                issues=issues,
            )
        pr_errors: list[str] = []
        pr_funcs: set[str] = set()
        pr_res: set[str] = set()
        if pkind == "reachable":
            _collect_predicate_refs(pr.get("goal"), functions=pr_funcs, resources=pr_res, errors=pr_errors)
            _check_predicate_resource_compat(pr.get("goal"), res_kind_map, property_id=p_label, issues=issues)
        elif pkind == "always":
            _collect_predicate_refs(pr.get("invariant"), functions=pr_funcs, resources=pr_res, errors=pr_errors)
            _check_predicate_resource_compat(pr.get("invariant"), res_kind_map, property_id=p_label, issues=issues)
        elif pkind == "function_completed":
            if pr.get("function"):
                pr_funcs.add(str(pr["function"]))
        else:
            pr_errors.append(f"unknown preserved kind '{pkind}'")

        for err in pr_errors:
            issues.append(SanityIssue("error", "malformed_preserved", f"preserved '{p_label}': {err}", property_id=p_label))

        if structured is not None:
            _validate_entity_grounding(
                pr_funcs,
                pr_res,
                owner_kind="preserved",
                owner_label=p_label,
                grounded_roles=grounded_roles,
                grounded_resources=grounded_resources,
                strict_entities=strict_entities,
                issues=issues,
            )

    has_errors = any(i.severity == "error" for i in issues)
    return SanityCheckReport(
        ok=not has_errors,
        issues=issues,
        checked_properties=len(props),
        checked_preserved=len(preserved_list),
    )


# ---------------------------------------------------------------------------
# Step 0.1 (nl_to_clauses) & Step 0.2 (clauses_to_contract)
# ---------------------------------------------------------------------------

def build_nl_to_clauses_prompt(nl_text: str, *, feedback: str | None = None) -> str:
    """Build the Step 0.1 user prompt (raw NL -> typed grounded entities + atomic clauses R1..Rn)."""
    parts = [
        "Decompose the following natural-language functional specification (which may be a high-level Chinese or English description without low-level thread/lock details) into:",
        "1. Canonical typed symbol table in module `main`: `roles` (ASCII worker function names), typed `resources` objects (`name`, `kind` in `Mutex|RwLock|Condvar|Channel|Semaphore|Var`, `domain`, `init`), and `domain_types` (pure business structs that belong in TypeHoles).",
        "2. Numbered atomic clauses `R1..Rn`, each classified into `safety | liveness | reachability | sequential_hole` with `verifiable: true` for concurrency clauses and `verifiable: false` for sequential/domain payload logic (`sequential_hole`).",
        "",
        "<natural_language_requirements>",
        nl_text.strip(),
        "</natural_language_requirements>",
    ]
    if feedback:
        parts += ["", "<validation_feedback>", feedback, "</validation_feedback>"]
    parts += ["", "Output ONLY the JSON object."]
    return "\n".join(parts)


def build_clauses_to_contract_prompt(
    structured: StructuredRequirements,
    *,
    feedback: str | None = None,
) -> str:
    """Build the isolated Step 0.2 user prompt (clauses + grounded entities -> contract.json).

    Deliberately NEVER includes any candidate SCIR/CIR program to prevent
    specification-model collusion!
    """
    verifiable_ids = [c.id for c in structured.verifiable_clauses()]
    unverifiable_ids = [c.id for c in structured.clauses if not c.verifiable]
    parts = [
        "Synthesize a formal ConcIR v5 verification contract JSON from the structured requirements below.",
        "IMPORTANT ISOLATION RULES:",
        "- Translate ONLY the verifiable clauses into formal properties (`deadlock_free`, `safety`, `reachability`, `always_reachable`, `unreachable`, `ctl`, `ltl`) and `preserved` goals.",
        f"- Verifiable clause IDs: {json.dumps(verifiable_ids)}",
        f"- Unverifiable sequential/domain clauses (DO NOT formalize as properties): {json.dumps(unverifiable_ids)}",
        "- Use ONLY the grounded entity names below in predicate `function` / `resource` fields.",
        "- For any CTL or LTL property containing an `implies` operator, ALWAYS set `\"non_vacuous\": true`.",
        "",
        "<grounded_entities>",
        json.dumps(structured.entities.as_dict(), ensure_ascii=False, indent=2),
        "</grounded_entities>",
        "",
        "<structured_requirements>",
        structured.to_markdown().strip(),
        "</structured_requirements>",
    ]
    if feedback:
        parts += ["", "<sanity_check_feedback>", feedback, "</sanity_check_feedback>"]
    parts += ["", "Output ONLY the contract JSON object."]
    return "\n".join(parts)


def nl_to_clauses(
    nl_text: str,
    clause_provider: CandidateProvider,
    *,
    title: str = "requirements",
    max_rounds: int = 2,
) -> StructuredRequirements:
    """Step 0.1: Convert free-form natural language into grounded entities + atomic clauses R1..Rn."""
    feedback: str | None = None
    last_error = "clause_provider produced no valid structured requirements"
    prev_text: str | None = None

    for attempt in range(1, max(1, max_rounds) + 1):
        prompt = build_nl_to_clauses_prompt(nl_text, feedback=feedback)
        req = CandidateRequest(
            requirements=prompt,
            contract={},
            feedback=feedback,
            attempt=attempt,
            previous_candidate=prev_text,
        )
        resp = clause_provider.propose(req)
        if resp.error:
            last_error = f"clause provider error: {resp.error}"
            feedback = last_error
            continue
        prev_text = resp.text
        try:
            raw = json.loads(extract_json(resp.text))
        except json.JSONDecodeError as exc:
            last_error = f"clause provider returned invalid JSON: {exc}"
            feedback = last_error
            continue
        if not isinstance(raw, dict):
            last_error = "clause provider output must be a JSON object"
            feedback = last_error
            continue

        if "title" not in raw:
            raw["title"] = title
        structured = StructuredRequirements.from_dict(raw)
        structured.source_nl = nl_text
        if not structured.clauses:
            last_error = "clause provider returned zero clauses; provide at least one R1..Rn clause"
            feedback = last_error
            continue
        # Ensure unique clause IDs.
        seen: set[str] = set()
        dups: list[str] = []
        for c in structured.clauses:
            if c.id in seen:
                dups.append(c.id)
            seen.add(c.id)
        if dups:
            last_error = f"duplicate clause IDs: {dups}"
            feedback = last_error
            continue
        return structured

    raise ValueError(last_error)


def clauses_to_contract(
    structured: StructuredRequirements,
    contract_provider: CandidateProvider,
    *,
    max_rounds: int = 2,
    enforce_non_vacuous: bool = True,
    strict_entities: bool = True,
) -> tuple[dict[str, Any], SanityCheckReport]:
    """Step 0.2: Synthesize ``contract.json`` from structured clauses in an isolated prompt.

    Never accepts or forwards any candidate SCIR/CIR program!
    """
    feedback: str | None = None
    prev_text: str | None = None
    last_contract: dict[str, Any] | None = None
    last_sanity: SanityCheckReport | None = None
    last_error = "contract_provider produced no valid contract JSON"

    for attempt in range(1, max(1, max_rounds) + 1):
        prompt = build_clauses_to_contract_prompt(structured, feedback=feedback)
        req = CandidateRequest(
            requirements=prompt,
            contract={},
            feedback=feedback,
            attempt=attempt,
            previous_candidate=prev_text,
            current_program=None,  # Strictly isolated from any SCIR model!
        )
        resp = contract_provider.propose(req)
        if resp.error:
            last_error = f"contract provider error: {resp.error}"
            feedback = last_error
            continue
        prev_text = resp.text
        try:
            raw = json.loads(extract_json(resp.text))
        except json.JSONDecodeError as exc:
            last_error = f"contract provider returned invalid JSON: {exc}"
            feedback = last_error
            continue
        if not isinstance(raw, dict):
            last_error = "contract provider output must be a JSON object"
            feedback = last_error
            continue

        if "name" not in raw or not str(raw["name"]).strip():
            raw["name"] = re.sub(r"[^A-Za-z0-9_-]+", "-", structured.title.lower()).strip("-") or "contract"
        contract = ensure_implication_non_vacuous(raw) if enforce_non_vacuous else raw
        sanity = validate_contract_sanity(
            contract,
            structured,
            strict_entities=strict_entities,
            require_non_vacuous=enforce_non_vacuous,
        )
        last_contract = contract
        last_sanity = sanity
        if sanity.ok:
            return contract, sanity
        feedback = json.dumps(sanity.as_dict(), ensure_ascii=False, indent=2)

    if last_contract is not None and last_sanity is not None:
        return last_contract, last_sanity
    raise ValueError(last_error)


# ---------------------------------------------------------------------------
# Dual-Entry Drivers: Entry A (formalize_from_nl) & Entry B (formalize_from_structured)
# ---------------------------------------------------------------------------

def _write_stage0_artifacts(
    out_dir: Path | str | None,
    structured: StructuredRequirements,
    req_md: str,
    contract: dict[str, Any],
    explanation: ContractExplanation,
) -> dict[str, str | None]:
    if out_dir is None:
        return {
            "out_dir": None,
            "requirements_md_path": None,
            "requirements_json_path": None,
            "contract_path": None,
            "explanation_path": None,
        }
    root = Path(out_dir).expanduser().resolve()
    root.mkdir(parents=True, exist_ok=True)
    req_md_path = root / "REQUIREMENTS.md"
    req_json_path = root / "requirements.json"
    contract_path = root / "contract.json"
    exp_path = root / "contract_explanation.json"

    req_md_path.write_text(req_md, encoding="utf-8")
    write_json(req_json_path, structured.as_dict())
    write_json(contract_path, contract)
    write_json(exp_path, explanation.as_dict())
    return {
        "out_dir": str(root),
        "requirements_md_path": str(req_md_path),
        "requirements_json_path": str(req_json_path),
        "contract_path": str(contract_path),
        "explanation_path": str(exp_path),
    }


def formalize_from_nl(
    nl_text: str,
    *,
    clause_provider: CandidateProvider,
    contract_provider: CandidateProvider | None = None,
    client: ConcirClient | None = None,
    out_dir: Path | str | None = None,
    title: str = "requirements",
    max_clause_rounds: int = 2,
    max_contract_rounds: int = 2,
    enforce_non_vacuous: bool = True,
    strict_entities: bool = True,
    prefer_backend_explain: bool = False,
) -> FormalizationResult:
    """Entry A: Raw NL -> Step 0.1 (`nl_to_clauses`) -> Step 0.2 (`clauses_to_contract`) -> Step 0.3."""
    structured = nl_to_clauses(
        nl_text,
        clause_provider,
        title=title,
        max_rounds=max_clause_rounds,
    )
    req_md = structured.to_markdown()
    c_prov = contract_provider or clause_provider
    contract, sanity = clauses_to_contract(
        structured,
        c_prov,
        max_rounds=max_contract_rounds,
        enforce_non_vacuous=enforce_non_vacuous,
        strict_entities=strict_entities,
    )
    explanation = explain_contract_data(
        contract, client=client, prefer_backend=prefer_backend_explain
    )
    paths = _write_stage0_artifacts(out_dir, structured, req_md, contract, explanation)
    warnings = [i.message for i in sanity.warnings]
    return FormalizationResult(
        entry="nl",
        requirements=structured,
        requirements_md=req_md,
        contract=contract,
        explanation=explanation,
        sanity=sanity,
        warnings=warnings,
        **paths,
    )


def formalize_from_structured(
    structured_input: StructuredRequirements | dict[str, Any] | Path | str,
    *,
    contract: dict[str, Any] | Path | str | None = None,
    contract_provider: CandidateProvider | None = None,
    client: ConcirClient | None = None,
    out_dir: Path | str | None = None,
    max_contract_rounds: int = 2,
    enforce_non_vacuous: bool = True,
    strict_entities: bool = True,
    prefer_backend_explain: bool = False,
) -> FormalizationResult:
    """Entry B: Pre-authored R1..Rn markdown/JSON (+ optional contract.json) -> validated contract + explanation."""
    structured = parse_structured_requirements(structured_input)
    req_md = structured.to_markdown()

    if contract is not None:
        if isinstance(contract, dict):
            contract_data = copy.deepcopy(contract)
        else:
            contract_data = json.loads(Path(contract).expanduser().resolve().read_text(encoding="utf-8"))
        if enforce_non_vacuous:
            contract_data = ensure_implication_non_vacuous(contract_data)
        sanity = validate_contract_sanity(
            contract_data,
            structured,
            strict_entities=strict_entities,
            require_non_vacuous=enforce_non_vacuous,
        )
    else:
        if contract_provider is None:
            raise ValueError("formalize_from_structured requires either 'contract' or 'contract_provider'")
        contract_data, sanity = clauses_to_contract(
            structured,
            contract_provider,
            max_rounds=max_contract_rounds,
            enforce_non_vacuous=enforce_non_vacuous,
            strict_entities=strict_entities,
        )

    explanation = explain_contract_data(
        contract_data, client=client, prefer_backend=prefer_backend_explain
    )
    paths = _write_stage0_artifacts(out_dir, structured, req_md, contract_data, explanation)
    warnings = [i.message for i in sanity.warnings]
    return FormalizationResult(
        entry="structured",
        requirements=structured,
        requirements_md=req_md,
        contract=contract_data,
        explanation=explanation,
        sanity=sanity,
        warnings=warnings,
        **paths,
    )
