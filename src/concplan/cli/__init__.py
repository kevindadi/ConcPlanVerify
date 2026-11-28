"""Modular CLI subpackage for ``concplan``."""

from .commands import main
from .parser import build_parser, default_binary

__all__ = ["build_parser", "default_binary", "main"]
