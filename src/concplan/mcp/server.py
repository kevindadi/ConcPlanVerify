"""ConcPlanVerify MCP server assembly (tools, resources, and workflow prompts)."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path

from ..core.env import load_dotenv
from ..core.prompts import (
    clauses_to_contract_system_prompt,
    hole_repair_system_prompt,
    nl_to_clauses_system_prompt,
    scir_generation_system_prompt,
)
from ..core.toolchain import REPO
from .protocol import McpServer, PromptArgument, PromptSpec, ResourceSpec
from .tools import McpWorkspace, register_concplan_tools

SERVER_INSTRUCTIONS = """ConcPlanVerify v5 MCP server for verified concurrent Rust development.

Recommended workflow:
1. Inspect available binaries via `concplan_toolchain_status`.
2. Ground requirements and validate the formal CTL/LTL contract (`concplan_validate_contract`).
3. Author a Structured ConcIR (`SCIR`) model and verify it (`concplan_lower_scir` -> `concplan_check_model` -> `concplan_summarize_modules` -> `concplan_explore_model`).
4. Generate the multi-file Rust skeleton (`concplan_codegen_skeleton`), fill only `SeqHole` (`h*`) and `TypeHole` (`th*`) slots with pure Rust (`concplan_fill_holes`), and verify O1 purity + O2 MIR stuttering refinement (`concplan_verify_code`).
5. Never place locks, atomics, channels, threads, unsafe, or unbounded loops inside holes; escalate architectural changes back to SCIR instead.
"""


def _read_repo_doc(rel_path: str) -> str:
    path = REPO / rel_path
    if not path.is_file():
        raise FileNotFoundError(f"repository documentation file not found: {path}")
    return path.read_text(encoding="utf-8")


def _register_resources(server: McpServer) -> None:
    resources = [
        ResourceSpec(
            uri="concplan://docs/agents",
            name="AGENTS.md",
            description="ConcPlanVerify developer & agent workflow guide, invariants, and quick reference.",
            mime_type="text/markdown",
            reader=lambda: _read_repo_doc("AGENTS.md"),
        ),
        ResourceSpec(
            uri="concplan://docs/architecture",
            name="ARCHITECTURE.md",
            description="ConcPlanVerify v5 layer-by-layer architecture across UniPN, ConcIR, SPN, and Python.",
            mime_type="text/markdown",
            reader=lambda: _read_repo_doc("docs/ARCHITECTURE.md"),
        ),
        ResourceSpec(
            uri="concplan://docs/refinement-spec",
            name="REFINEMENT_SPEC.md",
            description="Authoritative JSON wire schemas, CLI flags, and exit codes across backends and Python.",
            mime_type="text/markdown",
            reader=lambda: _read_repo_doc("docs/REFINEMENT_SPEC.md"),
        ),
        ResourceSpec(
            uri="concplan://prompts/nl-to-clauses",
            name="nl_to_clauses.md",
            description="Stage 0.1 prompt for grounding entities and splitting NL specs into R1..Rn clauses.",
            mime_type="text/markdown",
            reader=nl_to_clauses_system_prompt,
        ),
        ResourceSpec(
            uri="concplan://prompts/clauses-to-contract",
            name="clauses_to_contract.md",
            description="Stage 0.2 isolated prompt for synthesizing CTL/LTL verification contracts.",
            mime_type="text/markdown",
            reader=clauses_to_contract_system_prompt,
        ),
        ResourceSpec(
            uri="concplan://prompts/scir-generation",
            name="scir_generation.md",
            description="SCIR v5.0 authoring guide (lock_guard, rwlock_*_guard, while_wait, seq_hole, TypeHole, multi-module).",
            mime_type="text/markdown",
            reader=scir_generation_system_prompt,
        ),
        ResourceSpec(
            uri="concplan://prompts/hole-repair",
            name="hole_repair.md",
            description="Pure SeqHole & TypeHole filling rules, Loop B repair, and Loop C escalation protocol.",
            mime_type="text/markdown",
            reader=hole_repair_system_prompt,
        ),
    ]
    for spec in resources:
        server.register_resource(spec)


def _register_prompts(server: McpServer) -> None:
    def render_design(args: dict[str, str]) -> list[dict[str, object]]:
        goal = args.get("requirements", "")
        text = (
            "Design, verify, and implement a concurrent Rust program using ConcPlanVerify MCP tools.\n"
            "1. Read `concplan://prompts/scir-generation` if needed for exact SCIR v5 syntax.\n"
            "2. Formalize the contract and validate it with `concplan_validate_contract`.\n"
            "3. Author an SCIR model (`lock_guard`, `rwlock_read_guard`, `rwlock_write_guard`, `seq_hole`, `TypeHole`) and verify it via `concplan_lower_scir`, `concplan_check_model`, `concplan_summarize_modules`, and `concplan_explore_model`.\n"
            "4. Run `concplan_codegen_skeleton`, fill only `h*`/`th*` holes purely via `concplan_fill_holes`, and verify via `concplan_verify_code`.\n\n"
            f"Requirements:\n{goal}"
        )
        return [{"role": "user", "content": {"type": "text", "text": text}}]

    def render_formalize(args: dict[str, str]) -> list[dict[str, object]]:
        nl_spec = args.get("nl_spec", "")
        text = (
            "Convert the natural-language specification below into grounded entities (`roles`, `resources`, `domain_types`), "
            "atomic clauses `R1..Rn` (`[V]` concurrency vs. `[U] sequential_hole`), and a valid CTL/LTL `contract.json` "
            "(with `non_vacuous: true` on implications), then validate it using `concplan_validate_contract`.\n\n"
            f"Specification:\n{nl_spec}"
        )
        return [{"role": "user", "content": {"type": "text", "text": text}}]

    def render_repair(args: dict[str, str]) -> list[dict[str, object]]:
        violations = args.get("violations", "")
        holes = args.get("offending_holes", "")
        text = (
            "Repair ONLY the offending hole(s) below using pure Rust (`O1` purity: no locks, atomics, channels, threads, unsafe, or unbounded loops). "
            "Apply your fix with `concplan_fill_holes` and re-check with `concplan_verify_code`. "
            "If synchronization changes are required, update the SCIR model and re-verify from Stage 2 instead.\n\n"
            f"Offending holes: {holes}\nViolations:\n{violations}"
        )
        return [{"role": "user", "content": {"type": "text", "text": text}}]

    prompts = [
        PromptSpec(
            name="design_concurrent_rust",
            description="End-to-end guided workflow to model, verify, generate, and check concurrent Rust code.",
            arguments=[
                PromptArgument(
                    name="requirements",
                    description="Natural-language or structured R1..Rn concurrency requirements.",
                    required=True,
                )
            ],
            renderer=render_design,
        ),
        PromptSpec(
            name="formalize_requirements",
            description="Stage 0 workflow to ground entities and synthesize a validated CTL/LTL contract.",
            arguments=[
                PromptArgument(
                    name="nl_spec",
                    description="Free-form natural-language specification.",
                    required=True,
                )
            ],
            renderer=render_formalize,
        ),
        PromptSpec(
            name="repair_hole_violations",
            description="Loop B / Loop C workflow for repairing offending SeqHole or TypeHole violations.",
            arguments=[
                PromptArgument(
                    name="offending_holes",
                    description="Comma-separated list or JSON array of offending hole IDs.",
                    required=True,
                ),
                PromptArgument(
                    name="violations",
                    description="JSON diagnostics from `concplan_verify_code`.",
                    required=True,
                ),
            ],
            renderer=render_repair,
        ),
    ]
    for spec in prompts:
        server.register_prompt(spec)


def create_server(
    *,
    workdir: Path | str | None = None,
    concir_binary: Path | str | None = None,
    spn_binary: Path | str | None = None,
    lockbud_binary: Path | str | None = None,
    timeout: float = 120.0,
) -> McpServer:
    """Create and configure the ConcPlanVerify MCP server with all tools, resources, and prompts."""
    resolved_workdir = (
        Path(workdir).expanduser().resolve()
        if workdir is not None
        else Path(tempfile.gettempdir()) / "concplan-mcp-workdir"
    )
    ws = McpWorkspace(
        workdir=resolved_workdir,
        concir_binary=concir_binary,
        spn_binary=spn_binary,
        lockbud_binary=lockbud_binary,
        timeout=timeout,
    )
    server = McpServer(
        name="concplan-mcp",
        version="0.5.0",
        instructions=SERVER_INSTRUCTIONS,
    )
    register_concplan_tools(server, ws)
    _register_resources(server)
    _register_prompts(server)
    return server


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="concplan-mcp",
        description="ConcPlanVerify v5 Model Context Protocol (MCP) stdio server",
    )
    parser.add_argument(
        "--workdir",
        default=None,
        help="Workspace directory for intermediate verification/codegen artifacts",
    )
    parser.add_argument(
        "--binary",
        default=None,
        help="Path to concir-backend binary (overrides CONCIR_BACKEND)",
    )
    parser.add_argument(
        "--spn-binary",
        default=None,
        help="Path to spn binary (overrides CONCIR_SPN)",
    )
    parser.add_argument(
        "--lockbud-binary",
        default=None,
        help="Path to lockbud binary (overrides CONCIR_LOCKBUD)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=120.0,
        help="Subprocess timeout in seconds for backend tools (default: 120)",
    )
    parser.add_argument(
        "--list-tools",
        action="store_true",
        help="Print registered MCP tools, resources, and prompts as JSON and exit",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    load_dotenv(REPO / ".env")

    server = create_server(
        workdir=Path(args.workdir) if args.workdir else None,
        concir_binary=args.binary,
        spn_binary=args.spn_binary,
        lockbud_binary=args.lockbud_binary,
        timeout=args.timeout,
    )

    if args.list_tools:
        print(json.dumps(server.describe(), ensure_ascii=False, indent=2))
        return 0

    try:
        server.serve_stdio()
    except KeyboardInterrupt:
        print("concplan-mcp stopped", file=sys.stderr)
    return 0
