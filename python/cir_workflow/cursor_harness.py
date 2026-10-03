"""Cursor agent sessions that always send the system rules they claim to send.

CIR rounds may share one agent. Each code-stage call opens a new agent and
sends the full system prompt plus the user message. History is not required
for the next repair round.
"""

from __future__ import annotations

import hashlib
import threading
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any

# One native send. Tools are disabled, so this bounds the agent step loop.
# Cancel is the backstop if a run stays open past the wall clock.
MAX_RUN_S = 240


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
            api_key=self.api_key, model=self.model, mode="agent", tools=[],
            local=self.Local(
                cwd=str(self.inbox), setting_sources=[],
                sandbox_options={"enabled": True}),
        ))

    def _bounded_send(self, agent, message: str) -> tuple[Any, list[str] | None]:
        box: dict[str, Any] = {}

        def target() -> None:
            try:
                run = agent.send(message)
                box["run"] = run
                if hasattr(run, "messages"):
                    steps: list[str] = []
                    for item in run.messages():
                        kind = getattr(item, "type", None)
                        name = getattr(item, "name", None)
                        if kind in {"tool_call", "tool_use"} or name:
                            steps.append(str(name or kind))
                    box["tool_steps"] = steps
                else:
                    box["tool_steps"] = None
                box["result"] = run.wait() if hasattr(run, "wait") else run
            except Exception as exc:  # noqa: BLE001 - surfaced to the caller
                box["error"] = exc

        thread = threading.Thread(target=target, daemon=True)
        thread.start()
        thread.join(MAX_RUN_S)
        if thread.is_alive():
            run = box.get("run")
            cancel = getattr(run, "cancel", None) if run is not None else None
            if cancel:
                try:
                    cancel()
                except Exception as exc:  # noqa: BLE001
                    box["cancel_error"] = exc
            thread.join(20)
            if "result" not in box:
                raise TimeoutError(
                    f"cursor run exceeded {MAX_RUN_S}s and did not return a result")
        if "error" in box and "result" not in box:
            raise box["error"]
        return box["result"], box.get("tool_steps")

    def complete(self, system: str, user: str):
        reason = self.budget.exhausted()
        if reason:
            from .live import BudgetExhausted
            raise BudgetExhausted(reason)
        self.budget.reserve()
        message = full_message(system, user)
        started = time.time()
        self.close()
        agent = self._create()
        try:
            result, tool_steps = self._bounded_send(agent, message)
        finally:
            closer = getattr(agent, "close", None)
            if closer:
                closer()
            self.agent = None
        returned = None
        if getattr(result, "model", None) is not None:
            returned = getattr(result.model, "id", None)
        usage = None
        usage_reason = None
        if getattr(result, "usage", None) is not None:
            usage = {
                "input_tokens": result.usage.input_tokens,
                "output_tokens": result.usage.output_tokens,
                "cache_read_tokens": getattr(result.usage, "cache_read_tokens", None),
                "cache_write_tokens": getattr(result.usage, "cache_write_tokens", None),
                "reasoning_tokens": getattr(result.usage, "reasoning_tokens", None),
                "total_tokens": getattr(result.usage, "total_tokens", None),
            }
        else:
            usage_reason = "provider did not report usage"
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
            tool_steps=tool_steps,
            hidden_calls_observable=False,
            usage_reason=usage_reason,
            external_dispatches=1,
            visible_model_attempts=1,
            tools_disabled=True,
            max_run_s=MAX_RUN_S,
            agent_mode="agent",
            finish_reason=getattr(result, "status", None),
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
            "hidden_calls_observable": False,
            "tool_steps": tool_steps,
            "usage_reason": usage_reason,
            "stage": self.stage,
            "message_sha256": digest,
        })
        return outcome
