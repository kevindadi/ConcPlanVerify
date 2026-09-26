"""Unified per-call audit log for multi-model experiments.

Every model call produces exactly one :class:`RequestEvent` in a JSONL stream
plus the raw prompt/response payloads on disk (referenced by sha256). The log is
designed so that a reader can separate:

- candidate rounds (CIR/code proposals) from harness calls, from SDK/agent
  internal calls, from network retries, from non-model tool steps;
- server-reported usage from local estimates (never summed together);
- incremental from cumulative usage (so SDK totals are not double counted);
- observed tokens from unknown tokens (missing usage is ``null``, never ``0``).

The event does not itself decide experiment semantics; it records what
happened so the cell-level summary can.
"""

from __future__ import annotations

import hashlib
import json
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@dataclass
class UsageBreakdown:
    """Token usage for one model call.

    ``reasoning_tokens`` and ``cache_*`` are sub-items of the totals as defined
    by each provider; they are reported but never added again to
    ``total_tokens``. Any field the provider did not report is ``None``.
    """

    input_tokens: int | None = None
    output_tokens: int | None = None
    reasoning_tokens: int | None = None
    cache_read_tokens: int | None = None
    cache_write_tokens: int | None = None
    total_tokens: int | None = None
    source: str = "unknown"          # server | local-estimate | unknown
    complete: bool = False
    incremental: bool = True         # per-call increment, not a running total
    raw: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def _int_or_none(value: Any) -> int | None:
    try:
        if value is None:
            return None
        return int(value)
    except (TypeError, ValueError):
        return None


def _nested(raw: dict[str, Any], *paths: tuple[str, ...]) -> Any:
    for path in paths:
        cur: Any = raw
        ok = True
        for key in path:
            if isinstance(cur, dict) and key in cur:
                cur = cur[key]
            else:
                ok = False
                break
        if ok and cur is not None:
            return cur
    return None


def parse_usage(raw: dict[str, Any] | None, *, source: str = "server",
                incremental: bool = True) -> UsageBreakdown:
    """Map a provider ``usage`` payload onto :class:`UsageBreakdown`.

    Recognises Chat-Completions and Responses field names. Missing fields stay
    ``None``; ``complete`` is true only when both input and output are known.
    """

    if not isinstance(raw, dict) or not raw:
        return UsageBreakdown(source="unknown", complete=False, raw=raw or {})

    input_tokens = _int_or_none(_nested(raw, ("input_tokens",), ("prompt_tokens",)))
    output_tokens = _int_or_none(_nested(raw, ("output_tokens",), ("completion_tokens",)))
    total_tokens = _int_or_none(_nested(raw, ("total_tokens",)))
    reasoning = _int_or_none(_nested(
        raw, ("reasoning_tokens",),
        ("completion_tokens_details", "reasoning_tokens"),
        ("output_tokens_details", "reasoning_tokens")))
    cache_read = _int_or_none(_nested(
        raw, ("cache_read_input_tokens",),
        ("prompt_tokens_details", "cached_tokens"),
        ("input_tokens_details", "cached_tokens")))
    cache_write = _int_or_none(_nested(
        raw, ("cache_creation_input_tokens",),
        ("prompt_tokens_details", "cache_creation_tokens"),
        ("input_tokens_details", "cache_write_tokens")))
    complete = input_tokens is not None and output_tokens is not None
    return UsageBreakdown(
        input_tokens=input_tokens, output_tokens=output_tokens,
        reasoning_tokens=reasoning, cache_read_tokens=cache_read,
        cache_write_tokens=cache_write, total_tokens=total_tokens,
        source=source, complete=complete, incremental=incremental, raw=raw)


