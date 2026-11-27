"""CLI entry point for the ConcPlanVerify MCP stdio server (``python -m concplan.mcp`` / ``concplan-mcp``)."""

from __future__ import annotations

from .server import build_parser, main

__all__ = ["build_parser", "main"]

if __name__ == "__main__":
    raise SystemExit(main())
