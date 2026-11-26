"""Unified v5 command-line entry point for ConcPlanVerify (``python -m concplan``).

Commands:
- Stage 0 Auto formalization: ``formalize``, ``explain-contract``
- Backend pass-through: ``check``, ``support``, ``explore``, ``lower``,
  ``summarize``, ``verify-refinement``, ``export-cvn``, ``repair``, ``replay``
- Orchestration workflows: ``e2e``, ``offline``, ``revision``,
  ``patch-repair``, ``conformance``, ``list-benchmarks``
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import sys
from pathlib import Path
from typing import Any

from .code_verify.conformance import run_conformance_smoke
from .code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    SpnMirRefinementPlugin,
)
from .core.concir_client import ConcirClient
from .core.env import load_dotenv
from .core.llm import create_llm_client, default_base_url
from .core.models import ModelConfig
from .core.prompts import (
    clauses_to_contract_system_prompt,
    hole_repair_system_prompt,
    hole_user_prompt_builder,
    llm_user_prompt_builder,
    local_revision_system_prompt,
    nl_to_clauses_system_prompt,
    scir_generation_system_prompt,
)
from .core.providers import (
    CandidateProvider,
    LlmCandidateProvider,
    ScriptedPatchProvider,
    ScriptedProvider,
)
from .core.toolchain import SUBMODULE_RELEASE, find_concir_backend, resolve_exe_in_dir
from .formalize import (
    explain_contract_data,
    formalize_from_nl,
    formalize_from_structured,
)
from .model_verify.offline_workflow import OfflineWorkflow
from .model_verify.patch_repair import ExternalPatchRepairWorkflow
from .model_verify.revision_workflow import RevisionWorkflow
from .pipeline.benchmarks import benchmarks_summary, load_benchmark
from .pipeline.e2e_pipeline import run_e2e_pipeline


def _default_binary() -> str:
    found = find_concir_backend()
    if found is not None:
        return str(found)
    return str(resolve_exe_in_dir(SUBMODULE_RELEASE, "concir-backend"))


def _read_json(path: str | Path) -> Any:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _write_json(path: str | Path, payload: Any) -> None:
    dest = Path(path).expanduser().resolve()
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def _result_failure(result) -> dict[str, Any]:
    return {
        "status": result.status,
        "kind": result.kind,
        "exit_code": result.exit_code,
        "error": result.error,
        "stderr": result.stderr,
    }


def _emit(result) -> int:
    payload = result.payload if result.payload is not None else _result_failure(result)
    print(json.dumps(payload, ensure_ascii=False, indent=2))
    if result.kind == "semantic":
        return int(result.exit_code or 0)
    return 2


def _resolve_task_inputs(
    args: argparse.Namespace,
) -> tuple[str, dict[str, Any], dict[str, str] | None, str]:
    """Load (requirements_text, contract_dict, requirements_clauses, task_label)."""
    task_label = "cli-task"
    clauses: dict[str, str] | None = None
    requirements: str | None = None
    contract: dict[str, Any] | None = None

    if getattr(args, "task", None):
        task = load_benchmark(args.task, benchmarks_dir=getattr(args, "benchmarks_dir", None))
        task_label = task.task_id
        requirements = task.load_requirements_text()
        contract = task.load_contract()
        clauses = task.load_requirements_clauses() or None

    if getattr(args, "requirements_file", None):
        requirements = Path(args.requirements_file).read_text(encoding="utf-8")
    elif getattr(args, "requirements", None):
        requirements = args.requirements

    if getattr(args, "contract", None):
        contract = _read_json(args.contract)

    if not requirements:
        raise ValueError("provide --requirements, --requirements-file, or --task")
    if contract is None:
        raise ValueError("provide --contract or --task")
    return requirements, contract, clauses, task_label


def _build_provider(
    *,
    provider_name: str,
    script_path: str | None,
    model_id: str | None,
    api_key_env: str | None,
    base_url: str | None,
    llm_timeout: float,
    system_prompt: str,
    user_prompt_builder,
) -> CandidateProvider:
    if provider_name == "scripted":
        if not script_path:
            raise ValueError("--script is required when --provider=scripted")
        raw = _read_json(script_path)
        if not isinstance(raw, list):
            raise ValueError("--script JSON must be a list of {'text': ...} or {'error': ...} entries")
        return ScriptedProvider(raw)

    default_models = {"deepseek": "deepseek-chat", "qwen": "qwen-plus"}
    default_env_keys = {"deepseek": "DEEPSEEK_API_KEY", "qwen": "DASHSCOPE_API_KEY"}
    chosen_model = model_id or default_models.get(provider_name, provider_name)
    chosen_key_env = api_key_env or default_env_keys.get(provider_name, "OPENAI_API_KEY")
    chosen_url = base_url or default_base_url(provider_name)
    cfg = ModelConfig(
        name=provider_name,
        provider=provider_name,
        model_id=chosen_model,
        api_key_env=chosen_key_env,
        base_url=chosen_url,
    )
    llm_client = create_llm_client(cfg, timeout=llm_timeout)
    return LlmCandidateProvider(
        llm_client,
        cfg,
        system_prompt=system_prompt,
        user_prompt_builder=user_prompt_builder,
    )


def _add_provider_args(parser: argparse.ArgumentParser) -> None:
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
        default=_default_binary(),
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

    p = sub.add_parser("verify-refinement", help="verify code-to-model stuttering refinement")
    p.add_argument("--spec", required=True, help="verified CIR program JSON")
    p.add_argument("--impl-net", required=True, help="exported concir-impl-net/1 JSON")
    p.add_argument("--contract", help="optional contract JSON")
    p.add_argument("--codegen-map", help="optional codegen.json / codegen_filled.json")
    p.add_argument("--module", help="optional single module name for modular refinement")
    p.add_argument("--output", help="write concir-refinement-report/1 JSON here")

    p = sub.add_parser("explain-contract", help="render deterministic NL explanation of a contract JSON")
    p.add_argument("contract", help="contract JSON path")
    p.add_argument("--output", help="write concir-contract-explanation/1 JSON here")
    p.add_argument("--python-only", action="store_true", help="use pure-Python renderer without invoking concir-backend")

    p = sub.add_parser("export-cvn", help="export a CIR/SCIR program to UniPN CVN JSON")
    p.add_argument("program", help="CIR or SCIR program JSON path")
    p.add_argument("--output", help="write exported CVN JSON here")

    p = sub.add_parser("formalize", help="Stage 0 auto formalization (Entry A: NL or Entry B: structured -> contract)")
    p.add_argument("--nl-spec", help="raw natural language requirements text (Entry A)")
    p.add_argument("--nl-file", help="path to raw natural language requirements file (Entry A)")
    p.add_argument("--requirements", help="structured R1..Rn requirements text (Entry B)")
    p.add_argument("--requirements-file", help="path to REQUIREMENTS.md or requirements.json (Entry B)")
    p.add_argument("--task", help="benchmark task id (Entry B)")
    p.add_argument("--benchmarks-dir", help="override benchmarks directory")
    p.add_argument("--contract", help="optional existing contract JSON path (Entry B validate & explain)")
    p.add_argument("--title", default="requirements", help="title for generated REQUIREMENTS.md")
    _add_provider_args(p)
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
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--task", help="benchmark task id (e.g. lock-order/abba_2lock)")
    p.add_argument("--benchmarks-dir", help="override benchmarks directory")
    p.add_argument("--contract", help="frozen contract JSON path (optional: synthesized via Stage 0 if omitted)")
    _add_provider_args(p)
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

    p = sub.add_parser("offline", help="run offline generate -> check -> explore -> tool repair workflow")
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--task", help="benchmark task id (e.g. lock-order/abba_2lock)")
    p.add_argument("--benchmarks-dir", help="override benchmarks directory")
    p.add_argument("--contract", help="frozen contract JSON path (optional if --task is set)")
    _add_provider_args(p)
    p.add_argument("--max-generation-rounds", type=int, default=3)
    p.add_argument("--strategy", default="c", choices=["a", "b", "c"])
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p.add_argument("--candidate-budget", type=int, default=64)
    p.add_argument("--verification-budget", type=int, default=64)
    p.add_argument("--max-depth", type=int, default=4)
    p.add_argument("--max-total-edits", type=int, default=4)
    p.add_argument("--report", help="write OfflineResult JSON here")

    p = sub.add_parser("revision", help="run iterative CIR revision loop against frozen contract")
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--task", help="benchmark task id (e.g. lock-order/abba_2lock)")
    p.add_argument("--benchmarks-dir", help="override benchmarks directory")
    p.add_argument("--contract", help="frozen contract JSON path (optional if --task is set)")
    _add_provider_args(p)
    p.add_argument("--initial-program", help="optional initial CIR JSON path")
    p.add_argument("--max-rounds", type=int, default=4)
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p.add_argument("--reply-format", default="whole", choices=["whole", "local"])
    p.add_argument("--no-diagnostics", action="store_true")
    p.add_argument("--report", help="write RevisionResult JSON here")

    p = sub.add_parser("patch-repair", help="run constrained single-patch repair loop")
    p.add_argument("--model", required=True, help="frozen CIR program JSON")
    p.add_argument("--contract", required=True, help="frozen contract JSON")
    p.add_argument("--script", required=True, help="scripted patch responses JSON")
    p.add_argument("--max-rounds", type=int, default=3)
    p.add_argument("--report", help="write PatchRepairResult JSON here")

    p = sub.add_parser("conformance", help="run skeleton codegen + traces + dynamic conformance")
    p.add_argument("--cases", required=True, help="JSON list of [label, program_path] pairs")
    p.add_argument("--native-runs", type=int, default=10)
    p.add_argument("--miri-seeds", type=int, default=0)

    p = sub.add_parser("list-benchmarks", help="print JSON summary of active benchmark tasks")
    p.add_argument("--benchmarks-dir", help="override benchmarks directory")
    p.add_argument("--suite", choices=["families", "real-cases"], help="filter by benchmark suite")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    repo_root = Path(__file__).resolve().parents[2]
    load_dotenv(repo_root / ".env")

    try:
        if args.command == "list-benchmarks":
            summary = benchmarks_summary(args.benchmarks_dir)
            if args.suite:
                summary["tasks"] = [t for t in summary["tasks"] if t["suite"] == args.suite]
                summary["total"] = len(summary["tasks"])
            print(json.dumps(summary, ensure_ascii=False, indent=2))
            return 0

        client = ConcirClient(args.binary, workdir=args.out, timeout=args.timeout)

        # ---- Backend pass-throughs ----
        if args.command == "check":
            return _emit(client.check(args.program))
        if args.command == "support":
            return _emit(client.support(args.program))
        if args.command == "explore":
            return _emit(client.explore(args.program, args.contract, args.engine))
        if args.command == "lower":
            return _emit(client.lower(args.program, out_path=args.output, trace_map_path=args.trace_map))
        if args.command == "summarize":
            return _emit(client.summarize(args.program, module=args.module, out_path=args.output))
        if args.command == "verify-refinement":
            return _emit(client.verify_refinement(
                args.spec,
                args.impl_net,
                contract_path=args.contract,
                codegen_map_path=args.codegen_map,
                module=args.module,
                out_path=args.output,
            ))
        if args.command == "export-cvn":
            return _emit(client.export_cvn(args.program, out_path=args.output))
        if args.command == "explain-contract":
            contract_data = _read_json(args.contract)
            if not args.python_only and Path(args.binary).is_file():
                res = client.explain_contract(args.contract, out_path=args.output)
                if res.kind == "semantic":
                    return _emit(res)
            exp = explain_contract_data(contract_data)
            payload = exp.as_dict()
            if args.output:
                _write_json(args.output, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0
        if args.command == "formalize":
            nl_text: str | None = None
            if args.nl_file:
                nl_text = Path(args.nl_file).read_text(encoding="utf-8")
            elif args.nl_spec:
                nl_text = args.nl_spec

            if nl_text is not None:
                clause_prov = _build_provider(
                    provider_name=args.provider,
                    script_path=args.script,
                    model_id=args.model,
                    api_key_env=args.api_key_env,
                    base_url=args.base_url,
                    llm_timeout=args.llm_timeout,
                    system_prompt=nl_to_clauses_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )
                contract_prov: CandidateProvider | None = None
                if args.contract_script:
                    contract_prov = _build_provider(
                        provider_name="scripted",
                        script_path=args.contract_script,
                        model_id=None,
                        api_key_env=None,
                        base_url=None,
                        llm_timeout=args.llm_timeout,
                        system_prompt=clauses_to_contract_system_prompt(),
                        user_prompt_builder=hole_user_prompt_builder,
                    )
                elif args.provider != "scripted":
                    contract_prov = _build_provider(
                        provider_name=args.provider,
                        script_path=None,
                        model_id=args.model,
                        api_key_env=args.api_key_env,
                        base_url=args.base_url,
                        llm_timeout=args.llm_timeout,
                        system_prompt=clauses_to_contract_system_prompt(),
                        user_prompt_builder=hole_user_prompt_builder,
                    )
                fres = formalize_from_nl(
                    nl_text,
                    clause_provider=clause_prov,
                    contract_provider=contract_prov,
                    client=client if Path(args.binary).is_file() else None,
                    out_dir=args.out,
                    title=args.title,
                )
            else:
                structured_src: Any = None
                existing_contract: dict[str, Any] | None = None
                if args.task:
                    task = load_benchmark(args.task, benchmarks_dir=args.benchmarks_dir)
                    structured_src = task.load_requirements_json() or task.load_requirements_text()
                    if args.contract is None:
                        existing_contract = task.load_contract()
                if args.requirements_file:
                    structured_src = Path(args.requirements_file)
                elif args.requirements:
                    structured_src = args.requirements
                if args.contract:
                    existing_contract = _read_json(args.contract)
                if structured_src is None:
                    raise ValueError(
                        "provide --nl-spec/--nl-file (Entry A) or --requirements/--requirements-file/--task (Entry B)"
                    )
                contract_prov = None
                if existing_contract is None:
                    contract_prov = _build_provider(
                        provider_name=args.provider,
                        script_path=args.contract_script or args.script,
                        model_id=args.model,
                        api_key_env=args.api_key_env,
                        base_url=args.base_url,
                        llm_timeout=args.llm_timeout,
                        system_prompt=clauses_to_contract_system_prompt(),
                        user_prompt_builder=hole_user_prompt_builder,
                    )
                fres = formalize_from_structured(
                    structured_src,
                    contract=existing_contract,
                    contract_provider=contract_prov,
                    client=client if Path(args.binary).is_file() else None,
                    out_dir=args.out,
                )

            payload = fres.as_dict()
            if args.report:
                _write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if fres.ok else 1
        if args.command == "repair":
            return _emit(client.repair(
                args.program,
                args.contract,
                strategy=args.strategy,
                candidate_budget=args.candidate_budget,
                verification_budget=args.verification_budget,
                max_depth=args.max_depth,
                max_total_edits=args.max_total_edits,
            ))
        if args.command == "replay":
            return _emit(client.replay(args.artifact))

        # ---- Orchestration commands ----
        if args.command == "e2e":
            nl_text_e2e: str | None = None
            if args.nl_file:
                nl_text_e2e = Path(args.nl_file).read_text(encoding="utf-8")
            elif args.nl_spec:
                nl_text_e2e = args.nl_spec

            requirements: str | None = None
            contract: dict[str, Any] | None = None
            clauses: dict[str, str] | None = None
            if nl_text_e2e is None and (args.contract or args.task):
                requirements, contract, clauses, _task_label = _resolve_task_inputs(args)
            else:
                if args.task:
                    task = load_benchmark(args.task, benchmarks_dir=args.benchmarks_dir)
                    requirements = task.load_requirements_text()
                    contract = task.load_contract()
                    clauses = task.load_requirements_clauses() or None
                if args.requirements_file:
                    requirements = Path(args.requirements_file).read_text(encoding="utf-8")
                elif args.requirements:
                    requirements = args.requirements
                if args.contract:
                    contract = _read_json(args.contract)
                if nl_text_e2e is None and not requirements:
                    raise ValueError("provide --nl-spec, --nl-file, --requirements, --requirements-file, or --task")

            cir_provider = _build_provider(
                provider_name=args.provider,
                script_path=args.script,
                model_id=args.model,
                api_key_env=args.api_key_env,
                base_url=args.base_url,
                llm_timeout=args.llm_timeout,
                system_prompt=scir_generation_system_prompt(),
                user_prompt_builder=llm_user_prompt_builder,
            )
            clause_provider: CandidateProvider | None = None
            if args.clause_script:
                clause_provider = _build_provider(
                    provider_name="scripted",
                    script_path=args.clause_script,
                    model_id=None,
                    api_key_env=None,
                    base_url=None,
                    llm_timeout=args.llm_timeout,
                    system_prompt=nl_to_clauses_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )
            elif nl_text_e2e is not None and args.provider != "scripted":
                clause_provider = _build_provider(
                    provider_name=args.provider,
                    script_path=None,
                    model_id=args.model,
                    api_key_env=args.api_key_env,
                    base_url=args.base_url,
                    llm_timeout=args.llm_timeout,
                    system_prompt=nl_to_clauses_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )

            contract_provider: CandidateProvider | None = None
            if args.contract_script:
                contract_provider = _build_provider(
                    provider_name="scripted",
                    script_path=args.contract_script,
                    model_id=None,
                    api_key_env=None,
                    base_url=None,
                    llm_timeout=args.llm_timeout,
                    system_prompt=clauses_to_contract_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )
            elif contract is None and args.provider != "scripted":
                contract_provider = _build_provider(
                    provider_name=args.provider,
                    script_path=None,
                    model_id=args.model,
                    api_key_env=args.api_key_env,
                    base_url=args.base_url,
                    llm_timeout=args.llm_timeout,
                    system_prompt=clauses_to_contract_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )

            hole_provider: CandidateProvider | None = None
            if args.hole_script:
                hole_provider = _build_provider(
                    provider_name="scripted",
                    script_path=args.hole_script,
                    model_id=None,
                    api_key_env=None,
                    base_url=None,
                    llm_timeout=args.llm_timeout,
                    system_prompt=hole_repair_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )
            elif args.provider != "scripted":
                hole_provider = _build_provider(
                    provider_name=args.provider,
                    script_path=None,
                    model_id=args.model,
                    api_key_env=args.api_key_env,
                    base_url=args.base_url,
                    llm_timeout=args.llm_timeout,
                    system_prompt=hole_repair_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )

            plugins = [AstHolePurityPlugin()]
            if not args.ast_only:
                plugins.append(
                    SpnMirRefinementPlugin(
                        client=client,
                        spn_binary=args.spn_binary,
                        modular=args.modular_refinement,
                        focus_modules=args.focus_module,
                    )
                )
            verifier = CompositeStaticVerifier(plugins)

            result = run_e2e_pipeline(
                requirements=requirements,
                contract=contract,
                client=client,
                cir_provider=cir_provider,
                hole_provider=hole_provider,
                out_dir=args.out,
                static_verifier=verifier,
                max_cir_rounds=args.max_cir_rounds,
                max_hole_rounds=args.max_hole_rounds,
                max_escalations=args.max_escalations,
                engine=args.engine,
                initial_program=args.initial_program,
                run_summarize=args.run_summarize,
                run_dynamic_conform=args.run_dynamic_conform,
                native_runs=args.native_runs,
                miri_seeds=args.miri_seeds,
                requirements_clauses=clauses,
                nl_spec=nl_text_e2e,
                clause_provider=clause_provider,
                contract_provider=contract_provider,
                modular_refinement=args.modular_refinement,
                focus_modules=args.focus_module,
            )
            payload = result.as_dict()
            if args.report:
                _write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            if result.status == "verified":
                return 0
            if result.status == "unknown":
                return 3
            return 1

        if args.command == "offline":
            requirements, contract, _clauses, _task_label = _resolve_task_inputs(args)
            provider = _build_provider(
                provider_name=args.provider,
                script_path=args.script,
                model_id=args.model,
                api_key_env=args.api_key_env,
                base_url=args.base_url,
                llm_timeout=args.llm_timeout,
                system_prompt=scir_generation_system_prompt(),
                user_prompt_builder=llm_user_prompt_builder,
            )
            wf = OfflineWorkflow(
                client,
                provider,
                out_dir=args.out,
                max_generation_rounds=args.max_generation_rounds,
                engine=args.engine,
                repair_strategy=args.strategy,
                repair_config={
                    "candidate_budget": args.candidate_budget,
                    "verification_budget": args.verification_budget,
                    "max_depth": args.max_depth,
                    "max_total_edits": args.max_total_edits,
                },
            )
            result = wf.run(requirements, contract)
            payload = dataclasses.asdict(result)
            if args.report:
                _write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if result.status in {"repaired", "already_satisfied"} else 1

        if args.command == "revision":
            requirements, contract, _clauses, task_label = _resolve_task_inputs(args)
            sys_prompt = (
                local_revision_system_prompt()
                if args.reply_format == "local"
                else scir_generation_system_prompt()
            )
            provider = _build_provider(
                provider_name=args.provider,
                script_path=args.script,
                model_id=args.model,
                api_key_env=args.api_key_env,
                base_url=args.base_url,
                llm_timeout=args.llm_timeout,
                system_prompt=sys_prompt,
                user_prompt_builder=llm_user_prompt_builder,
            )
            wf = RevisionWorkflow(
                client,
                provider,
                out_dir=args.out,
                max_rounds=args.max_rounds,
                engine=args.engine,
                diagnostics=not args.no_diagnostics,
                reply_format=args.reply_format,
            )
            result = wf.run(
                requirements,
                contract,
                task_id=task_label,
                initial_program=args.initial_program,
            )
            payload = result.as_dict()
            if args.report:
                _write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if result.accepted else 1

        if args.command == "patch-repair":
            script = _read_json(args.script)
            provider = ScriptedPatchProvider(script)
            wf = ExternalPatchRepairWorkflow(
                client,
                provider,
                out_dir=args.out,
                max_rounds=args.max_rounds,
            )
            result = wf.run(args.model, args.contract)
            payload = dataclasses.asdict(result)
            if args.report:
                _write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if result.accepted else 1

        if args.command == "conformance":
            raw_cases = _read_json(args.cases)
            cases = [(str(label), Path(path)) for label, path in raw_cases]
            payload = run_conformance_smoke(
                cases,
                args.out,
                binary=args.binary,
                native_runs=args.native_runs,
                miri_seeds=args.miri_seeds,
            )
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if all(c.get("status") == "ready" for c in payload.get("cases", [])) else 1

    except (FileNotFoundError, KeyError, ValueError, json.JSONDecodeError) as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        return 2

    return 2


if __name__ == "__main__":
    raise SystemExit(main())
