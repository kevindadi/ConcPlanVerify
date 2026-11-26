"""Backend-first binary discovery for ConcIR and static verification tools.

Resolution order for each binary:
1. Explicit path passed by caller
2. Tool-specific environment variable (``CONCIR_BACKEND``, ``CONCIR_SPN``,
   ``CONCIR_LOCKBUD`` / ``LOCKBUD_BIN``, ``CONCIR_INSTRUMENT``,
   ``CONCIR_BIND_CHECK``)
3. ``CONCIR_TOOLCHAIN`` directory (for ConcIR binaries)
4. In-repo build outputs (``backends/concir/target/{release,debug}``,
   ``backends/spn/target/{release,debug}``,
   ``backends/third_party/lockbud/target/{release,debug}``)
5. Sibling ``../ConcIR/target/{release,debug}`` fallback for external checkouts

All helpers transparently resolve ``.exe`` suffixes on Windows.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path

from .json_utils import sha256_file


class ToolchainError(RuntimeError):
    """Raised when an explicit toolchain configuration is missing or invalid."""


REPO = Path(__file__).resolve().parents[3]
SUBMODULE_RELEASE = REPO / "backends/concir/target/release"
SUBMODULE_DEBUG = REPO / "backends/concir/target/debug"
LEGACY_RELEASE = REPO.parent / "ConcIR/target/release"
LEGACY_DEBUG = REPO.parent / "ConcIR/target/debug"
SPN_RELEASE = REPO / "backends/spn/target/release"
SPN_DEBUG = REPO / "backends/spn/target/debug"
LOCKBUD_RELEASE = REPO / "backends/third_party/lockbud/target/release"
LOCKBUD_DEBUG = REPO / "backends/third_party/lockbud/target/debug"

CONCIR_SEARCH_DIRS = (
    SUBMODULE_RELEASE,
    SUBMODULE_DEBUG,
    LEGACY_RELEASE,
    LEGACY_DEBUG,
)
SPN_SEARCH_DIRS = (SPN_RELEASE, SPN_DEBUG)
LOCKBUD_SEARCH_DIRS = (LOCKBUD_RELEASE, LOCKBUD_DEBUG)


def resolve_exe_in_dir(directory: Path, name: str) -> Path:
    """Resolve a binary inside ``directory``, handling Windows ``.exe`` seamlessly."""
    exe_name = name if name.lower().endswith(".exe") else f"{name}.exe"
    bare = directory / name
    exe = directory / exe_name
    if os.name == "nt":
        if exe.is_file():
            return exe
        if bare.is_file():
            return bare
        return exe
    if bare.is_file():
        return bare
    if exe.is_file():
        return exe
    return bare


def resolve_binary_path(path: Path | str) -> Path:
    """Resolve an explicit binary path, checking ``.exe`` suffix on Windows."""
    p = Path(path).expanduser().resolve()
    if p.is_file():
        return p
    if os.name == "nt" and not p.name.lower().endswith(".exe"):
        exe = p.with_name(p.name + ".exe")
        if exe.is_file():
            return exe
    return p


def find_binary(
    name: str,
    *,
    explicit: Path | str | None = None,
    env_var: str | None = None,
    search_dirs: tuple[Path, ...] | list[Path] = (),
    check_toolchain_env: bool = False,
) -> Path | None:
    """Discover a binary: explicit -> env var -> CONCIR_TOOLCHAIN -> search_dirs."""
    if explicit:
        p = resolve_binary_path(explicit)
        return p if p.is_file() else None
    if env_var:
        env_val = os.environ.get(env_var)
        if env_val:
            p = resolve_binary_path(env_val)
            if p.is_file():
                return p
    if check_toolchain_env:
        tc_dir = os.environ.get("CONCIR_TOOLCHAIN")
        if tc_dir:
            p = resolve_exe_in_dir(Path(tc_dir).expanduser().resolve(), name)
            if p.is_file():
                return p
    for d in search_dirs:
        p = resolve_exe_in_dir(d, name)
        if p.is_file():
            return p
    return None


def find_concir_backend(explicit: Path | str | None = None) -> Path | None:
    return find_binary(
        "concir-backend",
        explicit=explicit,
        env_var="CONCIR_BACKEND",
        search_dirs=CONCIR_SEARCH_DIRS,
        check_toolchain_env=True,
    )


def find_concir_instrument(explicit: Path | str | None = None) -> Path | None:
    return find_binary(
        "concir-instrument",
        explicit=explicit,
        env_var="CONCIR_INSTRUMENT",
        search_dirs=CONCIR_SEARCH_DIRS,
        check_toolchain_env=True,
    )


def find_bind_check(explicit: Path | str | None = None) -> Path | None:
    return find_binary(
        "bind_check",
        explicit=explicit,
        env_var="CONCIR_BIND_CHECK",
        search_dirs=CONCIR_SEARCH_DIRS,
        check_toolchain_env=True,
    )


def find_spn(explicit: Path | str | None = None) -> Path | None:
    return find_binary(
        "spn",
        explicit=explicit,
        env_var="CONCIR_SPN",
        search_dirs=SPN_SEARCH_DIRS,
        check_toolchain_env=False,
    )


def find_lockbud(explicit: Path | str | None = None) -> Path | None:
    if explicit:
        return find_binary("lockbud", explicit=explicit)
    env_var = "CONCIR_LOCKBUD" if os.environ.get("CONCIR_LOCKBUD") else "LOCKBUD_BIN"
    return find_binary(
        "lockbud",
        env_var=env_var,
        search_dirs=LOCKBUD_SEARCH_DIRS,
        check_toolchain_env=False,
    )


@dataclass(frozen=True)
class ToolchainConfig:
    """Explicit bundle of ConcIR binaries and optional static analyzers."""

    backend: Path
    instrument: Path | None = None
    bind_check: Path | None = None
    spn: Path | None = None
    concir_sync_runtime: Path | None = None
    label: str = "explicit"

    def validate(self) -> None:
        if not self.backend.is_file():
            raise ToolchainError(f"concir-backend not found: {self.backend}")
        if not os.access(self.backend, os.X_OK):
            raise ToolchainError(f"concir-backend not executable: {self.backend}")
        for name, path in (
            ("concir-instrument", self.instrument),
            ("bind_check", self.bind_check),
            ("spn", self.spn),
        ):
            if path is not None and not path.is_file():
                raise ToolchainError(f"{name} not found: {path}")

    def fingerprint(self) -> dict[str, str | None]:
        out: dict[str, str | None] = {
            "label": self.label,
            "backend": str(self.backend),
            "backend_sha256": sha256_file(self.backend),
        }
        for key, path in (
            ("instrument", self.instrument),
            ("bind_check", self.bind_check),
            ("spn", self.spn),
        ):
            if path is not None and path.is_file():
                out[key] = str(path)
                out[f"{key}_sha256"] = sha256_file(path)
            else:
                out[key] = None
                out[f"{key}_sha256"] = None
        return out

    def as_env(self) -> dict[str, str]:
        env = {"CONCIR_BACKEND": str(self.backend)}
        if self.instrument is not None:
            env["CONCIR_INSTRUMENT"] = str(self.instrument)
        if self.bind_check is not None:
            env["CONCIR_BIND_CHECK"] = str(self.bind_check)
        if self.spn is not None:
            env["CONCIR_SPN"] = str(self.spn)
        return env


def from_dir(directory: Path | str, *, label: str | None = None) -> ToolchainConfig:
    d = Path(directory).expanduser().resolve()
    inst = resolve_exe_in_dir(d, "concir-instrument")
    bind = resolve_exe_in_dir(d, "bind_check")
    return ToolchainConfig(
        backend=resolve_exe_in_dir(d, "concir-backend"),
        instrument=inst if inst.is_file() else None,
        bind_check=bind if bind.is_file() else None,
        spn=find_spn(),
        label=label or d.name,
    )


def submodule_toolchain() -> ToolchainConfig:
    """Resolve the in-repo ConcIR submodule release toolchain."""
    return from_dir(SUBMODULE_RELEASE, label="submodule-release")


def write_manifest(toolchain: ToolchainConfig, path: Path) -> dict:
    manifest = toolchain.fingerprint()
    manifest["source"] = {
        "src/concplan/core/toolchain.py": sha256_file(
            REPO / "src/concplan/core/toolchain.py"
        ),
    }
    Path(path).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest
