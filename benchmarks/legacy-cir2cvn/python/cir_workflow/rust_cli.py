"""Subprocess boundary to the Rust ConcIR/CVN verification CLI."""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path

from .models import RustCliResult

_BINARY_NAME = "cir2cvn.exe" if os.name == "nt" else "cir2cvn"


class RustCli:
    """Invoke ``cir2cvn`` with ConcIR JSON on stdin and JSON on stdout."""

    def __init__(
        self,
        *,
        repo_root: Path | str,
        binary: Path | str | None = None,
        build_if_missing: bool = True,
        timeout: float = 120.0,
    ) -> None:
        self.repo_root = Path(repo_root).resolve()
        self.binary = Path(binary).resolve() if binary else self.repo_root / "target" / "release" / _BINARY_NAME
        self.build_if_missing = build_if_missing
        self.timeout = timeout

    def _ensure_binary(self) -> Path:
        if self.binary.exists():
            return self.binary
        if not self.build_if_missing:
            raise FileNotFoundError(f"Rust CLI not found: {self.binary}")
        subprocess.run(
            ["cargo", "build", "--release", "--bin", "cir2cvn", "--quiet"],
            cwd=self.repo_root,
            check=True,
        )
        if not self.binary.exists():
            raise FileNotFoundError(f"Rust build completed but CLI is missing: {self.binary}")
        return self.binary

    def run(self, mode: str, cir_json: str) -> RustCliResult:
        started = time.perf_counter()
        try:
            binary = self._ensure_binary()
            # The Rust CLI speaks UTF-8 on both ends; never fall back to the
            # platform locale encoding (GBK on Chinese Windows breaks here).
            process = subprocess.run(
                [str(binary), mode, "-"],
                input=cir_json,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                cwd=self.repo_root,
                timeout=self.timeout,
            )
        except subprocess.TimeoutExpired as error:
            return self._failure(mode, -1, f"Rust CLI timed out after {self.timeout:g}s", started, error.stderr)
        except (OSError, subprocess.CalledProcessError) as error:
            return self._failure(mode, -1, str(error), started, "")

        stdout = process.stdout.strip()
        try:
            payload = json.loads(stdout) if stdout else None
        except json.JSONDecodeError as error:
            return RustCliResult(
                mode=mode,
                exit_code=process.returncode,
                status="protocol_error",
                payload=None,
                stderr=process.stderr.strip(),
                error=f"Rust CLI returned non-JSON stdout: {error}",
                duration_ms=self._elapsed(started),
            )

        if not isinstance(payload, dict):
            return RustCliResult(
                mode=mode,
                exit_code=process.returncode,
                status="protocol_error",
                payload=None,
                stderr=process.stderr.strip(),
                error="Rust CLI protocol payload must be a JSON object",
                duration_ms=self._elapsed(started),
            )

        status = str(payload.get("status", self._infer_status(mode, payload, process.returncode)))
        return RustCliResult(
            mode=mode,
            exit_code=process.returncode,
            status=status,
            payload=payload,
            stderr=process.stderr.strip(),
            error=None if process.returncode == 0 or payload else process.stderr.strip() or None,
            duration_ms=self._elapsed(started),
        )

    def validate(self, cir_json: str) -> RustCliResult:
        return self.run("--validate", cir_json)

    def analyze(self, cir_json: str) -> RustCliResult:
        return self.run("--analyze", cir_json)

    def goals(self, cir_json: str) -> RustCliResult:
        return self.run("--goals", cir_json)

    def analyze_no_goals(self, cir_json: str) -> RustCliResult:
        return self.run("--no-goals", cir_json)

    @staticmethod
    def _infer_status(mode: str, payload: dict, exit_code: int) -> str:
        if mode == "--validate":
            return "valid" if payload.get("valid") else "invalid_model"
        if "status" in payload:
            return str(payload["status"])
        return "ok" if exit_code == 0 else "tool_error"

    @staticmethod
    def _elapsed(started: float) -> float:
        return (time.perf_counter() - started) * 1000

    def _failure(
        self,
        mode: str,
        exit_code: int,
        error: str,
        started: float,
        stderr: str | bytes | None,
    ) -> RustCliResult:
        return RustCliResult(
            mode=mode,
            exit_code=exit_code,
            status="tool_error",
            payload=None,
            stderr=(stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr or "").strip(),
            error=error,
            duration_ms=self._elapsed(started),
        )
