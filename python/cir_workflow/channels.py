"""Client factory and audit wrapper that tie a model to its transport.

``build_client`` constructs the correct inner client for a :class:`ModelSpec`
(DeepSeek direct, OpenCode Go, or a blocked/unavailable channel error).
``AuditedClient`` wraps any inner client exposing ``complete(system, user)`` and
records one audit event per real call, enforcing model identity so a response
is never attributed to the wrong model.
"""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any, Callable

from .audit import AuditLog
from .call_context import bind_context
from .live import BudgetExhausted
from .transport import (
    CHANNELS, ModelIdentityError, ModelSpec, TransportError, verify_identity,
)


_CHANNEL_TIMEOUT = {"dashscope-direct": 300.0, "cursor": 300.0,
                    "deepseek-direct": 180.0, "opencode-go": 180.0}


class ChannelUnavailable(TransportError):
    """The model's channel is blocked or has no key; the caller must skip it."""


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
        inbox = Path(evidence_dir) / "cursor-inbox"
        inbox.mkdir(parents=True, exist_ok=True)
        return StagedCursorClient(spec.model_id or "", api_key, budget, inbox)
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
        attempt_id = f"a{attempt_n}"
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
            self.audit.model_call(
                run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
                provider=self.spec.provider, transport=self.spec.channel,
                arm=self.arm, task_id=self.task_id, replicate=self.replicate,
                stage=self.stage, requested_model=self.spec.model_id or "",
                returned_model=None, usage_raw=None, started_at=started,
                ended_at=time.time(), prompt=prompt, response="",
                candidate_round=candidate_round, attempt_id=attempt_id,
                transport_attempt=physical, transport_log=log, status="error",
                error_type=type(exc).__name__, error=str(exc))
            raise
        ended = time.time()
        returned = getattr(outcome, "response_model", None)
        requested = self.spec.model_id or ""
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
        self.audit.model_call(
            run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
            provider=self.spec.provider, transport=self.spec.channel,
            arm=self.arm, task_id=self.task_id, replicate=self.replicate,
            stage=self.stage, requested_model=requested,
            returned_model=returned, usage_raw=usage, started_at=started,
            ended_at=ended, prompt=prompt, response=getattr(outcome, "text", ""),
            candidate_round=candidate_round, attempt_id=attempt_id,
            request_id=getattr(outcome, "request_id", None),
            transport_attempt=physical, transport_log=log,
            cost=getattr(outcome, "cost", None),
            notes=(f"channel={self.spec.channel}" if confirmed
                   else f"channel={self.spec.channel}; identity unconfirmed (model not reported)"))
        return outcome


def key_for(spec: ModelSpec, env: dict[str, str], channel_api_key_env: str) -> str:
    api_key = env.get(channel_api_key_env, "")
    if not api_key:
        raise ChannelUnavailable(
            f"missing {channel_api_key_env} for {spec.display_name}")
    return api_key
