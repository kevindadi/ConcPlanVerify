"""Command-line entry point for the ConcIR Python workflow.

Three groups of commands:

- thin wrappers over the current ``concir-backend`` CLI: ``check``, ``support``,
  ``explore``, ``repair``, ``replay``, plus ``repair-context`` /
  ``evaluate-patch`` for single external patches;
- offline orchestration (no key, no network): ``offline`` and ``patch-repair``
  with a scripted provider;
- live DeepSeek Flash runs (real HTTP, pinned to ``deepseek-flash``, explicit
  thinking disabled, shared budget): ``live`` (generation) and ``live-repair``
  (single external patch).

The retired ``cir2cvn --validate/--analyze/--goals`` protocol and the
``generate``/``repair``/``plan``/``merge`` LLM commands are gone.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import sys
from pathlib import Path

from .concir_client import ConcirClient
from .env import load_dotenv
from .live import ALLOWED_MODEL, ALLOWED_PROVIDER, run_live_pilot, run_live_repair_pilot
from .offline_workflow import OfflineWorkflow
from .patch_repair import ExternalPatchRepairWorkflow
from .providers import ScriptedPatchProvider, ScriptedProvider


def _default_binary() -> str:
    env = os.environ.get("CONCIR_BACKEND")
    if env:
        return env
    repo_root = Path(__file__).resolve().parents[2]
    return str(repo_root.parent / "ConcIR" / "target" / "release" / "concir-backend")


def _read_json(path: str | None) -> dict:
    if not path or path == "-":
        return json.loads(sys.stdin.read())
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _emit(result) -> int:
    print(json.dumps(result.payload if result.payload is not None else _result_failure(result),
                     ensure_ascii=False, indent=2))
    if result.kind == "semantic":
        return int(result.exit_code or 0)
    return 2


def _result_failure(result) -> dict:
    return {
        "status": result.status,
        "kind": result.kind,
        "exit_code": result.exit_code,
        "error": result.error,
        "stderr": result.stderr,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="ConcIR Python workflow (current concir-backend CLI)")
    parser.add_argument("--binary", default=_default_binary(),
                        help="Path to the concir-backend binary (or set CONCIR_BACKEND)")
    parser.add_argument("--out", default=str(Path.cwd() / "cir-workflow-out"),
                        help="Directory for per-call inputs and raw CLI output")
    parser.add_argument("--timeout", type=float, default=30.0)
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("check"); p.add_argument("program")
    p = sub.add_parser("support"); p.add_argument("program")
    p = sub.add_parser("explore")
    p.add_argument("program"); p.add_argument("contract")
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p = sub.add_parser("repair")
    p.add_argument("program"); p.add_argument("contract")
    p.add_argument("--strategy", default="c", choices=["a", "b", "c"])
    p.add_argument("--candidate-budget", type=int, default=64)
    p.add_argument("--verification-budget", type=int, default=64)
    p.add_argument("--max-depth", type=int, default=4)
    p.add_argument("--max-total-edits", type=int, default=4)
    p = sub.add_parser("replay"); p.add_argument("artifact")

    p = sub.add_parser("offline")
    p.add_argument("--requirements", help="requirements text")
    p.add_argument("--requirements-file", help="read requirements from a file")
    p.add_argument("--contract", required=True, help="frozen contract JSON path")
    p.add_argument("--provider", default="scripted", choices=["scripted"])
    p.add_argument("--script", help="JSON file: list of scripted responses")
    p.add_argument("--max-generation-rounds", type=int, default=3)
    p.add_argument("--strategy", default="c", choices=["a", "b", "c"])
    p.add_argument("--engine", default="petri", choices=["petri", "interp"])
    p.add_argument("--candidate-budget", type=int, default=64)
    p.add_argument("--verification-budget", type=int, default=64)
    p.add_argument("--max-depth", type=int, default=4)
    p.add_argument("--max-total-edits", type=int, default=4)
    p.add_argument("--report", help="write the offline result JSON here")

    p = sub.add_parser("live", help="run frozen tasks against DeepSeek Flash (real HTTP)")
    p.add_argument("--tasks", required=True, help="frozen tasks JSON")
    p.add_argument("--provider", default=ALLOWED_PROVIDER)
    p.add_argument("--model", default=ALLOWED_MODEL)
    p.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    p.add_argument("--max-generation-rounds", type=int, default=3)
    p.add_argument("--max-tokens", type=int, default=4096)
    p.add_argument("--llm-timeout", type=float, default=90.0)

    p = sub.add_parser("patch-repair", help="offline scripted single-patch repair loop")
    p.add_argument("--model", required=True)
    p.add_argument("--contract", required=True)
    p.add_argument("--script", required=True, help="scripted patch responses JSON")
    p.add_argument("--max-rounds", type=int, default=3)
    p.add_argument("--report", help="write the patch-repair result JSON here")

    p = sub.add_parser("detection", help="Track D: run local detectors on the frozen benchmark")
    p.add_argument("--manifest", help="benchmarks/MANIFEST.json (default: repo manifest)")
    p.add_argument("--no-miri", action="store_true", help="skip Miri (fast)")

    p = sub.add_parser("repair-smoke",
                        help="Flash repair-type smoke batch (defect guaranteed)")
    p.add_argument("--tasks", help="benchmarks/MANIFEST.json (default: repo manifest)")
    p.add_argument("--protocol", required=True)
    p.add_argument("--protocol-sha256", required=True)
    p.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    p.add_argument("--max-tokens", type=int, default=4096)
    p.add_argument("--llm-timeout", type=float, default=90.0)

    p = sub.add_parser("contract-strength",
                       help="replay A3 accepted CIRs against frozen contracts (offline)")
    p.add_argument("--batches", required=True,
                   help="comma-separated batch directories to scan")
    p.add_argument("--report", help="output directory (default: --out)")

    p = sub.add_parser("fill-smoke", help="Flash hole-fill + A3_free conformance smoke")
    p.add_argument("--program", required=True)
    p.add_argument("--contract", required=True)
    p.add_argument("--protocol", required=True)
    p.add_argument("--protocol-sha256", required=True)
    p.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    p.add_argument("--max-tokens", type=int, default=4096)
    p.add_argument("--llm-timeout", type=float, default=90.0)

    p = sub.add_parser("conformance", help="offline conformance smoke (codegen+traces+conform)")
    p.add_argument("--cases", help="JSON list of [label, program_path]")
    p.add_argument("--native-runs", type=int, default=50)
    p.add_argument("--miri-seeds", type=int, default=64)

    p = sub.add_parser("scale", help="B5: generated lock-chain scale experiment (no LLM)")

    p = sub.add_parser("smoke", help="Flash smoke batch: validate the multi-arm chain")
    p.add_argument("--tasks", help="benchmarks/MANIFEST.json (default: repo manifest)")
    p.add_argument("--protocol", required=True, help="frozen smoke protocol path")
    p.add_argument("--protocol-sha256", required=True,
                   help="confirmation credential: the protocol file's sha256")
    p.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    p.add_argument("--max-tokens", type=int, default=4096)
    p.add_argument("--llm-timeout", type=float, default=90.0)

    p = sub.add_parser("live-repair", help="run frozen patch-repair tasks against DeepSeek Flash")
    p.add_argument("--tasks", required=True, help="frozen repair tasks JSON")
    p.add_argument("--provider", default=ALLOWED_PROVIDER)
    p.add_argument("--model", default=ALLOWED_MODEL)
    p.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    p.add_argument("--max-rounds", type=int, default=3)
    p.add_argument("--max-tokens", type=int, default=4096)
    p.add_argument("--llm-timeout", type=float, default=90.0)

    args = parser.parse_args(argv)
    repo_root = Path(__file__).resolve().parents[2]
    load_dotenv(repo_root / ".env")
    client = ConcirClient(args.binary, workdir=args.out, timeout=args.timeout)

    try:
        if args.command == "check":
            return _emit(client.check(args.program))
        if args.command == "support":
            return _emit(client.support(args.program))
        if args.command == "explore":
            return _emit(client.explore(args.program, args.contract, args.engine))
        if args.command == "repair":
            return _emit(client.repair(
                args.program, args.contract, strategy=args.strategy,
                candidate_budget=args.candidate_budget,
                verification_budget=args.verification_budget,
                max_depth=args.max_depth, max_total_edits=args.max_total_edits,
            ))
        if args.command == "replay":
            return _emit(client.replay(args.artifact))

        if args.command == "live":
            if args.provider != ALLOWED_PROVIDER or args.model != ALLOWED_MODEL:
                print(json.dumps({
                    "status": "error",
                    "error": f"only provider {ALLOWED_PROVIDER!r} and model {ALLOWED_MODEL!r} "
                             "are allowed (Pro/aliases/fallback are forbidden)",
                }, indent=2))
                return 2
            api_key = os.environ.get(args.api_key_env, "")
            if not api_key:
                print(json.dumps({
                    "status": "error",
                    "error": f"missing API key: set env var {args.api_key_env}",
                }, indent=2))
                return 2
            summary = run_live_pilot(
                args.tasks, out_dir=args.out, binary=args.binary, api_key=api_key,
                timeout=args.llm_timeout, max_tokens=args.max_tokens,
                max_generation_rounds=args.max_generation_rounds,
            )
            print(json.dumps(summary, ensure_ascii=False, indent=2))
            return 0

        if args.command == "patch-repair":
            responses = json.loads(Path(args.script).read_text(encoding="utf-8"))
            provider = ScriptedPatchProvider(responses)
            workflow = ExternalPatchRepairWorkflow(
                client, provider, out_dir=args.out, max_rounds=args.max_rounds)
            result = workflow.run(args.model, args.contract)
            payload = dataclasses.asdict(result)
            payload["rounds"] = [dataclasses.asdict(r) for r in result.rounds]
            if args.report:
                Path(args.report).write_text(
                    json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0 if result.accepted else 1

        if args.command == "smoke":
            from .flash_smoke import run_flash_smoke

            api_key = os.environ.get(args.api_key_env, "")
            if not api_key:
                print(json.dumps({"status": "error",
                                  "error": f"missing API key: set {args.api_key_env}"},
                                 indent=2))
                return 2
            manifest = args.tasks or str(repo_root / "benchmarks/MANIFEST.json")
            summary = run_flash_smoke(
                manifest, args.out, binary=args.binary, api_key=api_key,
                protocol_path=args.protocol, protocol_sha256=args.protocol_sha256,
                timeout=args.llm_timeout, max_tokens=args.max_tokens)
            print(json.dumps({"batch_dir": summary["batch_dir"],
                              "requests_used": summary["requests_used"],
                              "stop_reason": summary["stop_reason"]}, indent=2))
            return 0

        if args.command == "repair-smoke":
            from .flash_smoke import run_repair_smoke

            api_key = os.environ.get(args.api_key_env, "")
            if not api_key:
                print(json.dumps({"status": "error",
                                  "error": f"missing API key: set {args.api_key_env}"},
                                 indent=2))
                return 2
            manifest = args.tasks or str(repo_root / "benchmarks/MANIFEST.json")
            summary = run_repair_smoke(
                manifest, args.out, binary=args.binary, api_key=api_key,
                protocol_path=args.protocol, protocol_sha256=args.protocol_sha256,
                timeout=args.llm_timeout, max_tokens=args.max_tokens)
            print(json.dumps({"batch_dir": summary["batch_dir"],
                              "requests_used": summary["requests_used"],
                              "stop_reason": summary["stop_reason"]}, indent=2))
            return 0

        if args.command == "contract-strength":
            from .contract_strength import recompute, write_report

            batches = [Path(p) for p in args.batches.split(",") if p]
            records = recompute(batches, repo_root, binary=args.binary)
            report_dir = Path(args.report) if args.report else Path(args.out)
            write_report(records, report_dir)
            print(json.dumps({"records": len(records),
                              "new_outcomes": {r.new_outcome: 1 for r in records}}, indent=2))
            return 0

        if args.command == "fill-smoke":
            from .flash_smoke import run_fill_smoke

            api_key = os.environ.get(args.api_key_env, "")
            if not api_key:
                print(json.dumps({"status": "error",
                                  "error": f"missing API key: set {args.api_key_env}"},
                                 indent=2))
                return 2
            summary = run_fill_smoke(
                args.program, args.contract, args.out, binary=args.binary,
                api_key=api_key, protocol_path=args.protocol,
                protocol_sha256=args.protocol_sha256, timeout=args.llm_timeout,
                max_tokens=args.max_tokens)
            print(json.dumps({"batch_dir": summary["batch_dir"],
                              "requests_used": summary["requests_used"]}, indent=2))
            return 0

        if args.command == "conformance":
            from .conformance import run_conformance_smoke

            if args.cases:
                cases = [(str(a), str(b)) for a, b in json.loads(args.cases)]
            else:
                base = repo_root / "benchmarks/families"
                cases = [
                    ("abba_2lock__fixed", base / "lock-order/abba_2lock/fixed.cir.json"),
                    ("lost_wakeup__fixed",
                     base / "condvar/lost_wakeup_notify_before_wait/fixed.cir.json"),
                    ("permit_leak__fixed", base / "semaphore/permit_leak/fixed.cir.json"),
                    ("scope_bound__correct",
                     base / "structure/scope_bound_k_workers/correct.cir.json"),
                ]
            payload = run_conformance_smoke(
                cases, args.out, binary=args.binary, native_runs=args.native_runs,
                miri_seeds=args.miri_seeds)
            print(json.dumps({"cases": len(payload["cases"]),
                              "statuses": {r["case"]: r.get("status")
                                           for r in payload["cases"]}}, indent=2))
            return 0

        if args.command == "scale":
            from .scale import run_scale

            payload = run_scale(args.out)
            print(json.dumps({"runs": len(payload["runs"]), "out": str(args.out)}, indent=2))
            return 0

        if args.command == "detection":
            from .detection import render_markdown, run_detection

            manifest = args.manifest or str(repo_root / "benchmarks/MANIFEST.json")
            out = Path(args.out)
            result = run_detection(manifest, out, run_miri=not args.no_miri)
            (out / "DETECTION.md").write_text(render_markdown(result), encoding="utf-8")
            print(json.dumps({"records": len(result["records"]),
                              "lockbud_available": result["lockbud_available"],
                              "out": str(out)}, indent=2))
            return 0

        if args.command == "live-repair":
            if args.provider != ALLOWED_PROVIDER or args.model != ALLOWED_MODEL:
                print(json.dumps({
                    "status": "error",
                    "error": f"only provider {ALLOWED_PROVIDER!r} and model {ALLOWED_MODEL!r} "
                             "are allowed (Pro/aliases/fallback are forbidden)",
                }, indent=2))
                return 2
            api_key = os.environ.get(args.api_key_env, "")
            if not api_key:
                print(json.dumps({"status": "error",
                                  "error": f"missing API key: set env var {args.api_key_env}"}, indent=2))
                return 2
            summary = run_live_repair_pilot(
                args.tasks, out_dir=args.out, binary=args.binary, api_key=api_key,
                timeout=args.llm_timeout, max_tokens=args.max_tokens,
                max_rounds=args.max_rounds,
            )
            print(json.dumps(summary, ensure_ascii=False, indent=2))
            return 0

        # ---- offline ----
        requirements = args.requirements
        if args.requirements_file:
            requirements = Path(args.requirements_file).read_text(encoding="utf-8")
        if not requirements:
            parser.error("offline requires --requirements or --requirements-file")
        contract = json.loads(Path(args.contract).read_text(encoding="utf-8"))
        if not args.script:
            parser.error("offline --provider scripted requires --script")
        responses = json.loads(Path(args.script).read_text(encoding="utf-8"))
        provider = ScriptedProvider(responses)
        workflow = OfflineWorkflow(
            client, provider, out_dir=args.out,
            max_generation_rounds=args.max_generation_rounds, engine=args.engine,
            repair_strategy=args.strategy,
            repair_config={
                "candidate_budget": args.candidate_budget,
                "verification_budget": args.verification_budget,
                "max_depth": args.max_depth,
                "max_total_edits": args.max_total_edits,
            },
        )
        result = workflow.run(requirements, contract)
        payload = dataclasses.asdict(result)
        if args.report:
            Path(args.report).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                                         encoding="utf-8")
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return {"already_satisfied": 0, "repaired": 0, "unknown": 3,
                "unsupported": 5, "invalid": 4}.get(result.status, 1)
    except FileNotFoundError as exc:
        print(json.dumps({"status": "tool_error", "error": str(exc)}, indent=2))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
