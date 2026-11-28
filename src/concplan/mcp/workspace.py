"""Shared MCP workspace, binary resolution, and JSON payload helpers."""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ..core.concir_client import ConcirClient, ConcirResult
from ..core.toolchain import find_concir_backend

JSON_OR_PATH_SCHEMA: dict[str, Any] = {
    "type": ["object", "string"],
    "description": "JSON object, serialized JSON string, or path to a JSON file on disk.",
}


def result_meta(res: ConcirResult) -> dict[str, Any]:
    return {
        "kind": res.kind,
        "status": res.status,
        "exit_code": res.exit_code,
        "error": res.error,
        "stderr": res.stderr,
        "wall_ms": res.wall_ms,
        "artifact_path": res.artifact_path,
    }


@dataclass
class McpWorkspace:
    """Shared execution context and binary configuration for MCP tool handlers."""

    workdir: Path
    concir_binary: Path | str | None = None
    spn_binary: Path | str | None = None
    lockbud_binary: Path | str | None = None
    timeout: float = 120.0
    _counter: int = field(default=0, init=False, repr=False)

    def __post_init__(self) -> None:
        self.workdir = Path(self.workdir).expanduser().resolve()
        self.workdir.mkdir(parents=True, exist_ok=True)

    def resolve_concir_binary(self) -> Path:
        found = find_concir_backend(self.concir_binary)
        if found is None:
            raise FileNotFoundError(
                "concir-backend binary not found. Build it with "
                "`cargo build --release --manifest-path backends/concir/Cargo.toml --bin concir-backend` "
                "or set CONCIR_BACKEND."
            )
        return found

    def client(self) -> ConcirClient:
        return ConcirClient(
            self.resolve_concir_binary(),
            workdir=self.workdir / "concir-calls",
            timeout=self.timeout,
        )

    def new_call_dir(self, prefix: str) -> Path:
        self._counter += 1
        for _ in range(1000):
            token = f"{prefix}-{time.time_ns():x}-{os.getpid()}-{self._counter:04d}"
            path = self.workdir / token
            try:
                path.mkdir(parents=True, exist_ok=False)
                return path
            except FileExistsError:
                continue
        raise RuntimeError(f"could not allocate unique MCP call directory under {self.workdir}")

    @staticmethod
    def load_json_value(value: Any, *, field_name: str) -> dict[str, Any]:
        if isinstance(value, dict):
            return value
        if isinstance(value, str):
            stripped = value.strip()
            if "\n" not in stripped and len(stripped) < 4096:
                candidate = Path(stripped).expanduser()
                if candidate.is_file():
                    data = json.loads(candidate.read_text(encoding="utf-8"))
                    if not isinstance(data, dict):
                        raise ValueError(f"{field_name} file must contain a JSON object")
                    return data
            data = json.loads(stripped)
            if not isinstance(data, dict):
                raise ValueError(f"{field_name} must be a JSON object")
            return data
        raise ValueError(f"{field_name} must be a JSON object, JSON string, or file path")

    @staticmethod
    def materialize_json(value: Any, call_dir: Path, filename: str, *, field_name: str) -> Path:
        dest = call_dir / filename
        if isinstance(value, dict):
            dest.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            return dest
        if isinstance(value, str):
            stripped = value.strip()
            if "\n" not in stripped and len(stripped) < 4096:
                candidate = Path(stripped).expanduser()
                if candidate.is_file():
                    return candidate.resolve()
            data = json.loads(stripped)
            if not isinstance(data, dict):
                raise ValueError(f"{field_name} must be a JSON object")
            dest.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            return dest
        raise ValueError(f"{field_name} must be a JSON object, JSON string, or file path")
