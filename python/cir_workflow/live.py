"""Controlled live entry: DeepSeek **Flash only**.

This module is the only place that performs real model HTTP requests. It is
deliberately narrow:

- provider is fixed to ``deepseek`` and the model to ``deepseek-flash``; any other
  value fails *before* a request is made, and after every response the model
  identity is checked against the request (no Pro, no aliases, no fallback);
- thinking is explicitly disabled via ``extra_body={"thinking": {"type":
  "disabled"}}`` (the API enables thinking by default);
- a single shared, persisted request budget (counted before each HTTP attempt,
  including transport retries) and a single wall-clock deadline cover the whole
  batch, so a restart cannot silently reset the budget;
- every attempt is recorded sanitized (messages, prompt hashes, requested and
  response model, request id, finish_reason, usage or null, timing, retry index)
  with no API key or request headers.

The scripted/offline path remains keyless and network-free; nothing here runs
unless the ``live`` command is invoked.
"""

from __future__ import annotations

import hashlib
import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient
from .offline_workflow import OfflineWorkflow, _exclusive_run_dir
from .prompts import generation_system_prompt, generation_user_prompt, retry_user_prompt
from .providers import CandidateProvider, CandidateRequest, CandidateResponse
from .structural import run_structural_check

ALLOWED_PROVIDER = "deepseek"
ALLOWED_MODEL = "deepseek-flash"
DEEPSEEK_BASE_URL = "https://api.deepseek.com"
THINKING_DISABLED = {"thinking": {"type": "disabled"}}
DEFAULT_MAX_TOKENS = 4096
DEFAULT_TIMEOUT = 90.0
DEFAULT_MAX_REQUESTS = 12
DEFAULT_MAX_SECONDS = 1200.0
MAX_TRANSPORT_RETRIES = 1

RETRYABLE_STATUS = {408, 409, 425, 429, 500, 502, 503, 504}


class LiveError(RuntimeError):
    """Base class for live-run control errors."""


class BudgetExhausted(LiveError):
    """The shared request budget or deadline is used up; stop the batch."""


class ModelIdentityError(LiveError):
    """The response model is not the requested Flash model; stop the batch."""


class NonRetryableLlmError(LiveError):
    """Authentication/parameter/balance/model error; stop the batch."""


class TransientLlmError(LiveError):
    """A retryable transport error persisted past the retry budget."""


class EmptyResponseError(LiveError):
    """The model returned no usable text; the workflow may retry."""


def assert_allowed_model(provider: str, model_id: str) -> None:
    """Fail before any request if the provider/model is not exactly allowed."""

    if provider is None or provider.strip().lower() != ALLOWED_PROVIDER:
        raise ValueError(
            f"only provider {ALLOWED_PROVIDER!r} is allowed, got {provider!r}"
        )
    if model_id != ALLOWED_MODEL:
        raise ValueError(
            f"only model {ALLOWED_MODEL!r} is allowed, got {model_id!r} "
            "(Pro/aliases/fallback are forbidden)"
        )


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@dataclass
class LiveBudget:
    """Persisted, shared budget for the whole batch.

    ``reserve`` is called before *each* HTTP attempt and persists the count, so
    failed requests, transport retries and restarts all consume the budget.
    """

    budget_path: Path
    max_requests: int = DEFAULT_MAX_REQUESTS
    max_seconds: float = DEFAULT_MAX_SECONDS
    deadline_epoch: float = 0.0
    requests_used: int = 0

    def __post_init__(self) -> None:
        self.budget_path.parent.mkdir(parents=True, exist_ok=True)
        if self.budget_path.exists():
            state = json.loads(self.budget_path.read_text(encoding="utf-8"))
            self.requests_used = int(state.get("requests_used", 0))
            self.deadline_epoch = float(state.get("deadline_epoch", 0.0))
        if self.deadline_epoch <= 0:
            self.deadline_epoch = time.time() + self.max_seconds
        self._save()

    def _save(self) -> None:
        self.budget_path.write_text(json.dumps({
            "max_requests": self.max_requests,
            "max_seconds": self.max_seconds,
            "deadline_epoch": self.deadline_epoch,
            "requests_used": self.requests_used,
        }, indent=2) + "\n", encoding="utf-8")

    def exhausted(self) -> str | None:
        if self.requests_used >= self.max_requests:
            return f"request budget reached ({self.requests_used}/{self.max_requests})"
        if time.time() >= self.deadline_epoch:
            return "batch wall-clock deadline reached"
        return None

    def reserve(self) -> int:
        reason = self.exhausted()
        if reason:
            raise BudgetExhausted(reason)
        self.requests_used += 1
        self._save()
        return self.requests_used

    @property
    def remaining(self) -> int:
        return max(0, self.max_requests - self.requests_used)


