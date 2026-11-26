"""Data models for Stage 0 Auto formalization (NL / Structured -> Contract + Explanation)."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

CLAUSE_CATEGORIES: tuple[str, ...] = (
    "safety",
    "liveness",
    "reachability",
    "sequential_hole",
)


@dataclass
class DomainTypeEntity:
    """Grounded domain payload type discovered in requirements (often lowered to a TypeHole)."""

    name: str
    description: str = ""
    kind: str = "hole"  # "hole" | "struct" | "array" | "primitive"
    fields: list[dict[str, Any]] = field(default_factory=list)
    req: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "name": self.name,
            "description": self.description,
            "kind": self.kind,
        }
        if self.fields:
            out["fields"] = list(self.fields)
        if self.req:
            out["req"] = list(self.req)
        return out

    @classmethod
    def from_raw(cls, raw: Any) -> "DomainTypeEntity":
        if isinstance(raw, cls):
            return raw
        if isinstance(raw, str):
            return cls(name=raw.strip())
        if isinstance(raw, dict):
            return cls(
                name=str(raw.get("name") or raw.get("type_name") or "").strip(),
                description=str(raw.get("description") or raw.get("desc") or "").strip(),
                kind=str(raw.get("kind") or "hole").strip().lower(),
                fields=list(raw.get("fields") or []),
                req=[str(r) for r in (raw.get("req") or [])],
            )
        raise ValueError(f"invalid domain_type entry: {raw!r}")


@dataclass
class GroundedEntities:
    """Entities extracted from requirements to ground contract predicates and SCIR design."""

    roles: list[str] = field(default_factory=list)
    resources: list[str] = field(default_factory=list)
    domain_types: list[DomainTypeEntity] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "roles": list(self.roles),
            "resources": list(self.resources),
            "domain_types": [dt.as_dict() for dt in self.domain_types],
        }

    @classmethod
    def from_raw(cls, raw: Any) -> "GroundedEntities":
        if isinstance(raw, cls):
            return raw
        if not isinstance(raw, dict):
            return cls()
        roles = [str(r).strip() for r in (raw.get("roles") or raw.get("functions") or []) if str(r).strip()]
        resources = [str(r).strip() for r in (raw.get("resources") or []) if str(r).strip()]
        raw_dts = raw.get("domain_types") or raw.get("types") or []
        domain_types = [
            dt for dt in (DomainTypeEntity.from_raw(item) for item in raw_dts) if dt.name
        ]
        return cls(
            roles=list(dict.fromkeys(roles)),
            resources=list(dict.fromkeys(resources)),
            domain_types=domain_types,
        )


@dataclass
class RequirementClause:
    """One numbered atomic requirement clause (R1..Rn)."""

    id: str
    text: str
    category: str = "safety"  # safety | liveness | reachability | sequential_hole
    verifiable: bool = True
    entities: list[str] = field(default_factory=list)
    notes: str = ""

    def __post_init__(self) -> None:
        self.id = str(self.id).strip()
        self.text = str(self.text).strip()
        cat = str(self.category or "safety").strip().lower()
        if cat not in CLAUSE_CATEGORIES:
            cat = "sequential_hole" if not self.verifiable else "safety"
        self.category = cat
        if self.category == "sequential_hole":
            self.verifiable = False

    def as_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "id": self.id,
            "text": self.text,
            "category": self.category,
            "verifiable": bool(self.verifiable),
        }
        if self.entities:
            out["entities"] = list(self.entities)
        if self.notes:
            out["notes"] = self.notes
        return out

    @classmethod
    def from_raw(cls, raw: Any, *, fallback_index: int = 1) -> "RequirementClause":
        if isinstance(raw, cls):
            return raw
        if isinstance(raw, str):
            return cls(id=f"R{fallback_index}", text=raw.strip())
        if not isinstance(raw, dict):
            raise ValueError(f"invalid requirement clause: {raw!r}")
        cid = str(raw.get("id") or f"R{fallback_index}").strip()
        text = str(raw.get("text") or raw.get("description") or raw.get("clause") or "").strip()
        raw_cat = str(raw.get("category") or "").strip().lower()
        if "verifiable" in raw and isinstance(raw["verifiable"], bool):
            verifiable = raw["verifiable"]
        else:
            verifiable = raw_cat != "sequential_hole" and "[u]" not in text.lower()
        if not raw_cat:
            raw_cat = "sequential_hole" if not verifiable else "safety"
        if raw_cat == "sequential_hole":
            verifiable = False
        entities = [str(e).strip() for e in (raw.get("entities") or []) if str(e).strip()]
        notes = str(raw.get("notes") or "").strip()
        return cls(
            id=cid,
            text=text,
            category=raw_cat,
            verifiable=verifiable,
            entities=entities,
            notes=notes,
        )


@dataclass
class StructuredRequirements:
    """Canonical structured requirement specification (output of Step 0.1 or Entry B)."""

    title: str
    clauses: list[RequirementClause]
    entities: GroundedEntities = field(default_factory=GroundedEntities)
    summary: str = ""
    source_nl: str | None = None
    schema: str = "concir-requirements/2"

    def clause_map(self) -> dict[str, str]:
        return {c.id: c.text for c in self.clauses}

    def clause_by_id(self) -> dict[str, RequirementClause]:
        return {c.id: c for c in self.clauses}

    def verifiable_clauses(self) -> list[RequirementClause]:
        return [c for c in self.clauses if c.verifiable]

    def verifiable_clause_ids(self) -> set[str]:
        return {c.id for c in self.clauses if c.verifiable}

    def unverifiable_clause_ids(self) -> set[str]:
        return {c.id for c in self.clauses if not c.verifiable}

    def to_markdown(self) -> str:
        lines: list[str] = [f"# {self.title or 'Requirements'}", ""]
        if self.summary:
            lines.extend([self.summary.strip(), ""])

        lines.append("## Entities")
        roles_str = ", ".join(f"`{r}`" for r in self.entities.roles) if self.entities.roles else "_none_"
        res_str = (
            ", ".join(f"`{r}`" for r in self.entities.resources)
            if self.entities.resources
            else "_none_"
        )
        lines.append(f"- **Roles / Functions**: {roles_str}")
        lines.append(f"- **Resources**: {res_str}")
        if self.entities.domain_types:
            dt_parts = []
            for dt in self.entities.domain_types:
                desc = f": {dt.description}" if dt.description else ""
                dt_parts.append(f"`{dt.name}` ({dt.kind}{desc})")
            lines.append(f"- **Domain Types**: {', '.join(dt_parts)}")
        else:
            lines.append("- **Domain Types**: _none_")
        lines.append("")

        lines.append("## Requirements")
        for c in self.clauses:
            marker = "[V]" if c.verifiable else "[U]"
            lines.append(f"- **{c.id}.** {marker} ({c.category}) {c.text}")
        lines.append("")
        return "\n".join(lines)

    def as_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "schema": self.schema,
            "title": self.title,
            "summary": self.summary,
            "entities": self.entities.as_dict(),
            "clauses": [c.as_dict() for c in self.clauses],
            "requirements": [c.as_dict() for c in self.clauses],
        }
        if self.source_nl is not None:
            out["source_nl"] = self.source_nl
        return out

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "StructuredRequirements":
        title = str(data.get("title") or data.get("name") or data.get("case") or "requirements").strip()
        summary = str(data.get("summary") or data.get("description") or "").strip()
        entities = GroundedEntities.from_raw(data.get("entities") or {})
        raw_clauses = data.get("clauses")
        if raw_clauses is None:
            raw_clauses = data.get("requirements") or []
        clauses = [
            RequirementClause.from_raw(item, fallback_index=i + 1)
            for i, item in enumerate(raw_clauses)
        ]
        source_nl = data.get("source_nl")
        return cls(
            title=title,
            clauses=clauses,
            entities=entities,
            summary=summary,
            source_nl=str(source_nl) if source_nl is not None else None,
        )


@dataclass
class SanityIssue:
    """One finding emitted by Step 0.3 contract sanity checking."""

    severity: str  # "error" | "warning"
    code: str
    message: str
    property_id: str | None = None
    clause_id: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return {
            "severity": self.severity,
            "code": self.code,
            "message": self.message,
            "property_id": self.property_id,
            "clause_id": self.clause_id,
        }


@dataclass
class SanityCheckReport:
    """Aggregate result of Step 0.3 contract sanity checks."""

    ok: bool
    issues: list[SanityIssue] = field(default_factory=list)
    checked_properties: int = 0
    checked_preserved: int = 0

    @property
    def errors(self) -> list[SanityIssue]:
        return [i for i in self.issues if i.severity == "error"]

    @property
    def warnings(self) -> list[SanityIssue]:
        return [i for i in self.issues if i.severity != "error"]

    def as_dict(self) -> dict[str, Any]:
        return {
            "ok": self.ok,
            "checked_properties": self.checked_properties,
            "checked_preserved": self.checked_preserved,
            "error_count": len(self.errors),
            "warning_count": len(self.warnings),
            "issues": [i.as_dict() for i in self.issues],
        }


@dataclass
class ContractExplanation:
    """Deterministic natural-language back-translation of a verification contract."""

    name: str
    properties: list[dict[str, Any]] = field(default_factory=list)
    preserved: list[dict[str, Any]] = field(default_factory=list)
    schema: str = "concir-contract-explanation/1"
    source: str = "python"

    def as_dict(self) -> dict[str, Any]:
        return {
            "schema": self.schema,
            "name": self.name,
            "source": self.source,
            "properties": list(self.properties),
            "preserved": list(self.preserved),
        }

    def render_markdown(self) -> str:
        lines = [f"# Contract Explanation: {self.name}", ""]
        lines.append("## Properties")
        if not self.properties:
            lines.append("- _none_")
        for p in self.properties:
            req_tag = f" [req: {', '.join(p.get('req') or [])}]" if p.get("req") else ""
            nv_tag = " (non-vacuous)" if p.get("non_vacuous") else ""
            lines.append(
                f"- **`{p.get('id')}`** (`{p.get('kind')}`{nv_tag}){req_tag}: {p.get('explanation')}"
            )
        lines.append("")
        lines.append("## Preserved Behaviors")
        if not self.preserved:
            lines.append("- _none_")
        for pr in self.preserved:
            req_tag = f" [req: {', '.join(pr.get('req') or [])}]" if pr.get("req") else ""
            lines.append(f"- **{pr.get('description', 'preserved')}**{req_tag}: {pr.get('explanation')}")
        lines.append("")
        return "\n".join(lines)


@dataclass
class FormalizationResult:
    """End-to-end result of Stage 0 autoformalization (Entry A or Entry B)."""

    entry: str  # "nl" | "structured"
    requirements: StructuredRequirements
    requirements_md: str
    contract: dict[str, Any]
    explanation: ContractExplanation
    sanity: SanityCheckReport
    out_dir: str | None = None
    requirements_md_path: str | None = None
    requirements_json_path: str | None = None
    contract_path: str | None = None
    explanation_path: str | None = None
    warnings: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return self.sanity.ok

    def as_dict(self) -> dict[str, Any]:
        return {
            "entry": self.entry,
            "ok": self.ok,
            "requirements": self.requirements.as_dict(),
            "requirements_md": self.requirements_md,
            "contract": self.contract,
            "explanation": self.explanation.as_dict(),
            "sanity": self.sanity.as_dict(),
            "out_dir": self.out_dir,
            "requirements_md_path": self.requirements_md_path,
            "requirements_json_path": self.requirements_json_path,
            "contract_path": self.contract_path,
            "explanation_path": self.explanation_path,
            "warnings": list(self.warnings),
        }
