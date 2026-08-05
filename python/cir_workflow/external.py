"""External baselines: run benchmark reference Rust under Miri or Lockbud.

For every manifest case that has a reference ``rust.buggy`` (or, for safe-only
cases, whatever single reference file exists), this runner:

1. materializes a scratch cargo package with that file as ``src/main.rs``,
2. runs ``cargo +nightly miri run`` with ``-Zmiri-many-seeds`` so several
   schedules are explored (Miri only sees executed interleavings), and
3. classifies the outcome:

   - ``deadlock``   — Miri's "the evaluated program deadlocked" report
   - ``panic``      — assertion/panic (our goal-style defects)
   - ``ub``         — undefined behaviour / data race report
   - ``timeout``    — wall-clock budget exhausted (livelock / partial deadlock)
   - ``clean``      — program finished without findings
   - ``build_error``— the reference file failed to compile under Miri

The comparison against ``expected`` mirrors the CVN scoring: a gold ``bug``
case counts as detected when Miri reports deadlock/panic/ub *or* times out
(a hang is an observable defect, just not a diagnosed one — we track the
distinction in ``outcome``).

Usage:

    PYTHONPATH=python python -m cir_workflow.external --out results/miri_baseline.json
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

CARGO_TOML = """\
[package]
name = "scratchcase"
version = "0.0.0"
edition = "2021"

# Keep the scratch package out of the repository's cargo workspace.
[workspace]

[[bin]]
name = "case"
path = "src/main.rs"

