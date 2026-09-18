"""Whole-artifact CIR revision loop (arm ``A3_ours_revision``).

Unlike :mod:`offline_workflow` (which freezes the first statically valid CIR and
then only allows the backend's deterministic patch search), this workflow lets
the provider rewrite the **whole** modular-CIR artifact between rounds. It is the
only place a provider may replace the program after freezing, and it is labelled
``repair_mode=llm_revision`` so it is never confused with the backend's
deterministic repair or with ``external_single_patch``.

Acceptance is still decided by the Rust backend: a complete ``PASS`` of the
frozen contract. Fidelity (the task-level structural check) is reported
separately and never changes acceptance. Every version is archived and hashed.

Ablation switches (protocol B6):

- ``diagnostics=False``  -> only ``{"outcome", "complete"}`` feedback is sent
  (``A3_nodiag``);
- ``contract`` without ``preserved`` -> acceptance only checks ``deadlock_free``
  (``A3_nopreserved``);
- ``check_fidelity=False`` -> no fidelity record (``A3_nofidelity``).
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient, sha256_file
from .experiments_v2 import Consumption, sha256_text
from .json_utils import extract_json
from .prompts import build_check_feedback, build_explore_feedback, render_feedback
from .providers import CandidateProvider, CandidateRequest
from .structural import run_structural_check

SUCCESS_STATUSES = {"accepted"}


@dataclass
class RevisionVersion:
    version: int
    source: str
    raw_text: str | None = None
    program_sha256: str | None = None
    artifact_path: str | None = None
    parse_error: str | None = None
    provider_error: str | None = None
    check_status: str | None = None
    support_status: str | None = None
    explore_outcome: str | None = None
    explore_complete: bool | None = None
    fidelity: dict[str, Any] | None = None
    accepted: bool = False
    decision: str | None = None


@dataclass
class RevisionResult:
    status: str
    task: str
    requirements: str
    contract_sha256: str
    out_dir: str
    max_rounds: int
    engine: str
    diagnostics: bool
    check_fidelity: bool
    fidelity_name: str | None = None
    versions: list[RevisionVersion] = field(default_factory=list)
    accepted_version: int | None = None
    repair_mode: str = "llm_revision"
    candidate_source: str | None = None
    consumption: Consumption = field(default_factory=Consumption)
    error: str | None = None

    @property
    def accepted(self) -> bool:
        return self.status in SUCCESS_STATUSES

    def as_dict(self) -> dict[str, Any]:
        payload = dict(self.__dict__)
        payload["versions"] = [v.__dict__ for v in self.versions]
        payload["consumption"] = self.consumption.as_dict()
        payload["accepted"] = self.accepted
        return payload


def _exclusive_run_dir(base: Path) -> Path:
    import os

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


class WholeArtifactRevisionWorkflow:
    def __init__(
        self,
        client: ConcirClient,
        provider: CandidateProvider,
        *,
        out_dir: Path | str,
        max_rounds: int = 4,
        engine: str = "petri",
        diagnostics: bool = True,
        check_fidelity: bool = True,
        fidelity_name: str | None = None,
    ) -> None:
        self.client = client
        self.provider = provider
        self.out_dir = Path(out_dir).expanduser().resolve()
        self.max_rounds = max(1, max_rounds)
        self.engine = engine
        self.diagnostics = diagnostics
        self.check_fidelity = check_fidelity
        self.fidelity_name = fidelity_name

    def run(self, requirements: str, contract: dict[str, Any], *,
            task_id: str = "task", initial_program: Path | str | None = None) -> RevisionResult:
        run_dir = _exclusive_run_dir(self.out_dir)
        contract_path = run_dir / "contract.json"
        contract_path.write_text(json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
                                 encoding="utf-8")
        result = RevisionResult(
            status="generation_failed", task=task_id, requirements=requirements,
            contract_sha256=sha256_file(contract_path), out_dir=str(run_dir),
            max_rounds=self.max_rounds, engine=self.engine,
            diagnostics=self.diagnostics, check_fidelity=self.check_fidelity,
            fidelity_name=self.fidelity_name,
        )

        feedback_dict: dict[str, Any] | None = None
        previous_text: str | None = None
        next_program: Path | None = None
        if initial_program is not None:
            src = Path(initial_program).expanduser().resolve()
            next_program = run_dir / "revision-0.cir.json"
            next_program.write_bytes(src.read_bytes())

        for version in range(1, self.max_rounds + 1):
            record = RevisionVersion(version=version, source="llm")
            result.versions.append(record)

            if next_program is not None:
                program_path = next_program
                record.source = "frozen"
                next_program = None
            else:
                request = CandidateRequest(
                    requirements=requirements, contract=contract,
                    feedback=render_feedback(feedback_dict) if feedback_dict else None,
                    attempt=version, previous_candidate=previous_text,
                )
                response = self.provider.propose(request)
                result.candidate_source = response.source
                self._count_usage(result, response)
                if response.error:
                    record.provider_error = response.error
                    record.decision = "provider_error"
                    result.status = "model_error"
                    result.error = response.error
                    _write_result(run_dir, result)
                    return result
                record.raw_text = response.text
                previous_text = response.text
                extracted = extract_json(response.text)
                try:
                    parsed = json.loads(extracted)
                except json.JSONDecodeError as exc:
                    record.parse_error = f"invalid JSON: {exc}"
                    record.decision = "parse_error"
                    feedback_dict = {"stage": "parse", "error": f"invalid JSON: {exc}"}
                    continue
                if not isinstance(parsed, dict):
                    record.parse_error = "candidate is not a JSON object"
                    record.decision = "parse_error"
                    feedback_dict = {"stage": "parse", "error": "candidate is not a JSON object"}
                    continue
                program_path = run_dir / f"revision-{version}.cir.json"
                program_path.write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                                        encoding="utf-8")

            record.program_sha256 = sha256_file(program_path)
            record.artifact_path = str(program_path)

            check = self.client.check(program_path)
            record.check_status = check.status
            if check.kind != "semantic":
                result.status = "tool_error"
                result.error = check.error or "check failed"
                _write_result(run_dir, result)
                return result
            if check.status == "invalid":
                record.decision = "check_invalid"
                feedback_dict = (build_check_feedback(check) if self.diagnostics
                                 else {"stage": "check", "outcome": "INVALID"})
                continue

            support = self.client.support(program_path)
            record.support_status = support.status
            if support.kind != "semantic":
                result.status = "tool_error"
                result.error = support.error or "support failed"
                _write_result(run_dir, result)
                return result
            if support.status == "unsupported":
                record.decision = "unsupported"
                result.status = "unsupported"
                if self.check_fidelity:
                    self._fidelity(program_path, record)
                _write_result(run_dir, result)
                return result

            explore = self.client.explore(program_path, contract_path, self.engine)
            record.explore_outcome = explore.outcome
            record.explore_complete = explore.complete
            if explore.kind != "semantic":
                result.status = "tool_error"
                result.error = explore.error or "explore failed"
                _write_result(run_dir, result)
                return result

            if self.check_fidelity:
                self._fidelity(program_path, record)

            if explore.outcome == "PASS" and explore.complete is True:
                record.accepted = True
                record.decision = "accepted"
                result.status = "accepted"
                result.accepted_version = version
                result.consumption.first_correct_round = version
                _write_result(run_dir, result)
                return result

            if explore.status == "unknown":
                record.decision = "unknown"
                result.status = "unknown"
                _write_result(run_dir, result)
                return result
            if explore.status in ("invalid", "unsupported"):
                record.decision = explore.status
                result.status = explore.status
                _write_result(run_dir, result)
                return result

            record.decision = "explore_fail"
            feedback_dict = (build_explore_feedback(explore) if self.diagnostics
                             else {"stage": "explore",
                                   "outcome": explore.outcome,
                                   "complete": explore.complete})

        result.status = "exhausted"
        result.error = f"no complete PASS within {self.max_rounds} rounds"
        _write_result(run_dir, result)
        return result

    def _fidelity(self, program_path: Path, record: RevisionVersion) -> None:
        if not self.fidelity_name:
            return
        try:
            program = json.loads(program_path.read_text(encoding="utf-8"))
            record.fidelity = run_structural_check(self.fidelity_name, program)
        except (OSError, json.JSONDecodeError) as exc:
            record.fidelity = {"ok": None, "reason": f"unreadable: {exc}"}

    @staticmethod
    def _count_usage(result: RevisionResult, response) -> None:
        result.consumption.rounds += 1
        result.consumption.http_requests += 1 if response.source == "llm" else 0
        usage = response.usage
        if usage is None and response.input_tokens is not None:
            usage = {"prompt_tokens": response.input_tokens,
                     "completion_tokens": response.output_tokens or 0}
        result.consumption.add_usage(usage)
        # llm wall is recorded by the live provider's own evidence log; the
        # workflow only counts model-level calls here.


def _write_result(run_dir: Path, result: RevisionResult) -> None:
    (run_dir / "result.json").write_text(
        json.dumps(result.as_dict(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
