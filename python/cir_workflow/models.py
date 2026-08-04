"""Small protocol types shared by the Python orchestration layer."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class ModelConfig:
    """Configuration for one OpenAI-compatible model endpoint."""

    name: str
    provider: str
    model_id: str
    api_key_env: str
    base_url: str
    reasoning_effort: str | None = None
    thinking_enabled: bool = False


@dataclass
class RustCliResult:
    """One invocation of the Rust verification CLI.

    ``payload`` is the decoded stdout JSON when the process produced a valid
    protocol response. Process failures and protocol failures are represented
    by ``status`` and ``error`` so callers never need to inspect stderr text.
    """

    mode: str
    exit_code: int
    status: str
    payload: dict[str, Any] | None
    stderr: str = ""
    error: str | None = None
    duration_ms: float = 0.0

    @property
    def ok(self) -> bool:
        return self.exit_code == 0 and self.status in {"valid", "verified_safe"}

    @property
    def valid(self) -> bool:
        return bool(self.payload and self.payload.get("valid", False))


def normalize_token_usage(usage: dict[str, Any] | None) -> tuple[int, int]:
    """Map provider ``usage`` payloads onto ``(input_tokens, output_tokens)``.

    DeepSeek's Chat Completions endpoint reports ``prompt_tokens`` and
    ``completion_tokens``; Qwen's Responses endpoint reports ``input_tokens``
    and ``output_tokens``. Missing or malformed fields count as zero.
    """

    if not usage:
        return 0, 0
    input_tokens = usage.get("input_tokens", usage.get("prompt_tokens", 0))
    output_tokens = usage.get("output_tokens", usage.get("completion_tokens", 0))
    return _as_int(input_tokens), _as_int(output_tokens)


def _as_int(value: Any) -> int:
    try:
        return max(0, int(value))
    except (TypeError, ValueError):
        return 0


@dataclass
class GenerationRound:
    round: int
    candidate_json: str | None = None
    parse_error: str | None = None
    validation: RustCliResult | None = None
    llm_error: str | None = None
    accepted: bool = False
    duration_ms: float = 0.0
    input_tokens: int = 0
    output_tokens: int = 0


@dataclass
class GenerationResult:
    cir_json: str | None
    rounds: list[GenerationRound] = field(default_factory=list)
    error: str | None = None

    @property
    def success(self) -> bool:
        return self.cir_json is not None and self.error is None

    @property
    def total_input_tokens(self) -> int:
        return sum(r.input_tokens for r in self.rounds)

    @property
    def total_output_tokens(self) -> int:
        return sum(r.output_tokens for r in self.rounds)

    @property
    def total_tokens(self) -> int:
        return self.total_input_tokens + self.total_output_tokens


@dataclass
class RepairRound:
    round: int
    candidate_cir_json: str | None = None
    parse_error: str | None = None
    verification: RustCliResult | None = None
    llm_error: str | None = None
    accepted: bool = False
    rejection_reason: str | None = None
    duration_ms: float = 0.0
    input_tokens: int = 0
    output_tokens: int = 0


@dataclass
class RepairResult:
    fixed_cir_json: str | None
    rounds: list[RepairRound] = field(default_factory=list)
    initial_verification: RustCliResult | None = None
    last_feedback: str = ""
    error: str | None = None

    @property
    def success(self) -> bool:
        return self.fixed_cir_json is not None and self.error is None

    @property
    def total_input_tokens(self) -> int:
        return sum(r.input_tokens for r in self.rounds)

    @property
    def total_output_tokens(self) -> int:
        return sum(r.output_tokens for r in self.rounds)

    @property
    def total_tokens(self) -> int:
        return self.total_input_tokens + self.total_output_tokens

    @property
    def repair_rounds(self) -> int:
        if self.success:
            return len(self.rounds)
        return -1
