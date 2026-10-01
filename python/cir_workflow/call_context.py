"""Identity and usage shared by one logical call and its transport attempts."""

from __future__ import annotations

from typing import Any

from .audit import parse_usage


def bind_context(client: Any, ctx: dict[str, Any]) -> None:
    try:
        client.call_context = dict(ctx)
        client.last_transport_log = []
    except Exception:
        return


def _context_fields(client: Any) -> dict[str, Any]:
    ctx = getattr(client, "call_context", None) or {}
    return {key: ctx.get(key) for key in (
        "run_id", "cell_id", "arm", "candidate_round", "attempt_id", "model")
        if ctx.get(key) is not None}


def normalize_usage(raw: dict[str, Any] | None) -> dict[str, Any]:
    parsed = parse_usage(raw if isinstance(raw, dict) else None)
    return {
        "input_tokens": parsed.input_tokens,
        "output_tokens": parsed.output_tokens,
        "cache_read_tokens": parsed.cache_read_tokens,
        "cache_write_tokens": parsed.cache_write_tokens,
        "reasoning_tokens": parsed.reasoning_tokens,
        "total_tokens": parsed.total_tokens,
    }


def note_transport(client: Any, *, index: int, status: str,
                   usage_raw: dict[str, Any] | None = None,
                   error_type: str | None = None, error: str | None = None,
                   request_id: str | None = None) -> dict[str, Any]:
    raw = usage_raw if isinstance(usage_raw, dict) else None
    entry = {
        **_context_fields(client),
        "transport_index": index,
        "status": status,
        "usage_raw": raw,
        "usage": normalize_usage(raw),
        "error_type": error_type,
        "error": error,
        "request_id": request_id,
    }
    log = getattr(client, "last_transport_log", None)
    if log is None:
        client.last_transport_log = log = []
    log.append(entry)
    return entry
