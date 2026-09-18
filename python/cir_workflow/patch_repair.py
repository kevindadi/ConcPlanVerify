"""LLM single-patch repair loop: propose → Rust evaluate → real feedback → replay.

This is a separate entry from the generation workflow. The provider proposes one
constrained patch per round; the Rust CLI (`repair-context` / `evaluate-patch` /
`replay`) owns the model/contract binding, legality, static checks, full-contract
verification and replay. Python only orchestrates and records.

Labels: `candidate_source` (scripted/llm), `validator=tool`,
`repair_mode=external_single_patch`. A deterministic tool repair is never
counted as an LLM success.
"""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient, ConcirResult, sha256_file
from .json_utils import extract_json
from .prompts import patch_system_prompt, patch_user_prompt, render_patch_feedback
from .providers import PatchCandidateProvider, PatchRequest

CONTEXT_SCHEMA = "concir-repair-context-v1"
CANDIDATE_SCHEMA = "concir-external-patch-candidate-v1"
EXTERNAL_ARTIFACT_SCHEMA = "concir-external-patch-artifact-v1"
SUCCESS_STATUSES = {"accepted"}


def _exclusive_run_dir(base: Path) -> Path:
    base.mkdir(parents=True, exist_ok=True)
    for _ in range(1000):
        token = f"{time.strftime('%Y%m%dT%H%M%S', time.gmtime())}-{os.getpid()}-{os.urandom(3).hex()}"
        path = base / f"run-{token}"
        try:
            path.mkdir(parents=True, exist_ok=False)
            return path
        except FileExistsError:
            continue
    raise RuntimeError(f"could not allocate a unique run directory under {base}")


@dataclass
class PatchRound:
    round: int
    candidate_source: str
    raw_text: str | None = None
    parsed_candidate: dict[str, Any] | None = None
    parse_error: str | None = None
    provider_error: str | None = None
    duplicate: bool = False
    evaluate_status: str | None = None
    reject_reason: dict[str, Any] | None = None
    verification_outcome: str | None = None
    accepted: bool = False
    artifact_path: str | None = None
    replay_status: str | None = None


@dataclass
class PatchRepairResult:
    status: str
    model_path: str
    contract_path: str
    model_sha256: str
    contract_sha256: str
    context_fingerprint: str | None
    out_dir: str
    candidate_source: str | None = None
    validator: str = "tool"
    repair_mode: str = "external_single_patch"
    rounds: list[PatchRound] = field(default_factory=list)
    accepted_artifact_path: str | None = None
    replay: dict[str, Any] | None = None
    error: str | None = None

    @property
    def accepted(self) -> bool:
        return self.status in SUCCESS_STATUSES


