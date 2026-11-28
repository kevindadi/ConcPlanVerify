"""End-to-end bi-level verification driver (Stage 0 + Loops A, B, C)."""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any, Callable

from ..code_verify.conformance import (
    codegen as default_codegen,
    collect_traces,
    conform_all,
    holes_of,
    type_holes_of,
)
from ..code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    SpnMirRefinementPlugin,
    StaticVerificationReport,
    augment_codegen_map_with_hole_ranges,
    fill_holes_marked,
)
from ..core.concir_client import ConcirClient
from ..core.json_utils import exclusive_run_dir, extract_json, write_json
from ..core.prompts import build_check_feedback, build_explore_feedback, render_feedback
from ..core.providers import CandidateProvider, CandidateRequest
from ..formalize import (
    explain_contract_data,
    formalize_from_nl,
    formalize_from_structured,
    nl_to_clauses,
)
from ..model_verify.runner import verify_model_candidate
from .fill_prompts import build_hole_fill_prompt, build_hole_repair_prompt, parse_hole_reply
from .manifest import E2EResult, EscalationEvent, build_traceability_manifest


def run_e2e_pipeline(
    *,
    requirements: str | None = None,
    contract: dict[str, Any] | None = None,
    client: ConcirClient,
    cir_provider: CandidateProvider,
    hole_provider: CandidateProvider | None = None,
    out_dir: Path | str,
    static_verifier: CompositeStaticVerifier | None = None,
    max_cir_rounds: int = 4,
    max_hole_rounds: int = 3,
    max_escalations: int = 2,
    engine: str = "petri",
    initial_program: Path | str | None = None,
    run_summarize: bool = False,
    run_dynamic_conform: bool = False,
    native_runs: int = 10,
    miri_seeds: int = 0,
    requirements_clauses: dict[str, str] | None = None,
    codegen_fn: Callable[[Path, Path], dict[str, Any]] | None = None,
    nl_spec: str | None = None,
    clause_provider: CandidateProvider | None = None,
    contract_provider: CandidateProvider | None = None,
    modular_refinement: bool = False,
    focus_modules: list[str] | None = None,
    req_ir: dict[str, Any] | Path | str | None = None,
) -> E2EResult:
    """Run the complete traceable bi-level verification pipeline (Stage 0 + Loops A, B, and C)."""
    run_dir = exclusive_run_dir(out_dir, prefix="e2e")
    result = E2EResult(status="failed", out_dir=str(run_dir))

    # ---- Stage 0: Autoformalization ----
    formalization_dict: dict[str, Any] | None = None
    nl_explanations_dict: dict[str, Any] | None = None
    active_requirements: str = requirements or ""
    active_contract: dict[str, Any] | None = contract
    active_clauses: dict[str, str] | None = dict(requirements_clauses) if requirements_clauses else None
    active_req_ir: dict[str, Any] | None = None
    if req_ir is not None:
        if isinstance(req_ir, dict):
            active_req_ir = dict(req_ir)
        else:
            active_req_ir = json.loads(Path(req_ir).expanduser().resolve().read_text(encoding="utf-8"))

    def _write_e2e_result() -> None:
        write_json(run_dir / "e2e_result.json", result.as_dict())

    def _apply_stage0(form_res) -> bool:
        nonlocal formalization_dict, nl_explanations_dict, active_requirements, active_contract, active_clauses, active_req_ir
        formalization_dict = form_res.as_dict()
        nl_explanations_dict = form_res.explanation.as_dict()
        active_requirements = form_res.requirements_md
        active_contract = form_res.contract
        active_clauses = active_clauses or form_res.requirements.clause_map()
        if active_req_ir is None and form_res.req_ir is not None:
            active_req_ir = form_res.req_ir
        if form_res.req_check is not None:
            result.req_net_check = form_res.req_check
        result.formalization = formalization_dict
        result.nl_explanations = nl_explanations_dict
        if not form_res.sanity.ok:
            result.error = (
                "Stage 0 contract sanity check failed: "
                + "; ".join(i.message for i in form_res.sanity.errors)
            )
            _write_e2e_result()
            return False
        return True

    if nl_spec is not None and active_contract is None:
        try:
            form_res = formalize_from_nl(
                nl_spec,
                clause_provider=clause_provider or cir_provider,
                contract_provider=contract_provider or clause_provider or cir_provider,
                client=client,
                out_dir=run_dir / "stage0",
                req_ir=active_req_ir,
            )
        except Exception as exc:  # noqa: BLE001
            result.error = f"Stage 0 formalize_from_nl failed: {exc}"
            _write_e2e_result()
            return result
        if not _apply_stage0(form_res):
            return result
    elif nl_spec is not None and active_contract is not None:
        if clause_provider is not None:
            try:
                structured = nl_to_clauses(nl_spec, clause_provider)
                form_res = formalize_from_structured(
                    structured,
                    contract=active_contract,
                    client=client,
                    out_dir=run_dir / "stage0",
                    req_ir=active_req_ir,
                )
                _apply_stage0(form_res)
            except Exception as exc:  # noqa: BLE001
                result.error = f"Stage 0 nl_to_clauses failed: {exc}"
                _write_e2e_result()
                return result
        else:
            active_requirements = active_requirements or nl_spec
            nl_explanations_dict = explain_contract_data(active_contract, client=client).as_dict()
            result.nl_explanations = nl_explanations_dict
    elif active_contract is None:
        if not active_requirements:
            raise ValueError("run_e2e_pipeline requires requirements or nl_spec")
        try:
            form_res = formalize_from_structured(
                active_requirements,
                contract_provider=contract_provider or cir_provider,
                client=client,
                out_dir=run_dir / "stage0",
                req_ir=active_req_ir,
            )
        except Exception as exc:  # noqa: BLE001
            result.error = f"Stage 0 formalize_from_structured failed: {exc}"
            _write_e2e_result()
            return result
        if not _apply_stage0(form_res):
            return result
    else:
        nl_explanations_dict = explain_contract_data(active_contract, client=client).as_dict()
        result.nl_explanations = nl_explanations_dict

    assert active_contract is not None
    requirements = active_requirements
    contract = active_contract
    requirements_clauses = active_clauses

    contract_path = write_json(run_dir / "contract.json", contract)
    req_ir_path: Path | None = None
    if active_req_ir is not None:
        req_ir_path = write_json(run_dir / "req_ir.json", active_req_ir)
        if result.req_net_check is None:
            chk_res = client.check_req(
                req_ir_path, contract=contract_path, out_path=run_dir / "req_check.json"
            )
            result.req_net_check = chk_res.payload or {
                "status": chk_res.status,
                "error": chk_res.error,
            }
            if not chk_res.is_semantic:
                result.error = chk_res.error or f"req_ir check-req failed: {chk_res.status}"
                _write_e2e_result()
                return result
            if chk_res.status in ("fail", "invalid", "unsupported"):
                result.error = f"Requirement Net N_req check-req returned {chk_res.status}"
                _write_e2e_result()
                return result
            if chk_res.status == "unknown":
                result.status = "unknown"
                result.error = "Requirement Net N_req check-req returned unknown (truncated)"
                _write_e2e_result()
                return result

    hole_prov = hole_provider or cir_provider
    verifier = static_verifier or CompositeStaticVerifier([
        AstHolePurityPlugin(),
        SpnMirRefinementPlugin(
            client=client,
            modular=modular_refinement,
            focus_modules=focus_modules,
        ),
    ])

    def _run_codegen(prog: Path, skel: Path) -> dict[str, Any]:
        if codegen_fn is not None:
            return codegen_fn(prog, skel)
        return default_codegen(prog, skel, binary=client.binary)

    escalation_feedback: dict[str, Any] | None = None
    last_verified_program_dict: dict[str, Any] | None = None
    last_verified_text: str | None = None
    last_trace_map: dict[str, Any] | None = None
    last_codegen_filled: dict[str, Any] | None = None
    last_static_report: StaticVerificationReport | None = None
    use_initial = initial_program

    def _finalize(status: str, error: str | None = None) -> E2EResult:
        result.status = status
        result.error = error
        manifest = build_traceability_manifest(
            status=status,
            requirements=requirements,
            contract=contract,
            program_dict=last_verified_program_dict,
            trace_map=last_trace_map,
            codegen_filled=last_codegen_filled,
            model_report=result.model_report,
            static_report=last_static_report,
            dynamic_report=result.dynamic_report,
            escalations=result.escalations,
            hole_rounds=result.hole_rounds,
            requirements_clauses=requirements_clauses,
            formalization=formalization_dict,
            nl_explanations=nl_explanations_dict,
            req_net_check=result.req_net_check,
            design_refinement=result.design_refinement,
            client=client,
        )
        manifest_path = write_json(run_dir / "traceability_manifest.json", manifest)
        result.manifest = manifest
        result.manifest_path = str(manifest_path)
        result.vacuity_status = manifest.get("vacuity_status")
        result.modular_reports = manifest.get("modular_reports")
        _write_e2e_result()
        return result

    # Outer Loop C: cross-layer escalation epochs (0 .. max_escalations inclusive).
    for epoch in range(max_escalations + 1):
        epoch_dir = run_dir / f"epoch-{epoch}"
        epoch_dir.mkdir(parents=True, exist_ok=True)

        verified_cir_path: Path | None = None
        verified_program_dict: dict[str, Any] | None = None
        epoch_trace_map: dict[str, Any] | None = None
        epoch_trace_map_path: Path | None = None
        cir_feedback: dict[str, Any] | None = escalation_feedback
        previous_cir_text: str | None = last_verified_text

        for cir_attempt in range(1, max(1, max_cir_rounds) + 1):
            attempt_dir = epoch_dir / f"cir-{cir_attempt}"
            attempt_dir.mkdir(parents=True, exist_ok=True)
            round_rec: dict[str, Any] = {
                "epoch": epoch,
                "attempt": cir_attempt,
                "escalated": escalation_feedback is not None and cir_attempt == 1,
            }
            result.cir_rounds.append(round_rec)

            if use_initial is not None:
                raw_text = Path(use_initial).expanduser().resolve().read_text(encoding="utf-8")
                round_rec["source"] = "initial"
                use_initial = None
            else:
                req = CandidateRequest(
                    requirements=requirements,
                    contract=contract,
                    feedback=render_feedback(cir_feedback) if cir_feedback else None,
                    attempt=cir_attempt,
                    previous_candidate=previous_cir_text,
                    current_program=(
                        json.dumps(last_verified_program_dict, ensure_ascii=False)
                        if last_verified_program_dict is not None
                        else None
                    ),
                )
                resp = cir_provider.propose(req)
                round_rec["source"] = resp.source
                if resp.error:
                    round_rec["result"] = "provider_error"
                    round_rec["error"] = resp.error
                    cir_feedback = {"stage": "cir_provider", "error": resp.error}
                    continue
                raw_text = resp.text

            (attempt_dir / "raw.txt").write_text(raw_text, encoding="utf-8")
            previous_cir_text = raw_text

            try:
                parsed = json.loads(extract_json(raw_text))
            except json.JSONDecodeError as exc:
                round_rec["result"] = "parse_error"
                cir_feedback = {"stage": "parse", "error": f"invalid JSON: {exc}"}
                continue
            if not isinstance(parsed, dict):
                round_rec["result"] = "parse_error"
                cir_feedback = {"stage": "parse", "error": "candidate must be a JSON object"}
                continue

            raw_cir_path = attempt_dir / "candidate.cir.json"
            raw_cir_path.write_text(
                json.dumps(parsed, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )

            ver = verify_model_candidate(
                client=client,
                program_dict=parsed,
                program_path=raw_cir_path,
                contract_path=contract_path,
                attempt_dir=attempt_dir,
                engine=engine,
                run_summarize=run_summarize,
                req_ir_path=req_ir_path,
            )
            if ver.lower_res is not None:
                round_rec["lower_status"] = ver.lower_res.status
            if ver.trace_map is not None:
                epoch_trace_map = ver.trace_map
                epoch_trace_map_path = ver.trace_map_path
            if ver.check_res is not None:
                round_rec["check_status"] = ver.check_res.status
            if ver.support_res is not None:
                round_rec["support_status"] = ver.support_res.status
            if ver.summarize_res is not None:
                round_rec["summarize_status"] = ver.summarize_res.status
            if ver.explore_res is not None:
                round_rec["explore_outcome"] = ver.explore_res.outcome
                round_rec["explore_complete"] = ver.explore_res.complete
                result.model_report = ver.explore_res.payload
            if ver.design_res is not None:
                round_rec["design_refinement_status"] = ver.design_res.status
                result.design_refinement = ver.design_res.payload

            if ver.decision == "tool_error":
                return _finalize("tool_error", ver.error or "backend failed")
            if ver.decision == "design_refinement_unknown":
                return _finalize("unknown", ver.error)
            if ver.decision == "check_schema_error":
                round_rec["result"] = "check_schema_error"
                cir_feedback = ver.feedback
                continue
            if ver.decision == "check_invalid" and ver.check_res is not None:
                round_rec["result"] = "check_invalid"
                cir_feedback = build_check_feedback(ver.check_res)
                continue
            if ver.accepted:
                round_rec["result"] = "accepted"
                verified_cir_path = ver.active_cir_path
                verified_program_dict = ver.active_program
                break
            if ver.decision in (
                "lower_schema_error",
                "lower_invalid",
                "lower_fail",
                "lower_unsupported",
                "lower_unknown",
                "unsupported",
                "cross_module_lock_cycle",
            ) or ver.decision.startswith("design_refinement_"):
                round_rec["result"] = ver.decision
                cir_feedback = ver.feedback
                continue

            if ver.explore_res is not None:
                round_rec["result"] = f"explore_{ver.explore_res.status}"
                cir_feedback = build_explore_feedback(ver.explore_res)
            else:
                round_rec["result"] = ver.decision
                cir_feedback = ver.feedback

        if verified_cir_path is None or verified_program_dict is None:
            return _finalize(
                "failed",
                f"Stage 2 CIR verification did not reach complete PASS within {max_cir_rounds} rounds (epoch {epoch})",
            )

        last_verified_program_dict = verified_program_dict
        last_verified_text = json.dumps(verified_program_dict, ensure_ascii=False, indent=2)
        last_trace_map = epoch_trace_map
        result.verified_cir_path = str(verified_cir_path)
        result.trace_map_path = str(epoch_trace_map_path) if epoch_trace_map_path else None

        # ---- Stage 3: Codegen + Initial Hole Fill ----
        skeleton_dir = epoch_dir / "skeleton"
        filled_dir = epoch_dir / "filled"
        try:
            _run_codegen(verified_cir_path, skeleton_dir)
        except Exception as exc:  # noqa: BLE001
            return _finalize("tool_error", f"codegen failed: {exc}")

        result.skeleton_dir = str(skeleton_dir)
        result.filled_dir = str(filled_dir)

        holes = holes_of(skeleton_dir)
        type_holes = type_holes_of(skeleton_dir)
        all_hole_entries = list(type_holes) + list(holes)
        current_fills: dict[str, str] = {}
        escalate_now: EscalationEvent | None = None

        if all_hole_entries:
            prompt_text = build_hole_fill_prompt(skeleton_dir, requirements)
            hole_req = CandidateRequest(
                requirements=prompt_text,
                contract=contract,
                feedback=None,
                attempt=1,
                current_program=last_verified_text,
            )
            hole_resp = hole_prov.propose(hole_req)
            fill_rec: dict[str, Any] = {
                "epoch": epoch,
                "round": 0,
                "kind": "initial_fill",
                "target_holes": [h["id"] for h in all_hole_entries],
                "source": hole_resp.source,
            }
            result.hole_rounds.append(fill_rec)
            if hole_resp.error:
                return _finalize("failed", f"hole provider error: {hole_resp.error}")

            parsed_fills, esc = parse_hole_reply(hole_resp.text)
            if esc is not None:
                fill_rec["result"] = "escalate"
                fill_rec["escalation"] = esc
                escalate_now = EscalationEvent(
                    index=len(result.escalations) + 1,
                    source="hole_provider_explicit",
                    reason=esc["reason"],
                    module=esc.get("module"),
                    offending_holes=[h["id"] for h in all_hole_entries],
                    req_clauses=sorted({str(r) for h in all_hole_entries for r in (h.get("req") or [])}),
                )
            elif parsed_fills is None:
                return _finalize("failed", "hole provider returned invalid JSON")
            else:
                current_fills = parsed_fills
                fill_rec["result"] = "filled"
                fill_rec["applied"] = sorted(current_fills.keys())

            if escalate_now is not None:
                result.escalations.append(escalate_now.as_dict())
                if epoch < max_escalations:
                    escalation_feedback = {
                        "stage": "cross_layer_escalation",
                        "escalation": escalate_now.as_dict(),
                        "instruction": (
                            "Hole implementation requested a CIR/SCIR architecture revision. "
                            "Update the CIR program to address the escalation reason while "
                            "preserving all contract properties."
                        ),
                    }
                    continue
                return _finalize(
                    "failed",
                    f"cross-layer escalation exhausted ({max_escalations}): {escalate_now.reason}",
                )

            fill_info = fill_holes_marked(skeleton_dir, filled_dir, current_fills)
            cmap_path = Path(fill_info["codegen_filled_path"])
            last_codegen_filled = json.loads(cmap_path.read_text(encoding="utf-8"))
        else:
            if filled_dir.exists():
                shutil.rmtree(filled_dir)
            shutil.copytree(skeleton_dir, filled_dir)
            cmap_path = filled_dir / "codegen_filled.json"
            last_codegen_filled = augment_codegen_map_with_hole_ranges(
                skeleton_dir, filled_dir, {}, out_path=cmap_path
            )

        result.fills = dict(current_fills)

        # ---- Stage 4: Static Verification + Loop B (Hole-Local Repair) ----
        static_report = verifier.verify(
            skeleton_dir=skeleton_dir,
            filled_dir=filled_dir,
            spec_path=verified_cir_path,
            contract_path=contract_path,
            codegen_map_path=cmap_path,
            fills=current_fills,
        )
        last_static_report = static_report
        result.static_report = static_report.as_dict()
        if static_report.metadata.get("codegen_filled_path"):
            cf_path = Path(static_report.metadata["codegen_filled_path"])
            if cf_path.is_file():
                last_codegen_filled = json.loads(cf_path.read_text(encoding="utf-8"))

        repair_round = 0
        while (
            static_report.status in ("fail_hole", "fail_refinement")
            and static_report.offending_holes
            and repair_round < max_hole_rounds
        ):
            repair_round += 1
            offending = list(static_report.offending_holes)
            repair_prompt = build_hole_repair_prompt(
                skeleton_dir,
                requirements,
                offending,
                current_fills,
                static_report.violations,
            )
            repair_req = CandidateRequest(
                requirements=repair_prompt,
                contract=contract,
                feedback=json.dumps(
                    [v.as_dict() for v in static_report.violations],
                    ensure_ascii=False,
                    indent=2,
                ),
                attempt=repair_round,
                previous_candidate=json.dumps(
                    {h: current_fills.get(h, "") for h in offending}, ensure_ascii=False
                ),
                current_program=last_verified_text,
            )
            repair_resp = hole_prov.propose(repair_req)
            rep_rec: dict[str, Any] = {
                "epoch": epoch,
                "round": repair_round,
                "kind": "hole_repair",
                "target_holes": offending,
                "source": repair_resp.source,
            }
            result.hole_rounds.append(rep_rec)
            if repair_resp.error:
                rep_rec["result"] = "provider_error"
                rep_rec["error"] = repair_resp.error
                break

            parsed_patch, esc = parse_hole_reply(repair_resp.text)
            if esc is not None:
                rep_rec["result"] = "escalate"
                rep_rec["escalation"] = esc
                escalate_now = EscalationEvent(
                    index=len(result.escalations) + 1,
                    source="hole_provider_explicit",
                    reason=esc["reason"],
                    module=esc.get("module"),
                    offending_holes=offending,
                    req_clauses=sorted({r for v in static_report.violations for r in v.req_clauses}),
                    violations=[v.as_dict() for v in static_report.violations],
                )
                break

            if parsed_patch is None:
                rep_rec["result"] = "parse_error"
                continue

            updated: list[str] = []
            for hid in offending:
                if hid in parsed_patch:
                    current_fills[hid] = parsed_patch[hid]
                    updated.append(hid)
            rep_rec["updated_holes"] = updated
            result.fills = dict(current_fills)

            fill_info = fill_holes_marked(skeleton_dir, filled_dir, current_fills)
            cmap_path = Path(fill_info["codegen_filled_path"])
            last_codegen_filled = json.loads(cmap_path.read_text(encoding="utf-8"))

            static_report = verifier.verify(
                skeleton_dir=skeleton_dir,
                filled_dir=filled_dir,
                spec_path=verified_cir_path,
                contract_path=contract_path,
                codegen_map_path=cmap_path,
                fills=current_fills,
            )
            last_static_report = static_report
            result.static_report = static_report.as_dict()
            rep_rec["verify_status"] = static_report.status
            if static_report.status == "pass":
                rep_rec["result"] = "repaired"
                break
            rep_rec["result"] = static_report.status

        if escalate_now is None and static_report.status == "pass":
            if run_dynamic_conform:
                traces = collect_traces(
                    filled_dir,
                    native_runs=native_runs,
                    miri_seeds=miri_seeds,
                    run_miri=miri_seeds > 0,
                    calls_dir=epoch_dir / "dynamic-traces",
                )
                dyn = conform_all(verified_cir_path, traces, binary=client.binary)
                result.dynamic_report = dyn
                if dyn.get("violation", 0) > 0:
                    return _finalize(
                        "failed",
                        f"dynamic conformance reported {dyn['violation']} violation(s)",
                    )
            return _finalize("verified")

        if escalate_now is None and static_report.status == "unknown":
            return _finalize(
                "unknown",
                static_report.error or "static verification returned unknown",
            )

        if escalate_now is None:
            if static_report.offending_holes:
                src = "hole_repair_exhausted"
                reason = (
                    f"hole-local repair exhausted {max_hole_rounds} round(s) with "
                    f"unresolved violations in holes {static_report.offending_holes}"
                )
            elif static_report.status == "fail_skeleton":
                src = "skeleton_fail"
                reason = "skeleton integrity check failed outside holes"
            else:
                src = "refinement_non_hole"
                reason = (
                    static_report.error
                    or f"static verification failed with status '{static_report.status}'"
                )
            escalate_now = EscalationEvent(
                index=len(result.escalations) + 1,
                source=src,
                reason=reason,
                module=None,
                offending_holes=list(static_report.offending_holes),
                req_clauses=sorted({r for v in static_report.violations for r in v.req_clauses}),
                violations=[v.as_dict() for v in static_report.violations],
            )

        result.escalations.append(escalate_now.as_dict())
        if epoch < max_escalations:
            escalation_feedback = {
                "stage": "cross_layer_escalation",
                "escalation": escalate_now.as_dict(),
                "instruction": (
                    "Post-codegen static verification could not be resolved by hole-local repair. "
                    "Revise the CIR/SCIR architecture to address the violations/escalation."
                ),
            }
            continue

        return _finalize(
            "failed",
            f"pipeline failed after {max_escalations} escalation(s): {escalate_now.reason}",
        )

    return _finalize("failed", "pipeline terminated without verification")
