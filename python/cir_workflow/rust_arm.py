"""Isolated Rust arm: build/test/lockbud/miri over LLM-generated source.

The arm writes a candidate program into a fresh cargo project with a fixed
manifest (no dependencies, std only) and runs every tool as a subprocess with a
timeout, offline, in its own process group. Raw stdout/stderr and exit codes are
archived; every run is hashed so an experiment record can prove which bytes were
observed. A failed build is a normal outcome, not an exception.

Miri is dynamic: a clean run is recorded as ``detected=False`` and must never be
reported as "no bug". ``miri detected`` is a text classification of Miri's own
diagnostic output, not a proof.
"""

from __future__ import annotations

import os
import signal
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .experiments_v2 import MIRI_COMBOS, sha256_text

DEFAULT_TIMEOUT_S = 120.0
MIRI_COMBO_TIMEOUT_S = 60.0
FIXED_CARGO_TOML = """[package]
name = "cir_arm_probe"
version = "0.1.0"
edition = "2021"

[dependencies]
"""

DETECTION_TOKENS = {
    "deadlock": ("deadlock",),
    "data_race": ("data race", "data-race", "data race detected"),
    "unsupported": ("unsupported", "not supported", "is not supported"),
}

# Tools that are optional / may be absent on this machine.
LOCKBUD_UNAVAILABLE = "lockbud_unavailable"


@dataclass
class ToolRun:
    tool: str
    argv: list[str]
    exit_code: int | None
    wall_ms: int
    timed_out: bool
    stdout: str
    stderr: str
    stdout_sha256: str
    stderr_sha256: str
    error: str | None = None
    extra: dict[str, Any] = field(default_factory=dict)

    def as_dict(self, *, include_raw: bool = False) -> dict[str, Any]:
        out: dict[str, Any] = {
            "tool": self.tool,
            "argv": self.argv,
            "exit_code": self.exit_code,
            "wall_ms": self.wall_ms,
            "timed_out": self.timed_out,
            "stdout_sha256": self.stdout_sha256,
            "stderr_sha256": self.stderr_sha256,
            "error": self.error,
            "extra": self.extra,
        }
        if include_raw:
            out["stdout"] = self.stdout
            out["stderr"] = self.stderr
        return out


def classify_detection(text: str) -> list[str]:
    """Return the bug kinds suggested by a tool's raw output (heuristic)."""

    low = text.lower()
    found = []
    for kind, needles in DETECTION_TOKENS.items():
        if any(n in low for n in needles):
            found.append(kind)
    return found


def _run(argv: list[str], *, cwd: Path, timeout_s: float, env_extra: dict[str, str]) -> ToolRun:
    env = dict(os.environ)
    env.update({
        "CARGO_NET_OFFLINE": "true",
        "CARGO_TERM_COLOR": "never",
        "RUST_BACKTRACE": "1",
    })
    env.update(env_extra)
    started = time.monotonic()
    timed_out = False
    try:
        proc = subprocess.Popen(
            argv, cwd=str(cwd), env=env,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as exc:
        return ToolRun(tool=argv[0], argv=argv, exit_code=None, wall_ms=0,
                       timed_out=False, stdout="", stderr="",
                       stdout_sha256=sha256_text(""), stderr_sha256=sha256_text(""),
                       error=str(exc))
    try:
        out, err = proc.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        timed_out = True
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except ProcessLookupError:
            pass
        out, err = proc.communicate()
    wall_ms = int((time.monotonic() - started) * 1000)
    stdout = out.decode("utf-8", "replace")
    stderr = err.decode("utf-8", "replace")
    return ToolRun(
        tool=argv[0], argv=argv, exit_code=proc.returncode, wall_ms=wall_ms,
        timed_out=timed_out, stdout=stdout, stderr=stderr,
        stdout_sha256=sha256_text(stdout), stderr_sha256=sha256_text(stderr),
    )


def lockbud_available() -> bool:
    try:
        run = _run(["cargo", "lockbud", "--version"], cwd=Path.cwd(), timeout_s=30.0,
                   env_extra={})
    except Exception:
        return False
    return run.exit_code == 0


class RustArmProject:
    """A single isolated cargo project holding one candidate program."""

    def __init__(self, root: Path | str, source: str, *, name: str = "candidate") -> None:
        self.root = Path(root).expanduser().resolve()
        self.dir = self.root / name
        (self.dir / "src").mkdir(parents=True, exist_ok=True)
        (self.dir / "Cargo.toml").write_text(FIXED_CARGO_TOML, encoding="utf-8")
        self.source_path = self.dir / "src" / "main.rs"
        self.source_path.write_text(source, encoding="utf-8")
        self.source_sha256 = sha256_text(source)

    # -- tools -------------------------------------------------------------
    def build(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        return _run(["cargo", "build", "--offline", "--quiet"], cwd=self.dir,
                    timeout_s=timeout_s, env_extra={})

    def test(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        return _run(["cargo", "test", "--offline", "--quiet"], cwd=self.dir,
                    timeout_s=timeout_s, env_extra={})

    def miri(self, *, seed: int, preemption_rate: float,
             timeout_s: float = MIRI_COMBO_TIMEOUT_S) -> ToolRun:
        flags = (f"-Zmiri-seed={seed} "
                 f"-Zmiri-preemption-rate={preemption_rate} "
                 "-Zmiri-disable-isolation")
        return _run(["cargo", "miri", "run", "--offline", "--quiet"], cwd=self.dir,
                    timeout_s=timeout_s, env_extra={"MIRIFLAGS": flags})

    def lockbud(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        return _run(["cargo", "lockbud"], cwd=self.dir, timeout_s=timeout_s, env_extra={})

    def analyze(self, *, timeout_s: float = DEFAULT_TIMEOUT_S,
                run_miri: bool = True, run_lockbud: bool = True) -> dict[str, Any]:
        """Run the frozen tool suite and return a hash-complete record."""

        record: dict[str, Any] = {
            "source_sha256": self.source_sha256,
            "project_dir": str(self.dir),
            "build": self.build(timeout_s=timeout_s).as_dict(),
        }
        build_ok = record["build"]["exit_code"] == 0 and not record["build"]["timed_out"]
        record["build_ok"] = build_ok
        if not build_ok:
            record["behavior_test"] = None
            record["miri"] = []
            record["lockbud"] = {"tool": "lockbud", "status": LOCKBUD_UNAVAILABLE}
            return record

        test = self.test(timeout_s=timeout_s)
        record["behavior_test"] = test.as_dict()
        record["behavior_test_ok"] = test.exit_code == 0 and not test.timed_out

        if run_lockbud:
            if lockbud_available():
                lr = self.lockbud(timeout_s=timeout_s)
                lr.extra["detected"] = classify_detection(lr.stdout + "\n" + lr.stderr)
                record["lockbud"] = lr.as_dict()
            else:
                record["lockbud"] = {"tool": "lockbud", "status": LOCKBUD_UNAVAILABLE}
        else:
            record["lockbud"] = {"tool": "lockbud", "status": "skipped"}

        miri_runs = []
        if run_miri:
            for seed, rate in MIRI_COMBOS:
                mr = self.miri(seed=seed, preemption_rate=rate)
                detected = classify_detection(mr.stdout + "\n" + mr.stderr)
                mr.extra = {"seed": seed, "preemption_rate": rate, "detected": detected}
                miri_runs.append(mr.as_dict())
        record["miri"] = miri_runs
        return record
