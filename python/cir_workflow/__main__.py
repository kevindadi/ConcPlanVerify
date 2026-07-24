"""Command-line entry point for the Python CIR workflow."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .env import load_dotenv
from .generation import GenerationWorkflow
from .llm import create_llm_client, default_base_url
from .models import ModelConfig
from .repair import RepairWorkflow
from .rust_cli import RustCli


def main() -> int:
    parser = argparse.ArgumentParser(description="Python LLM orchestration for CIR")
    parser.add_argument(
        "command",
        choices=["validate", "analyze", "goals", "generate", "repair"],
    )
    parser.add_argument("input", nargs="?", help="CIR JSON path, or - for stdin")
    parser.add_argument("--requirements", help="Natural-language requirements for generate")
    parser.add_argument("--source", help="Source file to include in generate")
    parser.add_argument("--binary", help="Path to cir2cvn binary")
    parser.add_argument("--model", default="workflow-model")
    parser.add_argument(
        "--provider",
        default="deepseek",
        choices=["deepseek", "qwen"],
        help="LLM provider used by generate/repair (default: deepseek)",
    )
    parser.add_argument(
        "--model-id",
        help="Provider model id (defaults to the selected provider's model)",
    )
    parser.add_argument(
        "--api-key-env",
        help="Environment variable containing the API key",
    )
    parser.add_argument("--base-url")
    parser.add_argument("--reasoning-effort")
    thinking = parser.add_mutually_exclusive_group()
    thinking.add_argument("--thinking", dest="thinking", action="store_true")
    thinking.add_argument("--no-thinking", dest="thinking", action="store_false")
    parser.set_defaults(thinking=None)
    parser.add_argument("--max-rounds", type=int, default=5)
    parser.add_argument("--max-tokens", type=int, default=4096)
    parser.add_argument("--temperature", type=float, default=0.0)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    load_dotenv(root / ".env")
    rust = RustCli(repo_root=root, binary=args.binary)

    if args.command in {"validate", "analyze", "goals"}:
        cir_json = _read_input(args.input)
        if args.command == "validate":
            result = rust.validate(cir_json)
        elif args.command == "goals":
            result = rust.goals(cir_json)
        else:
            result = rust.analyze(cir_json)
        _print_payload(result.payload, result.error)
        return result.exit_code

    provider = args.provider.lower()
    provider_defaults = {
        "deepseek": {
            "model_id": "deepseek-v4-pro",
            "api_key_env": "DEEPSEEK_API_KEY",
            "reasoning_effort": "high",
            "thinking": True,
        },
        "qwen": {
            "model_id": "qwen3.7-plus",
            "api_key_env": "DASHSCOPE_API_KEY",
            "reasoning_effort": None,
            "thinking": False,
        },
    }[provider]
    model = ModelConfig(
        name=args.model,
        provider=args.provider,
        model_id=args.model_id or provider_defaults["model_id"],
        api_key_env=args.api_key_env or provider_defaults["api_key_env"],
        base_url=args.base_url or default_base_url(args.provider),
        reasoning_effort=(
            args.reasoning_effort
            if args.reasoning_effort is not None
            else provider_defaults["reasoning_effort"]
        ),
        thinking_enabled=(
            args.thinking
            if args.thinking is not None
            else provider_defaults["thinking"]
        ),
    )
    try:
        client = create_llm_client(model)
    except Exception as error:
        print(f"LLM configuration error: {error}", file=sys.stderr)
        return 2

    if args.command == "generate":
        requirements = args.requirements
        if args.source:
            requirements = (requirements + "\n\n" if requirements else "") + _read_file(args.source)
        if not requirements:
            parser.error("generate requires --requirements or --source")
        result = GenerationWorkflow(
            client, rust, max_rounds=args.max_rounds,
            temperature=args.temperature, max_tokens=args.max_tokens,
        ).run(requirements)
        if result.success:
            print(result.cir_json)
            return 0
        print(result.error or "generation failed", file=sys.stderr)
        return 1

    cir_json = _read_input(args.input)
    result = RepairWorkflow(
        client, rust, max_rounds=args.max_rounds,
        temperature=args.temperature, max_tokens=args.max_tokens,
    ).run(cir_json)
    if result.success:
        print(result.fixed_cir_json)
        return 0
    print(result.error or result.last_feedback or "repair failed", file=sys.stderr)
    return 1


def _read_input(path: str | None) -> str:
    if not path or path == "-":
        return sys.stdin.read()
    return _read_file(path)


def _read_file(path: str) -> str:
    return Path(path).read_text()


def _print_payload(payload: dict | None, error: str | None) -> None:
    if payload is not None:
        print(json.dumps(payload, ensure_ascii=False))
    else:
        print(json.dumps({"status": "tool_error", "error": error or "unknown error"}))


if __name__ == "__main__":
    raise SystemExit(main())
