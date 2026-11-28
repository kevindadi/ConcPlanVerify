"""Codegen invocation and dynamic native/Miri trace conformance."""

from __future__ import annotations

import json
import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from ..core.json_utils import sha256_bytes, sha256_text
from ..core.toolchain import find_concir_backend, resolve_binary_path, resolve_exe_in_dir
from .ast_purity import lint_filled
from .holes import holes_of, type_holes_of


def _binary(explicit: Path | str | None = None) -> Path:
    if explicit:
        return resolve_binary_path(explicit)
    found = find_concir_backend()
    if found is not None:
        return found
    raise FileNotFoundError("concir-backend not found (set CONCIR_BACKEND)")


def _run(
    argv: list[str],
    *,
    cwd: Path | None = None,
    timeout: float = 300.0,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess:
    full_env = dict(os.environ)
    if env:
        full_env.update(env)
    return subprocess.run(
        argv,
        cwd=str(cwd) if cwd else None,
        env=full_env,
        capture_output=True,
        text=True,
        timeout=timeout,
    )


@dataclass
class TraceRun:
    kind: str
    index: int
    trace_path: str | None
    hang: bool
    exit_code: int | None
    wall_ms: int
    stdout_sha256: str | None = None
    stderr_sha256: str | None = None
    hang_suspect: bool = False


def codegen(
    program: Path | str,
    out_dir: Path | str,
    *,
    binary: Path | str | None = None,
) -> dict[str, Any]:
    out = Path(out_dir).expanduser().resolve()
    result = _run(
        [str(_binary(binary)), "codegen", str(Path(program).resolve()), "--out", str(out)],
        timeout=120.0,
    )
    if result.returncode != 0:
        raise RuntimeError(f"codegen failed: {result.stderr.strip() or result.stdout.strip()}")
    return json.loads(result.stdout)


def _git_rev(binary: Path) -> str:
    try:
        repo = binary.resolve().parents[2]
        out = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if out.returncode == 0:
            return out.stdout.strip()
    except Exception:  # noqa: BLE001
        pass
    return "unknown"


def collect_traces(
    filled_dir: Path | str,
    *,
    native_runs: int = 50,
    miri_seeds: int = 64,
    timeout_s: float = 10.0,
    calls_dir: Path | str | None = None,
    run_miri: bool = True,
) -> list[TraceRun]:
    """Build the filled project, run it N times plus an optional Miri pass."""
    root = Path(filled_dir).expanduser().resolve()
    calls = Path(calls_dir).expanduser().resolve() if calls_dir else root / "calls"
    calls.mkdir(parents=True, exist_ok=True)
    build = _run(["cargo", "build", "--offline", "--quiet"], cwd=root, timeout=300.0)
    (calls / "build.stdout.txt").write_text(build.stdout, encoding="utf-8")
    (calls / "build.stderr.txt").write_text(build.stderr, encoding="utf-8")
    if build.returncode != 0:
        raise RuntimeError("filled project failed to build")

    runs: list[TraceRun] = []
    binary = resolve_exe_in_dir(root / "target/debug", "cir_generated")
    for i in range(native_runs):
        trace = calls / f"native-{i:03d}.jsonl"
        started = time.monotonic()
        try:
            proc = _run([str(binary)], cwd=root, timeout=timeout_s, env={"CIR_TRACE_OUT": str(trace)})
            hang = False
            code = proc.returncode
            out, err = proc.stdout, proc.stderr
        except subprocess.TimeoutExpired:
            hang = True
            code = None
            out, err = "", ""
        wall = int((time.monotonic() - started) * 1000)
        runs.append(
            TraceRun(
                kind="native",
                index=i,
                trace_path=str(trace) if trace.is_file() else None,
                hang=hang,
                exit_code=code,
                wall_ms=wall,
                stdout_sha256=sha256_text(out),
                stderr_sha256=sha256_text(err),
            )
        )
    if run_miri:
        for seed in range(miri_seeds):
            trace = calls / f"miri-{seed:03d}.jsonl"
            started = time.monotonic()
            _run(["cargo", "clean"], cwd=root, timeout=120.0)
            try:
                proc = _run(
                    ["cargo", "miri", "run", "--offline", "--quiet"],
                    cwd=root,
                    timeout=max(timeout_s, 120.0),
                    env={
                        "CIR_TRACE_OUT": str(trace),
                        "MIRIFLAGS": (
                            f"-Zmiri-seed={seed} "
                            "-Zmiri-preemption-rate=0.5 "
                            "-Zmiri-disable-isolation"
                        ),
                    },
                )
                hang = False
                code = proc.returncode
                out, err = proc.stdout, proc.stderr
            except subprocess.TimeoutExpired:
                hang = True
                code = None
                out, err = "", ""
            wall = int((time.monotonic() - started) * 1000)
            runs.append(
                TraceRun(
                    kind="miri",
                    index=seed,
                    trace_path=str(trace) if trace.is_file() else None,
                    hang=hang,
                    exit_code=code,
                    wall_ms=wall,
                    stdout_sha256=sha256_text(out),
                    stderr_sha256=sha256_text(err),
                    hang_suspect=hang,
                )
            )
    return runs


def render_conformance_md(payload: dict[str, Any]) -> str:
    lines = [
        "# Conformance smoke",
        "",
        "| case | status | holes | lint | traces | conformant | violation | timeout | hang_suspect | coverage |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for r in payload["cases"]:
        t = r.get("traces") or {}
        cov = t.get("coverage") or {}
        lines.append(
            f"| {r['case']} | {r.get('status')} | {len(r.get('holes', []))} | "
            f"{(r.get('lint') or {}).get('ok')} | {t.get('traces_total')} | "
            f"{t.get('conformant')} | {t.get('violation')} | {t.get('timeout')} | "
            f"{t.get('hang_suspect')} | {cov.get('sids_seen')}/{cov.get('sids_total')} |"
        )
    lines += [
        "",
        "A conformant trace means every observed concurrency step was a step",
        "the verified model could take. It is not a claim that the code is correct.",
    ]
    return "\n".join(lines) + "\n"


def conform_all(
    program: Path | str,
    traces: list[TraceRun],
    *,
    binary: Path | str | None = None,
    conform_runner: Callable[[Path, Path], dict] | None = None,
    lenient_unlock: bool = False,
    attempt_events: bool = False,
) -> dict[str, Any]:
    """Replay every trace and aggregate conformance."""

    def default_runner(program_path: Path, trace_path: Path) -> dict:
        argv = [str(_binary(binary)), "conform", str(program_path), str(trace_path)]
        if lenient_unlock:
            argv.append("--lenient-unlock")
        if attempt_events:
            argv.append("--attempt-events")
        result = _run(argv, timeout=120.0)
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            return {"status": "error", "detail": result.stderr.strip()}

    runner = conform_runner or default_runner
    program = Path(program).resolve()
    conformant = violation = hang = missing = hang_suspect = 0
    seen = total = 0
    details = []
    for run in traces:
        if run.hang:
            hang += 1
            if run.hang_suspect:
                hang_suspect += 1
            details.append({"kind": run.kind, "index": run.index, "status": "timeout", "hang_suspect": run.hang_suspect})
            continue
        if not run.trace_path:
            missing += 1
            details.append({"kind": run.kind, "index": run.index, "status": "missing_trace"})
            continue
        result = runner(program, Path(run.trace_path))
        status = result.get("status")
        if status == "conformant":
            conformant += 1
            seen = max(seen, int(result.get("coverage", {}).get("sids_seen", 0)))
            total = max(total, int(result.get("coverage", {}).get("sids_total", 0)))
        elif status == "violation":
            violation += 1
        else:
            missing += 1
        details.append({"kind": run.kind, "index": run.index, "status": status, "event_index": result.get("event_index")})
    return {
        "traces_total": len(traces),
        "conformant": conformant,
        "violation": violation,
        "timeout": hang,
        "hang_suspect": hang_suspect,
        "missing": missing,
        "coverage": {"sids_seen": seen, "sids_total": total},
        "details": details,
    }


def run_conformance_smoke(
    cases: list[tuple[str, Path | str]],
    out_dir: Path | str,
    *,
    binary: Path | str | None = None,
    native_runs: int = 50,
    miri_seeds: int = 64,
) -> dict[str, Any]:
    """Generate + build + trace + conform for each (label, program) case."""
    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    binary_path = _binary(binary)
    binary_sha = sha256_bytes(binary_path.read_bytes())
    git_rev = _git_rev(binary_path)
    records = []
    for label, program in cases:
        program = Path(program).resolve()
        case_dir = out / label
        skeleton = case_dir / "skeleton"
        record: dict[str, Any] = {
            "case": label,
            "program": str(program),
            "program_sha256": sha256_text(program.read_text(encoding="utf-8")),
            "binary_sha256": binary_sha,
            "git_rev": git_rev,
        }
        try:
            codegen(program, skeleton, binary=binary)
            holes = holes_of(skeleton)
            type_holes = type_holes_of(skeleton)
            record["holes"] = [h["id"] for h in holes]
            record["type_holes"] = [th["id"] for th in type_holes]
            if holes or type_holes:
                record["status"] = "needs_fill"
                records.append(record)
                continue
            lint = lint_filled(skeleton, skeleton)
            record["lint"] = lint
            traces = collect_traces(
                skeleton,
                native_runs=native_runs,
                miri_seeds=miri_seeds,
                calls_dir=case_dir / "calls",
            )
            aggregate = conform_all(program, traces, binary=binary)
            record["traces"] = aggregate
            record["traces_raw"] = [t.__dict__ for t in traces]
            expected = native_runs + miri_seeds
            record["status"] = (
                "ready"
                if aggregate["conformant"] == expected
                and aggregate["violation"] == 0
                and lint["ok"]
                else "review"
            )
        except Exception as exc:  # noqa: BLE001
            record["status"] = "error"
            record["error"] = f"{type(exc).__name__}: {exc}"
        records.append(record)
    payload = {
        "schema_version": "conformance-v1",
        "binary_sha256": binary_sha,
        "git_rev": git_rev,
        "cases": records,
    }
    (out / "CONFORMANCE.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (out / "CONFORMANCE.md").write_text(render_conformance_md(payload), encoding="utf-8")
    return payload
