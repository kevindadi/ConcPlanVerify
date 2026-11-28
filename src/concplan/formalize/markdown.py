"""Markdown and JSON parsers for pre-authored structured requirements (Entry B)."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from .models import DomainTypeEntity, GroundedEntities, RequirementClause, StructuredRequirements

_MD_CLAUSE_RE = re.compile(
    r"^\s*(?:[-*]\s*)?(?:\*\*)?(R\d+)[.:]?(?:\*\*)?\s*"
    r"(?:(\[V\]|\[U\])\s*)?"
    r"(?:\((safety|liveness|reachability|sequential_hole)\)\s*)?"
    r"(.+?)\s*$",
    re.IGNORECASE,
)
_MD_TITLE_RE = re.compile(r"^\s*#\s+(.+?)\s*$")
_BACKTICK_RE = re.compile(r"`([^`]+)`")


def _infer_clause_category(text: str) -> str:
    lower = text.lower()
    if any(k in lower for k in ("eventually", "starvation", "liveness", "always responds", "fair")):
        return "liveness"
    if any(k in lower for k in ("reachable", "can complete", "completes", "terminates")):
        return "reachability"
    return "safety"


def parse_structured_markdown(
    text: str,
    *,
    default_title: str = "requirements",
) -> StructuredRequirements:
    """Parse a structured ``REQUIREMENTS.md`` document with ``R1..Rn`` clauses and optional Entities."""
    lines = text.splitlines()
    title = default_title
    roles: list[str] = []
    resources: list[str] = []
    domain_types: list[DomainTypeEntity] = []
    milestones: list[str] = []
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
            if "milestone" in lower:
                milestones.extend(tokens)
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
            milestones=list(dict.fromkeys(milestones)),
        ),
    )


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
