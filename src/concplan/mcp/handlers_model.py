"""MCP tool handlers for Stage 0 contracts and Stage 1-2 SCIR/ReqIR model verification."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ..core.prompts import build_check_feedback, build_explore_feedback
from ..core.toolchain import find_concir_backend, find_lockbud, find_spn
from ..formalize import (
    ensure_implication_non_vacuous,
    explain_contract_data,
    parse_structured_requirements,
    validate_contract_sanity,
)
from ..pipeline.examples import examples_summary, load_example
from .protocol import ToolSpec
from .workspace import JSON_OR_PATH_SCHEMA, McpWorkspace, result_meta


def build_model_tool_specs(ws: McpWorkspace) -> list[ToolSpec]:
    """Build Stage 0-2 contract and concurrency model MCP tools."""

    def handle_toolchain_status(args: dict[str, Any]) -> dict[str, Any]:
        del args
        concir = find_concir_backend(ws.concir_binary)
        spn = find_spn(ws.spn_binary)
        lockbud = find_lockbud(ws.lockbud_binary)
        return {
            "workdir": str(ws.workdir),
            "concir_backend": {"available": concir is not None, "path": str(concir) if concir is not None else None},
            "spn": {"available": spn is not None, "path": str(spn) if spn is not None else None},
            "lockbud": {"available": lockbud is not None, "path": str(lockbud) if lockbud is not None else None},
        }

    def handle_list_examples(args: dict[str, Any]) -> dict[str, Any]:
        examples_dir = args.get("examples_dir")
        example_id = args.get("example_id")
        if example_id:
            ex = load_example(str(example_id), examples_dir=examples_dir)
            summary = ex.as_summary()
            summary["spec_md"] = ex.load_spec_text() if ex.spec_md_path and ex.spec_md_path.is_file() else None
            summary["requirements"] = (
                ex.load_requirements_text()
                if (ex.requirements_md_path and ex.requirements_md_path.is_file())
                or (ex.spec_md_path and ex.spec_md_path.is_file())
                else None
            )
            summary["contract"] = ex.load_contract() if ex.contract_path and ex.contract_path.is_file() else None
            summary["clauses"] = ex.load_requirements_clauses()
            summary["paths"] = {
                "spec_md": str(ex.spec_md_path) if ex.spec_md_path else None,
                "requirements_md": str(ex.requirements_md_path) if ex.requirements_md_path else None,
                "requirements_json": str(ex.requirements_json_path) if ex.requirements_json_path else None,
                "contract": str(ex.contract_path) if ex.contract_path else None,
                "contract_explanation": str(ex.contract_explanation_path) if ex.contract_explanation_path else None,
                "model_scir": str(ex.model_scir_path) if ex.model_scir_path else None,
                "traceability_manifest": str(ex.traceability_manifest_path) if ex.traceability_manifest_path else None,
            }
            return summary
        return examples_summary(examples_dir)

    def handle_validate_contract(args: dict[str, Any]) -> dict[str, Any]:
        if "contract" not in args:
            raise ValueError("missing required parameter 'contract'")
        raw_contract = ws.load_json_value(args["contract"], field_name="contract")
        enforce_non_vacuous = bool(args.get("enforce_non_vacuous", True))
        require_non_vacuous = bool(args.get("require_non_vacuous", True))
        strict_entities = bool(args.get("strict_entities", True))

        client = ws.client()
        contract = (
            ensure_implication_non_vacuous(raw_contract, client=client)
            if enforce_non_vacuous
            else raw_contract
        )
        structured = None
        if args.get("requirements") is not None:
            structured = parse_structured_requirements(args["requirements"])

        sanity = validate_contract_sanity(
            contract,
            structured,
            strict_entities=strict_entities,
            require_non_vacuous=require_non_vacuous,
            client=client,
        )
        explanation = explain_contract_data(contract, client=client)
        return {
            "ok": sanity.ok,
            "contract": contract,
            "sanity": sanity.as_dict(),
            "explanation": explanation.as_dict(),
        }

    def handle_lower_scir(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("lower")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.scir.json", field_name="program")
        lowered_path = call_dir / "lowered.cir.json"
        trace_map_path = call_dir / "trace_map.json"
        res = ws.client().lower(prog_path, out_path=lowered_path, trace_map_path=trace_map_path)
        lowered_program = (
            json.loads(lowered_path.read_text(encoding="utf-8")) if lowered_path.is_file() else res.payload
        )
        trace_map = (
            json.loads(trace_map_path.read_text(encoding="utf-8")) if trace_map_path.is_file() else None
        )
        return {
            **result_meta(res),
            "lowered_path": str(lowered_path) if lowered_path.is_file() else None,
            "trace_map_path": str(trace_map_path) if trace_map_path.is_file() else None,
            "lowered_program": lowered_program,
            "trace_map": trace_map,
        }

    def handle_check_model(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("check")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        client = ws.client()
        check_res = client.check(prog_path)
        out: dict[str, Any] = {
            "valid": check_res.valid is True,
            "check": {
                **result_meta(check_res),
                "feedback": build_check_feedback(check_res),
                "payload": check_res.payload,
            },
            "supported": False,
            "support": None,
        }
        if check_res.kind == "semantic" and check_res.status == "valid":
            sup_res = client.support(prog_path)
            out["supported"] = sup_res.kind == "semantic" and sup_res.status == "supported"
            out["support"] = {
                **result_meta(sup_res),
                "unsupported": sup_res.unsupported,
                "payload": sup_res.payload,
            }
        return out

    def handle_summarize_modules(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("summarize")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        out_path = call_dir / "summary.json"
        module = args.get("module")
        res = ws.client().summarize(prog_path, module=str(module) if module else None, out_path=out_path)
        payload = res.payload or {}
        return {
            **result_meta(res),
            "cross_module_lock_cycle": payload.get("cross_module_lock_cycle"),
            "summary": payload,
            "summary_path": str(out_path) if out_path.is_file() else None,
        }

    def handle_explore_model(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        if "contract" not in args:
            raise ValueError("missing required parameter 'contract'")
        engine = str(args.get("engine") or "petri")
        call_dir = ws.new_call_dir("explore")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        contract_path = ws.materialize_json(args["contract"], call_dir, "contract.json", field_name="contract")
        res = ws.client().explore(prog_path, contract_path, engine=engine)
        payload = res.payload or {}
        return {
            **result_meta(res),
            "outcome": res.outcome,
            "complete": res.complete,
            "properties": payload.get("properties", []),
            "diagnostics": res.diagnostics,
            "feedback": build_explore_feedback(res) if res.kind == "semantic" else None,
            "report": payload,
        }

    def handle_check_req(args: dict[str, Any]) -> dict[str, Any]:
        if "req_ir" not in args:
            raise ValueError("missing required parameter 'req_ir'")
        call_dir = ws.new_call_dir("check-req")
        req_path = ws.materialize_json(args["req_ir"], call_dir, "req_ir.json", field_name="req_ir")
        contract_path: Path | None = None
        if args.get("contract") is not None:
            contract_path = ws.materialize_json(args["contract"], call_dir, "contract.json", field_name="contract")
        out_path = call_dir / "req_check.json"
        res = ws.client().check_req(req_path, contract=contract_path, out_path=out_path)
        payload = res.payload or {}
        return {
            **result_meta(res),
            "outcome": payload.get("outcome"),
            "complete": payload.get("complete"),
            "deadlock_free": payload.get("deadlock_free"),
            "no_dead_milestones": payload.get("no_dead_milestones"),
            "missing_milestones": payload.get("missing_milestones", []),
            "report": payload,
            "report_path": str(out_path) if out_path.is_file() else None,
        }

    def handle_verify_design(args: dict[str, Any]) -> dict[str, Any]:
        if "req_ir" not in args:
            raise ValueError("missing required parameter 'req_ir'")
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("verify-design")
        req_path = ws.materialize_json(args["req_ir"], call_dir, "req_ir.json", field_name="req_ir")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.scir.json", field_name="program")
        contract_path: Path | None = None
        if args.get("contract") is not None:
            contract_path = ws.materialize_json(args["contract"], call_dir, "contract.json", field_name="contract")
        out_path = call_dir / "design_refinement.json"
        res = ws.client().verify_design(req_path, prog_path, contract=contract_path, out_path=out_path)
        payload = res.payload or {}
        return {
            **result_meta(res),
            "outcome": payload.get("outcome"),
            "complete": payload.get("complete"),
            "checks": payload.get("checks", {}),
            "missing_milestones": payload.get("missing_milestones", []),
            "violations": payload.get("violations", []),
            "report": payload,
            "report_path": str(out_path) if out_path.is_file() else None,
        }

    def handle_repair_model(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        if "contract" not in args:
            raise ValueError("missing required parameter 'contract'")
        call_dir = ws.new_call_dir("repair")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        contract_path = ws.materialize_json(args["contract"], call_dir, "contract.json", field_name="contract")
        res = ws.client().repair(
            prog_path,
            contract_path,
            strategy=str(args.get("strategy") or "c"),
            candidate_budget=int(args.get("candidate_budget", 64)),
            verification_budget=int(args.get("verification_budget", 64)),
            max_depth=int(args.get("max_depth", 4)),
            max_total_edits=int(args.get("max_total_edits", 4)),
        )
        return {**result_meta(res), "report": res.payload}

    def handle_export_cvn(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("export-cvn")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        out_path = call_dir / "program.cvn.json"
        res = ws.client().export_cvn(prog_path, out_path=out_path)
        return {
            **result_meta(res),
            "cvn_path": str(out_path) if out_path.is_file() else None,
            "export": res.payload,
        }

    return [
        ToolSpec(
            name="concplan_toolchain_status",
            description="Check availability and resolved paths of concir-backend, spn, and optional lockbud binaries.",
            input_schema={"type": "object", "properties": {}, "additionalProperties": False},
            handler=handle_toolchain_status,
        ),
        ToolSpec(
            name="concplan_list_examples",
            description="List Stage 0-driven ConcPlanVerify examples or inspect one example's spec.md, Stage 0 requirements, contract, and model.scir.json paths.",
            input_schema={
                "type": "object",
                "properties": {
                    "example_id": {"type": "string", "description": "Optional example id such as 'visit_counter'."},
                    "examples_dir": {"type": "string"},
                },
                "additionalProperties": False,
            },
            handler=handle_list_examples,
        ),
        ToolSpec(
            name="concplan_validate_contract",
            description="Run Stage 0.3 contract sanity checks (schema, clause binding, entity grounding, non-vacuity) and deterministic NL back-translation.",
            input_schema={
                "type": "object",
                "properties": {
                    "contract": JSON_OR_PATH_SCHEMA,
                    "requirements": {"type": ["object", "string"], "description": "Optional REQUIREMENTS.md text, requirements.json dict, or file path."},
                    "enforce_non_vacuous": {"type": "boolean", "default": True},
                    "require_non_vacuous": {"type": "boolean", "default": True},
                    "strict_entities": {"type": "boolean", "default": True},
                    "prefer_backend_explain": {"type": "boolean", "default": False},
                },
                "required": ["contract"],
                "additionalProperties": False,
            },
            handler=handle_validate_contract,
        ),
        ToolSpec(
            name="concplan_lower_scir",
            description="Deterministically lower an SCIR structured_body program into flat CIR and a concir-trace-map/1 traceability map.",
            input_schema={
                "type": "object",
                "properties": {"program": JSON_OR_PATH_SCHEMA},
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_lower_scir,
        ),
        ToolSpec(
            name="concplan_check_model",
            description="Validate static well-formedness (check) and verification supportability (support) of a CIR/SCIR program.",
            input_schema={
                "type": "object",
                "properties": {"program": JSON_OR_PATH_SCHEMA},
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_check_model,
        ),
        ToolSpec(
            name="concplan_summarize_modules",
            description="Compute compositional module summaries and detect cross-module lock-order cycles.",
            input_schema={
                "type": "object",
                "properties": {
                    "program": JSON_OR_PATH_SCHEMA,
                    "module": {"type": "string", "description": "Optional single module name to summarize."},
                },
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_summarize_modules,
        ),
        ToolSpec(
            name="concplan_explore_model",
            description="Exhaustively model-check a CIR/SCIR program against a contract (classic safety/liveness + UniPN CTL/LTL with fairness and non-vacuity).",
            input_schema={
                "type": "object",
                "properties": {
                    "program": JSON_OR_PATH_SCHEMA,
                    "contract": JSON_OR_PATH_SCHEMA,
                    "engine": {"type": "string", "enum": ["petri", "interp"], "default": "petri"},
                },
                "required": ["program", "contract"],
                "additionalProperties": False,
            },
            handler=handle_explore_model,
        ),
        ToolSpec(
            name="concplan_check_req",
            description="Validate and explore a Stage 0 Requirement Net (concir-req-ir/1) for non-terminal deadlock freedom, non-dead milestones, and optional requirement-level temporal properties.",
            input_schema={
                "type": "object",
                "properties": {
                    "req_ir": JSON_OR_PATH_SCHEMA,
                    "contract": JSON_OR_PATH_SCHEMA,
                },
                "required": ["req_ir"],
                "additionalProperties": False,
            },
            handler=handle_check_req,
        ),
        ToolSpec(
            name="concplan_verify_design",
            description="Verify Stage 0-2 dual-net extension/stuttering refinement (N_req <=_ext N_code), milestone preservation, and concurrency health obligations.",
            input_schema={
                "type": "object",
                "properties": {
                    "req_ir": JSON_OR_PATH_SCHEMA,
                    "program": JSON_OR_PATH_SCHEMA,
                    "contract": JSON_OR_PATH_SCHEMA,
                },
                "required": ["req_ir", "program"],
                "additionalProperties": False,
            },
            handler=handle_verify_design,
        ),
        ToolSpec(
            name="concplan_repair_model",
            description="Run concir-backend deterministic search-based model repair against a contract.",
            input_schema={
                "type": "object",
                "properties": {
                    "program": JSON_OR_PATH_SCHEMA,
                    "contract": JSON_OR_PATH_SCHEMA,
                    "strategy": {"type": "string", "enum": ["a", "b", "c"], "default": "c"},
                    "candidate_budget": {"type": "integer", "default": 64},
                    "verification_budget": {"type": "integer", "default": 64},
                    "max_depth": {"type": "integer", "default": 4},
                    "max_total_edits": {"type": "integer", "default": 4},
                },
                "required": ["program", "contract"],
                "additionalProperties": False,
            },
            handler=handle_repair_model,
        ),
        ToolSpec(
            name="concplan_export_cvn",
            description="Export a CIR/SCIR program to a UniPN Colored Verification Net (CvnNet) JSON + degradation report.",
            input_schema={
                "type": "object",
                "properties": {"program": JSON_OR_PATH_SCHEMA},
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_export_cvn,
        ),
    ]
