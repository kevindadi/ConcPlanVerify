"""ConcPlanVerify MCP tools for formal concurrency design, verification, and Rust codegen."""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ..code_verify.conformance import (
    codegen,
    collect_traces,
    conform_all,
    fill_prompt,
    holes_of,
    type_holes_of,
)
from ..code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    LockbudPlugin,
    SpnMirRefinementPlugin,
    _list_src_rs_files,
    fill_holes_marked,
)
from ..core.concir_client import ConcirClient, ConcirResult
from ..core.prompts import build_check_feedback, build_explore_feedback
from ..core.toolchain import find_concir_backend, find_lockbud, find_spn
from ..formalize import (
    ensure_implication_non_vacuous,
    explain_contract_data,
    parse_structured_requirements,
    validate_contract_sanity,
)
from ..pipeline.benchmarks import benchmarks_summary, load_benchmark
from .protocol import McpServer, ToolSpec


@dataclass
class McpWorkspace:
    """Shared execution context and binary configuration for MCP tool handlers."""

    workdir: Path
    concir_binary: Path | str | None = None
    spn_binary: Path | str | None = None
    lockbud_binary: Path | str | None = None
    timeout: float = 120.0
    _counter: int = field(default=0, init=False, repr=False)

    def __post_init__(self) -> None:
        self.workdir = Path(self.workdir).expanduser().resolve()
        self.workdir.mkdir(parents=True, exist_ok=True)

    def resolve_concir_binary(self) -> Path:
        found = find_concir_backend(self.concir_binary)
        if found is None:
            raise FileNotFoundError(
                "concir-backend binary not found. Build it with "
                "`cargo build --release --manifest-path backends/concir/Cargo.toml --bin concir-backend` "
                "or set CONCIR_BACKEND."
            )
        return found

    def client(self) -> ConcirClient:
        return ConcirClient(
            self.resolve_concir_binary(),
            workdir=self.workdir / "concir-calls",
            timeout=self.timeout,
        )

    def new_call_dir(self, prefix: str) -> Path:
        self._counter += 1
        for _ in range(1000):
            token = f"{prefix}-{time.time_ns():x}-{os.getpid()}-{self._counter:04d}"
            path = self.workdir / token
            try:
                path.mkdir(parents=True, exist_ok=False)
                return path
            except FileExistsError:
                continue
        raise RuntimeError(f"could not allocate unique MCP call directory under {self.workdir}")

    @staticmethod
    def load_json_value(value: Any, *, field_name: str) -> dict[str, Any]:
        if isinstance(value, dict):
            return value
        if isinstance(value, str):
            stripped = value.strip()
            if "\n" not in stripped and len(stripped) < 4096:
                candidate = Path(stripped).expanduser()
                if candidate.is_file():
                    data = json.loads(candidate.read_text(encoding="utf-8"))
                    if not isinstance(data, dict):
                        raise ValueError(f"{field_name} file must contain a JSON object")
                    return data
            data = json.loads(stripped)
            if not isinstance(data, dict):
                raise ValueError(f"{field_name} must be a JSON object")
            return data
        raise ValueError(f"{field_name} must be a JSON object, JSON string, or file path")

    @staticmethod
    def materialize_json(value: Any, call_dir: Path, filename: str, *, field_name: str) -> Path:
        dest = call_dir / filename
        if isinstance(value, dict):
            dest.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            return dest
        if isinstance(value, str):
            stripped = value.strip()
            if "\n" not in stripped and len(stripped) < 4096:
                candidate = Path(stripped).expanduser()
                if candidate.is_file():
                    return candidate.resolve()
            data = json.loads(stripped)
            if not isinstance(data, dict):
                raise ValueError(f"{field_name} must be a JSON object")
            dest.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            return dest
        raise ValueError(f"{field_name} must be a JSON object, JSON string, or file path")


def _result_meta(res: ConcirResult) -> dict[str, Any]:
    return {
        "kind": res.kind,
        "status": res.status,
        "exit_code": res.exit_code,
        "error": res.error,
        "stderr": res.stderr,
        "wall_ms": res.wall_ms,
        "artifact_path": res.artifact_path,
    }


_JSON_OR_PATH_SCHEMA: dict[str, Any] = {
    "type": ["object", "string"],
    "description": "JSON object, serialized JSON string, or path to a JSON file on disk.",
}


