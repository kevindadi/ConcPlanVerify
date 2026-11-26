"""Shared protocol and accounting types for the Python orchestration layer."""

from __future__ import annotations

from dataclasses import dataclass
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
class Consumption:
    """Measured request, token, and wall-clock consumption. Unknown usage stays ``None``."""

    http_requests: int = 0
    prompt_tokens: int | None = 0
    completion_tokens: int | None = 0
    llm_wall_ms: int = 0
    tool_wall_ms: int = 0
    rounds: int = 0
    first_correct_round: int | None = None

    def add_usage(self, usage: dict[str, Any] | None) -> None:
        if usage is None:
            self.prompt_tokens = None
            self.completion_tokens = None
            return
        prompt = usage.get("prompt_tokens", usage.get("input_tokens"))
        completion = usage.get("completion_tokens", usage.get("output_tokens"))
        if prompt is None or completion is None:
            self.prompt_tokens = None
            self.completion_tokens = None
            return
        if self.prompt_tokens is not None:
            self.prompt_tokens += int(prompt)
        if self.completion_tokens is not None:
            self.completion_tokens += int(completion)

    @property
    def total_tokens(self) -> int | None:
        if self.prompt_tokens is None or self.completion_tokens is None:
            return None
        return self.prompt_tokens + self.completion_tokens

    def as_dict(self) -> dict[str, Any]:
        return {
            "http_requests": self.http_requests,
            "prompt_tokens": self.prompt_tokens,
            "completion_tokens": self.completion_tokens,
            "total_tokens": self.total_tokens,
            "llm_wall_ms": self.llm_wall_ms,
            "tool_wall_ms": self.tool_wall_ms,
            "rounds": self.rounds,
            "first_correct_round": self.first_correct_round,
        }


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
