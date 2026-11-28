"""CLI command execution dispatch for ``concplan``."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from ..code_verify.conformance import run_conformance_smoke
from ..code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    SpnMirRefinementPlugin,
)
from ..core.concir_client import ConcirClient
from ..core.env import load_dotenv
from ..core.json_utils import write_json
from ..core.llm import create_llm_client, default_base_url
from ..core.models import ModelConfig
from ..core.prompts import (
    clauses_to_contract_system_prompt,
    hole_repair_system_prompt,
    hole_user_prompt_builder,
    llm_user_prompt_builder,
    local_revision_system_prompt,
    nl_to_clauses_system_prompt,
    scir_generation_system_prompt,
)
from ..core.providers import CandidateProvider, LlmCandidateProvider, ScriptedProvider
from ..formalize import formalize_from_nl, formalize_from_structured
from ..model_verify.revision_workflow import RevisionWorkflow
from ..pipeline.e2e_pipeline import run_e2e_pipeline
from ..pipeline.examples import examples_summary, load_example
from .parser import build_parser


def _read_json(path: str | Path) -> Any:
    return json.loads(Path(path).read_text(encoding="utf-8"))


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


def _resolve_task_inputs(args: argparse.Namespace) -> tuple[str, dict[str, Any], dict[str, str] | None, str]:
    task_label = "cli-task"
    clauses: dict[str, str] | None = None
    requirements: str | None = None
    contract: dict[str, Any] | None = None

    if getattr(args, "example", None):
        ex = load_example(args.example, examples_dir=getattr(args, "examples_dir", None))
        task_label = ex.example_id
        requirements = ex.load_requirements_text()
        contract = ex.load_contract()
        clauses = ex.load_requirements_clauses() or None

    if getattr(args, "requirements_file", None):
        requirements = Path(args.requirements_file).read_text(encoding="utf-8")
    elif getattr(args, "requirements", None):
        requirements = args.requirements

    if getattr(args, "contract", None):
        contract = _read_json(args.contract)

    if not requirements:
        raise ValueError("provide --requirements, --requirements-file, or --example")
    if contract is None:
        raise ValueError("provide --contract or --example")
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


def _resolve_nl_spec(args: argparse.Namespace) -> str | None:
    if args.nl_file:
        return Path(args.nl_file).read_text(encoding="utf-8")
    if args.nl_spec:
        return args.nl_spec
    if args.from_nl:
        if not args.example:
            raise ValueError("--from-nl requires --example")
        ex = load_example(args.example, examples_dir=args.examples_dir)
        return ex.load_spec_text()
    return None


def _build_optional_provider(
    args: argparse.Namespace,
    *,
    script_override: str | None,
    system_prompt: str,
    user_prompt_builder,
    active: bool = True,
) -> CandidateProvider | None:
    if script_override:
        return _build_provider(
            provider_name="scripted",
            script_path=script_override,
            model_id=None,
            api_key_env=None,
            base_url=None,
            llm_timeout=args.llm_timeout,
            system_prompt=system_prompt,
            user_prompt_builder=user_prompt_builder,
        )
    if active and args.provider != "scripted":
        return _build_provider(
            provider_name=args.provider,
            script_path=None,
            model_id=args.model,
            api_key_env=args.api_key_env,
            base_url=args.base_url,
            llm_timeout=args.llm_timeout,
            system_prompt=system_prompt,
            user_prompt_builder=user_prompt_builder,
        )
    return None


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    repo_root = Path(__file__).resolve().parents[2]
    load_dotenv(repo_root / ".env")

    try:
        if args.command == "list-examples":
            print(json.dumps(examples_summary(args.examples_dir), ensure_ascii=False, indent=2))
            return 0

        if args.command == "mcp":
            from ..mcp.server import create_server

            server = create_server(
                workdir=args.workdir or (Path(args.out) / "mcp"),
                concir_binary=args.binary if Path(args.binary).is_file() else None,
                spn_binary=args.spn_binary,
                lockbud_binary=args.lockbud_binary,
                timeout=args.timeout,
            )
            if args.list_tools:
                print(json.dumps(server.describe(), ensure_ascii=False, indent=2))
                return 0
            server.serve_stdio()
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
        if args.command == "check-req":
            return _emit(client.check_req(args.req_ir, contract=args.contract, out_path=args.output))
        if args.command == "verify-design":
            return _emit(client.verify_design(args.req, args.design, contract=args.contract, out_path=args.output))
        if args.command == "verify-refinement":
            return _emit(client.verify_refinement(
                args.spec,
                args.impl_net,
                contract_path=args.contract,
                codegen_map_path=args.codegen_map,
                module=args.module,
                out_path=args.output,
            ))
        if args.command == "validate-contract":
            return _emit(client.validate_contract(
                args.contract,
                requirements=args.requirements,
                strict_entities=not args.no_strict_entities,
                require_non_vacuous=not args.no_require_non_vacuous,
                normalize=args.normalize,
                out_path=args.output,
            ))
        if args.command == "explain-contract":
            return _emit(client.explain_contract(args.contract, out_path=args.output))
        if args.command == "export-cvn":
            return _emit(client.export_cvn(args.program, out_path=args.output))
        if args.command == "formalize":
            nl_text = _resolve_nl_spec(args)
            active_client = client if Path(args.binary).is_file() else None
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
                contract_prov = _build_optional_provider(
                    args,
                    script_override=args.contract_script,
                    system_prompt=clauses_to_contract_system_prompt(),
                    user_prompt_builder=hole_user_prompt_builder,
                )
                fres = formalize_from_nl(
                    nl_text,
                    clause_provider=clause_prov,
                    contract_provider=contract_prov,
                    client=active_client,
                    out_dir=args.out,
                    title=args.title,
                    req_ir=args.req_ir,
                )
            else:
                structured_src: Any = None
                existing_contract: dict[str, Any] | None = None
                existing_req_ir: Any = args.req_ir
                if args.example:
                    ex = load_example(args.example, examples_dir=args.examples_dir)
                    structured_src = ex.load_requirements_json() or ex.load_requirements_text()
                    if args.contract is None:
                        existing_contract = ex.load_contract()
                    if existing_req_ir is None:
                        existing_req_ir = ex.load_req_ir()
                if args.requirements_file:
                    structured_src = Path(args.requirements_file)
                elif args.requirements:
                    structured_src = args.requirements
                if args.contract:
                    existing_contract = _read_json(args.contract)
                if structured_src is None:
                    raise ValueError(
                        "provide --nl-spec/--nl-file (Entry A) or --requirements/--requirements-file/--example (Entry B)"
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
                    client=active_client,
                    out_dir=args.out,
                    req_ir=existing_req_ir,
                )

            payload = fres.as_dict()
            if args.report:
                write_json(args.report, payload)
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

        if args.command == "e2e":
            nl_text_e2e = _resolve_nl_spec(args)
            requirements: str | None = None
            contract: dict[str, Any] | None = None
            clauses: dict[str, str] | None = None
            req_ir_arg: Any = args.req_ir
            if nl_text_e2e is None and (args.contract or args.example):
                requirements, contract, clauses, _task_label = _resolve_task_inputs(args)
                if args.example and req_ir_arg is None:
                    req_ir_arg = load_example(args.example, examples_dir=args.examples_dir).load_req_ir()
            else:
                if args.example and nl_text_e2e is None:
                    ex = load_example(args.example, examples_dir=args.examples_dir)
                    requirements = ex.load_requirements_text()
                    contract = ex.load_contract()
                    clauses = ex.load_requirements_clauses() or None
                    if req_ir_arg is None:
                        req_ir_arg = ex.load_req_ir()
                if args.requirements_file:
                    requirements = Path(args.requirements_file).read_text(encoding="utf-8")
                elif args.requirements:
                    requirements = args.requirements
                if args.contract:
                    contract = _read_json(args.contract)
                if nl_text_e2e is None and not requirements:
                    raise ValueError("provide --nl-spec, --nl-file, --requirements, --requirements-file, or --example")

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
            clause_provider = _build_optional_provider(
                args,
                script_override=args.clause_script,
                system_prompt=nl_to_clauses_system_prompt(),
                user_prompt_builder=hole_user_prompt_builder,
                active=nl_text_e2e is not None,
            )
            contract_provider = _build_optional_provider(
                args,
                script_override=args.contract_script,
                system_prompt=clauses_to_contract_system_prompt(),
                user_prompt_builder=hole_user_prompt_builder,
                active=contract is None,
            )
            hole_provider = _build_optional_provider(
                args,
                script_override=args.hole_script,
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
                req_ir=req_ir_arg,
            )
            payload = result.as_dict()
            if args.report:
                write_json(args.report, payload)
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            if result.status == "verified":
                return 0
            if result.status == "unknown":
                return 3
            return 1

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
                write_json(args.report, payload)
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