class ExternalPatchRepairWorkflow:
    def __init__(
        self,
        client: ConcirClient,
        provider: PatchCandidateProvider,
        *,
        out_dir: Path | str,
        max_rounds: int = 3,
    ) -> None:
        self.client = client
        self.provider = provider
        self.out_dir = Path(out_dir).expanduser().resolve()
        self.max_rounds = max(1, max_rounds)

    def run(self, model: Path | str, contract: Path | str) -> PatchRepairResult:
        run_dir = _exclusive_run_dir(self.out_dir)
        model_path = Path(model).expanduser().resolve()
        contract_path = Path(contract).expanduser().resolve()
        model_sha = sha256_file(model_path)
        contract_sha = sha256_file(contract_path)
        result = PatchRepairResult(
            status="tool_error", model_path=str(model_path), contract_path=str(contract_path),
            model_sha256=model_sha, contract_sha256=contract_sha,
            context_fingerprint=None, out_dir=str(run_dir),
        )
        (run_dir / "model.json").write_bytes(model_path.read_bytes())
        (run_dir / "contract.json").write_bytes(contract_path.read_bytes())

        # 1. Rust repair context (fingerprints, scope, root report, function sids/hashes)
        ctx_result = self.client.repair_context(model_path, contract_path)
        if ctx_result.kind != "semantic":
            result.error = ctx_result.error or "repair-context failed"
            return result
        context = ctx_result.payload
        if context.get("schema_version") != CONTEXT_SCHEMA:
            result.error = f"unexpected context schema {context.get('schema_version')!r}"
            return result
        result.context_fingerprint = context.get("context_fingerprint")
        context_path = run_dir / "context.json"
        context_path.write_text(json.dumps(context, ensure_ascii=False, indent=2) + "\n")

        # 2. bounded propose -> evaluate -> feedback loop
        feedback: str | None = None
        previous_raw: str | None = None
        seen: set[str] = set()
        for attempt in range(1, self.max_rounds + 1):
            request = PatchRequest(context=context, attempt=attempt, feedback=feedback,
                                   previous_candidate=previous_raw)
            response = self.provider.propose_patch(request)
            round_dir = run_dir / f"round-{attempt}"
            round_dir.mkdir(parents=True, exist_ok=True)
            record = PatchRound(round=attempt, candidate_source=response.source,
                                raw_text=response.text or None,
                                provider_error=response.error)
            result.rounds.append(record)
            if response.error:
                result.status = "model_error"
                result.error = response.error
                _write_result(run_dir, result)
                return result
            (round_dir / "raw.txt").write_text(response.text, encoding="utf-8")
            previous_raw = response.text

            extracted = extract_json(response.text)
            try:
                candidate = json.loads(extracted)
            except json.JSONDecodeError as exc:
                record.parse_error = f"invalid JSON: {exc}"
                feedback = json.dumps({"stage": "parse", "error": f"invalid JSON: {exc}"})
                continue
            if not isinstance(candidate, dict):
                record.parse_error = "candidate is not a JSON object"
                feedback = json.dumps({"stage": "parse", "error": "candidate is not a JSON object"})
                continue
            record.parsed_candidate = candidate
            (round_dir / "candidate.json").write_text(
                json.dumps(candidate, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

            fingerprint = json.dumps(candidate.get("patch"), sort_keys=True, ensure_ascii=False)
            if fingerprint in seen:
                record.duplicate = True
                feedback = json.dumps({"stage": "duplicate",
                                       "error": "this exact patch was already evaluated"})
                continue
            seen.add(fingerprint)

            evaluate = self.client.evaluate_patch(context_path, round_dir / "candidate.json")
            if evaluate.kind != "semantic":
                result.status = "tool_error"
                result.error = evaluate.error or "evaluate-patch failed"
                _write_result(run_dir, result)
                return result
            record.evaluate_status = evaluate.status
            artifact = evaluate.payload or {}
            (round_dir / "evaluate.json").write_text(
                json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            record.reject_reason = artifact.get("reject_reason")
            verification = artifact.get("verification") or {}
            record.verification_outcome = verification.get("outcome")
            record.artifact_path = evaluate.artifact_path

            if evaluate.status == "accepted":
                # Success gate: this run's artifact, accepted, full PASS, replay ok.
                if not _accepted_artifact_ok(artifact):
                    result.status = "tool_error"
                    result.error = "evaluate-patch reported accepted without a bound PASS artifact"
                    _write_result(run_dir, result)
                    return result
                replay = self.client.replay(evaluate.artifact_path)
                result.replay = _replay_record(replay)
                record.replay_status = replay.status
                if not _replay_accepts(replay):
                    result.status = "tool_error"
                    result.error = replay.error or "accepted artifact replay did not confirm acceptance"
                    _write_result(run_dir, result)
                    return result
                record.accepted = True
                result.status = "accepted"
                result.candidate_source = response.source
                result.accepted_artifact_path = evaluate.artifact_path
                if sha256_file(model_path) != model_sha or sha256_file(contract_path) != contract_sha:
                    result.status = "tool_error"
                    result.error = "model or contract changed during the workflow"
                _write_result(run_dir, result)
                return result

            # rejected: feed the real structured reason back and try again
            feedback = render_patch_feedback(artifact)
            (round_dir / "feedback.json").write_text(feedback + "\n", encoding="utf-8")

        result.status = "rejected"
        result.error = "no accepted patch within the round budget"
        _write_result(run_dir, result)
        return result


def _accepted_artifact_ok(artifact: dict[str, Any]) -> bool:
    if artifact.get("schema_version") != EXTERNAL_ARTIFACT_SCHEMA:
        return False
    if artifact.get("accepted") is not True or artifact.get("status") != "accepted":
        return False
    verification = artifact.get("verification") or {}
    if verification.get("outcome") != "PASS" or verification.get("complete") is not True:
        return False
    chain = artifact.get("patch_chain")
    return isinstance(chain, list) and len(chain) == 1


def _replay_record(result: ConcirResult) -> dict[str, Any]:
    return {
        "kind": result.kind, "status": result.status, "exit_code": result.exit_code,
        "payload": result.payload, "error": result.error, "wall_ms": result.wall_ms,
    }


def _replay_accepts(result: ConcirResult) -> bool:
    if result.kind != "semantic" or result.status != "replayed":
        return False
    payload = result.payload or {}
    return (payload.get("accepted_ok") is True and payload.get("accepted_node") == 0
            and payload.get("outcome") == "repaired")


def _write_result(run_dir: Path, result: PatchRepairResult):
    payload = {**result.__dict__}
    payload["rounds"] = [r.__dict__ for r in result.rounds]
    (run_dir / "result.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
