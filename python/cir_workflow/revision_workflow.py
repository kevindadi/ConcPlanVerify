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
from .models import normalize_token_usage
from .normalize import normalize as normalize_program
from .prompts import build_explore_feedback, render_feedback
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
    # measured per-round consumption / evidence
    request_sha256: str | None = None
    feedback_sha256: str | None = None
    prompt_tokens: int | None = None
    completion_tokens: int | None = None
    llm_wall_ms: int = 0
    tool_wall_ms: int = 0
    tool_output_sha256: str | None = None
    normalizations: list[dict[str, Any]] = field(default_factory=list)
    sid_issues: list[dict[str, Any]] = field(default_factory=list)
    local_patch_function: str | None = None


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
        last_norm_sha: str | None = None
        last_normalized: dict[str, Any] | None = None
        stall_count = 0
        local_patch_function: str | None = None
        if initial_program is not None:
            src = Path(initial_program).expanduser().resolve()
            next_program = run_dir / "revision-0.cir.json"
            next_program.write_bytes(src.read_bytes())

        for version in range(1, self.max_rounds + 1):
            record = RevisionVersion(version=version, source="llm")
            result.versions.append(record)

            if next_program is not None:
                record.source = "frozen"
                parsed = json.loads(next_program.read_text(encoding="utf-8"))
                parsed, norm_records, sid_issues = normalize_program(parsed)
                record.normalizations = norm_records
                record.sid_issues = sid_issues
                program_path = run_dir / f"revision-{version}.normalized.cir.json"
                program_path.write_text(
                    json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8")
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
                record.request_sha256 = sha256_text(json.dumps({
                    "requirements": requirements,
                    "contract_sha256": result.contract_sha256,
                    "attempt": version,
                    "feedback": feedback_dict,
                    "previous_sha256": (sha256_text(previous_text)
                                        if previous_text else None),
                }, sort_keys=True, default=str))
                record.llm_wall_ms = response.wall_ms
                record.prompt_tokens, record.completion_tokens = _tokens(response)
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
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                if not isinstance(parsed, dict):
                    record.parse_error = "candidate is not a JSON object"
                    record.decision = "parse_error"
                    feedback_dict = {"stage": "parse", "error": "candidate is not a JSON object"}
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                # Local-patch reply: {function, body:[...]} merged into the last
                # accepted program instead of a whole-program replacement.
                if (local_patch_function and "modules" not in parsed
                        and isinstance(parsed.get("body"), list)
                        and last_normalized is not None):
                    merged = json.loads(json.dumps(last_normalized))
                    for module in merged.get("modules", []):
                        for function in module.get("functions", []):
                            if function.get("name") == local_patch_function:
                                function["body"] = parsed["body"]
                    parsed = merged
                    record.decision = "local_patch"

                # Tier-1 deterministic normalisation (recorded, sid-safe).
                parsed, norm_records, sid_issues = normalize_program(parsed)
                record.normalizations = norm_records
                record.sid_issues = sid_issues
                program_path = run_dir / f"revision-{version}.normalized.cir.json"
                program_path.write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                                        encoding="utf-8")
                (run_dir / f"revision-{version}.cir.json").write_text(
                    json.dumps(parsed, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

            record.program_sha256 = sha256_file(program_path)
            record.artifact_path = str(program_path)

            if record.sid_issues:
                record.decision = "sid_invalid"
                feedback_dict = {
                    "stage": "schema",
                    "sid_issues": record.sid_issues,
                    "expected": "every statement sid must match ^s[0-9]+$ (s1, s2, ...)",
                }
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue

            # Stall detection: identical normalised program -> local-patch, then stop.
            if last_norm_sha is not None and record.program_sha256 == last_norm_sha:
                stall_count += 1
                if stall_count >= 2:
                    record.decision = "stalled"
                    result.status = "stalled"
                    result.error = "candidate unchanged twice; stopping"
                    _write_result(run_dir, result)
                    return result
                local_patch_function = record.local_patch_function
                feedback_dict = {
                    "stage": "stalled",
                    "message": "the program is byte-identical to the previous attempt; "
                               "change only the offending function body",
                    "function": local_patch_function,
                    "expected": "reply with {\"function\": name, \"body\": [ ...statements... ]}",
                }
                record.decision = "stalled_local_patch"
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue
            stall_count = 0
            last_norm_sha = record.program_sha256
            last_normalized = parsed

            check = self.client.check(program_path)
            record.check_status = check.status
            record.tool_wall_ms += check.wall_ms
            result.consumption.tool_wall_ms += check.wall_ms
            record.tool_output_sha256 = _payload_hash("check", check.status, check.payload)
            if check.kind != "semantic":
                # A malformed candidate (usage/protocol/parse) is a schema error
                # the LLM can fix: feed it back and spend the round instead of
                # stopping the batch. Only a missing/broken binary is a tool error.
                if check.kind in ("usage_error", "protocol_error"):
                    record.decision = "check_schema_error"
                    feedback_dict = {
                        "stage": "check",
                        "schema_error": check.error or check.stderr,
                        "detail": "the program JSON did not parse or had an invalid shape; "
                                  "fix the schema (e.g. `expr` must be a string) and "
                                  "resend the whole program",
                    }
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                result.status = "tool_error"
                result.error = check.error or "check failed"
                _write_result(run_dir, result)
                return result
            if check.status == "invalid":
                record.decision = "check_invalid"
                record.local_patch_function = _first_function(check)
                local_patch_function = record.local_patch_function
                feedback_dict = (build_schema_feedback(check, record.normalizations)
                                 if self.diagnostics
                                 else {"stage": "check", "outcome": "INVALID"})
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue

            support = self.client.support(program_path)
            record.support_status = support.status
            record.tool_wall_ms += support.wall_ms
            result.consumption.tool_wall_ms += support.wall_ms
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
            record.tool_wall_ms += explore.wall_ms
            result.consumption.tool_wall_ms += explore.wall_ms
            record.tool_output_sha256 = _payload_hash("explore", explore.status,
                                                      explore.payload)
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
            record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))

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
        result.consumption.llm_wall_ms += int(getattr(response, "wall_ms", 0) or 0)
        usage = response.usage
        if usage is None and response.input_tokens is not None:
            usage = {"prompt_tokens": response.input_tokens,
                     "completion_tokens": response.output_tokens or 0}
        result.consumption.add_usage(usage)
        # llm wall is recorded by the live provider's own evidence log; the
        # workflow only counts model-level calls here.


def _tokens(response) -> tuple[int | None, int | None]:
    if response.usage:
        prompt, completion = normalize_token_usage(response.usage)
        return prompt, completion
    if response.input_tokens is not None or response.output_tokens is not None:
        return response.input_tokens, response.output_tokens
    return None, None


def _payload_hash(stage: str, status: str | None, payload: Any) -> str:
    return sha256_text(json.dumps(
        {"stage": stage, "status": status, "payload": payload},
        sort_keys=True, default=str))


SHAPE_HINTS = {
    "E001": "a Var/Atomic resource requires both `base` (e.g. \"Bool\"/\"Int\") and "
            "`init`; a Channel requires `base` and `capacity`",
    "E005": "every statement `sid` must match ^s[0-9]+$ (s1, s2, ...)",
    "E008": "sync resource `type` must be one of Mutex, Condvar, Semaphore, Channel",
    "E205": "atomic_cas `dst` receives the old value and must have the Atomic `base` "
            "type, not a Bool",
    "E510": "mutex_unlock requires holding that mutex",
    "E511": "condvar_wait requires holding the paired `lock`",
}


def _first_function(check) -> str | None:
    for diag in ((check.payload or {}).get("diagnostics") or []):
        location = str(diag.get("location") or "")
        if "::" in location:
            return location.split(".")[0]
    return None


def build_schema_feedback(check, normalizations: list[dict[str, Any]]) -> dict[str, Any]:
    """Schema feedback with JSON pointers and shapes, never file paths/lines."""

    payload = check.payload or {}
    diagnostics = []
    hints: list[str] = []
    for diag in payload.get("diagnostics", []) or []:
        diagnostics.append({k: diag.get(k) for k in ("code", "message", "path")
                            if diag.get(k) is not None})
        hint = SHAPE_HINTS.get(str(diag.get("code")))
        if hint and hint not in hints:
            hints.append(hint)
    return {
        "stage": "check",
        "status": "INVALID",
        "diagnostics": diagnostics,
        "expected_shapes": hints,
        "normalizations": normalizations,
        "note": "`path` is a JSON pointer into the program; resend the whole program "
                "with those fields fixed (do not add fields the schema forbids).",
    }


def _write_result(run_dir: Path, result: RevisionResult) -> None:
    (run_dir / "result.json").write_text(
        json.dumps(result.as_dict(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
