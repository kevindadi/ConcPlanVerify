"""CLI argument parser definitions for ``concplan``."""

from __future__ import annotations

import argparse
from pathlib import Path

from ..core.toolchain import SUBMODULE_RELEASE, find_concir_backend, resolve_exe_in_dir


def default_binary() -> str:
    found = find_concir_backend()
    if found is not None:
        return str(found)
    return str(resolve_exe_in_dir(SUBMODULE_RELEASE, "concir-backend"))


def add_provider_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--provider", default="scripted", choices=["scripted", "deepseek", "qwen"])
    parser.add_argument("--script", help="JSON list of scripted responses (for --provider scripted)")
    parser.add_argument("--model", help="LLM model id (for deepseek/qwen)")
    parser.add_argument("--api-key-env", help="Env var holding the LLM API key")
    parser.add_argument("--base-url", help="Override OpenAI-compatible endpoint base URL")
    parser.add_argument("--llm-timeout", type=float, default=120.0)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="concplan",
        description="ConcPlanVerify v5 CLI: ConcIR model checking, MIR refinement, and bi-level verification",
    )
    parser.add_argument(
        "--binary",
        default=default_binary(),
        help="Path to concir-backend binary (or set CONCIR_BACKEND)",
    )
    parser.add_argument(
        "--out",
        default=str(Path.cwd() / "concplan-out"),
        help="Directory for per-call inputs, outputs, and workflow artifacts",
    )
    parser.add_argument("--timeout", type=float, default=60.0)
    sub = parser.add_subparsers(dest="command", required=True)

    # ---- Backend pass-through commands ----
    p = sub.add_parser("check", help="validate CIR/SCIR program")
    p.add_argument("program")

    p = sub.add_parser("support", help="check verification supportability")
    p.add_argument("program")

    p = sub.add_parser("explore", help="explore CIR program against frozen contract")
    p.add_argument("program")
    p.add_argument("contract")
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])

    p = sub.add_parser("lower", help="lower SCIR structured_body to flat CIR + TraceMap")
    p.add_argument("program")
    p.add_argument("--output", help="write lowered CIR JSON here")
    p.add_argument("--trace-map", help="write concir-trace-map/1 JSON here")

    p = sub.add_parser("summarize", help="compute modular sync summaries and lock-cycle check")
    p.add_argument("program")
    p.add_argument("--module", help="optional single module name to summarize")
    p.add_argument("--output", help="write concir-module-summary/1 JSON here")

    p = sub.add_parser("check-req", help="validate & explore abstract Requirement Net (concir-req-ir/1)")
    p.add_argument("req_ir", help="Requirement Net IR JSON path")
    p.add_argument("--contract", help="optional contract JSON path")
    p.add_argument("--output", help="write concir-req-check/1 JSON here")

    p = sub.add_parser("verify-design", help="verify SCIR/CIR extension stuttering refinement against N_req")
    p.add_argument("--req", required=True, help="Requirement Net IR JSON path (N_req)")
    p.add_argument("--design", required=True, help="SCIR/CIR design JSON path (N_code)")
    p.add_argument("--contract", help="optional contract JSON path")
    p.add_argument("--output", help="write concir-design-refinement/1 JSON here")

    p = sub.add_parser("verify-refinement", help="verify code-to-model stuttering refinement")
    p.add_argument("--spec", required=True, help="verified CIR program JSON")
    p.add_argument("--impl-net", required=True, help="exported concir-impl-net/1 JSON")
    p.add_argument("--contract", help="optional contract JSON")
    p.add_argument("--codegen-map", help="optional codegen.json / codegen_filled.json")
    p.add_argument("--module", help="optional single module name for modular refinement")
    p.add_argument("--output", help="write concir-refinement-report/1 JSON here")

    p = sub.add_parser("validate-contract", help="validate contract schema, grounding, clauses, and vacuity (Rust SSoT)")
    p.add_argument("contract", help="contract JSON path")
    p.add_argument("--requirements", help="optional requirements JSON path")
    p.add_argument("--no-strict-entities", action="store_true")
    p.add_argument("--no-require-non-vacuous", action="store_true")
    p.add_argument("--normalize", action="store_true")
    p.add_argument("--output", help="write concir-contract-validation/1 JSON here")

    p = sub.add_parser("explain-contract", help="render deterministic NL explanation of a contract JSON")
    p.add_argument("contract", help="contract JSON path")
    p.add_argument("--output", help="write concir-contract-explanation/1 JSON here")

    p = sub.add_parser("export-cvn", help="export a CIR/SCIR program to UniPN CVN JSON")
    p.add_argument("program", help="CIR or SCIR program JSON path")
    p.add_argument("--output", help="write exported CVN JSON here")

    p = sub.add_parser("formalize", help="Stage 0 auto formalization (Entry A: NL or Entry B: structured -> contract)")
    p.add_argument("--nl-spec", help="raw natural language requirements text (Entry A)")
    p.add_argument("--nl-file", help="path to raw natural language requirements file (Entry A)")
    p.add_argument("--from-nl", action="store_true", help="when combined with --example, run Stage 0 Entry A from the example's spec.md")
    p.add_argument("--requirements", help="structured R1..Rn requirements text (Entry B)")
    p.add_argument("--requirements-file", help="path to REQUIREMENTS.md or requirements.json (Entry B)")
    p.add_argument("--example", help="example id (Entry B, or Entry A with --from-nl)")
    p.add_argument("--examples-dir", help="override examples directory")
    p.add_argument("--contract", help="optional existing contract JSON path (Entry B validate & explain)")
    p.add_argument("--req-ir", help="optional Requirement Net IR JSON path (concir-req-ir/1)")
    p.add_argument("--title", default="requirements", help="title for generated REQUIREMENTS.md")
    add_provider_args(p)
    p.add_argument("--contract-script", help="optional separate scripted JSON for Step 0.2 contract synthesis")
    p.add_argument("--report", help="write FormalizationResult JSON here")

    p = sub.add_parser("repair", help="run deterministic backend repair search")
    p.add_argument("program")
    p.add_argument("contract")
    p.add_argument("--strategy", default="c", choices=["a", "b", "c"])
    p.add_argument("--candidate-budget", type=int, default=64)
    p.add_argument("--verification-budget", type=int, default=64)
    p.add_argument("--max-depth", type=int, default=4)
    p.add_argument("--max-total-edits", type=int, default=4)

    p = sub.add_parser("replay", help="replay and verify a repair artifact")
    p.add_argument("artifact")

    # ---- Orchestration commands ----
    p = sub.add_parser("e2e", help="run end-to-end traceable bi-level pipeline (Stage 0 + Loops A/B/C)")
    p.add_argument("--nl-spec", help="raw natural language requirements text (runs Stage 0 Entry A)")
    p.add_argument("--nl-file", help="path to raw natural language requirements file (runs Stage 0 Entry A)")
    p.add_argument("--from-nl", action="store_true", help="when combined with --example, run Stage 0 Entry A from the example's spec.md")
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--example", help="example id (e.g. visit_counter)")
    p.add_argument("--examples-dir", help="override examples directory")
    p.add_argument("--contract", help="frozen contract JSON path (optional: synthesized via Stage 0 if omitted)")
    p.add_argument("--req-ir", help="optional Requirement Net IR JSON path (concir-req-ir/1)")
    add_provider_args(p)
    p.add_argument("--clause-script", help="optional scripted JSON for Stage 0.1 nl_to_clauses")
    p.add_argument("--contract-script", help="optional scripted JSON for Stage 0.2 clauses_to_contract")
    p.add_argument("--hole-script", help="optional separate scripted JSON for hole filling/repair")
    p.add_argument("--initial-program", help="optional initial CIR/SCIR JSON path")
    p.add_argument("--max-cir-rounds", type=int, default=4)
    p.add_argument("--max-hole-rounds", type=int, default=3)
    p.add_argument("--max-escalations", type=int, default=2)
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p.add_argument("--run-summarize", action="store_true", help="enable modular lock-cycle summary check")
    p.add_argument("--modular-refinement", action="store_true", help="enable per-module SPN + verify-refinement --module")
    p.add_argument("--focus-module", action="append", help="explicit module(s) for modular refinement")
    p.add_argument("--run-dynamic-conform", action="store_true", help="enable post-static dynamic conformance")
    p.add_argument("--native-runs", type=int, default=10)
    p.add_argument("--miri-seeds", type=int, default=0)
    p.add_argument("--ast-only", action="store_true", help="run only AstHolePurityPlugin (skip SPN)")
    p.add_argument("--spn-binary", help="explicit path to spn binary")
    p.add_argument("--report", help="write E2EResult JSON here")

    p = sub.add_parser("revision", help="run iterative CIR revision loop against frozen contract")
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--example", help="example id (e.g. visit_counter)")
    p.add_argument("--examples-dir", help="override examples directory")
    p.add_argument("--contract", help="frozen contract JSON path (optional if --example is set)")
    add_provider_args(p)
    p.add_argument("--initial-program", help="optional initial CIR JSON path")
    p.add_argument("--max-rounds", type=int, default=4)
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p.add_argument("--reply-format", default="whole", choices=["whole", "local"])
    p.add_argument("--no-diagnostics", action="store_true")
    p.add_argument("--report", help="write RevisionResult JSON here")

    p = sub.add_parser("conformance", help="run skeleton codegen + traces + dynamic conformance")
    p.add_argument("--cases", required=True, help="JSON list of [label, program_path] pairs")
    p.add_argument("--native-runs", type=int, default=10)
    p.add_argument("--miri-seeds", type=int, default=0)

    p = sub.add_parser("list-examples", help="print JSON summary of active Stage 0 examples")
    p.add_argument("--examples-dir", help="override examples directory")

    p = sub.add_parser("mcp", help="run the ConcPlanVerify MCP stdio server (or inspect tools)")
    p.add_argument("--workdir", help="override workspace directory for MCP tool artifacts")
    p.add_argument("--spn-binary", help="explicit path to spn binary (overrides CONCIR_SPN)")
    p.add_argument("--lockbud-binary", help="explicit path to lockbud binary")
    p.add_argument("--list-tools", action="store_true", help="print registered MCP tools/resources/prompts and exit")

    return parser