def estimate_tokens(text: str) -> int:
    """A crude local estimate (never mixed with server usage)."""

    return max(1, len(text) // 4)


@dataclass
class RequestEvent:
    run_id: str
    cell_id: str
    model: str
    provider: str
    transport: str
    arm: str
    task_id: str
    replicate: int
    stage: str                       # cir | code | repair | connectivity
    candidate_round: int | None
    attempt_id: str
    request_id: str | None
    parent_id: str | None
    kind: str                        # model-call | tool-step | harness-call
    status: str                      # ok | error | blocked
    requested_model: str
    returned_model: str | None
    identity_confirmed: bool
    started_at: float
    ended_at: float
    latency_ms: int
    usage: UsageBreakdown
    transport_attempt: int = 1
    sdk_calls_visible: int | None = None   # None = not observable
    cost: float | None = None
    cost_basis: str | None = None
    prompt_sha256: str | None = None
    prompt_path: str | None = None
    response_sha256: str | None = None
    response_path: str | None = None
    error_type: str | None = None
    error: str | None = None
    notes: str | None = None

    def to_dict(self) -> dict[str, Any]:
        data = asdict(self)
        data["usage"] = self.usage.to_dict()
        return data


class AuditLog:
    """Append-only JSONL audit log with raw payload side files."""

    def __init__(self, path: Path | str, raw_dir: Path | str | None = None) -> None:
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.raw_dir = Path(raw_dir) if raw_dir else self.path.parent / "raw"
        self.raw_dir.mkdir(parents=True, exist_ok=True)
        self._seq = 0

    def _write_raw(self, prefix: str, text: str) -> tuple[str, str]:
        self._seq += 1
        digest = sha256_text(text)
        name = f"{prefix}-{self._seq:04d}-{digest[:12]}.txt"
        target = self.raw_dir / name
        target.write_text(text, encoding="utf-8")
        return digest, str(target)

    def record(self, event: RequestEvent, *, prompt: str | None = None,
               response: str | None = None) -> RequestEvent:
        if prompt is not None:
            digest, path = self._write_raw("prompt", prompt)
            event.prompt_sha256 = digest
            event.prompt_path = path
        if response is not None:
            digest, path = self._write_raw("response", response)
            event.response_sha256 = digest
            event.response_path = path
        with self.path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(event.to_dict(), ensure_ascii=False,
                                    sort_keys=True) + "\n")
        return event

    def model_call(self, *, run_id: str, cell_id: str, model: str, provider: str,
                   transport: str, arm: str, task_id: str, replicate: int,
                   stage: str, requested_model: str, returned_model: str | None,
                   usage_raw: dict[str, Any] | None, started_at: float,
                   ended_at: float, prompt: str, response: str,
                   candidate_round: int | None = None,
                   attempt_id: str = "a0", request_id: str | None = None,
                   parent_id: str | None = None, transport_attempt: int = 1,
                   sdk_calls_visible: int | None = None,
                   cost: float | None = None, cost_basis: str | None = None,
                   status: str = "ok", error_type: str | None = None,
                   error: str | None = None, notes: str | None = None) -> RequestEvent:
        usage = parse_usage(usage_raw)
        confirmed = returned_model is not None and returned_model == requested_model
        event = RequestEvent(
            run_id=run_id, cell_id=cell_id, model=model, provider=provider,
            transport=transport, arm=arm, task_id=task_id, replicate=replicate,
            stage=stage, candidate_round=candidate_round, attempt_id=attempt_id,
            request_id=request_id, parent_id=parent_id, kind="model-call",
            status=status, requested_model=requested_model,
            returned_model=returned_model, identity_confirmed=confirmed,
            started_at=started_at, ended_at=ended_at,
            latency_ms=int((ended_at - started_at) * 1000), usage=usage,
            transport_attempt=transport_attempt, sdk_calls_visible=sdk_calls_visible,
            cost=cost, cost_basis=cost_basis, error_type=error_type, error=error,
            notes=notes)
        return self.record(event, prompt=prompt, response=response)

    def tool_step(self, *, run_id: str, cell_id: str, arm: str, task_id: str,
                  replicate: int, stage: str, tool: str, started_at: float,
                  ended_at: float, result: str, notes: str | None = None,
                  parent_id: str | None = None) -> RequestEvent:
        """A non-model tool step: time and result only, no fabricated tokens."""

        event = RequestEvent(
            run_id=run_id, cell_id=cell_id, model="(none)", provider=tool,
            transport="local", arm=arm, task_id=task_id, replicate=replicate,
            stage=stage, candidate_round=None, attempt_id="tool",
            request_id=None, parent_id=parent_id, kind="tool-step",
            status="ok", requested_model="(none)", returned_model=None,
            identity_confirmed=False, started_at=started_at, ended_at=ended_at,
            latency_ms=int((ended_at - started_at) * 1000),
            usage=UsageBreakdown(source="unknown", complete=False),
            notes=notes, response_sha256=sha256_text(result))
        return self.record(event)


def read_events(path: Path | str) -> list[dict[str, Any]]:
    events = []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            events.append(json.loads(line))
    return events
