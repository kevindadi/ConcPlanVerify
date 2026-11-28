"""Unified v5 command-line entry point for ConcPlanVerify (``python -m concplan``)."""

from .cli import build_parser, main

__all__ = ["build_parser", "main"]

if __name__ == "__main__":
    raise SystemExit(main())
