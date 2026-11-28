"""Shared Stage 2 model verification runner (lower -> check -> support -> summarize -> explore -> verify-design)."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient, ConcirResult


def has_structured_body(program: dict[str, Any]) -> bool:
    """Return True if any module function carries SCIR ``structured_body``."""
    for mod in program.get("modules", []) or []:
        if not isinstance(mod, dict):
            continue
        for fn in mod.get("functions", []) or []:
            if isinstance(fn, dict) and fn.get("structured_body"):
                return True
    return False


@dataclass
class ModelVerificationOutcome:
    """Structured result of verifying one CIR/SCIR candidate through the backend pipeline."""

    decision: str
    accepted: bool
    active_cir_path: Path
    active_program: dict[str, Any]
    trace_map: dict[str, Any] | None = None
    trace_map_path: Path | None = None
    lower_res: ConcirResult | None = None
    check_res: ConcirResult | None = None
    support_res: ConcirResult | None = None
    summarize_res: ConcirResult | None = None
    explore_res: ConcirResult | None = None
    design_res: ConcirResult | None = None
    tool_wall_ms: int = 0
    error: str | None = None
    feedback: dict[str, Any] = field(default_factory=dict)


def verify_model_candidate(
    *,
    client: ConcirClient,
    program_dict: dict[str, Any],
    program_path: Path,
    contract_path: Path,
    attempt_dir: Path,
    engine: str = "petri",
    run_summarize: bool = False,
    req_ir_path: Path | None = None,
) -> ModelVerificationOutcome:
    """Run lowering, static check, support check, optional summarize, model exploration, and optional ReqIR refinement."""
    attempt_dir.mkdir(parents=True, exist_ok=True)
    active_cir_path = program_path
    active_program = program_dict
    trace_map: dict[str, Any] | None = None
    trace_map_path: Path | None = None
    wall_ms = 0

    lower_res: ConcirResult | None = None
    if has_structured_body(program_dict):
        lowered_path = attempt_dir / "lowered.cir.json"
        tm_path = attempt_dir / "trace_map.json"
        lower_res = client.lower(program_path, out_path=lowered_path, trace_map_path=tm_path)
        wall_ms += lower_res.wall_ms
        if lower_res.kind != "semantic":
            decision = "lower_schema_error" if lower_res.kind in ("usage_error", "protocol_error") else "tool_error"
            return ModelVerificationOutcome(
                decision=decision,
                accepted=False,
                active_cir_path=active_cir_path,
                active_program=active_program,
                lower_res=lower_res,
                tool_wall_ms=wall_ms,
                error=lower_res.error or lower_res.stderr or "lower failed",
                feedback={"stage": "lower", "error": lower_res.error or lower_res.stderr},
            )
        if lower_res.status != "pass":
            return ModelVerificationOutcome(
                decision=f"lower_{lower_res.status}",
                accepted=False,
                active_cir_path=active_cir_path,
                active_program=active_program,
                lower_res=lower_res,
                tool_wall_ms=wall_ms,
                feedback={"stage": "lower", "status": lower_res.status, "payload": lower_res.payload},
            )
        if lowered_path.is_file():
            active_cir_path = lowered_path
            active_program = json.loads(lowered_path.read_text(encoding="utf-8"))
        elif isinstance(lower_res.payload, dict) and "modules" in lower_res.payload:
            lowered_path.write_text(
                json.dumps(lower_res.payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            active_cir_path = lowered_path
            active_program = lower_res.payload
        if tm_path.is_file():
            trace_map = json.loads(tm_path.read_text(encoding="utf-8"))
            trace_map_path = tm_path

    check_res = client.check(active_cir_path)
    wall_ms += check_res.wall_ms
    if check_res.kind != "semantic":
        decision = "check_schema_error" if check_res.kind in ("usage_error", "protocol_error") else "tool_error"
        return ModelVerificationOutcome(
            decision=decision,
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            tool_wall_ms=wall_ms,
            error=check_res.error or check_res.stderr or "check failed",
            feedback={"stage": "check", "error": check_res.error or check_res.stderr},
        )
    if check_res.status != "valid":
        return ModelVerificationOutcome(
            decision="check_invalid",
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            tool_wall_ms=wall_ms,
        )

    support_res = client.support(active_cir_path)
    wall_ms += support_res.wall_ms
    if support_res.kind != "semantic":
        return ModelVerificationOutcome(
            decision="tool_error",
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            support_res=support_res,
            tool_wall_ms=wall_ms,
            error=support_res.error or "support failed",
        )
    if support_res.status != "supported":
        return ModelVerificationOutcome(
            decision="unsupported",
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            support_res=support_res,
            tool_wall_ms=wall_ms,
            feedback={"stage": "support", "status": support_res.status, "unsupported": support_res.unsupported},
        )

    summarize_res: ConcirResult | None = None
    if run_summarize:
        summarize_res = client.summarize(active_cir_path, out_path=attempt_dir / "module_summary.json")
        wall_ms += summarize_res.wall_ms
        if summarize_res.kind == "semantic" and summarize_res.status == "fail":
            return ModelVerificationOutcome(
                decision="cross_module_lock_cycle",
                accepted=False,
                active_cir_path=active_cir_path,
                active_program=active_program,
                trace_map=trace_map,
                trace_map_path=trace_map_path,
                lower_res=lower_res,
                check_res=check_res,
                support_res=support_res,
                summarize_res=summarize_res,
                tool_wall_ms=wall_ms,
                feedback={"stage": "summarize", "status": "fail", "summary": summarize_res.payload},
            )

    explore_res = client.explore(active_cir_path, contract_path, engine)
    wall_ms += explore_res.wall_ms
    if explore_res.kind != "semantic":
        return ModelVerificationOutcome(
            decision="tool_error",
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            support_res=support_res,
            summarize_res=summarize_res,
            explore_res=explore_res,
            tool_wall_ms=wall_ms,
            error=explore_res.error or "explore failed",
        )

    design_res: ConcirResult | None = None
    if explore_res.outcome == "PASS" and explore_res.complete is True:
        if req_ir_path is not None:
            design_res = client.verify_design(
                req_ir_path,
                active_cir_path,
                contract=contract_path,
                out_path=attempt_dir / "design_refinement.json",
            )
            wall_ms += design_res.wall_ms
            if not design_res.is_semantic:
                return ModelVerificationOutcome(
                    decision="tool_error",
                    accepted=False,
                    active_cir_path=active_cir_path,
                    active_program=active_program,
                    trace_map=trace_map,
                    trace_map_path=trace_map_path,
                    lower_res=lower_res,
                    check_res=check_res,
                    support_res=support_res,
                    summarize_res=summarize_res,
                    explore_res=explore_res,
                    design_res=design_res,
                    tool_wall_ms=wall_ms,
                    error=design_res.error or "verify-design failed",
                )
            if design_res.status == "unknown":
                return ModelVerificationOutcome(
                    decision="design_refinement_unknown",
                    accepted=False,
                    active_cir_path=active_cir_path,
                    active_program=active_program,
                    trace_map=trace_map,
                    trace_map_path=trace_map_path,
                    lower_res=lower_res,
                    check_res=check_res,
                    support_res=support_res,
                    summarize_res=summarize_res,
                    explore_res=explore_res,
                    design_res=design_res,
                    tool_wall_ms=wall_ms,
                    error="Stage 2 design refinement (N_req <=_ext N_code) returned unknown (truncated bounds)",
                )
            if design_res.status != "pass":
                payload = design_res.payload or {}
                return ModelVerificationOutcome(
                    decision=f"design_refinement_{design_res.status}",
                    accepted=False,
                    active_cir_path=active_cir_path,
                    active_program=active_program,
                    trace_map=trace_map,
                    trace_map_path=trace_map_path,
                    lower_res=lower_res,
                    check_res=check_res,
                    support_res=support_res,
                    summarize_res=summarize_res,
                    explore_res=explore_res,
                    design_res=design_res,
                    tool_wall_ms=wall_ms,
                    feedback={
                        "stage": "verify_design",
                        "status": design_res.status,
                        "missing_milestones": payload.get("missing_milestones", []),
                        "violations": payload.get("violations", []),
                        "checks": payload.get("checks", {}),
                    },
                )
        return ModelVerificationOutcome(
            decision="accepted",
            accepted=True,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            support_res=support_res,
            summarize_res=summarize_res,
            explore_res=explore_res,
            design_res=design_res,
            tool_wall_ms=wall_ms,
        )

    if explore_res.status in ("unknown", "invalid", "unsupported"):
        return ModelVerificationOutcome(
            decision=explore_res.status,
            accepted=False,
            active_cir_path=active_cir_path,
            active_program=active_program,
            trace_map=trace_map,
            trace_map_path=trace_map_path,
            lower_res=lower_res,
            check_res=check_res,
            support_res=support_res,
            summarize_res=summarize_res,
            explore_res=explore_res,
            tool_wall_ms=wall_ms,
        )

    return ModelVerificationOutcome(
        decision="explore_fail",
        accepted=False,
        active_cir_path=active_cir_path,
        active_program=active_program,
        trace_map=trace_map,
        trace_map_path=trace_map_path,
        lower_res=lower_res,
        check_res=check_res,
        support_res=support_res,
        summarize_res=summarize_res,
        explore_res=explore_res,
        tool_wall_ms=wall_ms,
    )