@dataclass
class LiveChatOutcome:
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


class DeepSeekFlashClient:
    """Chat Completions client pinned to ``deepseek-flash`` with evidence capture."""

    def __init__(
        self,
        *,
        api_key: str,
        budget: LiveBudget,
        evidence_dir: Path,
        sdk_client: Any | None = None,
        timeout: float = DEFAULT_TIMEOUT,
        max_tokens: int = DEFAULT_MAX_TOKENS,
        max_transport_retries: int = MAX_TRANSPORT_RETRIES,
    ) -> None:
        assert_allowed_model(ALLOWED_PROVIDER, ALLOWED_MODEL)
        if not api_key:
            raise ValueError("missing DeepSeek API key")
        self.api_key = api_key
        self.budget = budget
        self.evidence_dir = Path(evidence_dir)
        self.evidence_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.evidence_dir / "llm_requests.jsonl"
        self.timeout = float(timeout)
        self.max_tokens = int(max_tokens)
        self.max_transport_retries = max(0, int(max_transport_retries))
        self._sdk = sdk_client

    # ---- SDK -------------------------------------------------------------
    def _client(self):
        if self._sdk is not None:
            return self._sdk
        from openai import OpenAI  # imported lazily so offline use needs no key/SDK

        self._sdk = OpenAI(
            api_key=self.api_key,
            base_url=DEEPSEEK_BASE_URL,
            timeout=self.timeout,
            max_retries=0,  # transport retries are handled here and counted
        )
        return self._sdk

    def _request_kwargs(self, messages: list[dict[str, str]]) -> dict[str, Any]:
        return {
            "model": ALLOWED_MODEL,
            "messages": messages,
            "temperature": 0.0,
            "max_tokens": self.max_tokens,
            "stream": False,
            "extra_body": {"thinking": dict(THINKING_DISABLED["thinking"])},
        }

    def complete(self, system_prompt: str, user_prompt: str) -> LiveChatOutcome:
        assert_allowed_model(ALLOWED_PROVIDER, ALLOWED_MODEL)
        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ]
        prompt_sha = _sha(json.dumps(messages, ensure_ascii=False, sort_keys=True))
        kwargs = self._request_kwargs(messages)
        transport_attempt = 0
        while True:
            self.budget.reserve()  # raises BudgetExhausted before sending
            transport_attempt += 1
            started = time.monotonic()
            try:
                response = self._client().chat.completions.create(**kwargs)
            except Exception as exc:  # provider SDK exception types vary
                retryable = _is_retryable(exc)
                self._record({
                    "status": "error",
                    "requested_model": ALLOWED_MODEL,
                    "messages": messages,
                    "prompt_sha256": prompt_sha,
                    "max_tokens": self.max_tokens,
                    "temperature": kwargs["temperature"],
                    "thinking": kwargs["extra_body"],
                    "transport_attempt": transport_attempt,
                    "retryable": retryable,
                    "status_code": _status_code(exc),
                    "error_type": type(exc).__name__,
                    "error": str(exc),
                    "wall_ms": int((time.monotonic() - started) * 1000),
                })
                if retryable and transport_attempt <= self.max_transport_retries:
                    continue
                if retryable:
                    raise TransientLlmError(str(exc)) from exc
                raise NonRetryableLlmError(str(exc)) from exc

            wall_ms = int((time.monotonic() - started) * 1000)
            usage = _usage_dict(getattr(response, "usage", None))
            choice = (getattr(response, "choices", None) or [None])[0]
            message = getattr(choice, "message", None)
            content = getattr(message, "content", None)
            response_model = getattr(response, "model", None)
            record = {
                "status": "ok",
                "requested_model": ALLOWED_MODEL,
                "response_model": response_model,
                "request_id": getattr(response, "id", None),
                "messages": messages,
                "prompt_sha256": prompt_sha,
                "max_tokens": self.max_tokens,
                "temperature": kwargs["temperature"],
                "thinking": kwargs["extra_body"],
                "transport_attempt": transport_attempt,
                "finish_reason": getattr(choice, "finish_reason", None),
                "usage": usage,
                "content_sha256": _sha(content) if isinstance(content, str) else None,
                "content": content if isinstance(content, str) else None,
                "wall_ms": wall_ms,
            }
            self._record(record)
            if response_model != ALLOWED_MODEL:
                raise ModelIdentityError(
                    f"response model {response_model!r} is not {ALLOWED_MODEL!r}; stopping batch"
                )
            if not isinstance(content, str) or not content.strip():
                raise EmptyResponseError("model returned empty content")
            return LiveChatOutcome(
                text=content, messages=messages, requested_model=ALLOWED_MODEL,
                response_model=response_model, request_id=getattr(response, "id", None),
                finish_reason=getattr(choice, "finish_reason", None), usage=usage,
                wall_ms=wall_ms, transport_attempt=transport_attempt, prompt_sha256=prompt_sha,
            )

    def _record(self, record: dict[str, Any]) -> None:
        self.evidence_dir.mkdir(parents=True, exist_ok=True)
        with self.log_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")
        with self.log_path.open(encoding="utf-8") as handle:
            index = sum(1 for _ in handle)
        (self.evidence_dir / f"llm-{index:02d}.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


class RecordingProvider:
    """Adapt :class:`DeepSeekFlashClient` to the workflow's provider protocol."""

    name = "llm"

    def __init__(self, client: DeepSeekFlashClient) -> None:
        self.client = client
        self.calls: list[dict[str, Any]] = []

    def propose(self, request: CandidateRequest) -> CandidateResponse:
        system = generation_system_prompt()
        if request.feedback is None:
            user = generation_user_prompt(request.requirements, request.contract)
        else:
            user = retry_user_prompt(
                request.requirements, request.contract,
                previous_candidate=request.previous_candidate or "",
                feedback={"rendered": request.feedback},
            )
        try:
            outcome = self.client.complete(system, user)
        except EmptyResponseError as exc:
            # A candidate-level failure the workflow may retry within its rounds.
            return CandidateResponse(text="", source="llm", provider=ALLOWED_PROVIDER,
                                     model_id=ALLOWED_MODEL, error=str(exc))
        # BudgetExhausted / ModelIdentityError / NonRetryable / Transient propagate
        # to stop the batch.
        self.calls.append({
            "attempt": request.attempt,
            "request_id": outcome.request_id,
            "response_model": outcome.response_model,
            "finish_reason": outcome.finish_reason,
            "usage": outcome.usage,
            "transport_attempt": outcome.transport_attempt,
            "prompt_sha256": outcome.prompt_sha256,
            "wall_ms": outcome.wall_ms,
            "had_feedback": request.feedback is not None,
        })
        return CandidateResponse.from_usage(
            outcome.text, "llm", ALLOWED_PROVIDER,
            model_id=outcome.response_model or ALLOWED_MODEL, usage=outcome.usage,
        )


def _usage_dict(usage: Any) -> dict[str, Any] | None:
    """Return usage as a dict, or ``None`` when the provider omitted it."""
    if usage is None:
        return None
    if isinstance(usage, dict):
        return usage
    if hasattr(usage, "model_dump"):
        dumped = usage.model_dump()
        return dumped if isinstance(dumped, dict) else None
    return None


def _status_code(exc: Exception) -> int | None:
    for attr in ("status_code", "http_status", "code"):
        value = getattr(exc, attr, None)
        if isinstance(value, int):
            return value
    return None


def _is_retryable(exc: Exception) -> bool:
    code = _status_code(exc)
    if code is not None:
        return code in RETRYABLE_STATUS
    name = type(exc).__name__.lower()
    if any(token in name for token in ("timeout", "connection", "ratelimit", "internalserver", "apistatus")):
        # APIConnectionError/timeout/rate-limit/5xx are retryable; APIConnection
        # covers DNS/socket failures too.
        return True
    return False


# ───────────────────────────── live pilot runner ─────────────────────────────


def run_live_pilot(
    tasks_path: Path | str,
    *,
    out_dir: Path | str,
    binary: Path | str,
    api_key: str,
    timeout: float = DEFAULT_TIMEOUT,
    max_tokens: int = DEFAULT_MAX_TOKENS,
    max_generation_rounds: int = 3,
    sdk_client: Any | None = None,
) -> dict[str, Any]:
    """Run the frozen tasks against DeepSeek Flash with a shared budget.

    Returns a summary dict and writes ``summary.json`` plus all raw evidence
    under an exclusive batch directory. Stops the batch on the first budget,
    model-identity or non-retryable provider error.
    """
    tasks_path = Path(tasks_path).expanduser().resolve()
    spec = json.loads(tasks_path.read_text(encoding="utf-8"))
    tasks = spec.get("tasks", [])
    out_dir = Path(out_dir).expanduser().resolve()
    batch_dir = _exclusive_run_dir(out_dir)

    budget = LiveBudget(
        batch_dir / "budget.json",
        max_requests=int(spec.get("max_requests", DEFAULT_MAX_REQUESTS)),
        max_seconds=float(spec.get("max_seconds", DEFAULT_MAX_SECONDS)),
    )
    summary: dict[str, Any] = {
        "batch_dir": str(batch_dir),
        "provider": ALLOWED_PROVIDER,
        "requested_model": ALLOWED_MODEL,
        "base_url": DEEPSEEK_BASE_URL,
        "thinking": THINKING_DISABLED,
        "max_tokens": max_tokens,
        "timeout_s": timeout,
        "max_requests": budget.max_requests,
        "max_seconds": budget.max_seconds,
        "deadline_epoch": budget.deadline_epoch,
        "max_generation_rounds": max_generation_rounds,
        "tasks_path": str(tasks_path),
        "tasks": [],
        "stop_reason": None,
    }

    stop_reason = None
    for task in tasks:
        reason = budget.exhausted()
        if reason:
            summary["tasks"].append({"id": task.get("id"), "skipped": True, "stop_reason": reason})
            stop_reason = reason
            break

        task_id = task["id"]
        contract_path = (tasks_path.parent / task["contract"]).resolve()
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        task_dir = batch_dir / task_id

        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        llm = DeepSeekFlashClient(
            api_key=api_key, budget=budget, evidence_dir=batch_dir / "llm",
            timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client,
        )
        provider = RecordingProvider(llm)
        workflow = OfflineWorkflow(
            client, provider, out_dir=task_dir,
            max_generation_rounds=max_generation_rounds, engine="petri", repair_strategy="c",
        )

        result = None
        stop_reason = None
        try:
            result = workflow.run(task["requirements"], contract)
        except Exception as exc:  # record and stop the batch; never reset the budget silently
            stop_reason = f"{type(exc).__name__}: {exc}"

        modeling = None
        frozen_path = None
        if result is not None and result.frozen_cir_sha256:
            frozen_path = Path(result.out_dir) / "frozen_initial.cir.json"
            try:
                program = json.loads(frozen_path.read_text(encoding="utf-8"))
                modeling = run_structural_check(task["structural_check"], program)
            except (OSError, json.JSONDecodeError) as exc:
                modeling = {"ok": None, "reason": f"frozen CIR unreadable: {exc}"}

        record = {
            "id": task_id,
            "structural_check": task["structural_check"],
            "calls": provider.calls,
            "requests_used_total": budget.requests_used,
            "status": None if result is None else result.status,
            "repaired_by_tool": None if result is None else result.repaired_by_tool,
            "frozen_cir_path": str(frozen_path) if frozen_path else None,
            "frozen_cir_sha256": None if result is None else result.frozen_cir_sha256,
            "generation_rounds": None if result is None else result.generation_rounds,
            "check": None if result is None else result.check,
            "support": None if result is None else result.support,
            "explore": None if result is None else result.explore,
            "repair": None if result is None else result.repair,
            "replay": None if result is None else result.replay,
            "error": None if result is None else result.error,
            "modeling": modeling,
            "stop_reason": stop_reason,
            "run_dir": None if result is None else result.out_dir,
        }
        summary["tasks"].append(record)
        if stop_reason:
            summary["stop_reason"] = stop_reason
            break

    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    (batch_dir / "summary.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return summary
