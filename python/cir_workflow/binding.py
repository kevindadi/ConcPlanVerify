"""Single binding-check entry (strong-link-v4).

Both the online pipeline and the offline re-execution call this; it invokes the
ConcIR ``bind_check`` CLI. There is no silent fallback to a weaker name mapping:
if the CLI is missing, exits non-zero, or returns unparseable output, a
:class:`BindingUnavailable` is raised so the caller records a tool error.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


class BindingUnavailable(RuntimeError):
    """The binding-check CLI could not be run or its output is unusable."""


def default_binary() -> Path:
    import os
    explicit = os.environ.get("CONCIR_BIND_CHECK")
    if explicit:
        return Path(explicit).expanduser().resolve()
    toolchain = os.environ.get("CONCIR_TOOLCHAIN")
    if toolchain:
        return (Path(toolchain).expanduser().resolve() / "bind_check")
    return REPO / "ConcIR/target/release/bind_check"


def bind(resources_path: Path | str, cir_path: Path | str, *,
         manifest_path: Path | str | None = None, binary: Path | str | None = None
         ) -> dict:
    """Run the Rust binding check; raise BindingUnavailable on any failure."""

    binary = Path(binary) if binary else default_binary()
    if not binary.is_file():
        raise BindingUnavailable(f"bind_check binary not found: {binary}")
    argv = [str(binary), "--resources", str(resources_path), "--cir", str(cir_path)]
    if manifest_path is not None:
        argv += ["--manifest", str(manifest_path)]
    try:
        proc = subprocess.run(argv, capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.SubprocessError) as exc:  # noqa: BLE001
        raise BindingUnavailable(f"bind_check failed to run: {exc}") from exc
    if proc.returncode != 0:
        raise BindingUnavailable(f"bind_check exit {proc.returncode}: {proc.stderr.strip()[:200]}")
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise BindingUnavailable(f"bind_check output not JSON: {exc}") from exc
    if isinstance(data, dict):
        data["_raw_stdout"] = proc.stdout
    return data
