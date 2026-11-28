"""Stage 0 prompt builders and Dual-Entry workflows (NL / Structured -> Contract)."""

from __future__ import annotations

import copy
import json
import re
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient
from ..core.json_utils import extract_json, write_json
from ..core.providers import CandidateProvider, CandidateRequest
from .backend_contract import (
    ensure_implication_non_vacuous,
    explain_contract_data,
    validate_contract_sanity,
)
from .markdown import parse_structured_requirements
from .models import ContractExplanation, FormalizationResult, SanityCheckReport, SanityIssue, StructuredRequirements


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
    client: ConcirClient | None = None,
) -> tuple[dict[str, Any], SanityCheckReport]:
    """Step 0.2: Synthesize ``contract.json`` from structured clauses in an isolated prompt."""
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
            current_program=None,
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
        contract = ensure_implication_non_vacuous(raw, client=client) if enforce_non_vacuous else raw
        sanity = validate_contract_sanity(
            contract,
            structured,
            strict_entities=strict_entities,
            require_non_vacuous=enforce_non_vacuous,
            client=client,
        )
        last_contract = contract
        last_sanity = sanity
        if sanity.ok:
            return contract, sanity
        feedback = json.dumps(sanity.as_dict(), ensure_ascii=False, indent=2)

    if last_contract is not None and last_sanity is not None:
        return last_contract, last_sanity
    raise ValueError(last_error)


def _write_stage0_artifacts(
    out_dir: Path | str | None,
    structured: StructuredRequirements,
    req_md: str,
    contract: dict[str, Any],
    explanation: ContractExplanation,
    req_ir: dict[str, Any] | None = None,
) -> dict[str, str | None]:
    if out_dir is None:
        return {
            "out_dir": None,
            "requirements_md_path": None,
            "requirements_json_path": None,
            "contract_path": None,
            "explanation_path": None,
            "req_ir_path": None,
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
    req_ir_path_str: str | None = None
    if req_ir is not None:
        req_ir_path = root / "req_ir.json"
        write_json(req_ir_path, req_ir)
        req_ir_path_str = str(req_ir_path)
    return {
        "out_dir": str(root),
        "requirements_md_path": str(req_md_path),
        "requirements_json_path": str(req_json_path),
        "contract_path": str(contract_path),
        "explanation_path": str(exp_path),
        "req_ir_path": req_ir_path_str,
    }


def _check_optional_req_ir(
    req_ir_input: dict[str, Any] | Path | str | None,
    structured: StructuredRequirements,
    contract: dict[str, Any],
    client: ConcirClient | None,
    sanity: SanityCheckReport,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    req_ir_data: dict[str, Any] | None = None
    if req_ir_input is not None:
        if isinstance(req_ir_input, dict):
            req_ir_data = copy.deepcopy(req_ir_input)
        else:
            req_ir_data = json.loads(Path(req_ir_input).expanduser().resolve().read_text(encoding="utf-8"))
    elif structured.req_ir is not None:
        req_ir_data = copy.deepcopy(structured.req_ir)

    if req_ir_data is None:
        return None, None

    req_check_payload: dict[str, Any] | None = None
    if client is not None:
        res = client.check_req(req_ir_data, contract=contract)
        req_check_payload = res.payload if isinstance(res.payload, dict) else {"status": res.status, "error": res.error}
        if not res.is_semantic or res.status in ("fail", "invalid", "unsupported"):
            msg = res.error or "; ".join(
                d.get("message", "") for d in (req_check_payload or {}).get("diagnostics") or []
            ) or f"req_ir check-req returned {res.status}"
            sanity.issues.append(SanityIssue("error", f"req_ir_{res.status}", msg))
            sanity.ok = False
        elif res.status == "unknown":
            sanity.issues.append(
                SanityIssue("warning", "req_ir_unknown", "req_ir check-req was truncated (unknown)")
            )
    return req_ir_data, req_check_payload


def formalize_from_nl(
    nl_text: str,
    *,
    clause_provider: CandidateProvider,
    contract_provider: CandidateProvider | None = None,
    client: ConcirClient | None = None,
    out_dir: Path | str | None = None,
    title: str = "requirements",
    req_ir: dict[str, Any] | Path | str | None = None,
    max_clause_rounds: int = 2,
    max_contract_rounds: int = 2,
    enforce_non_vacuous: bool = True,
    strict_entities: bool = True,
    prefer_backend_explain: bool = True,
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
        client=client,
    )
    req_ir_data, req_check = _check_optional_req_ir(req_ir, structured, contract, client, sanity)
    explanation = explain_contract_data(contract, client=client, prefer_backend=prefer_backend_explain)
    paths = _write_stage0_artifacts(out_dir, structured, req_md, contract, explanation, req_ir=req_ir_data)
    warnings = [i.message for i in sanity.warnings]
    return FormalizationResult(
        entry="nl",
        requirements=structured,
        requirements_md=req_md,
        contract=contract,
        explanation=explanation,
        sanity=sanity,
        req_ir=req_ir_data,
        req_check=req_check,
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
    req_ir: dict[str, Any] | Path | str | None = None,
    max_contract_rounds: int = 2,
    enforce_non_vacuous: bool = True,
    strict_entities: bool = True,
    prefer_backend_explain: bool = True,
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
            contract_data = ensure_implication_non_vacuous(contract_data, client=client)
        sanity = validate_contract_sanity(
            contract_data,
            structured,
            strict_entities=strict_entities,
            require_non_vacuous=enforce_non_vacuous,
            client=client,
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
            client=client,
        )

    req_ir_data, req_check = _check_optional_req_ir(req_ir, structured, contract_data, client, sanity)
    explanation = explain_contract_data(contract_data, client=client, prefer_backend=prefer_backend_explain)
    paths = _write_stage0_artifacts(out_dir, structured, req_md, contract_data, explanation, req_ir=req_ir_data)
    warnings = [i.message for i in sanity.warnings]
    return FormalizationResult(
        entry="structured",
        requirements=structured,
        requirements_md=req_md,
        contract=contract_data,
        explanation=explanation,
        sanity=sanity,
        req_ir=req_ir_data,
        req_check=req_check,
        warnings=warnings,
        **paths,
    )
