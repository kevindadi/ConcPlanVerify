"""Cursor agent sessions that always send the system rules they claim to send.

CIR rounds may share one agent. Each code-stage call opens a new agent and
sends the full system prompt plus the user message. History is not required
for the next repair round.
"""

from __future__ import annotations

import hashlib
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any


def full_message(system: str, user: str) -> str:
    return system.strip() + "\n\n" + user.strip()


def redact(text: str) -> str:
    """Drop home-directory paths. Never store credentials; callers must not pass them."""
    home = str(Path.home())
    return text.replace(home, "<home>")


class StagedCursorClient:
    def __init__(self, model: str, api_key: str, budget: Any, inbox: Path, *,
                 agent_cls: Any | None = None, options_cls: Any | None = None,
                 local_cls: Any | None = None) -> None:
        self.model = model
        self.api_key = api_key
        self.budget = budget
        self.inbox = Path(inbox)
        self.stage = "cir"
        self.agent = None
        self.calls: list[dict] = []
        self.sent: list[dict] = []
        if agent_cls is None:
            from cursor_sdk import Agent, AgentOptions, LocalAgentOptions
            agent_cls, options_cls, local_cls = Agent, AgentOptions, LocalAgentOptions
        self.Agent = agent_cls
        self.Options = options_cls
        self.Local = local_cls

    def close(self) -> None:
        if self.agent is not None:
            closer = getattr(self.agent, "close", None)
            if closer:
                closer()
            self.agent = None

    def set_stage(self, stage: str) -> None:
        if stage != self.stage:
            self.close()
            self.stage = stage

    def _create(self):
        return self.Agent.create(self.Options(
            api_key=self.api_key, model=self.model, tools=[],
            local=self.Local(cwd=str(self.inbox)),
        ))

    def complete(self, system: str, user: str):
        reason = self.budget.exhausted()
        if reason:
            raise RuntimeError(reason)
        self.budget.reserve()
        message = full_message(system, user)
        started = time.time()
        if self.stage == "code":
            self.close()
            agent = self._create()
            try:
                result = agent.send(message).wait()
            finally:
                closer = getattr(agent, "close", None)
                if closer:
                    closer()
        else:
            if self.agent is None:
                self.agent = self._create()
            result = self.agent.send(message).wait()
        returned = None
        if getattr(result, "model", None) is not None:
            returned = getattr(result.model, "id", None)
        if returned is not None and returned != self.model:
            raise RuntimeError(f"model mismatch: requested {self.model}, got {returned}")
        usage = None
        if getattr(result, "usage", None) is not None:
            usage = {
                "prompt_tokens": result.usage.input_tokens,
                "completion_tokens": result.usage.output_tokens,
                "total_tokens": result.usage.total_tokens,
            }
        digest = hashlib.sha256(message.encode()).hexdigest()
        record = {
            "stage": self.stage,
            "message_sha256": digest,
            "contains_system": system.strip() in message,
            "message": redact(message),
        }
        self.sent.append(record)
        outcome = SimpleNamespace(
            text=getattr(result, "result", None) or "",
            request_id=getattr(result, "id", None),
            response_model=returned,
            usage=usage,
            wall_ms=int((time.time() - started) * 1000),
            prompt_sha256=digest,
            transport_attempt=1,
            sent=record,
        )
        self.calls.append({
            "request_id": outcome.request_id,
            "requested_model": self.model,
            "response_model": returned,
            "identity_confirmed": returned == self.model,
            "usage": usage,
            "usage_available": usage is not None,
            "harness_request": 1,
            "api_requests_visible": None,
            "stage": self.stage,
            "message_sha256": digest,
        })
        return outcome
