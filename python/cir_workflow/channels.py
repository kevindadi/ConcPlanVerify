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
from .transport import ModelSpec, TransportError, verify_identity


class ChannelUnavailable(TransportError):
    """The model's channel is blocked or has no key; the caller must skip it."""


def build_client(spec: ModelSpec, *, budget: Any, evidence_dir: Path | str,
                 api_key: str, timeout: float = 90.0, max_tokens: int = 4096):
    """Construct the inner client for a model, or raise ChannelUnavailable."""

    if spec.status != "available" or not spec.model_id:
        raise ChannelUnavailable(
            f"{spec.display_name} is {spec.status}: {spec.blocked_reason}")
    if spec.channel == "deepseek-direct":
        from .live import DeepSeekFlashClient
        return DeepSeekFlashClient(api_key=api_key, budget=budget,
                                   evidence_dir=evidence_dir, timeout=timeout,
                                   max_tokens=max_tokens)
    if spec.channel == "opencode-go":
        from .opencode_go import OpenCodeGoClient, OpenCodeGoResponsesClient
        cls = OpenCodeGoResponsesClient if spec.surface == "responses" else OpenCodeGoClient
        return cls(api_key=api_key, budget=budget,
                   evidence_dir=evidence_dir, model=spec.model_id,
                   timeout=timeout, max_tokens=max_tokens)
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

    def complete(self, system: str, user: str):
        self.round += 1
        prompt = system.strip() + "\n\n" + user.strip()
        started = time.time()
        try:
            outcome = self.inner.complete(system, user)
        except Exception as exc:  # noqa: BLE001 - recorded then re-raised
            self.audit.model_call(
                run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
                provider=self.spec.provider, transport=self.spec.channel,
                arm=self.arm, task_id=self.task_id, replicate=self.replicate,
                stage=self.stage, requested_model=self.spec.model_id or "",
                returned_model=None, usage_raw=None, started_at=started,
                ended_at=time.time(), prompt=prompt, response="",
                candidate_round=self.round, status="error",
                error_type=type(exc).__name__, error=str(exc))
            raise
        ended = time.time()
        returned = getattr(outcome, "response_model", None)
        confirmed = verify_identity(self.spec.model_id or "", returned)
        usage = getattr(outcome, "usage", None)
        self.audit.model_call(
            run_id=self.run_id, cell_id=self.cell_id, model=self.spec.display_name,
            provider=self.spec.provider, transport=self.spec.channel,
            arm=self.arm, task_id=self.task_id, replicate=self.replicate,
            stage=self.stage, requested_model=self.spec.model_id or "",
            returned_model=returned, usage_raw=usage, started_at=started,
            ended_at=ended, prompt=prompt, response=getattr(outcome, "text", ""),
            candidate_round=self.round, attempt_id=f"a{self.round}",
            request_id=getattr(outcome, "request_id", None),
            transport_attempt=getattr(outcome, "transport_attempt", 1),
            cost=getattr(outcome, "cost", None),
            notes=None if confirmed else "identity unconfirmed (model not reported)")
        return outcome


def key_for(spec: ModelSpec, env: dict[str, str], channel_api_key_env: str) -> str:
    api_key = env.get(channel_api_key_env, "")
    if not api_key:
        raise ChannelUnavailable(
            f"missing {channel_api_key_env} for {spec.display_name}")
    return api_key
