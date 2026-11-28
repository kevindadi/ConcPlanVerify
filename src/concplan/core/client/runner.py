"""Subprocess runner, call directory allocation, and timeout handling for concir-backend."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import time
from pathlib import Path
from typing import Any

from ..json_utils import sha256_file
from .protocol import ConcirIdentity, ConcirResult, _classify, validate_replay_payload


class ConcirSubprocessRunner:
    """Manages isolated per-call directories, process execution, and raw output capture."""

    def __init__(
        self,
        binary: Path,
        *,
        workdir: Path,
        timeout: float,
        env: dict[str, str],
        binary_sha256: str | None = None,
    ) -> None:
        self.binary = binary
        self.workdir = workdir
        self.timeout = timeout
        self.env = env
        self._counter = 0
        self._binary_sha256 = binary_sha256

    @property
    def binary_sha256(self) -> str:
        if self._binary_sha256 is None:
            if not self.binary.is_file():
                raise FileNotFoundError(
                    f"concir-backend binary not found: {self.binary}. "
                    "Build it in the ConcIR repo (cargo build --release --bin concir-backend) "
                    "or pass --binary / CONCIR_BACKEND."
                )
            self._binary_sha256 = sha256_file(self.binary)
        return self._binary_sha256

    def require_binary(self) -> None:
        if not self.binary.is_file():
            raise FileNotFoundError(
                f"concir-backend binary not found: {self.binary}. "
                "Build it in the ConcIR repo or pass --binary / CONCIR_BACKEND."
            )
        if not os.access(self.binary, os.X_OK):
            raise PermissionError(f"concir-backend is not executable: {self.binary}")

    def call_dir(self, command: str) -> Path:
        self._counter += 1
        for _ in range(1000):
            token = f"{time.time_ns():x}-{os.getpid()}-{self._counter:04d}-{os.urandom(3).hex()}"
            path = self.workdir / f"{token}-{command}"
            try:
                path.mkdir(parents=True, exist_ok=False)
                return path
            except FileExistsError:
                continue
        raise RuntimeError("could not allocate a unique call directory")

    @staticmethod
    def materialize(value: Path | str | dict[str, Any], dest: Path) -> Path:
        if isinstance(value, dict):
            dest.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
            return dest
        if isinstance(value, Path) or (
            isinstance(value, str) and os.path.exists(value) and "\n" not in value
        ):
            dest.write_bytes(Path(value).read_bytes())
            return dest
        dest.write_text(str(value), encoding="utf-8")
        return dest

    @staticmethod
    def prepare_out_path(out_path: Path | str | None) -> Path | None:
        if out_path is None:
            return None
        resolved = Path(out_path).expanduser().resolve()
        resolved.parent.mkdir(parents=True, exist_ok=True)
        return resolved

    def invoke(
        self,
        command: str,
        argv: list[str],
        *,
        call_dir: Path,
        input_path: Path | None,
        contract_path: Path | None,
        config: dict[str, Any] | None = None,
        engine: str | None = None,
    ) -> ConcirResult:
        self.require_binary()
        identity = ConcirIdentity(
            binary=str(self.binary),
            binary_sha256=self.binary_sha256,
            argv=[str(self.binary), *argv],
            cwd=str(call_dir),
            input_sha256=sha256_file(input_path) if input_path else None,
            contract_sha256=sha256_file(contract_path) if contract_path else None,
            config=dict(config or {}),
            engine=engine,
        )
        started = time.monotonic()
        timed_out = False
        try:
            popen_kwargs: dict[str, Any] = {
                "cwd": str(call_dir),
                "env": self.env,
                "stdout": subprocess.PIPE,
                "stderr": subprocess.PIPE,
            }
            if os.name != "nt":
                popen_kwargs["start_new_session"] = True
            proc = subprocess.Popen([str(self.binary), *argv], **popen_kwargs)
        except OSError as exc:
            return ConcirResult(
                command=command,
                argv=identity.argv,
                exit_code=None,
                status="process_error",
                kind="process_error",
                payload=None,
                identity=identity,
                error=str(exc),
                wall_ms=int((time.monotonic() - started) * 1000),
                call_dir=str(call_dir),
                input_path=str(input_path) if input_path else None,
                contract_path=str(contract_path) if contract_path else None,
            )

        try:
            out, err = proc.communicate(timeout=self.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            if os.name == "nt":
                try:
                    proc.kill()
                except OSError:
                    pass
                try:
                    out, err = proc.communicate(timeout=2.0)
                except subprocess.TimeoutExpired:
                    out, err = b"", b""
            else:
                try:
                    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                except ProcessLookupError:
                    pass
                out, err = proc.communicate()
        wall_ms = int((time.monotonic() - started) * 1000)

        stdout = out.decode("utf-8", "replace")
        stderr = err.decode("utf-8", "replace")
        stdout_path = call_dir / "stdout.json"
        stderr_path = call_dir / "stderr.txt"
        exit_path = call_dir / "exit.txt"
        stdout_path.write_text(stdout, encoding="utf-8")
        stderr_path.write_text(stderr, encoding="utf-8")
        exit_path.write_text(str(proc.returncode), encoding="utf-8")

        result = ConcirResult(
            command=command,
            argv=identity.argv,
            exit_code=proc.returncode,
            status="unknown",
            kind="semantic",
            payload=None,
            identity=identity,
            stderr=stderr.strip(),
            wall_ms=wall_ms,
            timed_out=timed_out,
            call_dir=str(call_dir),
            stdout_path=str(stdout_path),
            stderr_path=str(stderr_path),
            exit_path=str(exit_path),
            input_path=str(input_path) if input_path else None,
            contract_path=str(contract_path) if contract_path else None,
        )
        if timed_out:
            result.kind, result.status = "process_error", "timeout"
            result.error = f"concir-backend timed out after {self.timeout:g}s"
            return result
        if proc.returncode == 2:
            result.kind, result.status = "usage_error", "usage_error"
            result.error = stderr.strip() or "usage error"
            return result
        if command == "replay":
            return _parse_replay_result(result, stdout, stderr, input_path)

        text = stdout.strip()
        if not text:
            if command == "export-cvn" and proc.returncode == 5:
                result.kind, result.status = "semantic", "unsupported"
                result.error = stderr.strip() or "export-cvn unsupported"
                return result
            result.kind, result.status = "protocol_error", "empty_output"
            result.error = "concir-backend produced no stdout"
            return result
        try:
            payload = json.loads(text)
        except json.JSONDecodeError as exc:
            result.kind, result.status = "protocol_error", "non_json_output"
            result.error = f"stdout is not JSON: {exc}"
            return result
        if not isinstance(payload, dict):
            result.kind, result.status = "protocol_error", "payload_not_object"
            result.error = "stdout JSON is not an object"
            return result
        result.payload = payload
        return _classify(result)


def _parse_replay_result(
    result: ConcirResult,
    stdout: str,
    stderr: str,
    input_path: Path | None,
) -> ConcirResult:
    if result.exit_code != 0:
        result.kind, result.status = "semantic", "replay_failed"
        result.error = stderr.strip() or "artifact replay failed"
        return result
    text = stdout.strip()
    if not text:
        result.kind, result.status = "protocol_error", "empty_replay_output"
        result.error = "replay exited 0 but produced no stdout"
        return result
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as exc:
        result.kind, result.status = "protocol_error", "non_json_replay_output"
        result.error = f"replay stdout is not JSON: {exc}"
        return result
    if not isinstance(payload, dict):
        result.kind, result.status = "protocol_error", "replay_payload_not_object"
        result.error = "replay stdout JSON is not an object"
        return result
    problem = validate_replay_payload(payload, input_path)
    if problem is not None:
        result.kind, result.status = "protocol_error", "invalid_replay_payload"
        result.error = problem
        return result
    result.payload = payload
    result.kind, result.status = "semantic", "replayed"
    return result
