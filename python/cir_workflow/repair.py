"""CIR verification and LLM repair loop."""

from __future__ import annotations

import json
import time

from .json_utils import extract_json
from .llm import LlmClient
from .models import RepairResult, RepairRound, RustCliResult
from .prompts import repair_system_prompt, repair_user_prompt, verification_feedback
from .rust_cli import RustCli


class RepairWorkflow:
    def __init__(
        self,
        client: LlmClient,
        rust_cli: RustCli,
        *,
        max_rounds: int = 5,
        temperature: float = 0.0,
        max_tokens: int = 4096,
        system_prompt: str | None = None,
    ) -> None:
        self.client = client
        self.rust_cli = rust_cli
        self.max_rounds = max(0, max_rounds)
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.system_prompt = system_prompt or repair_system_prompt()

    def run(self, cir_json: str) -> RepairResult:
        current = cir_json
        rounds: list[RepairRound] = []
        initial = self.rust_cli.analyze(current)
        if initial.status == "verified_safe":
            return RepairResult(
                fixed_cir_json=_canonical_json(current),
                rounds=rounds,
                initial_verification=initial,
            )

        feedback = verification_feedback(
            initial.payload,
            initial.error or initial.stderr,
        )
        initial_kind = _bug_kind(initial.payload)

        for round_number in range(1, self.max_rounds + 1):
            started = time.perf_counter()
            try:
                content, _ = self.client.chat(
                    self.system_prompt,
                    repair_user_prompt(current, feedback),
                    temperature=self.temperature,
                    max_tokens=self.max_tokens,
                )
            except Exception as error:
                rounds.append(RepairRound(
                    round=round_number,
                    llm_error=str(error),
                    rejection_reason=str(error),
                    duration_ms=_elapsed(started),
                ))
                feedback = f"LLM request failed; try a different repair. Error: {error}"
                continue

            candidate = extract_json(content)
            try:
                parsed = json.loads(candidate)
                if not isinstance(parsed, dict):
                    raise ValueError("CIR JSON root must be an object")
                canonical = json.dumps(parsed, ensure_ascii=False, indent=2)
            except (json.JSONDecodeError, ValueError) as error:
                rounds.append(RepairRound(
                    round=round_number,
                    candidate_cir_json=candidate,
                    parse_error=str(error),
                    rejection_reason=str(error),
                    duration_ms=_elapsed(started),
                ))
                feedback = f"CIR JSON parse error: {error}\nCurrent candidate:\n{candidate}"
                current = candidate
                continue

            verification = self.rust_cli.analyze(canonical)
            accepted = verification.status == "verified_safe"
            reason = verification_feedback(
                verification.payload,
                verification.error or verification.stderr,
            )
            rounds.append(RepairRound(
                round=round_number,
                candidate_cir_json=canonical,
                verification=verification,
                accepted=accepted,
                rejection_reason=None if accepted else reason,
                duration_ms=_elapsed(started),
            ))
            if accepted:
                return RepairResult(
                    fixed_cir_json=canonical,
                    rounds=rounds,
                    initial_verification=initial,
                )

            next_kind = _bug_kind(verification.payload)
            regression_note = ""
            if initial_kind and next_kind and initial_kind != next_kind:
                regression_note = (
                    f"\nThe candidate changed the primary bug from {initial_kind} "
                    f"to {next_kind}; preserve the original behavior while fixing it."
                )
            feedback = reason + regression_note
            current = canonical

        return RepairResult(
            fixed_cir_json=None,
            rounds=rounds,
            initial_verification=initial,
            last_feedback=feedback,
            error=f"exhausted {self.max_rounds} repair rounds",
        )


def _canonical_json(value: str) -> str:
    try:
        return json.dumps(json.loads(value), ensure_ascii=False, indent=2)
    except (json.JSONDecodeError, TypeError):
        return value


def _bug_kind(payload: dict | None) -> str | None:
    if not payload:
        return None
    bugs = payload.get("bugs", [])
    if not bugs:
        return "GoalUnreachable" if payload.get("unmet_goals") else payload.get("status")
    kind = bugs[0].get("kind", {})
    return next(iter(kind), None) if isinstance(kind, dict) and kind else str(kind)


def _elapsed(started: float) -> float:
    return (time.perf_counter() - started) * 1000
