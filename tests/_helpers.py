"""Shared test helpers (not a test module)."""

from __future__ import annotations

import json
import os
import stat
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "programs"


def real_binary() -> Path | None:
    from concplan.core.toolchain import find_concir_backend
    return find_concir_backend()


def make_stub_binary(directory: Path) -> Path:
    """Create an executable stub that echoes env-controlled output/exit.

    ``STUB_STDOUT``, ``STUB_STDERR``, ``STUB_EXIT`` and ``STUB_SLEEP`` shape the
    process behavior; the client treats it exactly like the real CLI.
    """
    import sys

    directory.mkdir(parents=True, exist_ok=True)
    script_body = (
        "import os, sys, time\n"
        "time.sleep(float(os.environ.get('STUB_SLEEP', '0')))\n"
        "sys.stdout.write(os.environ.get('STUB_STDOUT', ''))\n"
        "sys.stderr.write(os.environ.get('STUB_STDERR', ''))\n"
        "sys.exit(int(os.environ.get('STUB_EXIT', '0')))\n"
    )
    if os.name == "nt":
        py_script = directory / "stub-concir-backend.py"
        py_script.write_text(script_body, encoding="utf-8")
        stub = directory / "stub-concir-backend.cmd"
        stub.write_text(
            f'@echo off\r\n"{sys.executable}" "{py_script}" %*\r\n',
            encoding="utf-8",
        )
        return stub
    stub = directory / "stub-concir-backend"
    stub.write_text("#!/usr/bin/env python3\n" + script_body, encoding="utf-8")
    stub.chmod(stub.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)
    return stub


def stub_env(**overrides: str) -> dict[str, str]:
    env = dict(os.environ)
    env.update({k: v for k, v in overrides.items() if v is not None})
    return env


def load_fixture(name: str) -> dict:
    return json.loads((FIXTURES / name).read_text(encoding="utf-8"))


def fixture_text(name: str) -> str:
    return (FIXTURES / name).read_text(encoding="utf-8")


def broken_program() -> dict:
    """A program with an unknown resource reference (static INVALID)."""

    return {
        "program": "broken",
        "version": "3.5.0",
        "entry": "main::main",
        "modules": [{
            "name": "main",
            "provides": {"resources": ["a"], "functions": ["main", "t1"]},
            "requires": {"resources": [], "functions": []},
            "resources": [{"name": "a", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
            "protection": [],
            "functions": [
                {"name": "main", "kind": "normal", "body": [
                    {"sid": "s1", "kind": "scope", "funcs": ["t1"]},
                    {"sid": "s2", "kind": "return"},
                ]},
                {"name": "t1", "kind": "normal", "form": "closure", "body": [
                    {"sid": "s1", "kind": "mutex_lock", "resource": "missing"},
                    {"sid": "s2", "kind": "return"},
                ]},
            ],
        }],
    }
