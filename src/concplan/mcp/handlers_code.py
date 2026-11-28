"""MCP tool handlers for Stage 3 Rust skeleton codegen, hole filling, and Stage 4 verification."""

from __future__ import annotations

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
from ..code_verify.holes import list_src_rs_files
from ..code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    LockbudPlugin,
    SpnMirRefinementPlugin,
    fill_holes_marked,
)
from .protocol import ToolSpec
from .workspace import JSON_OR_PATH_SCHEMA, McpWorkspace


def build_code_tool_specs(ws: McpWorkspace) -> list[ToolSpec]:
    """Build Stage 3-4 Rust skeleton codegen, hole-fill, and MIR verification MCP tools."""

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
        files = list_src_rs_files(skel_dir)
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
            "files": list_src_rs_files(filled_dir),
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

    return [
        ToolSpec(
            name="concplan_codegen_skeleton",
            description="Generate a MIR-clean multi-file Rust skeleton crate (src/main.rs, src/<mod>.rs, src/domain.rs) with SeqHole and TypeHole placeholders.",
            input_schema={
                "type": "object",
                "properties": {
                    "program": JSON_OR_PATH_SCHEMA,
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
                    "program": JSON_OR_PATH_SCHEMA,
                    "contract": JSON_OR_PATH_SCHEMA,
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
