"""Benchmark experiment runner.

Runs the methods from the experiment design in ``todo.md`` over the cases of
``benchmarks/manifest.json`` and writes one structured results JSON.

Methods:

- ``analyze``        — no LLM; run ``cir2cvn --analyze`` on the gold buggy CIR
                       and score the verdict against the manifest expectation.
- ``validate_only``  — no LLM; static CIR validation only (ablation showing
                       what schema checks alone can catch).
- ``repair_cvn``     — full CVN pipeline: repair loop with structured feedback.
- ``repair_status_only`` — diagnostic ablation: feedback reduced to
                       status + primary bug kind.
- ``repair_llm_only``    — LLM-only baseline: no CVN diagnostics in the prompt
                       (the Rust analyzer still judges acceptance).
- ``generate``       — natural-language generation from the manifest
                       requirements, then a full analyze of the produced CIR.

All experiments use DeepSeek (``deepseek-v4-pro`` by default; pass
``--model-id deepseek-v4-flash`` for the flash model).

Usage (from the repository root):

    PYTHONPATH=python python -m cir_workflow.experiment \
        --manifest benchmarks/manifest.json \
        --methods analyze,validate_only \
        --out results/offline.json
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .env import load_dotenv
from .generation import GenerationWorkflow
from .llm import create_llm_client, default_base_url
from .metrics import cir_metrics
from .models import ModelConfig, RustCliResult
from .repair import RepairWorkflow
from .rust_cli import RustCli

OFFLINE_METHODS = ("analyze", "validate_only")
LLM_METHODS = ("repair_cvn", "repair_status_only", "repair_llm_only", "generate")
ALL_METHODS = OFFLINE_METHODS + LLM_METHODS

REPAIR_FEEDBACK_MODES = {
    "repair_cvn": "full",
    "repair_status_only": "status_only",
    "repair_llm_only": "none",
}


def load_manifest(path: Path) -> dict[str, Any]:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict) or not isinstance(manifest.get("cases"), list):
        raise ValueError(f"{path} is not a valid benchmark manifest")
    return manifest


def expected_verdict(case: dict[str, Any]) -> str:
    return str(case.get("expected", {}).get("outcome", "bug"))


def verdict_from_status(status: str) -> str:
    if status == "verified_unsafe":
        return "bug"
    if status == "goals_unmet":
        return "goals_unmet"
    if status == "verified_safe":
        return "safe"
    return status


def cvn_metrics_from_payload(payload: dict[str, Any] | None) -> dict[str, Any]:
    """Project the CVN size/analysis fields out of an --analyze payload."""

    if not payload:
        return {}
    bugs = payload.get("bugs") or []
    return {
        "places": payload.get("places"),
        "transitions": payload.get("transitions"),
        "places_by_kind": payload.get("places_by_kind"),
        "input_arcs": payload.get("input_arcs"),
        "output_arcs": payload.get("output_arcs"),
        "state_count": payload.get("state_count"),
        "analysis_complete": payload.get("analysis_complete"),
        "max_states": payload.get("max_states"),
        "declared_goal_count": payload.get("declared_goal_count"),
        "unmet_goal_count": len(payload.get("unmet_goals") or []),
        "bug_count": len(bugs),
        "bug_kinds": [_kind_name(bug.get("kind")) for bug in bugs],
        "timings": payload.get("timings"),
    }


def _kind_name(kind: Any) -> str | None:
    if isinstance(kind, dict) and kind:
        return next(iter(kind))
    if isinstance(kind, str):
        return kind
    return None


def _score(case: dict[str, Any], analyze_result: RustCliResult) -> dict[str, Any]:
    """Compare one analyze verdict against the manifest gold label."""

    expected = expected_verdict(case)
    actual = verdict_from_status(analyze_result.status)
    expected_kind = case.get("expected", {}).get("bug_kind")
    payload_metrics = cvn_metrics_from_payload(analyze_result.payload)
    actual_kinds = payload_metrics.get("bug_kinds") or []
    kind_match = None
    if expected == "bug" and expected_kind:
        kind_match = expected_kind in actual_kinds
    correct = actual == expected and (kind_match is not False)
    if expected == "goals_unmet" and actual == "bug":
        # Goals are checked even when bugs dominate the status; unmet goals in
        # the payload still count as a detection of the goal-level defect.
        correct = (payload_metrics.get("unmet_goal_count") or 0) > 0
    return {
        "expected": expected,
        "actual": actual,
        "correct": correct,
        "expected_bug_kind": expected_kind,
        "actual_bug_kinds": actual_kinds,
        "false_positive": expected == "safe" and actual != "safe",
        "missed": expected != "safe" and actual == "safe",
    }


def _safe_cir_metrics(cir_json: str | None) -> dict[str, Any] | None:
    """CIR metrics for possibly-malformed LLM candidates; None when unparseable."""

    if not cir_json:
        return None
    try:
        return cir_metrics(cir_json)
    except (ValueError, json.JSONDecodeError):
        return None


def _rust_cli_record(result: RustCliResult) -> dict[str, Any]:
    return {
        "mode": result.mode,
        "exit_code": result.exit_code,
        "status": result.status,
        "error": result.error,
        "duration_ms": result.duration_ms,
    }


class ExperimentRunner:
    def __init__(
        self,
        *,
        repo_root: Path,
        rust_cli: RustCli,
        model: ModelConfig | None,
        max_rounds: int,
        max_tokens: int,
        temperature: float,
        include_paraphrases: bool,
    ) -> None:
        self.repo_root = repo_root
        self.rust_cli = rust_cli
        self.model = model
        self.max_rounds = max_rounds
        self.max_tokens = max_tokens
        self.temperature = temperature
        self.include_paraphrases = include_paraphrases
        self._client = None

    def client(self):
        if self._client is None:
            if self.model is None:
                raise RuntimeError("LLM methods require a model configuration")
            self._client = create_llm_client(self.model)
        return self._client

    def _buggy_cir(self, case: dict[str, Any]) -> str | None:
        rel = case.get("cir", {}).get("buggy")
        if not rel:
            return None
        return (self.repo_root / rel).read_text(encoding="utf-8")

    def run_case(self, case: dict[str, Any], method: str) -> dict[str, Any]:
        started = time.perf_counter()
        record: dict[str, Any] = {
            "case_id": case["id"],
            "defect_type": case.get("defect_type"),
            "method": method,
        }
        try:
            if method == "analyze":
                record.update(self._run_analyze(case))
            elif method == "validate_only":
                record.update(self._run_validate_only(case))
            elif method in REPAIR_FEEDBACK_MODES:
                record.update(self._run_repair(case, REPAIR_FEEDBACK_MODES[method]))
            elif method == "generate":
                record.update(self._run_generate(case))
            else:
                raise ValueError(f"unknown method: {method}")
        except Exception as error:  # keep one failure from killing the sweep
            record["error"] = str(error)
        record["wall_ms"] = (time.perf_counter() - started) * 1000
        return record

    # ── offline methods ──

    def _run_analyze(self, case: dict[str, Any]) -> dict[str, Any]:
        cir_json = self._buggy_cir(case)
        if cir_json is None:
            # Safe-only cases (no buggy CIR) are analyzed through `fixed`.
            rel = case.get("cir", {}).get("fixed")
            if not rel:
                return {"skipped": "case has neither buggy nor fixed CIR"}
            cir_json = (self.repo_root / rel).read_text(encoding="utf-8")
        result = self.rust_cli.analyze(cir_json)
        return {
            "rust_cli": _rust_cli_record(result),
            "score": _score(case, result),
            "cir_metrics": cir_metrics(cir_json),
            "cvn_metrics": cvn_metrics_from_payload(result.payload),
        }

    def _run_validate_only(self, case: dict[str, Any]) -> dict[str, Any]:
        cir_json = self._buggy_cir(case)
        if cir_json is None:
            return {"skipped": "case has no buggy CIR"}
        result = self.rust_cli.validate(cir_json)
        expected = expected_verdict(case)
        if result.payload is None:
            actual = "tool_error"
            correct = False
        else:
            # Behavioral defects are invisible to static validation; a `valid`
            # verdict on an expected-bug case counts as a miss for this method.
            actual = "safe" if result.valid else "invalid_model"
            correct = result.valid == (expected == "safe")
        return {
            "rust_cli": _rust_cli_record(result),
            "valid": result.valid,
            "score": {
                "expected": expected,
                "actual": actual,
                "correct": correct,
                "missed": expected != "safe" and actual == "safe",
                "false_positive": expected == "safe" and actual == "invalid_model",
            },
            "cir_metrics": cir_metrics(cir_json),
        }

    # ── LLM methods ──

    def _run_repair(self, case: dict[str, Any], feedback_mode: str) -> dict[str, Any]:
        cir_json = self._buggy_cir(case)
        if cir_json is None:
            return {"skipped": "case has no buggy CIR"}
        workflow = RepairWorkflow(
            self.client(),
            self.rust_cli,
            max_rounds=self.max_rounds,
            temperature=self.temperature,
            max_tokens=self.max_tokens,
            feedback_mode=feedback_mode,
        )
        result = workflow.run(cir_json)
        record: dict[str, Any] = {
            "feedback_mode": feedback_mode,
            "success": result.success,
            "repair_rounds": result.repair_rounds,
            "total_input_tokens": result.total_input_tokens,
            "total_output_tokens": result.total_output_tokens,
            "total_tokens": result.total_tokens,
            "initial_verification": (
                _rust_cli_record(result.initial_verification)
                if result.initial_verification
                else None
            ),
            "initial_cvn_metrics": (
                cvn_metrics_from_payload(result.initial_verification.payload)
                if result.initial_verification
                else None
            ),
            "cir_metrics_input": cir_metrics(cir_json),
            "rounds": [self._repair_round_record(r) for r in result.rounds],
            "error": result.error,
        }
        if result.success and result.fixed_cir_json:
            record["cir_metrics_fixed"] = cir_metrics(result.fixed_cir_json)
        return record

    def _repair_round_record(self, round_) -> dict[str, Any]:
        return {
            "round": round_.round,
            "accepted": round_.accepted,
            "parse_error": round_.parse_error,
            "llm_error": round_.llm_error,
            "duration_ms": round_.duration_ms,
            "input_tokens": round_.input_tokens,
            "output_tokens": round_.output_tokens,
            "verification": (
                _rust_cli_record(round_.verification) if round_.verification else None
            ),
            "cvn_metrics": (
                cvn_metrics_from_payload(round_.verification.payload)
                if round_.verification
                else None
            ),
            "cir_metrics": _safe_cir_metrics(round_.candidate_cir_json),
        }

    def _run_generate(self, case: dict[str, Any]) -> dict[str, Any]:
        requirements = case.get("requirements", {})
        prompts = [("canonical", requirements.get("canonical"))]
        if self.include_paraphrases:
            for index, text in enumerate(requirements.get("paraphrases") or [], 1):
                prompts.append((f"paraphrase_{index}", text))

        runs = []
        for label, text in prompts:
            if not text:
                continue
            workflow = GenerationWorkflow(
                self.client(),
                self.rust_cli,
                max_rounds=self.max_rounds,
                temperature=self.temperature,
                max_tokens=self.max_tokens,
            )
            result = workflow.run(text)
            run: dict[str, Any] = {
                "requirement": label,
                "success": result.success,
                "generation_rounds": len(result.rounds),
                "total_input_tokens": result.total_input_tokens,
                "total_output_tokens": result.total_output_tokens,
                "total_tokens": result.total_tokens,
                "rounds": [
                    {
                        "round": r.round,
                        "accepted": r.accepted,
                        "parse_error": r.parse_error,
                        "llm_error": r.llm_error,
                        "duration_ms": r.duration_ms,
                        "input_tokens": r.input_tokens,
                        "output_tokens": r.output_tokens,
                        "validation": (
                            _rust_cli_record(r.validation) if r.validation else None
                        ),
                    }
                    for r in result.rounds
                ],
                "error": result.error,
            }
            if result.success and result.cir_json:
                analyze = self.rust_cli.analyze(result.cir_json)
                run["cir_metrics"] = cir_metrics(result.cir_json)
                run["analyze"] = _rust_cli_record(analyze)
                run["cvn_metrics"] = cvn_metrics_from_payload(analyze.payload)
                run["verified_safe"] = analyze.status == "verified_safe"
            runs.append(run)
        return {"generation_runs": runs}


def main() -> int:
    parser = argparse.ArgumentParser(description="ConcPlanVerify experiment runner")
    parser.add_argument("--manifest", default="benchmarks/manifest.json")
    parser.add_argument(
        "--methods",
        default="analyze,validate_only",
        help=f"comma-separated subset of: {','.join(ALL_METHODS)}",
    )
    parser.add_argument("--cases", help="comma-separated case ids (default: all)")
    parser.add_argument("--out", help="output JSON path (default: stdout)")
    parser.add_argument("--binary", help="path to the cir2cvn binary")
    parser.add_argument("--model-id", default="deepseek-v4-pro")
    parser.add_argument("--api-key-env", default="DEEPSEEK_API_KEY")
    parser.add_argument("--base-url")
    parser.add_argument("--reasoning-effort", default="high")
    parser.add_argument("--max-rounds", type=int, default=5)
    # Thinking-enabled DeepSeek runs consume completion budget on reasoning;
    # 4096 starves hard cases into empty responses.
    parser.add_argument("--max-tokens", type=int, default=8192)
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument(
        "--no-paraphrases",
        action="store_true",
        help="generate: only run the canonical requirement",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    load_dotenv(repo_root / ".env")

    manifest_path = (repo_root / args.manifest).resolve()
    manifest = load_manifest(manifest_path)
    cases = manifest["cases"]
    if args.cases:
        wanted = {c.strip() for c in args.cases.split(",") if c.strip()}
        unknown = wanted - {case["id"] for case in cases}
        if unknown:
            parser.error(f"unknown case ids: {sorted(unknown)}")
        cases = [case for case in cases if case["id"] in wanted]

    methods = [m.strip() for m in args.methods.split(",") if m.strip()]
    unknown_methods = set(methods) - set(ALL_METHODS)
    if unknown_methods:
        parser.error(f"unknown methods: {sorted(unknown_methods)}")

    needs_llm = any(m in LLM_METHODS for m in methods)
    model = None
    if needs_llm:
        model = ModelConfig(
            name="experiment",
            provider="deepseek",
            model_id=args.model_id,
            api_key_env=args.api_key_env,
            base_url=args.base_url or default_base_url("deepseek"),
            reasoning_effort=args.reasoning_effort,
            thinking_enabled=True,
        )

    runner = ExperimentRunner(
        repo_root=repo_root,
        rust_cli=RustCli(repo_root=repo_root, binary=args.binary),
        model=model,
        max_rounds=args.max_rounds,
        max_tokens=args.max_tokens,
        temperature=args.temperature,
        include_paraphrases=not args.no_paraphrases,
    )

    records = []
    for case in cases:
        for method in methods:
            print(f"[experiment] {case['id']} :: {method}", file=sys.stderr)
            records.append(runner.run_case(case, method))

    output = {
        "meta": {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "manifest": str(manifest_path),
            "methods": methods,
            "case_ids": [case["id"] for case in cases],
            "model": asdict(model) if model else None,
            "max_rounds": args.max_rounds,
            "max_tokens": args.max_tokens,
            "temperature": args.temperature,
        },
        "records": records,
    }
    text = json.dumps(output, ensure_ascii=False, indent=2)
    if args.out:
        out_path = (repo_root / args.out).resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(text, encoding="utf-8")
        print(f"[experiment] wrote {out_path}", file=sys.stderr)
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
