"""One cargo-project template shared by every generation arm.

The template links the `concir_sync` counting-semaphore crate by path so that a
program written from the same prompt instruction ("the crate is already
linked") compiles on all arms, baseline included.
"""

from __future__ import annotations

from pathlib import Path

CONCIR_SYNC_CRATE = Path(__file__).resolve().parents[2] / "runtime/concir_sync"


def cargo_toml(name: str, *, bin_path: str = "src/main.rs") -> str:
    return f"""[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{name}"
path = "{bin_path}"

[dependencies]
concir_sync = {{ path = "{CONCIR_SYNC_CRATE}" }}
"""
