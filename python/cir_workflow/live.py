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

import fcntl
import hashlib
import sqlite3
import json
import os
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient
from .offline_workflow import OfflineWorkflow, _exclusive_run_dir
from .patch_repair import ExternalPatchRepairWorkflow
from .prompts import (
    generation_system_prompt,
    generation_user_prompt,
    patch_system_prompt,
    patch_user_prompt,
    retry_user_prompt,
)
from .providers import (
    CandidateProvider,
    CandidateRequest,
    CandidateResponse,
    PatchRequest,
    PatchResponse,
)
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

    def __init__(self, message: str = "", *, physical_attempts: int = 0) -> None:
        super().__init__(message)
        self.physical_attempts = physical_attempts


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
        self._db_path = self.budget_path.with_suffix(".sqlite")
        conn = self._connect()
        try:
            conn.execute("BEGIN IMMEDIATE")
            conn.execute(
                "CREATE TABLE IF NOT EXISTS budget ("
                "id INTEGER PRIMARY KEY CHECK (id = 1),"
                "max_requests INTEGER NOT NULL,"
                "max_seconds REAL NOT NULL,"
                "deadline_epoch REAL NOT NULL,"
                "requests_used INTEGER NOT NULL)")
            conn.execute(
                "CREATE TABLE IF NOT EXISTS attempt ("
                "id INTEGER PRIMARY KEY AUTOINCREMENT,"
                "reserved_at REAL NOT NULL,"
                "status TEXT NOT NULL,"
                "detail TEXT,"
                "run_id TEXT,"
                "cell_id TEXT,"
                "logical_attempt TEXT,"
                "transport_retry INTEGER,"
                "stage TEXT,"
                "model TEXT)")
            # Upgrade a database created before the stage/model columns existed.
            attempt_cols = {info[1] for info in conn.execute("PRAGMA table_info(attempt)")}
            for column in ("stage", "model"):
                if column not in attempt_cols:
                    conn.execute(f"ALTER TABLE attempt ADD COLUMN {column} TEXT")
            row = conn.execute(
                "SELECT max_requests, max_seconds, deadline_epoch, requests_used FROM budget WHERE id = 1"
            ).fetchone()
            if row is None and self.budget_path.exists():
                prior = json.loads(self.budget_path.read_text(encoding="utf-8"))
                used = int(prior.get("requests_used", 0))
                deadline = float(prior.get("deadline_epoch", 0.0))
                if deadline <= 0:
                    deadline = time.time() + self.max_seconds
                conn.execute(
                    "INSERT INTO budget (id, max_requests, max_seconds, deadline_epoch, requests_used) "
                    "VALUES (1, ?, ?, ?, ?)",
                    (self.max_requests, self.max_seconds, deadline, used))
                row = (self.max_requests, self.max_seconds, deadline, used)
            elif row is None:
                deadline = self.deadline_epoch if self.deadline_epoch > 0 else time.time() + self.max_seconds
                conn.execute(
                    "INSERT INTO budget (id, max_requests, max_seconds, deadline_epoch, requests_used) "
                    "VALUES (1, ?, ?, ?, ?)",
                    (self.max_requests, self.max_seconds, deadline, 0))
                row = (self.max_requests, self.max_seconds, deadline, 0)
            self.max_requests = int(row[0])
            self.max_seconds = float(row[1])
            self.deadline_epoch = float(row[2])
            self.requests_used = int(row[3])
            conn.commit()
        finally:
            conn.close()
        self._tls = threading.local()
        self.mirror_error = None
        self.lifecycle_error = None
        self._mirror()

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self._db_path, timeout=30, isolation_level=None)
        conn.execute("PRAGMA busy_timeout=30000")
        return conn

    def _mirror(self) -> None:
        """Rewrite the JSON from the committed SQLite row.

        The lock plus a per-writer temp file stops a stale instance from
        replacing a newer count, and a mirror error does not undo the reserve.
        """

        lock_path = self.budget_path.with_name(self.budget_path.name + ".lock")
        try:
            with lock_path.open("a", encoding="utf-8") as handle:
                fcntl.flock(handle, fcntl.LOCK_EX)
                conn = self._connect()
                try:
                    row = conn.execute(
                        "SELECT max_requests, max_seconds, deadline_epoch, requests_used "
                        "FROM budget WHERE id = 1"
                    ).fetchone()
                finally:
                    conn.close()
                if row is None:
                    return
                self.max_requests = int(row[0])
                self.max_seconds = float(row[1])
                self.deadline_epoch = float(row[2])
                self.requests_used = int(row[3])
                payload = json.dumps({
                    "max_requests": self.max_requests,
                    "max_seconds": self.max_seconds,
                    "deadline_epoch": self.deadline_epoch,
                    "requests_used": self.requests_used,
                }, indent=2) + "\n"
                temporary = self.budget_path.with_name(
                    f"{self.budget_path.name}.{os.getpid()}.{threading.get_ident()}.tmp")
                temporary.write_text(payload, encoding="utf-8")
                os.replace(temporary, self.budget_path)
            self.mirror_error = None
        except OSError as exc:
            self.mirror_error = f"{type(exc).__name__}: {exc}"

    def exhausted(self) -> str | None:
        if self.requests_used >= self.max_requests:
            return f"request budget reached ({self.requests_used}/{self.max_requests})"
        if time.time() >= self.deadline_epoch:
            return "batch wall-clock deadline reached"
        return None

    def reserve(self, link: dict | None = None) -> int:
        """Reserve one send. The count is read and updated in one transaction.

        A second process or LiveBudget instance sees the committed count.
        A failed send is not refunded, and a restart does not move the deadline.
        ``link`` stores the run, cell, and logical attempt beside the reservation.
        Existing databases without those columns are not altered.
        """

        link = link or {}
        conn = self._connect()
        try:
            conn.execute("BEGIN IMMEDIATE")
            row = conn.execute(
                "SELECT max_requests, deadline_epoch, requests_used FROM budget WHERE id = 1"
            ).fetchone()
            used = int(row[2])
            deadline = float(row[1])
            limit = int(row[0])
            if used >= limit:
                conn.rollback()
                self.requests_used = used
                raise BudgetExhausted(f"request budget reached ({used}/{limit})")
            if time.time() >= deadline:
                conn.rollback()
                self.deadline_epoch = deadline
                raise BudgetExhausted("batch wall-clock deadline reached")
            used += 1
            conn.execute("UPDATE budget SET requests_used = ? WHERE id = 1", (used,))
            columns = {info[1] for info in conn.execute("PRAGMA table_info(attempt)")}
            if "logical_attempt" in columns:
                cursor = conn.execute(
                    "INSERT INTO attempt (reserved_at, status, detail, run_id, cell_id, "
                    "logical_attempt, transport_retry, stage, model) "
                    "VALUES (?, 'reserved', NULL, ?, ?, ?, ?, ?, ?)",
                    (time.time(), link.get("run_id"), link.get("cell_id"),
                     link.get("logical_attempt"), link.get("transport_retry"),
                     link.get("stage"), link.get("model")))
            else:
                cursor = conn.execute(
                    "INSERT INTO attempt (reserved_at, status, detail) VALUES (?, 'reserved', NULL)",
                    (time.time(),))
            reservation_id = int(cursor.lastrowid)
            conn.commit()
        finally:
            conn.close()
        self._tls.reservation_id = reservation_id
        ids = getattr(self._tls, "ids", None)
        if ids is None:
            self._tls.ids = ids = []
        ids.append(reservation_id)
        self.requests_used = used
        self.deadline_epoch = deadline
        self._mirror()
        return used

    def begin_logical_call(self) -> None:
        """Start a fresh reservation list for one logical model call."""

        self._tls.ids = []

    def reservation_ids(self) -> list[int]:
        return list(getattr(self._tls, "ids", []) or [])

    def mark_attempt(self, status: str, *, detail: str | None = None) -> None:
        """Record dispatch or return for the reservation made on this thread.

        A database error after the response exists must not discard that response
        or cause another paid send.
        """

        reservation_id = getattr(self._tls, "reservation_id", None)
        if reservation_id is None:
            return
        try:
            conn = self._connect()
            try:
                conn.execute("BEGIN IMMEDIATE")
                conn.execute(
                    "UPDATE attempt SET status = ?, detail = ? WHERE id = ?",
                    (status, detail, reservation_id))
                conn.commit()
            finally:
                conn.close()
        except Exception as exc:  # noqa: BLE001 - keep the already received response
            self.lifecycle_error = f"{type(exc).__name__}: {exc}"

    def attempt_rows(self) -> list[dict]:
        conn = self._connect()
        try:
            columns = [info[1] for info in conn.execute("PRAGMA table_info(attempt)")]
            rows = conn.execute(
                "SELECT " + ", ".join(columns) + " FROM attempt ORDER BY id"
            ).fetchall()
        finally:
            conn.close()
        return [dict(zip(columns, row)) for row in rows]

    @property
    def remaining(self) -> int:
        return max(0, self.max_requests - self.requests_used)

    def cell_attempt_count(self, cell_id: str) -> int:
        """Physical attempts already reserved for one cell, across restarts."""
        if not cell_id:
            return 0
        conn = self._connect()
        try:
            row = conn.execute("SELECT COUNT(*) FROM attempt WHERE cell_id = ?",
                               (cell_id,)).fetchone()
            return int(row[0]) if row else 0
        finally:
            conn.close()


def preserve_response_record(budget: Any, write, record: dict[str, Any]) -> None:
    """A failed evidence write must not drop a response that already came back."""

    try:
        write(record)
    except OSError as exc:
        if budget is not None and hasattr(budget, "lifecycle_error"):
            budget.lifecycle_error = f"{type(exc).__name__}: {exc}"


def reservation_link(client: Any, transport_retry: int) -> dict[str, Any]:
    """Identity stored with a budget reservation. Callers pass the client."""

    ctx = getattr(client, "call_context", None) or {}
    return {
        "run_id": ctx.get("run_id"),
        "cell_id": ctx.get("cell_id"),
        "logical_attempt": ctx.get("attempt_id"),
        "transport_retry": int(transport_retry),
        "stage": ctx.get("stage") or getattr(client, "stage", None),
        "model": ctx.get("model"),
    }


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
        max_tokens: int | None = DEFAULT_MAX_TOKENS,
        max_transport_retries: int = MAX_TRANSPORT_RETRIES,
        thinking: dict[str, Any] | None = None,
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
        self.max_tokens = int(max_tokens) if max_tokens is not None else None
        self.max_transport_retries = max(0, int(max_transport_retries))
        # None keeps the historical default (thinking explicitly disabled).
        self.thinking = dict(thinking) if thinking else None
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
        thinking = self.thinking or {"thinking": dict(THINKING_DISABLED["thinking"])}
        kwargs = {
            "model": ALLOWED_MODEL,
            "messages": messages,
            "temperature": 0.0,
            "stream": False,
            "extra_body": thinking,
        }
        if self.max_tokens is not None:
            kwargs["max_tokens"] = self.max_tokens
        self.last_request_params = {k: v for k, v in kwargs.items() if k != "messages"}
        return kwargs

    def complete(self, system_prompt: str, user_prompt: str) -> LiveChatOutcome:
        assert_allowed_model(ALLOWED_PROVIDER, ALLOWED_MODEL)
        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ]
        from .call_context import note_transport
        prompt_sha = _sha(json.dumps(messages, ensure_ascii=False, sort_keys=True))
        kwargs = self._request_kwargs(messages)
        self.last_transport_log = []
        sdk = self._client()
        transport_attempt = 0
        while True:
            try:
                self.budget.reserve(reservation_link(self, transport_attempt + 1))
            except BudgetExhausted as exc:
                exc.physical_attempts = transport_attempt
                raise
            transport_attempt += 1
            self.budget.mark_attempt("dispatched")
            started = time.monotonic()
            try:
                response = sdk.chat.completions.create(**kwargs)
            except Exception as exc:  # provider SDK exception types vary
                self.budget.mark_attempt("awaiting_reconciliation", detail=type(exc).__name__)
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
                    **getattr(self, "call_context", {}),
                })
                note_transport(self, index=transport_attempt, status="error",
                               error_type=type(exc).__name__, error=str(exc))
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
            record.update(getattr(self, "call_context", {}))
            self.budget.mark_attempt("returned")
            preserve_response_record(self.budget, self._record, record)
            note_transport(self, index=transport_attempt, status="ok", usage_raw=usage,
                           request_id=getattr(response, "id", None))
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
        response = CandidateResponse.from_usage(
            outcome.text, "llm", ALLOWED_PROVIDER,
            model_id=outcome.response_model or ALLOWED_MODEL, usage=outcome.usage,
        )
        response.wall_ms = outcome.wall_ms
        return response


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


class RecordingPatchProvider:
    """Adapt :class:`DeepSeekFlashClient` to the single-patch provider protocol."""

    name = "llm"

    def __init__(self, client: DeepSeekFlashClient) -> None:
        self.client = client
        self.calls: list[dict[str, Any]] = []

    def propose_patch(self, request: PatchRequest) -> PatchResponse:
        system = patch_system_prompt()
        user = patch_user_prompt(request.context, feedback=request.feedback,
                                 previous_candidate=request.previous_candidate)
        try:
            outcome = self.client.complete(system, user)
        except EmptyResponseError as exc:
            return PatchResponse(text="", source="llm", provider=ALLOWED_PROVIDER,
                                 model_id=ALLOWED_MODEL, error=str(exc))
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
        return PatchResponse(text=outcome.text, source="llm", provider=ALLOWED_PROVIDER,
                             model_id=outcome.response_model or ALLOWED_MODEL,
                             usage=outcome.usage, wall_ms=outcome.wall_ms)


def run_live_repair_pilot(
    tasks_path: Path | str,
    *,
    out_dir: Path | str,
    binary: Path | str,
    api_key: str,
    timeout: float = DEFAULT_TIMEOUT,
    max_tokens: int = DEFAULT_MAX_TOKENS,
    max_rounds: int = 3,
    sdk_client: Any | None = None,
) -> dict[str, Any]:
    """Run the frozen single-patch repair tasks against DeepSeek Flash."""
    tasks_path = Path(tasks_path).expanduser().resolve()
    spec = json.loads(tasks_path.read_text(encoding="utf-8"))
    tasks = spec.get("tasks", [])
    out_dir = Path(out_dir).expanduser().resolve()
    batch_dir = _exclusive_run_dir(out_dir)
    budget = LiveBudget(
        batch_dir / "budget.json",
        max_requests=int(spec.get("max_requests", 6)),
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
        "max_rounds": max_rounds,
        "max_requests": budget.max_requests,
        "max_seconds": budget.max_seconds,
        "deadline_epoch": budget.deadline_epoch,
        "tasks_path": str(tasks_path),
        "tasks": [],
        "stop_reason": None,
    }
    from .concir_client import sha256_file as _sha

    stop_reason = None
    for task in tasks:
        reason = budget.exhausted()
        if reason:
            summary["tasks"].append({"id": task.get("id"), "skipped": True, "stop_reason": reason})
            stop_reason = reason
            break

        task_id = task["id"]
        task_dir = batch_dir / task_id
        model = Path(task["model"]).expanduser()
        contract = Path(task["contract"]).expanduser()
        identity_ok = (_sha(model) == task.get("model_sha256")
                       and _sha(contract) == task.get("contract_sha256"))
        if not identity_ok:
            stop_reason = f"{task_id}: frozen model/contract hash mismatch"
            summary["tasks"].append({"id": task_id, "identity_ok": False, "stop_reason": stop_reason})
            summary["stop_reason"] = stop_reason
            break

        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        # Reconfirm the frozen root on the current backend before any patch.
        root = client.explore(model, contract, "petri")
        llm = DeepSeekFlashClient(
            api_key=api_key, budget=budget, evidence_dir=batch_dir / "llm",
            timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client,
        )
        provider = RecordingPatchProvider(llm)
        workflow = ExternalPatchRepairWorkflow(client, provider, out_dir=task_dir,
                                               max_rounds=max_rounds)
        result = None
        stop_reason = None
        try:
            result = workflow.run(model, contract)
        except Exception as exc:
            stop_reason = f"{type(exc).__name__}: {exc}"

        record = {
            "id": task_id,
            "identity_ok": True,
            "root": {"status": root.status, "outcome": root.outcome, "complete": root.complete},
            "calls": provider.calls,
            "requests_used_total": budget.requests_used,
            "status": None if result is None else result.status,
            "candidate_source": None if result is None else result.candidate_source,
            "validator": None if result is None else result.validator,
            "repair_mode": None if result is None else result.repair_mode,
            "context_fingerprint": None if result is None else result.context_fingerprint,
            "rounds": None if result is None else [r.__dict__ for r in result.rounds],
            "accepted_artifact_path": None if result is None else result.accepted_artifact_path,
            "replay": None if result is None else result.replay,
            "error": None if result is None else result.error,
            "run_dir": None if result is None else result.out_dir,
            "stop_reason": stop_reason,
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


class RecordingLocalProvider:
    """A3_local provider: current program + diagnostics in, function-map out."""

    name = "llm"

    def __init__(self, client: "DeepSeekFlashClient", base_program: str) -> None:
        self.client = client
        self.base_program = base_program
        self.calls: list[dict[str, Any]] = []

    def propose(self, request):
        from .prompts import local_revision_system_prompt
        from .providers import CandidateResponse

        system = local_revision_system_prompt()
        program = request.current_program or self.base_program
        user = (f"Current program:\n```json\n{program}\n```\n\n"
                f"Verifier diagnostics:\n{request.feedback}\n\n"
                "Return only the JSON object {\"functions\": {...}, "
                "\"new_resources\": [...], \"removed_resources\": [...]}.")
        outcome = self.client.complete(system, user)
        self.calls.append({
            "attempt": request.attempt, "request_id": outcome.request_id,
            "response_model": outcome.response_model,
            "finish_reason": outcome.finish_reason, "usage": outcome.usage,
            "transport_attempt": outcome.transport_attempt,
            "prompt_sha256": outcome.prompt_sha256, "wall_ms": outcome.wall_ms,
            "had_feedback": request.feedback is not None,
        })
        response = CandidateResponse(
            text=outcome.text, source="llm", provider=ALLOWED_PROVIDER,
            model_id=outcome.response_model or ALLOWED_MODEL, usage=outcome.usage)
        response.wall_ms = outcome.wall_ms
        return response
