"""strong-link-v28: one explicit toolchain for every stage.

The v27 loss came from ``v23_live`` and ``binding.default_binary`` silently
defaulting to ``ConcIR/target/release`` while the fixed tools lived in
``target/v27-verified``. Every stage (online generation, baseline independent
evaluation, G3 candidate evaluation, offline replay) must use ONE
:class:`ToolchainConfig`; there is no silent fallback to ``target/release``.

Resolve order: explicit directory argument -> ``CONCIR_TOOLCHAIN`` env ->
individual ``CONCIR_BACKEND``/``CONCIR_INSTRUMENT``/``CONCIR_BIND_CHECK`` env.
If nothing is configured, resolution FAILS rather than defaulting to an old
build. Callers that must keep the legacy default (unit tests) use
``legacy_toolchain`` explicitly.
"""

from __future__ import annotations

import hashlib
import json
import os
from dataclasses import dataclass
from pathlib import Path


class ToolchainError(RuntimeError):
    pass


REPO = Path(__file__).resolve().parents[2]
LEGACY_RELEASE = REPO / "ConcIR/target/release"


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@dataclass(frozen=True)
class ToolchainConfig:
    backend: Path
    instrument: Path
    bind_check: Path
    concir_sync_runtime: Path
    label: str = "explicit"

    def validate(self) -> None:
        for name, path in (("concir-backend", self.backend),
                           ("concir-instrument", self.instrument),
                           ("bind_check", self.bind_check)):
            if not path.is_file():
                raise ToolchainError(f"{name} not found: {path}")
            if not os.access(path, os.X_OK):
                raise ToolchainError(f"{name} not executable: {path}")
        if not self.concir_sync_runtime.is_file():
            raise ToolchainError(f"concir_sync runtime not found: {self.concir_sync_runtime}")

    @property
    def is_legacy_release(self) -> bool:
        return self.backend.parent == LEGACY_RELEASE

    def fingerprint(self) -> dict:
        return {
            "label": self.label,
            "backend": str(self.backend), "backend_sha256": _sha(self.backend),
            "instrument": str(self.instrument), "instrument_sha256": _sha(self.instrument),
            "bind_check": str(self.bind_check), "bind_check_sha256": _sha(self.bind_check),
            "concir_sync_runtime": str(self.concir_sync_runtime),
            "concir_sync_runtime_sha256": _sha(self.concir_sync_runtime),
        }

    def as_env(self) -> dict[str, str]:
        return {"CONCIR_BACKEND": str(self.backend),
                "CONCIR_INSTRUMENT": str(self.instrument),
                "CONCIR_BIND_CHECK": str(self.bind_check)}


def _runtime_path() -> Path:
    return REPO / "runtime/concir_sync/src/lib.rs"


def from_dir(directory: Path | str, *, label: str | None = None) -> ToolchainConfig:
    d = Path(directory).expanduser().resolve()
    return ToolchainConfig(
        backend=d / "concir-backend", instrument=d / "concir-instrument",
        bind_check=d / "bind_check", concir_sync_runtime=_runtime_path(),
        label=label or d.name)


def resolve(explicit_dir: Path | str | None = None) -> ToolchainConfig:
    """Resolve the one toolchain, or raise. Never falls back to target/release."""
    if explicit_dir:
        return from_dir(explicit_dir)
    env_dir = os.environ.get("CONCIR_TOOLCHAIN")
    if env_dir:
        return from_dir(env_dir)
    backend = os.environ.get("CONCIR_BACKEND")
    instrument = os.environ.get("CONCIR_INSTRUMENT")
    bind_check = os.environ.get("CONCIR_BIND_CHECK")
    if backend and instrument and bind_check:
        return ToolchainConfig(Path(backend).resolve(), Path(instrument).resolve(),
                               Path(bind_check).resolve(), _runtime_path(),
                               label="env")
    raise ToolchainError(
        "no toolchain configured: pass an explicit directory or set "
        "CONCIR_TOOLCHAIN / CONCIR_BACKEND+CONCIR_INSTRUMENT+CONCIR_BIND_CHECK. "
        "Refusing to fall back to target/release.")


def legacy_toolchain() -> ToolchainConfig:
    """Only for unit tests / offline smoke that explicitly want the old build."""
    return from_dir(LEGACY_RELEASE, label="legacy-release")


def write_manifest(toolchain: ToolchainConfig, path: Path) -> dict:
    manifest = toolchain.fingerprint()
    manifest["source"] = {
        "python/cir_workflow/toolchain.py": _sha(REPO / "python/cir_workflow/toolchain.py"),
    }
    Path(path).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest
