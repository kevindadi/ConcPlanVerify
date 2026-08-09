"""Benchmark experiment runner.

Runs the methods from the experiment design in ``todo.md`` over the cases of
``benchmarks/manifest.json`` and writes one structured results JSON.

Methods:

- ``analyze``        — no LLM; run ``cir2cvn --analyze`` on the gold buggy ConcIR
                       and score the verdict against the manifest expectation.
- ``validate_only``  — no LLM; static ConcIR validation only (ablation showing
                       what schema checks alone can catch).
- ``repair_cvn``     — full CVN pipeline: repair loop with structured feedback.
- ``repair_status_only`` — diagnostic ablation: feedback reduced to
                       status + primary bug kind.
- ``repair_llm_only``    — LLM-only baseline: no CVN diagnostics in the prompt
                       (the Rust analyzer still judges acceptance).
- ``repair_local``   — slice-based repair: only the functions implicated by
                       the bug reports are regenerated and spliced back into
                       the otherwise-frozen ConcIR; falls back to whole-ConcIR
                       repair when the slice cannot be determined or is
                       exhausted.
- ``generate``       — natural-language generation from the manifest
                       requirements, then a full analyze of the produced ConcIR.
- ``codegen``        — verified ConcIR -> Rust: only runs on a ConcIR that analyzes
                       as verified_safe; the LLM implements the plan and
                       ``cargo check`` judges acceptance (up to 3 rounds).
- ``llm_judge``      — LLM-only detection baseline: one-shot classification
                       of the case ConcIR (bug / safe, kind, suspect sids) with
                       no verifier in the loop, scored against gold.
- ``pipeline``       — the full user story: NL requirement -> ConcIR generation
                       -> CVN verification (with repair rounds inside the
                       generation loop) -> Rust codegen, gated on
                       verified_safe. Uses the canonical requirement only.

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

from .codegen import CodegenWorkflow, code_metrics
from .env import load_dotenv
from .generation import GenerationWorkflow
from .json_utils import extract_json
from .llm import create_llm_client, default_base_url
from .metrics import cir_metrics
from .models import ModelConfig, RustCliResult, normalize_token_usage
from .repair import RepairWorkflow
from .repair_local import LocalRepairWorkflow
from .rust_cli import RustCli

OFFLINE_METHODS = ("analyze", "validate_only", "analyze_no_goals")
LLM_METHODS = (
    "repair_cvn",
    "repair_status_only",
    "repair_llm_only",
    "repair_local",
    "generate",
    "codegen",
    "llm_judge",
    "pipeline",
)
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
    """ConcIR metrics for possibly-malformed LLM candidates; None when unparseable."""

    if not cir_json:
        return None
    try:
        return cir_metrics(cir_json)
    except (ValueError, json.JSONDecodeError):
        return None


def _goal_preservation_probe(cir_json: str) -> dict[str, Any] | None:
    """Extract goals and write ops so A/B runs can check normalize-style drift."""

    try:
        fixed = json.loads(cir_json)
    except (json.JSONDecodeError, TypeError):
        return None
    writes = []
    for fn in fixed.get("functions") or []:
        for stmt in fn.get("body") or []:
            op = stmt.get("op")
            if (
                isinstance(op, list)
                and len(op) >= 4
                and op[0] == "res_op"
                and op[2] == "write"
            ):
                writes.append({
                    "function": fn.get("name"),
                    "sid": stmt.get("sid"),
                    "var": op[1],
                    "value": op[3],
                })
    return {
        "goal_ids": [g.get("id") for g in (fixed.get("goals") or [])],
        "goal_variables": [g.get("variables") for g in (fixed.get("goals") or [])],
        "writes": writes,
        "has_result_99": any(
            w.get("var") == "result" and str(w.get("value")) == "99" for w in writes
        ),
    }


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
            elif method == "analyze_no_goals":
                record.update(self._run_analyze_no_goals(case))
            elif method in REPAIR_FEEDBACK_MODES:
                record.update(self._run_repair(case, REPAIR_FEEDBACK_MODES[method]))
            elif method == "repair_local":
                record.update(self._run_repair_local(case))
            elif method == "generate":
                record.update(self._run_generate(case))
            elif method == "codegen":
                record.update(self._run_codegen(case))
            elif method == "llm_judge":
                record.update(self._run_llm_judge(case))
            elif method == "pipeline":
                record.update(self._run_pipeline(case))
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
            # Safe-only cases (no buggy ConcIR) are analyzed through `fixed`.
            rel = case.get("cir", {}).get("fixed")
            if not rel:
                return {"skipped": "case has neither buggy nor fixed ConcIR"}
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
            return {"skipped": "case has no buggy ConcIR"}
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

    def _run_analyze_no_goals(self, case: dict[str, Any]) -> dict[str, Any]:
        """Goal-reachability ablation: analyze with goal checking disabled.

        The interesting outcome is a `goals_unmet` gold case that gets
        accepted as `verified_safe` once goals are switched off.
        """

        cir_json = self._buggy_cir(case)
        if cir_json is None:
            return {"skipped": "case has no buggy ConcIR"}
        result = self.rust_cli.analyze_no_goals(cir_json)
        expected = expected_verdict(case)
        actual = verdict_from_status(result.status)
        return {
            "rust_cli": _rust_cli_record(result),
            "score": {
                "expected": expected,
                "actual": actual,
                "misaccepted": expected == "goals_unmet" and actual == "safe",
            },
            "cvn_metrics": cvn_metrics_from_payload(result.payload),
        }

    # ── LLM methods ──

    def _run_repair(self, case: dict[str, Any], feedback_mode: str) -> dict[str, Any]:
        cir_json = self._buggy_cir(case)
        if cir_json is None:
            return {"skipped": "case has no buggy ConcIR"}
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
            record["fixed_goal_probe"] = _goal_preservation_probe(result.fixed_cir_json)
        return record

    def _run_repair_local(self, case: dict[str, Any]) -> dict[str, Any]:
        """Slice-based repair: regenerate only the implicated functions."""

        cir_json = self._buggy_cir(case)
        if cir_json is None:
            return {"skipped": "case has no buggy ConcIR"}
        workflow = LocalRepairWorkflow(
            self.client(),
            self.rust_cli,
            max_slice_rounds=min(self.max_rounds, 3),
            max_full_rounds=2,
            temperature=self.temperature,
            max_tokens=self.max_tokens,
        )
        result = workflow.run(cir_json)
        record: dict[str, Any] = {
            "success": result.success,
            "repair_rounds": result.repair_rounds,
            "total_input_tokens": result.total_input_tokens,
            "total_output_tokens": result.total_output_tokens,
            "total_tokens": result.total_tokens,
            "initial_slice": result.initial_slice,
            "final_slice": result.final_slice,
            "total_functions": result.total_functions,
            "slice_expanded": result.slice_expanded,
            "fell_back": result.fell_back,
            "fallback_reason": result.fallback_reason,
            "initial_verification": (
                _rust_cli_record(result.initial_verification)
                if result.initial_verification
                else None
            ),
            "cir_metrics_input": cir_metrics(cir_json),
            "rounds": [
                {**self._repair_round_record(r), "phase": r.phase,
                 "slice_functions": r.slice_functions}
                for r in result.rounds
            ],
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

    def _run_codegen(self, case: dict[str, Any]) -> dict[str, Any]:
        """Verified ConcIR -> Rust. Gate: the source ConcIR must analyze verified_safe."""

        rel = case.get("cir", {}).get("fixed") or (
            case.get("cir", {}).get("buggy")
            if expected_verdict(case) == "safe"
            else None
        )
        if not rel:
            return {"skipped": "case has no verified-safe ConcIR (fixed or safe buggy)"}
        cir_json = (self.repo_root / rel).read_text(encoding="utf-8")

        gate = self.rust_cli.analyze(cir_json)
        if gate.status != "verified_safe":
            return {
                "skipped": f"source ConcIR is not verified_safe (status={gate.status})",
                "rust_cli": _rust_cli_record(gate),
            }

        requirement = (case.get("requirements") or {}).get("canonical")
        workflow = CodegenWorkflow(
            self.client(),
            scratch=self.repo_root / "target" / "codegen-check",
            max_rounds=min(self.max_rounds, 3),
            temperature=self.temperature,
            max_tokens=self.max_tokens,
        )
        result = workflow.run(cir_json, requirement)

        record: dict[str, Any] = {
            "source_cir": rel,
            "success": result.success,
            "codegen_rounds": len(result.rounds),
            "total_input_tokens": result.total_input_tokens,
            "total_output_tokens": result.total_output_tokens,
            "cir_metrics": cir_metrics(cir_json),
            "rounds": [
                {
                    "round": r.round,
                    "cargo_check_ok": r.cargo_check_ok,
                    "cargo_errors": r.cargo_errors,
                    "cargo_wall_s": r.cargo_wall_s,
                    "llm_error": r.llm_error,
                    "duration_ms": r.duration_ms,
                    "input_tokens": r.input_tokens,
                    "output_tokens": r.output_tokens,
                }
                for r in result.rounds
            ],
            "error": result.error,
        }
        if result.success and result.code:
            record["code_metrics"] = code_metrics(result.code)
            out_dir = self.repo_root / "results" / "codegen"
            out_dir.mkdir(parents=True, exist_ok=True)
            out_path = out_dir / f"{case['id']}.rs"
            out_path.write_text(result.code, encoding="utf-8")
            record["code_path"] = str(out_path.relative_to(self.repo_root))
        return record

    def _run_llm_judge(self, case: dict[str, Any]) -> dict[str, Any]:
        """LLM-only detection baseline: no verifier, one-shot verdict."""

        cir_json = self._buggy_cir(case)
        if cir_json is None:
            rel = case.get("cir", {}).get("fixed")
            if not rel:
                return {"skipped": "case has neither buggy nor fixed ConcIR"}
            cir_json = (self.repo_root / rel).read_text(encoding="utf-8")

        system_prompt = (
            "You are an expert reviewer of ConcIR (Concurrency Intermediate "
            "Representation) plans. Analyze the given ConcIR for concurrency "
            "defects: deadlocks (circular lock waits, channel/join blocking), "
            "lost condvar signals, statements that can never execute, and "
            "declared goals that no execution can satisfy.\n\n"
            "Answer with exactly one JSON object, no other text:\n"
            "{\n"
            '  "verdict": "bug" | "safe",\n'
            '  "bug_kind": "Deadlock" | "SignalLoss" | "ChannelBlock" | '
            '"DeadTransition" | "GoalUnreachable" | null,\n'
            '  "suspect_functions": [names],\n'
            '  "suspect_sids": [sids],\n'
            '  "explanation": "one short paragraph"\n'
            "}"
        )
        started = time.perf_counter()
        try:
            content, usage = self.client().chat(
                system_prompt,
                f"ConcIR to review:\n```json\n{cir_json}\n```",
                temperature=self.temperature,
                max_tokens=self.max_tokens,
            )
        except Exception as error:
            return {"error": f"llm_judge request failed: {error}"}
        duration_ms = (time.perf_counter() - started) * 1000
        input_tokens, output_tokens = normalize_token_usage(usage)

        verdict_raw: dict[str, Any] | None
        try:
            parsed = json.loads(extract_json(content))
            verdict_raw = parsed if isinstance(parsed, dict) else None
        except json.JSONDecodeError:
            verdict_raw = None

        expected = expected_verdict(case)
        expected_bug = expected != "safe"
        claimed_bug = bool(verdict_raw and verdict_raw.get("verdict") == "bug")
        expected_kind = case.get("expected", {}).get("bug_kind")
        claimed_kind = verdict_raw.get("bug_kind") if verdict_raw else None
        return {
            "judge": verdict_raw,
            "judge_parse_failed": verdict_raw is None,
            "duration_ms": duration_ms,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "score": {
                "expected": expected,
                "claimed": "bug" if claimed_bug else "safe",
                "detected": expected_bug and claimed_bug,
                "missed": expected_bug and not claimed_bug,
                "false_positive": (not expected_bug) and claimed_bug,
                "kind_match": (
                    expected_kind == claimed_kind
                    if expected_bug and claimed_bug and expected_kind
                    else None
                ),
            },
            "cir_metrics": _safe_cir_metrics(cir_json),
        }

    def _run_pipeline(self, case: dict[str, Any]) -> dict[str, Any]:
        """NL -> ConcIR -> verify (+repair) -> Rust, the end-to-end user story."""

        requirement = (case.get("requirements") or {}).get("canonical")
        if not requirement:
            return {"skipped": "case has no canonical requirement"}
        record: dict[str, Any] = {"stages": {}}

        # Stage 1: NL -> ConcIR (static-validation retry loop inside).
        generation = GenerationWorkflow(
            self.client(),
            self.rust_cli,
            max_rounds=self.max_rounds,
            temperature=self.temperature,
            max_tokens=self.max_tokens,
        ).run(requirement)
        record["stages"]["generate"] = {
            "success": generation.success,
            "rounds": len(generation.rounds),
            "input_tokens": generation.total_input_tokens,
            "output_tokens": generation.total_output_tokens,
            "error": generation.error,
        }
        if not generation.success or not generation.cir_json:
            record["outcome"] = "generation_failed"
            return record
        cir_json = generation.cir_json

        # Stage 2: full verification; on failure, the CVN repair loop.
        verification = self.rust_cli.analyze(cir_json)
        record["stages"]["verify"] = {
            "status": verification.status,
            "cvn_metrics": cvn_metrics_from_payload(verification.payload),
        }
        if verification.status != "verified_safe":
            repair = RepairWorkflow(
                self.client(),
                self.rust_cli,
                max_rounds=self.max_rounds,
                temperature=self.temperature,
                max_tokens=self.max_tokens,
                feedback_mode="full",
            ).run(cir_json)
            record["stages"]["repair"] = {
                "success": repair.success,
                "rounds": repair.repair_rounds,
                "input_tokens": repair.total_input_tokens,
                "output_tokens": repair.total_output_tokens,
                "error": repair.error,
            }
            if not repair.success or not repair.fixed_cir_json:
                record["outcome"] = "verification_failed"
                return record
            cir_json = repair.fixed_cir_json

        record["cir_metrics"] = cir_metrics(cir_json)

        # Stage 3: verified ConcIR -> Rust, cargo check as the oracle.
        codegen = CodegenWorkflow(
            self.client(),
            scratch=self.repo_root / "target" / "codegen-check",
            max_rounds=min(self.max_rounds, 3),
            temperature=self.temperature,
            max_tokens=self.max_tokens,
        ).run(cir_json, requirement)
        record["stages"]["codegen"] = {
            "success": codegen.success,
            "rounds": len(codegen.rounds),
            "input_tokens": codegen.total_input_tokens,
            "output_tokens": codegen.total_output_tokens,
            "error": codegen.error,
        }
        if codegen.success and codegen.code:
            record["code_metrics"] = code_metrics(codegen.code)
            out_dir = self.repo_root / "results" / "pipeline"
            out_dir.mkdir(parents=True, exist_ok=True)
            out_path = out_dir / f"{case['id']}.rs"
            out_path.write_text(codegen.code, encoding="utf-8")
            record["code_path"] = str(out_path.relative_to(self.repo_root))
            record["outcome"] = "success"
        else:
            record["outcome"] = "codegen_failed"
        return record

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
