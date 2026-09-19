"""Isolated Rust arm: build/test/lockbud/miri over LLM-generated source.

The arm writes a candidate program into a fresh cargo project with a fixed
manifest (no dependencies, std only) and runs every tool as a subprocess with a
timeout, offline, in its own process group. For every invocation the raw
``argv``, the relevant environment (``MIRIFLAGS``, ``CARGO_*``, ``RUST_*``),
``stdout``, ``stderr``, ``exit`` and wall-clock are archived under
``calls/<seq>-<tool>/`` and referenced by hash, so a reviewer can reproduce the
exact command without trusting the harness.

Miri is dynamic: a clean run is recorded as ``detected=False`` and must never be
reported as "no bug". ``behavior_test_ok`` requires at least one real test
(``test result: ok. N passed`` with ``N >= 1``); a zero-test project is
``None`` (``no_tests``), never ``True``.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .experiments_v2 import MIRI_COMBOS, sha256_text

DEFAULT_TIMEOUT_S = 120.0
MIRI_COMBO_TIMEOUT_S = 60.0
MIRI_MANY_SEEDS = 64
FIXED_CARGO_TOML = """[package]
name = "cir_arm_probe"
version = "0.1.0"
edition = "2021"

[dependencies]
"""

# Only genuine bug signals count as "detected"; a tool that fails for another
# reason (unsupported API, missing crate, bad flag) is a separate tool_error so
# a miss is never confused with a crash.
DETECTION_TOKENS = {
    "deadlock": ("deadlock",),
    "data_race": ("data race", "data-race", "data race detected"),
}

LOCKBUD_UNAVAILABLE = "lockbud_unavailable"
LOCKBUD_TOOLCHAIN = "nightly-2026-02-07"
LOCKBUD_BIN_ENV = "LOCKBUD_BIN"
LOCKBUD_DEADLOCK_KINDS = {"ConflictLock", "DoubleLock", "CondvarDeadlock"}

TEST_RESULT_RE = re.compile(
    r"test result: (\w+)\.\s+(\d+) passed;\s+(\d+) failed;\s+(\d+) ignored")
LOCKBUD_BUG_KIND_RE = re.compile(r'"bug_kind"\s*:\s*"([^"]+)"')


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
    evidence_dir: str | None = None
    files: dict[str, str] = field(default_factory=dict)

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
            "evidence_dir": self.evidence_dir,
            "files": self.files,
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


def classify_lockbud(text: str) -> dict[str, Any]:
    """Classify Lockbud output by parsed ``bug_kind`` records only.

    The tool always prints a summary containing the literal ``conflictlock``
    with a zero count, so a text scan would false-positive on every run.
    """

    kinds = sorted({m.group(1) for m in LOCKBUD_BUG_KIND_RE.finditer(text)})
    deadlocks = [k for k in kinds if k in LOCKBUD_DEADLOCK_KINDS]
    return {"status": "detected" if deadlocks else "clean",
            "detected": deadlocks, "bug_kinds": kinds}


THREAD_LEAK_MARKER = "the main thread terminated without waiting for all remaining threads"


def classify_tool_run(run: ToolRun) -> dict[str, Any]:
    """Distinguish detection, thread leak, clean, timeout and tool errors."""

    text = run.stdout + "\n" + run.stderr
    detected = classify_detection(text)
    thread_leak = THREAD_LEAK_MARKER in text.lower()
    if run.timed_out:
        status = "timeout"
    elif detected:
        status = "detected"
    elif thread_leak:
        status = "thread_leak"
    elif run.exit_code != 0:
        status = "tool_error"  # failed for a non-detection reason
    else:
        status = "clean"
    return {"status": status, "detected": detected, "thread_leak": thread_leak}


def parse_test_result(text: str) -> tuple[int | None, int | None]:
    """Sum ``test result: ok. N passed; M failed`` lines across test binaries."""

    passed = failed = 0
    found = False
    for match in TEST_RESULT_RE.finditer(text):
        found = True
        passed += int(match.group(2))
        failed += int(match.group(3))
    if not found:
        return None, None
    return passed, failed


def _env_record(env: dict[str, str]) -> dict[str, str]:
    keys = ("MIRIFLAGS", "RUST_BACKTRACE", "RUSTUP_TOOLCHAIN", "CARGO_NET_OFFLINE",
            "CARGO_TERM_COLOR", "RUSTC_WRAPPER", "LOCKBUD_FLAGS", "LOCKBUD_LOG")
    out = {k: env[k] for k in keys if k in env}
    out.update({k: v for k, v in env.items() if k.startswith("CARGO_") and k not in out})
    return out


class _Runner:
    """Runs subprocesses and archives their evidence."""

    def __init__(self, calls_dir: Path | str | None) -> None:
        self.calls_dir = Path(calls_dir) if calls_dir is not None else None
        self._seq = 0

    def _evidence_dir(self, tool: str) -> Path | None:
        if self.calls_dir is None:
            return None
        self._seq += 1
        path = self.calls_dir / f"{self._seq:03d}-{tool}"
        path.mkdir(parents=True, exist_ok=True)
        return path

    def run(self, tool: str, argv: list[str], *, cwd: Path, timeout_s: float,
            env_extra: dict[str, str]) -> ToolRun:
        env = dict(os.environ)
        env.update({
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TERM_COLOR": "never",
            "RUST_BACKTRACE": "1",
        })
        env.update(env_extra)
        evidence = self._evidence_dir(tool)
        started = time.monotonic()
        timed_out = False
        error: str | None = None
        try:
            proc = subprocess.Popen(
                argv, cwd=str(cwd), env=env,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except OSError as exc:
            return ToolRun(tool=tool, argv=argv, exit_code=None, wall_ms=0,
                           timed_out=False, stdout="", stderr="",
                           stdout_sha256=sha256_text(""), stderr_sha256=sha256_text(""),
                           error=str(exc), evidence_dir=str(evidence) if evidence else None)
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
        run = ToolRun(
            tool=tool, argv=argv, exit_code=proc.returncode, wall_ms=wall_ms,
            timed_out=timed_out, stdout=stdout, stderr=stderr,
            stdout_sha256=sha256_text(stdout), stderr_sha256=sha256_text(stderr),
            error=error, evidence_dir=str(evidence) if evidence else None,
        )
        if evidence is not None:
            self._write_evidence(evidence, run, env, cwd)
        return run

    @staticmethod
    def _write_evidence(evidence: Path, run: ToolRun, env: dict[str, str],
                        cwd: Path) -> None:
        files = {
            "argv.json": json.dumps({"argv": run.argv, "cwd": str(cwd)},
                                    ensure_ascii=False, indent=2) + "\n",
            "env.json": json.dumps(_env_record(env), ensure_ascii=False, indent=2) + "\n",
            "stdout.txt": run.stdout,
            "stderr.txt": run.stderr,
            "exit.txt": f"{run.exit_code}\n{timed_out_str(run)}\n",
            "wall_ms.txt": f"{run.wall_ms}\n",
        }
        run.files = {}
        for name, text in files.items():
            path = evidence / name
            path.write_text(text, encoding="utf-8")
            run.files[name] = str(path)
        # argv/env are hashed too, for a compact record
        run.extra["argv_sha256"] = sha256_text(files["argv.json"])
        run.extra["env_sha256"] = sha256_text(files["env.json"])


def timed_out_str(run: ToolRun) -> str:
    return "timed_out" if run.timed_out else "completed"


def lockbud_path() -> Path | None:
    """Locate the built Lockbud wrapper (env override, else repo ``tools/``)."""

    env = os.environ.get(LOCKBUD_BIN_ENV)
    if env and Path(env).is_file():
        return Path(env).resolve()
    repo = Path(__file__).resolve().parents[2]
    candidate = repo / "tools/lockbud/target/release/lockbud"
    return candidate if candidate.is_file() else None


def lockbud_available() -> bool:
    return lockbud_path() is not None


def miri_many_seeds_supported() -> bool:
    """Probe one project for ``-Zmiri-many-seeds`` support."""

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "src").mkdir()
        (root / "Cargo.toml").write_text(FIXED_CARGO_TOML, encoding="utf-8")
        (root / "src/main.rs").write_text("fn main() {}\n", encoding="utf-8")
        runner = _Runner(None)
        run = runner.run("miri-many-seeds-probe",
                         ["cargo", "miri", "run", "--offline", "--quiet"],
                         cwd=root, timeout_s=180.0,
                         env_extra={"MIRIFLAGS": "-Zmiri-many-seeds=0..1"})
        text = run.stdout + run.stderr
        return "unknown unstable option" not in text and "many-seeds" not in text.lower()


class RustArmProject:
    """A single isolated cargo project holding one candidate program."""

    def __init__(self, root: Path | str, source: str, *, name: str = "candidate",
                 calls_dir: Path | str | None = None) -> None:
        self.root = Path(root).expanduser().resolve()
        self.dir = self.root / name
        (self.dir / "src").mkdir(parents=True, exist_ok=True)
        (self.dir / "Cargo.toml").write_text(FIXED_CARGO_TOML, encoding="utf-8")
        self.source_path = self.dir / "src" / "main.rs"
        self.source_path.write_text(source, encoding="utf-8")
        self.source_sha256 = sha256_text(source)
        self.runner = _Runner(calls_dir if calls_dir is not None else self.dir.parent / "calls")

    # -- tools -------------------------------------------------------------
    def build(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        return self.runner.run("build", ["cargo", "build", "--offline", "--quiet"],
                               cwd=self.dir, timeout_s=timeout_s, env_extra={})

    def test(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        return self.runner.run("test", ["cargo", "test", "--offline", "--quiet"],
                               cwd=self.dir, timeout_s=timeout_s, env_extra={})

    def miri(self, *, seed: int, preemption_rate: float,
             timeout_s: float = MIRI_COMBO_TIMEOUT_S) -> ToolRun:
        flags = (f"-Zmiri-seed={seed} "
                 f"-Zmiri-preemption-rate={preemption_rate} "
                 "-Zmiri-disable-isolation")
        return self.runner.run("miri", ["cargo", "miri", "run", "--offline", "--quiet"],
                               cwd=self.dir, timeout_s=timeout_s,
                               env_extra={"MIRIFLAGS": flags})

    def miri_many_seeds(self, *, seed_from: int = 0, seed_to: int = MIRI_MANY_SEEDS,
                        preemption_rate: float = 0.5,
                        timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        flags = (f"-Zmiri-many-seeds={seed_from}..{seed_to} "
                 f"-Zmiri-preemption-rate={preemption_rate} "
                 "-Zmiri-disable-isolation")
        return self.runner.run("miri-many-seeds",
                               ["cargo", "miri", "run", "--offline", "--quiet"],
                               cwd=self.dir, timeout_s=timeout_s,
                               env_extra={"MIRIFLAGS": flags})

    def behavior_run(self, *, timeout_s: float = 10.0,
                     binary_name: str = "cir_arm_probe") -> ToolRun:
        """Run the built candidate with a watchdog. Terminates -> behavior ok;
        a timeout is a `hang` (a deadlock/livelock observation)."""

        binary = self.dir / "target/debug" / binary_name
        if not binary.is_file():
            return ToolRun(tool="behavior", argv=[str(binary)], exit_code=None,
                           wall_ms=0, timed_out=False, stdout="", stderr="",
                           stdout_sha256=sha256_text(""), stderr_sha256=sha256_text(""),
                           error="no built binary")
        return self.runner.run("behavior", [str(binary)], cwd=self.dir,
                               timeout_s=timeout_s, env_extra={})

    def lockbud(self, *, timeout_s: float = DEFAULT_TIMEOUT_S) -> ToolRun:
        """Run Lockbud as a ``RUSTC_WRAPPER`` on a fresh copy of the program.

        Lockbud is pinned to its own nightly, so the candidate is compiled in a
        separate project directory with that toolchain to avoid target-dir
        conflicts with the default-toolchain build.
        """

        binary = lockbud_path()
        if binary is None:
            return ToolRun(
                tool="lockbud", argv=["lockbud"], exit_code=None, wall_ms=0,
                timed_out=False, stdout="", stderr="",
                stdout_sha256=sha256_text(""), stderr_sha256=sha256_text(""),
                error=LOCKBUD_UNAVAILABLE)
        probe = self.dir.parent / "lockbud-probe"
        (probe / "src").mkdir(parents=True, exist_ok=True)
        shutil.copyfile(self.dir / "Cargo.toml", probe / "Cargo.toml")
        shutil.copyfile(self.source_path, probe / "src" / "main.rs")
        env_extra = {
            "RUSTC_WRAPPER": str(binary),
            "LOCKBUD_FLAGS": "-k deadlock -l cir_arm_probe",
            "LOCKBUD_LOG": "info",
            "RUSTUP_TOOLCHAIN": LOCKBUD_TOOLCHAIN,
        }
        return self.runner.run("lockbud", ["cargo", f"+{LOCKBUD_TOOLCHAIN}", "build"],
                               cwd=probe, timeout_s=timeout_s, env_extra=env_extra)

    def analyze(self, *, timeout_s: float = DEFAULT_TIMEOUT_S,
                run_miri: bool = True, run_lockbud: bool = True,
                run_miri_many_seeds: bool = True,
                miri_seed_count: int | None = None) -> dict[str, Any]:
        """Run the frozen tool suite and return a hash-complete record."""

        record: dict[str, Any] = {
            "source_sha256": self.source_sha256,
            "project_dir": str(self.dir),
            "calls_dir": str(self.runner.calls_dir) if self.runner.calls_dir else None,
            "build": self.build(timeout_s=timeout_s).as_dict(),
        }
        build_ok = record["build"]["exit_code"] == 0 and not record["build"]["timed_out"]
        record["build_ok"] = build_ok
        if not build_ok:
            record["behavior_test"] = None
            record["behavior_test_ok"] = None
            record["behavior_test_reason"] = "build_failed"
            record["miri"] = []
            record["miri_extended"] = None
            record["lockbud"] = {"tool": "lockbud", "status": LOCKBUD_UNAVAILABLE}
            return record

        test = self.test(timeout_s=timeout_s)
        record["behavior_test"] = test.as_dict()
        passed, failed = parse_test_result(test.stdout)
        record["behavior_test_passed"] = passed
        if test.timed_out or test.exit_code != 0:
            record["behavior_test_ok"] = False
            record["behavior_test_reason"] = "test_failed"
        elif passed is None:
            record["behavior_test_ok"] = None
            record["behavior_test_reason"] = "no_tests"
        elif passed == 0 or failed:
            record["behavior_test_ok"] = False if failed else None
            record["behavior_test_reason"] = "test_failed" if failed else "no_tests"
        else:
            record["behavior_test_ok"] = True
            record["behavior_test_reason"] = "ok"

        if run_lockbud:
            if lockbud_available():
                lr = self.lockbud(timeout_s=timeout_s)
                lr.extra.update(classify_lockbud(lr.stdout + "\n" + lr.stderr))
                record["lockbud"] = lr.as_dict()
            else:
                record["lockbud"] = {"tool": "lockbud", "status": LOCKBUD_UNAVAILABLE}
        else:
            record["lockbud"] = {"tool": "lockbud", "status": "skipped"}

        miri_runs = []
        if run_miri and miri_seed_count:
            # One run per seed 0..N-1 (each bounded); a timeout is a
            # `hang_suspect` observation, not a detection.
            for seed in range(miri_seed_count):
                mr = self.miri(seed=seed, preemption_rate=0.5, timeout_s=timeout_s)
                mr.extra.update({"seed": seed, "preemption_rate": 0.5,
                                 "hang_suspect": mr.timed_out})
                mr.extra.update(classify_tool_run(mr))
                miri_runs.append(mr.as_dict())
            record["miri"] = miri_runs
            record["miri_extended"] = None
            return record
        if run_miri:
            for seed, rate in MIRI_COMBOS:
                mr = self.miri(seed=seed, preemption_rate=rate, timeout_s=timeout_s)
                mr.extra.update({"seed": seed, "preemption_rate": rate})
                mr.extra.update(classify_tool_run(mr))
                miri_runs.append(mr.as_dict())
        record["miri"] = miri_runs

        if run_miri and run_miri_many_seeds:
            if miri_many_seeds_supported():
                mr = self.miri_many_seeds(preemption_rate=0.5, timeout_s=timeout_s)
                mr.extra.update({"mode": "many_seeds", "seed_from": 0,
                                 "seed_to": MIRI_MANY_SEEDS,
                                 "preemption_rate": 0.5})
                mr.extra.update(classify_tool_run(mr))
                extended = mr.as_dict()
                extended["mode"] = "many_seeds"
                record["miri_extended"] = extended
            else:
                runs = []
                for seed in range(MIRI_MANY_SEEDS):
                    mr = self.miri(seed=seed, preemption_rate=0.5, timeout_s=timeout_s)
                    mr.extra.update({"seed": seed, "preemption_rate": 0.5})
                    mr.extra.update(classify_tool_run(mr))
                    runs.append(mr.as_dict())
                record["miri_extended"] = {"mode": "loop", "seed_to": MIRI_MANY_SEEDS,
                                           "runs": runs}
        else:
            record["miri_extended"] = None
        return record
