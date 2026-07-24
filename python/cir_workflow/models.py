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


@dataclass
class GenerationRound:
    round: int
    candidate_json: str | None = None
    parse_error: str | None = None
    validation: RustCliResult | None = None
    llm_error: str | None = None
    accepted: bool = False
    duration_ms: float = 0.0


@dataclass
class GenerationResult:
    cir_json: str | None
    rounds: list[GenerationRound] = field(default_factory=list)
    error: str | None = None

    @property
    def success(self) -> bool:
        return self.cir_json is not None and self.error is None


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
    def repair_rounds(self) -> int:
        if self.success:
            return len(self.rounds)
        return -1