def register_concplan_tools(server: McpServer, ws: McpWorkspace) -> None:
    """Register the 12 ConcPlanVerify concurrency-development tools on ``server``."""

    def handle_toolchain_status(args: dict[str, Any]) -> dict[str, Any]:
        del args
        concir = find_concir_backend(ws.concir_binary)
        spn = find_spn(ws.spn_binary)
        lockbud = find_lockbud(ws.lockbud_binary)
        return {
            "workdir": str(ws.workdir),
            "concir_backend": {
                "available": concir is not None,
                "path": str(concir) if concir is not None else None,
            },
            "spn": {
                "available": spn is not None,
                "path": str(spn) if spn is not None else None,
            },
            "lockbud": {
                "available": lockbud is not None,
                "path": str(lockbud) if lockbud is not None else None,
            },
        }

    def handle_list_benchmarks(args: dict[str, Any]) -> dict[str, Any]:
        benchmarks_dir = args.get("benchmarks_dir")
        task_id = args.get("task_id")
        if task_id:
            task = load_benchmark(str(task_id), benchmarks_dir=benchmarks_dir)
            summary = task.as_summary()
            summary["requirements"] = task.load_requirements_text()
            summary["contract"] = task.load_contract() if task.contract_path and task.contract_path.is_file() else None
            summary["clauses"] = task.load_requirements_clauses()
            summary["paths"] = {
                "contract": str(task.contract_path) if task.contract_path else None,
                "requirements_md": str(task.requirements_md_path) if task.requirements_md_path else None,
                "requirements_json": str(task.requirements_json_path) if task.requirements_json_path else None,
                "buggy_cir": str(task.buggy_cir_path) if task.buggy_cir_path else None,
                "fixed_cir": str(task.fixed_cir_path) if task.fixed_cir_path else None,
                "correct_cir": str(task.correct_cir_path) if task.correct_cir_path else None,
                "repair_input_cir": str(task.repair_input_cir_path) if task.repair_input_cir_path else None,
            }
            return summary

        suite = args.get("suite")
        summary = benchmarks_summary(benchmarks_dir)
        if suite:
            summary["tasks"] = [t for t in summary["tasks"] if t["suite"] == suite]
            summary["total"] = len(summary["tasks"])
        return summary

    def handle_validate_contract(args: dict[str, Any]) -> dict[str, Any]:
        if "contract" not in args:
            raise ValueError("missing required parameter 'contract'")
        raw_contract = ws.load_json_value(args["contract"], field_name="contract")
        enforce_non_vacuous = bool(args.get("enforce_non_vacuous", True))
        require_non_vacuous = bool(args.get("require_non_vacuous", True))
        strict_entities = bool(args.get("strict_entities", True))

        contract = (
            ensure_implication_non_vacuous(raw_contract)
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
        )
        client = None
        if bool(args.get("prefer_backend_explain", False)):
            try:
                client = ws.client()
            except FileNotFoundError:
                client = None
        explanation = explain_contract_data(
            contract,
            client=client,
            prefer_backend=bool(args.get("prefer_backend_explain", False) and client is not None),
        )
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
            json.loads(lowered_path.read_text(encoding="utf-8"))
            if lowered_path.is_file()
            else res.payload
        )
        trace_map = (
            json.loads(trace_map_path.read_text(encoding="utf-8"))
            if trace_map_path.is_file()
            else None
        )
        return {
            **_result_meta(res),
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
                **_result_meta(check_res),
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
                **_result_meta(sup_res),
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
        res = ws.client().summarize(
            prog_path,
            module=str(module) if module else None,
            out_path=out_path,
        )
        payload = res.payload or {}
        return {
            **_result_meta(res),
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
            **_result_meta(res),
            "outcome": res.outcome,
            "complete": res.complete,
            "properties": payload.get("properties", []),
            "diagnostics": res.diagnostics,
            "feedback": build_explore_feedback(res) if res.kind == "semantic" else None,
            "report": payload,
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
        return {
            **_result_meta(res),
            "report": res.payload,
        }

    def handle_export_cvn(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("export-cvn")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        out_path = call_dir / "program.cvn.json"
        res = ws.client().export_cvn(prog_path, out_path=out_path)
        return {
            **_result_meta(res),
            "cvn_path": str(out_path) if out_path.is_file() else None,
            "export": res.payload,
        }

    def handle_codegen_skeleton(args: dict[str, Any]) -> dict[str, Any]:
        if "program" not in args:
            raise ValueError("missing required parameter 'program'")
        call_dir = ws.new_call_dir("codegen")
        prog_path = ws.materialize_json(args["program"], call_dir, "program.json", field_name="program")
        if args.get("out_dir"):
            skel_dir = Path(str(args["out_dir"])).expanduser().resolve()
        else:
            skel_dir = call_dir / "skeleton"

        binary = ws.resolve_concir_binary()
        cg_map = codegen(prog_path, skel_dir, binary=binary)
        holes = holes_of(skel_dir)
        type_holes = type_holes_of(skel_dir)
        files = _list_src_rs_files(skel_dir)
        out: dict[str, Any] = {
            "program_path": str(prog_path),
            "skeleton_dir": str(skel_dir),
            "files": files,
            "holes": holes,
            "type_holes": type_holes,
            "codegen_map": cg_map,
        }
        if args.get("requirements"):
            out["fill_prompt"] = fill_prompt(skel_dir, str(args["requirements"]))
        return out

    def handle_fill_holes(args: dict[str, Any]) -> dict[str, Any]:
        if not args.get("skeleton_dir"):
            raise ValueError("missing required parameter 'skeleton_dir'")
        skel_dir = Path(str(args["skeleton_dir"])).expanduser().resolve()
        if not skel_dir.is_dir():
            raise FileNotFoundError(f"skeleton_dir does not exist: {skel_dir}")

        if args.get("filled_dir"):
            filled_dir = Path(str(args["filled_dir"])).expanduser().resolve()
        else:
            filled_dir = ws.new_call_dir("filled") / "crate"

        fills_raw = args.get("fills") or {}
        if not isinstance(fills_raw, dict):
            raise ValueError("'fills' must be an object mapping hole IDs to Rust code")
        fills = {str(k): str(v) for k, v in fills_raw.items()}

        type_fills_raw = args.get("type_fills")
        type_fills: dict[str, str] | None = None
        if type_fills_raw is not None:
            if not isinstance(type_fills_raw, dict):
                raise ValueError("'type_fills' must be an object mapping type-hole IDs to Rust code")
            type_fills = {str(k): str(v) for k, v in type_fills_raw.items()}

        info = fill_holes_marked(skel_dir, filled_dir, fills, type_fills=type_fills)
        return {
            "skeleton_dir": str(skel_dir),
            "filled_dir": str(filled_dir),
            "files": _list_src_rs_files(filled_dir),
            **info,
        }

    def handle_verify_code(args: dict[str, Any]) -> dict[str, Any]:
        if not args.get("skeleton_dir"):
            raise ValueError("missing required parameter 'skeleton_dir'")
        if not args.get("filled_dir"):
            raise ValueError("missing required parameter 'filled_dir'")
        skel_dir = Path(str(args["skeleton_dir"])).expanduser().resolve()
        filled_dir = Path(str(args["filled_dir"])).expanduser().resolve()

        call_dir = ws.new_call_dir("verify-code")
        spec_path: Path | None = None
        if args.get("program") is not None:
            spec_path = ws.materialize_json(args["program"], call_dir, "spec.json", field_name="program")
        contract_path: Path | None = None
        if args.get("contract") is not None:
            contract_path = ws.materialize_json(args["contract"], call_dir, "contract.json", field_name="contract")

        cmap_path: Path | None = None
        if args.get("codegen_map_path"):
            cmap_path = Path(str(args["codegen_map_path"])).expanduser().resolve()
        elif (filled_dir / "codegen_filled.json").is_file():
            cmap_path = filled_dir / "codegen_filled.json"

        ast_only = bool(args.get("ast_only", False))
        modular = bool(args.get("modular", False))
        focus_modules = args.get("focus_modules")
        if focus_modules is not None:
            focus_modules = [str(m) for m in focus_modules]

        plugins: list[Any] = [AstHolePurityPlugin()]
        if not ast_only:
            if spec_path is None:
                raise ValueError("'program' (verified CIR/SCIR spec) is required unless 'ast_only' is true")
            plugins.append(
                SpnMirRefinementPlugin(
                    concir_binary=ws.concir_binary,
                    spn_binary=ws.spn_binary,
                    timeout=ws.timeout,
                    modular=modular,
                    focus_modules=focus_modules,
                )
            )
        if bool(args.get("run_lockbud", False)):
            plugins.append(LockbudPlugin(lockbud_binary=ws.lockbud_binary, timeout=ws.timeout))

        verifier = CompositeStaticVerifier(plugins)
        static_report = verifier.verify(
            skeleton_dir=skel_dir,
            filled_dir=filled_dir,
            spec_path=spec_path,
            contract_path=contract_path,
            codegen_map_path=cmap_path,
        )
        out = static_report.as_dict()

        if bool(args.get("run_dynamic_conform", False)) and static_report.status == "pass":
            if spec_path is None:
                raise ValueError("'program' is required when 'run_dynamic_conform' is true")
            native_runs = int(args.get("native_runs", 5))
            miri_seeds = int(args.get("miri_seeds", 0))
            traces = collect_traces(
                filled_dir,
                native_runs=native_runs,
                miri_seeds=miri_seeds,
                run_miri=miri_seeds > 0,
                calls_dir=call_dir / "dynamic-traces",
            )
            dyn = conform_all(spec_path, traces, binary=ws.resolve_concir_binary())
            out["dynamic_report"] = dyn
            if dyn.get("violation", 0) > 0:
                out["status"] = "fail_refinement"
                out["ok"] = False

        return out

    specs = [
        ToolSpec(
            name="concplan_toolchain_status",
            description="Check availability and resolved paths of concir-backend, spn, and optional lockbud binaries.",
            input_schema={"type": "object", "properties": {}, "additionalProperties": False},
            handler=handle_toolchain_status,
        ),
        ToolSpec(
            name="concplan_list_benchmarks",
            description="List ConcPlanVerify benchmark tasks or inspect one benchmark task's requirements, contract, and reference CIR paths.",
            input_schema={
                "type": "object",
                "properties": {
                    "suite": {"type": "string", "enum": ["families", "real-cases"]},
                    "task_id": {"type": "string", "description": "Optional benchmark id such as 'lock-order/abba_2lock'."},
                    "benchmarks_dir": {"type": "string"},
                },
                "additionalProperties": False,
            },
            handler=handle_list_benchmarks,
        ),
        ToolSpec(
            name="concplan_validate_contract",
            description="Run Stage 0.3 contract sanity checks (schema, clause binding, entity grounding, non-vacuity) and deterministic NL back-translation.",
            input_schema={
                "type": "object",
                "properties": {
                    "contract": _JSON_OR_PATH_SCHEMA,
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
                "properties": {"program": _JSON_OR_PATH_SCHEMA},
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
                "properties": {"program": _JSON_OR_PATH_SCHEMA},
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
                    "program": _JSON_OR_PATH_SCHEMA,
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
                    "program": _JSON_OR_PATH_SCHEMA,
                    "contract": _JSON_OR_PATH_SCHEMA,
                    "engine": {"type": "string", "enum": ["petri", "interp"], "default": "petri"},
                },
                "required": ["program", "contract"],
                "additionalProperties": False,
            },
            handler=handle_explore_model,
        ),
        ToolSpec(
            name="concplan_repair_model",
            description="Run concir-backend deterministic search-based model repair against a contract.",
            input_schema={
                "type": "object",
                "properties": {
                    "program": _JSON_OR_PATH_SCHEMA,
                    "contract": _JSON_OR_PATH_SCHEMA,
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
                "properties": {"program": _JSON_OR_PATH_SCHEMA},
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_export_cvn,
        ),
        ToolSpec(
            name="concplan_codegen_skeleton",
            description="Generate a MIR-clean multi-file Rust skeleton crate (src/main.rs, src/<mod>.rs, src/domain.rs) with SeqHole and TypeHole placeholders.",
            input_schema={
                "type": "object",
                "properties": {
                    "program": _JSON_OR_PATH_SCHEMA,
                    "out_dir": {"type": "string", "description": "Optional destination directory for the generated Cargo project."},
                    "requirements": {"type": "string", "description": "Optional requirements text to include a formatted fill_prompt."},
                },
                "required": ["program"],
                "additionalProperties": False,
            },
            handler=handle_codegen_skeleton,
        ),
        ToolSpec(
            name="concplan_fill_holes",
            description="Fill SeqHole (h*) and TypeHole (th*) placeholders in a generated Rust skeleton and emit codegen_filled.json with exact line ranges.",
            input_schema={
                "type": "object",
                "properties": {
                    "skeleton_dir": {"type": "string"},
                    "filled_dir": {"type": "string", "description": "Optional output crate directory."},
                    "fills": {"type": "object", "description": "Map of hole ID (h* or th*) to pure Rust code."},
                    "type_fills": {"type": "object", "description": "Optional explicit map of TypeHole ID (th*) to pure Rust struct/impl code."},
                },
                "required": ["skeleton_dir", "fills"],
                "additionalProperties": False,
            },
            handler=handle_fill_holes,
        ),
        ToolSpec(
            name="concplan_verify_code",
            description="Verify a filled Rust crate using AstHolePurityPlugin (O1), SpnMirRefinementPlugin (O2 whole-program or modular), optional Lockbud, and optional dynamic trace conformance.",
            input_schema={
                "type": "object",
                "properties": {
                    "skeleton_dir": {"type": "string"},
                    "filled_dir": {"type": "string"},
                    "program": _JSON_OR_PATH_SCHEMA,
                    "contract": _JSON_OR_PATH_SCHEMA,
                    "codegen_map_path": {"type": "string"},
                    "ast_only": {"type": "boolean", "default": False},
                    "modular": {"type": "boolean", "default": False},
                    "focus_modules": {"type": "array", "items": {"type": "string"}},
                    "run_lockbud": {"type": "boolean", "default": False},
                    "run_dynamic_conform": {"type": "boolean", "default": False},
                    "native_runs": {"type": "integer", "default": 5},
                    "miri_seeds": {"type": "integer", "default": 0},
                },
                "required": ["skeleton_dir", "filled_dir"],
                "additionalProperties": False,
            },
            handler=handle_verify_code,
        ),
    ]

    for spec in specs:
        server.register_tool(spec)
