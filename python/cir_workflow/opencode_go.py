"""OpenCode Go (OpenAI-compatible) client for the second/third-model probe.

Only `/chat/completions` is used. The client mirrors `DeepSeekFlashClient`'s
`complete(system, user)` surface so the existing providers/workflows can drive
it unchanged.
"""

from __future__ import annotations

import hashlib
import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

OPENCODE_GO_BASE_URL = "https://opencode.ai/zen/go/v1"


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@dataclass
class OpenCodeOutcome:
    text: str
    messages: list[dict[str, str]]
    requested_model: str
    response_model: str | None
    request_id: str | None
    finish_reason: str | None
    usage: dict[str, Any] | None
    wall_ms: int
    transport_attempt: int
    prompt_sha256: str
    cost: float | None = None


class OpenCodeGoClient:
    def __init__(self, *, api_key: str, budget, evidence_dir: Path | str,
                 model: str, timeout: float = 90.0, max_tokens: int = 4096,
                 temperature: float = 0.0) -> None:
        from openai import OpenAI

        self.model = model
        self.budget = budget
        self.evidence_dir = Path(evidence_dir)
        self.evidence_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.evidence_dir / "requests.jsonl"
        self.timeout = timeout
        self.max_tokens = max_tokens
        self.temperature = temperature
        import uuid

        self._client = OpenAI(
            api_key=api_key, base_url=OPENCODE_GO_BASE_URL, timeout=timeout,
            max_retries=0,
            default_headers={"x-opencode-session": str(uuid.uuid4())})

    def _record(self, record: dict[str, Any]) -> None:
        with self.log_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    def _create_with_retry(self, kwargs: dict, messages: list, prompt_sha: str,
                           started: float):
        from .call_context import note_transport
        from .live import BudgetExhausted
        retryable = ("APIConnectionError", "APITimeoutError", "InternalServerError",
                     "RateLimitError")
        last_exc = None
        self.last_transport_log = []
        for attempt in range(3):
            try:
                self.budget.reserve()
            except BudgetExhausted as exc:
                exc.physical_attempts = len(self.last_transport_log)
                raise
            try:
                response = self._client.chat.completions.create(**kwargs)
            except Exception as exc:  # noqa: BLE001
                last_exc = exc
                note_transport(self, index=attempt + 1, status="error",
                               error_type=type(exc).__name__, error=str(exc))
                self._record({"status": "error", "model": self.model,
                              "messages": messages, "prompt_sha256": prompt_sha,
                              "transport_index": attempt + 1,
                              "error": str(exc), "error_type": type(exc).__name__,
                              "wall_ms": int((time.monotonic() - started) * 1000),
                              **getattr(self, "call_context", {})})
                status_code = getattr(exc, "status_code", None)
                retryable_status = status_code in {408, 429, 500, 502, 503, 504}
                wants_retry = (type(exc).__name__ in retryable or retryable_status) and attempt < 2
                if "temperature" in str(exc).lower() and kwargs.get("temperature") not in (None, 1, 1.0):
                    kwargs["temperature"] = 1.0
                    self.temperature = 1.0
                    wants_retry = attempt < 2
                if wants_retry:
                    if self.budget.exhausted():
                        from .live import BudgetExhausted
                        raise BudgetExhausted(
                            self.budget.exhausted() or "request budget reached",
                            physical_attempts=len(self.last_transport_log))
                    time.sleep(3 * (attempt + 1))
                    continue
                raise
            usage = None
            if getattr(response, "usage", None) is not None:
                usage = response.usage.model_dump() if hasattr(response.usage, "model_dump") \
                    else dict(response.usage)
            note_transport(self, index=attempt + 1, status="ok", usage_raw=usage,
                           request_id=getattr(response, "id", None))
            return response
        raise last_exc if last_exc else RuntimeError("no response")

    def complete(self, system_prompt: str, user_prompt: str) -> OpenCodeOutcome:
        messages = [{"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}]
        prompt_sha = _sha(json.dumps(messages, ensure_ascii=False, sort_keys=True))
        started = time.monotonic()
        kwargs: dict[str, Any] = {
            "model": self.model, "messages": messages,
            "temperature": self.temperature, "max_tokens": self.max_tokens,
        }
        response = self._create_with_retry(kwargs, messages, prompt_sha, started)
        wall_ms = int((time.monotonic() - started) * 1000)
        choice = (response.choices or [None])[0]
        message = getattr(choice, "message", None)
        content = getattr(message, "content", None) or ""
        usage = None
        if getattr(response, "usage", None) is not None:
            usage = response.usage.model_dump() if hasattr(response.usage, "model_dump") \
                else dict(response.usage)
        cost = getattr(response, "cost", None)
        response_model = getattr(response, "model", None)
        record = {"status": "ok", "model": self.model, "temperature": kwargs["temperature"],
                  "response_model": response_model,
                  "request_id": getattr(response, "id", None), "messages": messages,
                  "prompt_sha256": prompt_sha, "usage": usage, "cost": cost,
                  "finish_reason": getattr(choice, "finish_reason", None),
                  "content_sha256": _sha(content), "content": content, "wall_ms": wall_ms}
        self._record(record)
        return OpenCodeOutcome(
            text=content, messages=messages, requested_model=self.model,
            response_model=response_model, request_id=getattr(response, "id", None),
            finish_reason=getattr(choice, "finish_reason", None), usage=usage,
            wall_ms=wall_ms, transport_attempt=len(getattr(self, "last_transport_log", []) or []) or 1,
            prompt_sha256=prompt_sha, cost=cost)


class OpenCodeGoResponsesClient:
    """OpenCode Go via the Responses API, for models that reject chat/completions.

    ``gpt-6-luna`` and ``grok-4.7`` return ``ModelProtocolUnsupported`` on
    ``/chat/completions`` and require ``/responses`` plus the session header.
    Mirrors :class:`OpenCodeGoClient`'s ``complete(system, user)`` surface.
    """

    def __init__(self, *, api_key: str, budget, evidence_dir: Path | str,
                 model: str, timeout: float = 90.0, max_tokens: int = 4096,
                 temperature: float = 0.0) -> None:
        from openai import OpenAI

        self.model = model
        self.budget = budget
        self.evidence_dir = Path(evidence_dir)
        self.evidence_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.evidence_dir / "requests.jsonl"
        self.timeout = timeout
        self.max_tokens = max_tokens
        self.temperature = temperature
        import uuid

        self._client = OpenAI(
            api_key=api_key, base_url=OPENCODE_GO_BASE_URL, timeout=timeout,
            max_retries=0,
            default_headers={"x-opencode-session": str(uuid.uuid4())})

    def _record(self, record: dict[str, Any]) -> None:
        with self.log_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    def complete(self, system_prompt: str, user_prompt: str) -> OpenCodeOutcome:
        messages = [{"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}]
        from .call_context import note_transport
        from .live import BudgetExhausted
        prompt_sha = _sha(json.dumps(messages, ensure_ascii=False, sort_keys=True))
        self.last_transport_log = []
        started = time.monotonic()
        kwargs: dict[str, Any] = {
            "model": self.model, "instructions": system_prompt,
            "input": user_prompt, "max_output_tokens": self.max_tokens,
        }
        response = None
        for attempt in range(3):
            try:
                self.budget.reserve()
            except BudgetExhausted as exc:
                exc.physical_attempts = len(self.last_transport_log)
                raise
            try:
                response = self._client.responses.create(**kwargs)
                break
            except Exception as exc:  # noqa: BLE001
                note_transport(self, index=attempt + 1, status="error",
                               error_type=type(exc).__name__, error=str(exc))
                self._record({"status": "error", "model": self.model,
                              "surface": "responses", "messages": messages,
                              "prompt_sha256": prompt_sha, "transport_index": attempt + 1,
                              "error": str(exc), "error_type": type(exc).__name__,
                              "wall_ms": int((time.monotonic() - started) * 1000),
                              **getattr(self, "call_context", {})})
                status_code = getattr(exc, "status_code", None)
                retryable = type(exc).__name__ in {
                    "APIConnectionError", "APITimeoutError", "InternalServerError", "RateLimitError",
                } or status_code in {408, 429, 500, 502, 503, 504}
                if retryable and attempt < 2 and not self.budget.exhausted():
                    time.sleep(3 * (attempt + 1))
                    continue
                if retryable and attempt < 2 and self.budget.exhausted():
                    raise BudgetExhausted(self.budget.exhausted() or "request budget reached",
                                          physical_attempts=len(self.last_transport_log))
                raise
        wall_ms = int((time.monotonic() - started) * 1000)
        content = getattr(response, "output_text", None) or ""
        usage = None
        if getattr(response, "usage", None) is not None:
            usage = response.usage.model_dump() if hasattr(response.usage, "model_dump") \
                else dict(response.usage)
        response_model = getattr(response, "model", None)
        index = len(self.last_transport_log) + 1
        note_transport(self, index=index, status="ok", usage_raw=usage,
                       request_id=getattr(response, "id", None))
        self._record({"status": "ok", "model": self.model, "surface": "responses",
                      "response_model": response_model,
                      "request_id": getattr(response, "id", None),
                      "messages": messages, "prompt_sha256": prompt_sha,
                      "usage": usage, "transport_index": index,
                      "content_sha256": _sha(content),
                      "content": content, "wall_ms": wall_ms,
                      **getattr(self, "call_context", {})})
        return OpenCodeOutcome(
            text=content, messages=messages, requested_model=self.model,
            response_model=response_model, request_id=getattr(response, "id", None),
            finish_reason=None, usage=usage, wall_ms=wall_ms,
            transport_attempt=len(self.last_transport_log), prompt_sha256=prompt_sha, cost=None)


def list_models(api_key: str, timeout: float = 30.0) -> dict[str, Any]:
    from openai import OpenAI

    client = OpenAI(api_key=api_key, base_url=OPENCODE_GO_BASE_URL, timeout=timeout)
    models = client.models.list()
    return models.model_dump() if hasattr(models, "model_dump") else {"data": []}
