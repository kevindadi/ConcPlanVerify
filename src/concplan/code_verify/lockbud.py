"""Optional Lockbud static deadlock analyzer plugin."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any

from ..core.toolchain import find_lockbud
from .verifier import StaticVerificationReport, StaticViolation

LOCKBUD_TOOLCHAIN = "nightly-2026-02-07"
LOCKBUD_DEADLOCK_KINDS = {"ConflictLock", "DoubleLock", "CondvarDeadlock"}
LOCKBUD_BUG_KIND_RE = re.compile(r'"bug_kind"\s*:\s*"([^"]+)"')


def classify_lockbud(text: str) -> dict[str, Any]:
    """Classify Lockbud output by parsed ``bug_kind`` records only."""
    kinds = sorted({m.group(1) for m in LOCKBUD_BUG_KIND_RE.finditer(text)})
    deadlocks = [k for k in kinds if k in LOCKBUD_DEADLOCK_KINDS]
    return {
        "status": "detected" if deadlocks else "clean",
        "detected": deadlocks,
        "bug_kinds": kinds,
    }


class LockbudPlugin:
    """Optional static deadlock detector wrapping ``backends/third_party/lockbud``."""

    name = "lockbud"

    def __init__(
        self,
        *,
        lockbud_binary: Path | str | None = None,
        toolchain: str = LOCKBUD_TOOLCHAIN,
        crate_name: str = "cir_generated",
        timeout: float = 180.0,
        cargo_offline: bool = True,
    ) -> None:
        self.lockbud_binary = lockbud_binary
        self.toolchain = toolchain
        self.crate_name = crate_name
        self.timeout = float(timeout)
        self.cargo_offline = cargo_offline

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        del skeleton_dir, spec_path, contract_path, codegen_map_path, fills
        binary = find_lockbud(self.lockbud_binary)
        if binary is None:
            return StaticVerificationReport(
                status="unknown",
                plugin=self.name,
                checks={"lockbud_available": False},
                error="lockbud binary not found (set CONCIR_LOCKBUD or build backends/third_party/lockbud)",
            )

        filled = Path(filled_dir).expanduser().resolve()
        target_dir = filled / ".lockbud-verify" / "target"
        target_dir.parent.mkdir(parents=True, exist_ok=True)
        if target_dir.exists():
            shutil.rmtree(target_dir, ignore_errors=True)

        argv = ["cargo", f"+{self.toolchain}", "check"]
        if self.cargo_offline:
            argv.append("--offline")
        argv.append("--no-default-features")

        env = dict(os.environ)
        env.update({
            "RUSTC_WRAPPER": str(binary),
            "LOCKBUD_FLAGS": f"-k deadlock -l {self.crate_name}",
            "LOCKBUD_LOG": "info",
            "RUSTUP_TOOLCHAIN": self.toolchain,
            "CARGO_TARGET_DIR": str(target_dir),
        })
        if self.cargo_offline:
            env["CARGO_NET_OFFLINE"] = "true"

        try:
            proc = subprocess.run(
                argv,
                cwd=str(filled),
                env=env,
                capture_output=True,
                text=True,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"lockbud timed out after {self.timeout:g}s",
            )
        except OSError as exc:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"failed to run lockbud: {exc}",
            )

        combined = (proc.stdout or "") + "\n" + (proc.stderr or "")
        classified = classify_lockbud(combined)
        detected = classified.get("detected") or []

        if detected:
            violations = [
                StaticViolation(
                    plugin=self.name,
                    obligation="O_deadlock",
                    kind=f"lockbud_{bug_kind}",
                    message=f"lockbud reported potential deadlock ({bug_kind})",
                )
                for bug_kind in detected
            ]
            return StaticVerificationReport(
                status="fail_refinement",
                plugin=self.name,
                violations=violations,
                checks={"lockbud_available": True, "lockbud_deadlock_free": False},
                metadata={"bug_kinds": classified.get("bug_kinds", [])},
            )

        if proc.returncode != 0:
            return StaticVerificationReport(
                status="error",
                plugin=self.name,
                checks={"lockbud_available": True},
                error=f"lockbud failed with exit {proc.returncode}: {(proc.stderr or '').strip()[-500:]}",
            )

        return StaticVerificationReport(
            status="pass",
            plugin=self.name,
            checks={"lockbud_available": True, "lockbud_deadlock_free": True},
            metadata={"bug_kinds": classified.get("bug_kinds", [])},
        )
