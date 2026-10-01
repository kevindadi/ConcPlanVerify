"""Client factory and audit wrapper that tie a model to its transport.

``build_client`` constructs the correct inner client for a :class:`ModelSpec`
(DeepSeek direct, OpenCode Go, or a blocked/unavailable channel error).
``AuditedClient`` wraps any inner client exposing ``complete(system, user)`` and
records one audit event per real call, enforcing model identity so a response
is never attributed to the wrong model.
"""

from __future__ import annotations

import tempfile
import threading
import time
from pathlib import Path
from typing import Any, Callable

from .audit import AuditLog
from .call_context import bind_context
from .live import BudgetExhausted
from .transport import (
    CHANNELS, ModelIdentityError, ModelSpec, TransportError, server_model_id, verify_identity,
)


_CHANNEL_TIMEOUT = {"dashscope-direct": 300.0, "cursor": 300.0,
                    "deepseek-direct": 180.0, "opencode-go": 180.0}
_cursor_listed: list[str] | None = None
_cursor_list_lock = threading.Lock()


class ChannelUnavailable(TransportError):
    """The model's channel is blocked or has no key; the caller must skip it."""


def _scrub(text: str, secret: str) -> str:
    return text.replace(secret, "<redacted>") if secret else text


def _require_cursor_model(api_key: str, model_id: str) -> None:
    """Confirm the native id is in this account's model list. Do not substitute."""

    global _cursor_listed
    with _cursor_list_lock:
        if _cursor_listed is None:
            try:
                from cursor_sdk import Cursor
                listed = Cursor.models.list(api_key=api_key)
            except Exception as exc:  # noqa: BLE001 - the channel is blocked, not swapped
                raise ChannelUnavailable(
                    "Cursor.models.list failed: "
                    f"{type(exc).__name__}: {_scrub(str(exc), api_key)}") from exc
            ids = []
            for item in listed:
                ident = getattr(item, "id", None)
                if isinstance(ident, str):
                    ids.append(ident)
            _cursor_listed = ids
        ids = list(_cursor_listed)
    if model_id not in ids:
        raise ChannelUnavailable(
            f"Cursor.models.list did not include {model_id}; "
            "Cursor Agent is identity_unconfirmed")


def build_client(spec: ModelSpec, *, budget: Any, evidence_dir: Path | str,
                 api_key: str, timeout: float = 90.0, max_tokens: int = 4096):
    """Construct the inner client for a model, or raise ChannelUnavailable."""

    if spec.status != "available" or not spec.model_id:
        raise ChannelUnavailable(
            f"{spec.display_name} is {spec.status}: {spec.blocked_reason}")
    # Direct reasoning models can exceed a chat timeout; give them more room.
    timeout = max(timeout, _CHANNEL_TIMEOUT.get(spec.channel, timeout))
    if spec.channel == "deepseek-direct":
        from .live import DeepSeekFlashClient
        return DeepSeekFlashClient(api_key=api_key, budget=budget,
                                   evidence_dir=evidence_dir, timeout=timeout,
                                   max_tokens=max_tokens)
    if spec.channel == "dashscope-direct":
        from .direct import DirectChatClient
        return DirectChatClient(api_key=api_key,
                                base_url=CHANNELS[spec.channel].base_url or "",
                                model=spec.model_id, budget=budget,
                                evidence_dir=evidence_dir, timeout=timeout,
                                max_tokens=max_tokens,
                                extra_body={"enable_thinking": False})
    if spec.channel == "opencode-go":
        from .opencode_go import OpenCodeGoClient, OpenCodeGoResponsesClient
        cls = OpenCodeGoResponsesClient if spec.surface == "responses" else OpenCodeGoClient
        return cls(api_key=api_key, budget=budget,
                   evidence_dir=evidence_dir, model=spec.model_id,
                   timeout=timeout, max_tokens=max_tokens)
    if spec.channel == "cursor":
        from .cursor_harness import StagedCursorClient
        _require_cursor_model(api_key, server_model_id(spec))
        inbox = Path(tempfile.mkdtemp(prefix="cpv-cursor-"))
        return StagedCursorClient(server_model_id(spec), api_key, budget, inbox)
    raise ChannelUnavailable(f"no client for channel {spec.channel!r}")