[profile.dev]
panic = "abort"
"""

DEFAULT_MIRIFLAGS = "-Zmiri-disable-isolation -Zmiri-many-seeds=0..8"
LOCKBUD_TOOLCHAIN = "nightly-2026-02-07"
BUG_KIND_RE = re.compile(r'"bug_kind":\s*"(\w+)"')


def classify(returncode: int | None, output: str) -> str:
    if returncode is None:
        return "timeout"
    lowered = output.lower()
    if "error: deadlock" in lowered or "program deadlocked" in lowered:
        return "deadlock"
    if "undefined behavior" in lowered or "data race" in lowered:
        return "ub"
    if "panicked at" in lowered:
        return "panic"
    if "error[e" in lowered or "error: could not compile" in lowered:
        return "build_error"
    if returncode == 0:
        return "clean"
    return "other_error"


def run_miri(
    rust_file: Path,
    scratch: Path,
    timeout_s: float,
    miriflags: str,
) -> dict[str, Any]:
    _prepare_scratch(rust_file, scratch)
    cmd = ["cargo", "+nightly", "miri", "run", "--quiet"]
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            cmd,
            cwd=scratch,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_s,
            env={**os.environ, "MIRIFLAGS": miriflags},
        )
        returncode: int | None = proc.returncode
        output = (proc.stdout or "") + (proc.stderr or "")
    except subprocess.TimeoutExpired as exc:
        returncode = None
        output = "".join(
            part.decode("utf-8", "replace") if isinstance(part, bytes) else (part or "")
            for part in (exc.stdout, exc.stderr)
        )
    wall_s = time.perf_counter() - started

    outcome = classify(returncode, output)
    tail = "\n".join(output.strip().splitlines()[-15:])
    return {
        "outcome": outcome,
        "returncode": returncode,
        "wall_s": wall_s,
        "output_tail": tail,
    }


def _prepare_scratch(rust_file: Path, scratch: Path) -> None:
    src = scratch / "src"
    src.mkdir(parents=True, exist_ok=True)
    (scratch / "Cargo.toml").write_text(CARGO_TOML, encoding="utf-8")
    shutil.copyfile(rust_file, src / "main.rs")


def run_lockbud(
    rust_file: Path,
    scratch: Path,
    timeout_s: float,
    lockbud_exe: Path,
) -> dict[str, Any]:
    """Static analysis: build the scratch crate with lockbud as RUSTC_WRAPPER."""

    _prepare_scratch(rust_file, scratch)
    # Force a fresh compile so the wrapper actually runs for the crate.
    shutil.rmtree(scratch / "target", ignore_errors=True)

    # lockbud.exe links against rustc_driver; the pinned toolchain's bin
    # directory must be on PATH for the DLLs to resolve on Windows.
    toolchain_bin = (
        Path.home()
        / ".rustup"
        / "toolchains"
        / f"{LOCKBUD_TOOLCHAIN}-x86_64-pc-windows-msvc"
        / "bin"
    )
    env = {
        **os.environ,
        "RUSTC_WRAPPER": str(lockbud_exe),
        # cargo passes --crate-name case for the [[bin]] target
        "LOCKBUD_FLAGS": "-k deadlock -l case,scratchcase",
        "LOCKBUD_LOG": "info",
        "PATH": f"{toolchain_bin}{os.pathsep}{os.environ.get('PATH', '')}",
    }
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            ["cargo", f"+{LOCKBUD_TOOLCHAIN}", "build"],
            cwd=scratch,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_s,
            env=env,
        )
        returncode: int | None = proc.returncode
        output = (proc.stdout or "") + (proc.stderr or "")
    except subprocess.TimeoutExpired as exc:
        returncode = None
        output = "".join(
            part.decode("utf-8", "replace") if isinstance(part, bytes) else (part or "")
            for part in (exc.stdout, exc.stderr)
        )
    wall_s = time.perf_counter() - started

    bug_kinds = sorted(set(BUG_KIND_RE.findall(output)))
    if returncode is None:
        outcome = "timeout"
    elif returncode != 0 and not bug_kinds:
        outcome = "build_error"
    elif bug_kinds:
        outcome = "reported"
    else:
        outcome = "clean"
    tail = "\n".join(output.strip().splitlines()[-15:])
    return {
        "outcome": outcome,
        "bug_kinds": bug_kinds,
        "returncode": returncode,
        "wall_s": wall_s,
        "output_tail": tail,
    }


def score(expected_outcome: str, tool_outcome: str) -> dict[str, Any]:
    defect_found = tool_outcome in ("deadlock", "panic", "ub", "timeout", "reported")
    diagnosed = tool_outcome in ("deadlock", "panic", "ub", "reported")
    if expected_outcome == "safe":
        return {
            "expected": "safe",
            "detected": defect_found,
            "correct": not defect_found and tool_outcome == "clean",
            "false_positive": defect_found,
        }
    # gold bug / goals_unmet
    return {
        "expected": expected_outcome,
        "detected": defect_found,
        "diagnosed": diagnosed,
        "correct": defect_found,
        "false_positive": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="External baselines (Miri / Lockbud)")
    parser.add_argument("--tool", choices=("miri", "lockbud"), default="miri")
    parser.add_argument("--manifest", default="benchmarks/manifest.json")
    parser.add_argument("--out", help="output JSON path (default: stdout)")
    parser.add_argument("--cases", help="comma-separated case ids (default: all)")
    parser.add_argument("--timeout", type=float, default=120.0)
    parser.add_argument("--miriflags", default=DEFAULT_MIRIFLAGS)
    parser.add_argument(
        "--lockbud-exe",
        default="E:/local-repos/lockbud/target/release/lockbud.exe",
        help="path to the lockbud RUSTC_WRAPPER binary",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    manifest = json.loads((repo_root / args.manifest).read_text(encoding="utf-8"))
    wanted = set(args.cases.split(",")) if args.cases else None

    tool = args.tool
    scratch = repo_root / "target" / f"{tool}-scratch"
    records: list[dict[str, Any]] = []
    for case in manifest["cases"]:
        case_id = case["id"]
        if wanted and case_id not in wanted:
            continue
        rust = case.get("rust") or {}
        rust_rel = rust.get("buggy") or rust.get("fixed")
        if not rust_rel:
            records.append({"case_id": case_id, "skipped": "no reference rust"})
            continue
        rust_file = repo_root / rust_rel
        print(f"[{tool}] {case_id} :: {rust_rel}", file=sys.stderr)
        if tool == "miri":
            run = run_miri(rust_file, scratch, args.timeout, args.miriflags)
        else:
            run = run_lockbud(rust_file, scratch, args.timeout, Path(args.lockbud_exe))
        record = {
            "case_id": case_id,
            "defect_type": case.get("defect_type"),
            "rust_file": rust_rel,
            tool: run,
            "score": score(case["expected"]["outcome"], run["outcome"]),
        }
        print(
            f"[{tool}] {case_id} -> {run['outcome']} ({run['wall_s']:.1f}s)",
            file=sys.stderr,
        )
        records.append(record)

    output = {
        "meta": {
            "tool": tool,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "miriflags": args.miriflags if tool == "miri" else None,
            "lockbud_toolchain": LOCKBUD_TOOLCHAIN if tool == "lockbud" else None,
            "timeout_s": args.timeout,
        },
        "records": records,
    }
    text = json.dumps(output, ensure_ascii=False, indent=2)
    if args.out:
        out_path = (repo_root / args.out).resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(text, encoding="utf-8")
        print(f"[{tool}] wrote {out_path}", file=sys.stderr)
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
