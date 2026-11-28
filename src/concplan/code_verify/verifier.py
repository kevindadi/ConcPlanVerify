"""Core static verification datatypes, plugin protocol, and composite runner."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Protocol, runtime_checkable


@dataclass
class StaticViolation:
    plugin: str
    obligation: str
    kind: str
    message: str
    hole_id: str | None = None
    sid: str | None = None
    req_clauses: list[str] = field(default_factory=list)
    rust_span: str | None = None
    witness_trace: list[dict[str, Any]] = field(default_factory=list)
    expected_spec_ops: list[str] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "plugin": self.plugin,
            "obligation": self.obligation,
            "kind": self.kind,
            "message": self.message,
            "hole_id": self.hole_id,
            "sid": self.sid,
            "req_clauses": list(self.req_clauses),
            "rust_span": self.rust_span,
            "witness_trace": list(self.witness_trace),
            "expected_spec_ops": list(self.expected_spec_ops),
        }


@dataclass
class StaticVerificationReport:
    """Outcome of one plugin or a composite verification pass."""

    status: str
    plugin: str
    violations: list[StaticViolation] = field(default_factory=list)
    checks: dict[str, bool] = field(default_factory=dict)
    plugin_reports: list[dict[str, Any]] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)
    error: str | None = None

    @property
    def ok(self) -> bool:
        return self.status == "pass"

    @property
    def offending_holes(self) -> list[str]:
        seen: list[str] = []
        for v in self.violations:
            if v.hole_id and v.hole_id not in seen:
                seen.append(v.hole_id)
        return seen

    def as_dict(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "plugin": self.plugin,
            "ok": self.ok,
            "offending_holes": self.offending_holes,
            "violations": [v.as_dict() for v in self.violations],
            "checks": dict(self.checks),
            "plugin_reports": list(self.plugin_reports),
            "metadata": dict(self.metadata),
            "error": self.error,
        }


@runtime_checkable
class StaticVerifierPlugin(Protocol):
    name: str

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport: ...


def aggregate_statuses(statuses: list[str]) -> str:
    """Aggregate plugin statuses without ever downgrading failures or unknowns."""
    for status in ("fail_skeleton", "fail_hole", "fail_refinement", "error", "unknown"):
        if status in statuses:
            return status
    return "pass"


class CompositeStaticVerifier:
    """Run an ordered sequence of :class:`StaticVerifierPlugin` instances and aggregate verdicts."""

    def __init__(
        self,
        plugins: list[StaticVerifierPlugin],
        *,
        stop_on_skeleton_fail: bool = True,
    ) -> None:
        self.plugins = list(plugins)
        self.stop_on_skeleton_fail = stop_on_skeleton_fail

    def verify(
        self,
        *,
        skeleton_dir: Path | str,
        filled_dir: Path | str,
        spec_path: Path | str | None = None,
        contract_path: Path | str | None = None,
        codegen_map_path: Path | str | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        if not self.plugins:
            return StaticVerificationReport(
                status="unknown",
                plugin="composite",
                error="no static verifier plugins configured",
            )

        skeleton = Path(skeleton_dir).expanduser().resolve()
        filled = Path(filled_dir).expanduser().resolve()
        spec = Path(spec_path).expanduser().resolve() if spec_path is not None else None
        contract = Path(contract_path).expanduser().resolve() if contract_path is not None else None
        cmap = Path(codegen_map_path).expanduser().resolve() if codegen_map_path is not None else None

        all_violations: list[StaticViolation] = []
        merged_checks: dict[str, bool] = {}
        plugin_reports: list[dict[str, Any]] = []
        merged_meta: dict[str, Any] = {}
        statuses: list[str] = []
        errors: list[str] = []

        for plugin in self.plugins:
            rep = plugin.verify(
                skeleton_dir=skeleton,
                filled_dir=filled,
                spec_path=spec,
                contract_path=contract,
                codegen_map_path=cmap,
                fills=fills,
            )
            statuses.append(rep.status)
            plugin_reports.append(rep.as_dict())
            all_violations.extend(rep.violations)
            merged_checks.update(rep.checks)
            merged_meta.update(rep.metadata)
            if rep.error:
                errors.append(f"{plugin.name}: {rep.error}")

            if cmap is None and rep.metadata.get("codegen_filled_path"):
                candidate = Path(rep.metadata["codegen_filled_path"])
                if candidate.is_file():
                    cmap = candidate

            if self.stop_on_skeleton_fail and rep.status == "fail_skeleton":
                break

        return StaticVerificationReport(
            status=aggregate_statuses(statuses),
            plugin="composite",
            violations=all_violations,
            checks=merged_checks,
            plugin_reports=plugin_reports,
            metadata=merged_meta,
            error="; ".join(errors) if errors else None,
        )