class AuditedClient:
    """Wrap an inner client and emit one :class:`RequestEvent` per call."""

    def __init__(self, inner: Any, *, audit: AuditLog, run_id: str, cell_id: str,
                 spec: ModelSpec, arm: str, task_id: str, replicate: int,
                 stage: str = "cir") -> None:
        self.inner = inner
        self.audit = audit
        self.run_id = run_id
        self.cell_id = cell_id
        self.spec = spec
        self.arm = arm
        self.task_id = task_id
        self.replicate = replicate
        self.stage = stage
        self.round = 0

    def set_stage(self, stage: str) -> None:
        """Match the inner clients' stage-switch hook (cir vs code)."""

        self.stage = stage
        self.round = 0

    def complete(self, system: str, user: str, *,
                 candidate_round: int | None = None, attempt: int | None = None):
        if candidate_round is None:
            self.round += 1
            candidate_round = self.round
        else:
            self.round = int(candidate_round)
        attempt_n = int(attempt if attempt is not None else candidate_round)
        attempt_id = f"{self.cell_id}-r{candidate_round}-a{attempt_n}"
        bind_context(self.inner, {
            "run_id": self.run_id, "cell_id": self.cell_id, "arm": self.arm,
            "model": self.spec.model_id, "candidate_round": candidate_round,
            "attempt_id": attempt_id,
        })
        prompt = system.strip() + "\n\n" + user.strip()
        started = time.time()

        def _physical(exc: BaseException | None = None) -> tuple[int, list]:
            log = list(getattr(self.inner, "last_transport_log", None) or [])
            if isinstance(exc, BudgetExhausted):
                return int(getattr(exc, "physical_attempts", len(log)) or 0), log
            if log:
                return len(log), log
            return (0, log) if exc is not None else (1, log)

        try:
            outcome = self.inner.complete(system, user)
        except Exception as exc:  # noqa: BLE001 - recorded then re-raised
            physical, log = _physical(exc)
            self.last_physical_attempts = physical
            self.last_transport_log = log
            self._audit_attempts(
                log, prompt=prompt, response="", requested=server_model_id(self.spec),
                returned=None, usage=None, started=started, ended=time.time(),
                candidate_round=candidate_round, attempt_id=attempt_id,
                status="error", error_type=type(exc).__name__, error=str(exc),
                request_id=None, cost=None,
                notes=f"channel={self.spec.channel}")
            raise
        ended = time.time()
        returned = getattr(outcome, "response_model", None)
        requested = server_model_id(self.spec)
        physical, log = _physical(None)
        if getattr(outcome, "transport_attempt", None):
            physical = int(outcome.transport_attempt)
        self.last_physical_attempts = physical
        self.last_transport_log = log
        try:
            confirmed = verify_identity(requested, returned)
        except ModelIdentityError as exc:
            self.audit.model_call(
                run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
                provider=self.spec.provider, transport=self.spec.channel,
                arm=self.arm, task_id=self.task_id, replicate=self.replicate,
                stage=self.stage, requested_model=requested, returned_model=returned,
                usage_raw=getattr(outcome, "usage", None), started_at=started,
                ended_at=ended, prompt=prompt, response=getattr(outcome, "text", ""),
                candidate_round=candidate_round, attempt_id=attempt_id,
                request_id=getattr(outcome, "request_id", None),
                transport_attempt=physical, transport_log=log,
                cost=getattr(outcome, "cost", None), status="identity_mismatch",
                error_type=type(exc).__name__, error=str(exc),
                notes=f"channel={self.spec.channel}; identity rejected")
            exc.outcome = outcome
            raise
        usage = getattr(outcome, "usage", None)
        self._audit_attempts(
            log, prompt=prompt, response=getattr(outcome, "text", ""),
            requested=requested, returned=returned, usage=usage, started=started,
            ended=ended, candidate_round=candidate_round, attempt_id=attempt_id,
            status="ok",
            error_type=None, error=None,
            request_id=getattr(outcome, "request_id", None),
            cost=getattr(outcome, "cost", None),
            notes=(f"channel={self.spec.channel}" if confirmed
                   else f"channel={self.spec.channel}; identity unconfirmed (model not reported)"))
        return outcome

    def _audit_attempts(self, log: list, *, prompt: str, response: str, requested: str,
                        returned: str | None, usage: dict | None, started: float,
                        ended: float, candidate_round: int, attempt_id: str,
                        status: str, error_type: str | None, error: str | None,
                        request_id: str | None, cost: float | None, notes: str) -> None:
        """One audit event per transport attempt. A single call stays one event.

        A retry keeps the candidate attempt id on the final attempt so recovery
        still matches it. Earlier attempts get their own ids and name the first
        attempt as parent_id.
        """

        rows = list(log or [])
        if len(rows) <= 1:
            self.audit.model_call(
                run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
                provider=self.spec.provider, transport=self.spec.channel,
                arm=self.arm, task_id=self.task_id, replicate=self.replicate,
                stage=self.stage, requested_model=requested, returned_model=returned,
                usage_raw=usage, started_at=started, ended_at=ended, prompt=prompt,
                response=response, candidate_round=candidate_round, attempt_id=attempt_id,
                request_id=request_id, transport_attempt=int(rows[0].get("index", 1)) if rows else 1,
                transport_log=rows, cost=cost, status=status, error_type=error_type,
                error=error, notes=notes)
            return
        for index, item in enumerate(rows, start=1):
            last = index == len(rows)
            this_id = attempt_id if last else f"{attempt_id}-t{index}"
            parent = None if index == 1 else f"{attempt_id}-t1"
            item_status = status if last else str(item.get("status") or "error")
            self.audit.model_call(
                run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
                provider=self.spec.provider, transport=self.spec.channel,
                arm=self.arm, task_id=self.task_id, replicate=self.replicate,
                stage=self.stage, requested_model=requested,
                returned_model=returned if last else None,
                usage_raw=usage if last else None, started_at=started, ended_at=ended,
                prompt=prompt, response=response if last else "",
                candidate_round=candidate_round, attempt_id=this_id, parent_id=parent,
                request_id=request_id if last else None, transport_attempt=index,
                transport_log=[item], cost=cost if last else None, status=item_status,
                error_type=None if last else item.get("error_type"),
                error=None if last else item.get("error"), notes=notes)


def key_for(spec: ModelSpec, env: dict[str, str], channel_api_key_env: str) -> str:
    api_key = env.get(channel_api_key_env, "")
    if not api_key:
        raise ChannelUnavailable(
            f"missing {channel_api_key_env} for {spec.display_name}")
    return api_key
