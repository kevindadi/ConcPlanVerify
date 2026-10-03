"""Generic direct OpenAI-compatible chat client (for Qwen/DashScope direct).

DeepSeek keeps its own pinned ``DeepSeekFlashClient`` (thinking disabled,
Flash-only). This client covers the other direct provider (DashScope/Qwen)
whose compatible-mode endpoint speaks Chat Completions with a usage payload.
It records the same evidence as the other clients and never falls back to a
different model.
"""

from __future__ import annotations

import hashlib
import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@dataclass
class DirectOutcome:
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


class DirectChatClient:
    def __init__(self, *, api_key: str, base_url: str, model: str, budget: Any,
                 evidence_dir: Path | str, timeout: float = 90.0,
                 max_tokens: int = 4096, temperature: float = 0.0,
                 extra_body: dict[str, Any] | None = None) -> None:
        from openai import OpenAI

        self.model = model
        self.base_url = base_url
        self.budget = budget
        self.evidence_dir = Path(evidence_dir)
        self.evidence_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.evidence_dir / "requests.jsonl"
        self.timeout = float(timeout)
        self.max_tokens = int(max_tokens)
        self.temperature = temperature
        self.extra_body = extra_body or {}
        self._client = OpenAI(api_key=api_key, base_url=base_url, timeout=timeout,
                              max_retries=0)

    def _record(self, record: dict[str, Any]) -> None:
        with self.log_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    def complete(self, system_prompt: str, user_prompt: str) -> DirectOutcome:
        messages = [{"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}]
        from .call_context import note_transport
        from .live import BudgetExhausted, reservation_link
        prompt_sha = _sha(json.dumps(messages, ensure_ascii=False, sort_keys=True))
        self.last_transport_log = []
        try:
            self.budget.reserve(reservation_link(self, 1))
        except BudgetExhausted as exc:
            exc.physical_attempts = 0
            raise
        if hasattr(self.budget, "mark_attempt"):
            self.budget.mark_attempt("dispatched")
        started = time.monotonic()
        kwargs: dict[str, Any] = {"model": self.model, "messages": messages,
                                  "temperature": self.temperature,
                                  "max_tokens": self.max_tokens}
        if self.extra_body:
            kwargs["extra_body"] = self.extra_body
        try:
            response = self._client.chat.completions.create(**kwargs)
        except Exception as exc:  # noqa: BLE001
            if hasattr(self.budget, "mark_attempt"):
                self.budget.mark_attempt("awaiting_reconciliation", detail=type(exc).__name__)
            note_transport(self, index=1, status="error",
                           error_type=type(exc).__name__, error=str(exc))
            self._record({"status": "error", "model": self.model,
                          "base_url": self.base_url, "messages": messages,
                          "prompt_sha256": prompt_sha, "transport_index": 1,
                          "error": str(exc), "error_type": type(exc).__name__,
                          "wall_ms": int((time.monotonic() - started) * 1000),
                          **getattr(self, "call_context", {})})
            raise
        wall_ms = int((time.monotonic() - started) * 1000)
        choice = (getattr(response, "choices", None) or [None])[0]
        content = getattr(getattr(choice, "message", None), "content", None) or ""
        usage = None
        if getattr(response, "usage", None) is not None:
            usage = response.usage.model_dump() if hasattr(response.usage, "model_dump") \
                else dict(response.usage)
        response_model = getattr(response, "model", None)
        note_transport(self, index=1, status="ok", usage_raw=usage,
                       request_id=getattr(response, "id", None))
        if hasattr(self.budget, "mark_attempt"):
            self.budget.mark_attempt("returned")
        from .live import preserve_response_record
        preserve_response_record(self.budget, self._record, {"status": "ok", "model": self.model,
                      "base_url": self.base_url, "response_model": response_model,
                      "request_id": getattr(response, "id", None),
                      "messages": messages, "prompt_sha256": prompt_sha,
                      "usage": usage, "transport_index": 1,
                      "content_sha256": _sha(content),
                      "content": content, "wall_ms": wall_ms,
                      **getattr(self, "call_context", {})})
        return DirectOutcome(
            text=content, messages=messages, requested_model=self.model,
            response_model=response_model, request_id=getattr(response, "id", None),
            finish_reason=getattr(choice, "finish_reason", None), usage=usage,
            wall_ms=wall_ms, transport_attempt=1, prompt_sha256=prompt_sha, cost=None)
