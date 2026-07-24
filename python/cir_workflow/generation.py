"""Natural-language to CIR generation loop."""

from __future__ import annotations

import json
import time
from typing import Any

from .json_utils import extract_json
from .llm import LlmClient
from .models import GenerationResult, GenerationRound
from .prompts import (
    generation_retry_prompt,
    generation_system_prompt,
    generation_user_prompt,
    verification_feedback,
)
from .rust_cli import RustCli


class GenerationWorkflow:
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
        self.max_rounds = max(1, max_rounds)
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.system_prompt = system_prompt or generation_system_prompt()

    def run(self, requirements: str) -> GenerationResult:
        requirements = requirements.strip()
        user_prompt = generation_user_prompt(requirements)
        rounds: list[GenerationRound] = []

        for round_number in range(1, self.max_rounds + 1):
            started = time.perf_counter()
            try:
                content, _ = self.client.chat(
                    self.system_prompt,
                    user_prompt,
                    temperature=self.temperature,
                    max_tokens=self.max_tokens,
                )
            except Exception as error:
                rounds.append(GenerationRound(
                    round=round_number,
                    llm_error=str(error),
                    duration_ms=_elapsed(started),
                ))
                user_prompt = generation_retry_prompt(
                    requirements,
                    issue=(
                        "The previous model request failed. Generate the complete CIR "
                        f"again. Previous error: {error}"
                    ),
                )
                continue

            candidate = extract_json(content)
            try:
                parsed: dict[str, Any] = json.loads(candidate)
                if not isinstance(parsed, dict):
                    raise ValueError("CIR JSON root must be an object")
            except (json.JSONDecodeError, ValueError) as error:
                rounds.append(GenerationRound(
                    round=round_number,
                    candidate_json=candidate,
                    parse_error=str(error),
                    duration_ms=_elapsed(started),
                ))
                user_prompt = generation_retry_prompt(
                    requirements,
                    issue=f"The previous answer was not valid CIR JSON. Parse error: {error}",
                    current_cir=candidate,
                )
                continue

            canonical = json.dumps(parsed, ensure_ascii=False, indent=2)
            validation = self.rust_cli.validate(canonical)
            accepted = validation.valid
            rounds.append(GenerationRound(
                round=round_number,
                candidate_json=canonical,
                validation=validation,
                accepted=accepted,
                duration_ms=_elapsed(started),
            ))
            if accepted:
                return GenerationResult(cir_json=canonical, rounds=rounds)

            feedback = verification_feedback(
                validation.payload,
                validation.error or validation.stderr,
            )
            user_prompt = generation_retry_prompt(
                requirements,
                issue=(
                    "The Rust CIR validator rejected the previous candidate. Fix every "
                    "reported issue.\n\n"
                    f"{feedback}"
                ),
                current_cir=canonical,
            )

        return GenerationResult(
            cir_json=None,
            rounds=rounds,
            error=f"exhausted {self.max_rounds} generation rounds",
        )


def _elapsed(started: float) -> float:
    return (time.perf_counter() - started) * 1000
