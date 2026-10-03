"""A hard wall-clock bound for one model call.

The OpenAI SDK's ``read`` timeout only bounds a single socket read; a stalled
connection or a server that trickles bytes can exceed it. This wrapper runs the
call on a daemon thread and, at the deadline, *closes the inner client's HTTP
connection* so the blocked read actually unblocks, then raises. It does not wait
indefinitely on the worker thread; the abort plus a short grace is the bound.

A timeout is recorded as an unknown outcome and consumes the reservation (no
refund): the reservation is made inside the inner client before it blocks.
"""

from __future__ import annotations

import threading
from typing import Any


class HardTimeoutError(RuntimeError):
    def __init__(self, seconds: float) -> None:
        super().__init__(f"hard wall-clock timeout after {seconds:g}s")
        self.seconds = seconds


class HardTimeoutClient:
    def __init__(self, inner: Any, seconds: float, *, grace: float = 2.0) -> None:
        self.inner = inner
        self.seconds = float(seconds)
        self.grace = float(grace)
        self.stage = getattr(inner, "stage", "code")
        self.call_context: dict[str, Any] = {}

    # passthroughs -----------------------------------------------------------
    def set_stage(self, stage: str) -> None:
        self.stage = stage
        if hasattr(self.inner, "set_stage"):
            self.inner.set_stage(stage)

    def protocol_record(self) -> dict[str, Any]:
        record = getattr(self.inner, "protocol_record", None)
        base = record() if callable(record) else {}
        base = dict(base) if isinstance(base, dict) else {}
        base["hard_timeout_seconds"] = self.seconds
        return base

    @property
    def budget(self):
        return getattr(self.inner, "budget", None)

    @property
    def last_transport_log(self):
        return getattr(self.inner, "last_transport_log", [])

    def __getattr__(self, name: str) -> Any:
        # Expose the inner client's send parameters (temperature, max_tokens,
        # thinking, model, extra_body) so the fingerprint sees them.
        return getattr(self.inner, name)

    def close(self) -> None:
        close = getattr(self.inner, "close", None)
        if callable(close):
            try:
                close()
            except Exception:  # noqa: BLE001 - abort is best effort
                pass

    def _abort(self) -> None:
        """Close the HTTP client so a blocked read raises instead of hanging."""
        self.close()

    def complete(self, system: str, user: str):
        ctx = getattr(self, "call_context", None)
        if ctx:
            from .call_context import bind_context
            bind_context(self.inner, ctx)
        result: dict[str, Any] = {}
        done = threading.Event()

        def _worker() -> None:
            try:
                result["outcome"] = self.inner.complete(system, user)
            except BaseException as exc:  # noqa: BLE001 - re-raised on the caller
                result["error"] = exc
            finally:
                done.set()

        thread = threading.Thread(target=_worker, daemon=True,
                                  name="v23-hard-timeout")
        thread.start()
        if not done.wait(self.seconds):
            self._abort()
            done.wait(self.grace)  # give the aborted socket a moment to unblock
            raise HardTimeoutError(self.seconds)
        if "error" in result:
            raise result["error"]
        return result["outcome"]
