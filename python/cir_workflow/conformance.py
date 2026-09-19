"""Code-level post-verification (conformance) for the skeleton workflow.

Pipeline: ConcIR `codegen` emits a sid-annotated, std-only Rust skeleton with
`// HOLE(id)` placeholders for sequential local computation. The LLM may only
fill the holes (``fill_holes``); ``lint_filled`` enforces that nothing outside a
hole changed and that hole code contains no synchronization/thread/unsafe
constructs. Traces from native runs and Miri are replayed by ConcIR `conform`,
which checks that every observed concurrency statement is a step the verified
model could take.

The conclusion is only ever "every observed execution is a model execution"
plus a static lint result; never "the code is correct".
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

HOLE_RE = re.compile(r"/\* HOLE\((?P<id>[A-Za-z0-9_]+)\) expected: (?P<expected>.*?) \*/")
PLACEHOLDER_RE = re.compile(
    r"/\* HOLE\((?P<id>[A-Za-z0-9_]+)\) expected: (?P<expected>.*?) \*/ Default::default\(\)")
FORBIDDEN = re.compile(
    r"std::sync|std::thread|Mutex|Condvar|RwLock|channel|Semaphore|"
    r"thread::spawn|scope\(|\.lock\(|\.wait|notify|\.send\(|\.recv\(|Atomic|"
    r"unsafe|cir_trace")


def _binary(explicit: Path | str | None = None) -> Path:
    if explicit:
        return Path(explicit).expanduser().resolve()
    env = os.environ.get("CONCIR_BACKEND")
    candidates = [Path(env)] if env else []
    repo = Path(__file__).resolve().parents[2]
    candidates += [repo.parent / "ConcIR/target/release/concir-backend",
                   repo.parent / "ConcIR/target/debug/concir-backend"]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError("concir-backend not found (set CONCIR_BACKEND)")


def _run(argv: list[str], *, cwd: Path | None = None, timeout: float = 300.0,
         env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
    full_env = dict(os.environ)
    if env:
        full_env.update(env)
    return subprocess.run(argv, cwd=str(cwd) if cwd else None, env=full_env,
                          capture_output=True, text=True, timeout=timeout)


@dataclass
class HoleFill:
    hole_id: str
    expected: str
    code: str


def holes_of(skeleton_dir: Path | str) -> list[dict[str, str]]:
    map_path = Path(skeleton_dir) / "codegen.json"
    data = json.loads(map_path.read_text(encoding="utf-8"))
    return list(data.get("holes", []))


def codegen(program: Path | str, out_dir: Path | str, *,
            binary: Path | str | None = None) -> dict[str, Any]:
    out = Path(out_dir).expanduser().resolve()
    result = _run([str(_binary(binary)), "codegen", str(Path(program).resolve()),
                   "--out", str(out)], timeout=120.0)
    if result.returncode != 0:
        raise RuntimeError(f"codegen failed: {result.stderr.strip() or result.stdout.strip()}")
    return json.loads(result.stdout)


def fill_prompt(skeleton_dir: Path | str, requirements: str, *,
                context_lines: int = 8) -> str:
    """Build the hole-filling prompt (holes + surrounding 8 lines)."""

    root = Path(skeleton_dir)
    source = (root / "src/main.rs").read_text(encoding="utf-8").splitlines()
    parts = [
        "You are filling only the marked holes in a Rust skeleton.",
        "Rules:",
        "- Replace ONLY the text inside a `/* HOLE(id) expected: T */` marker's",
        "  placeholder; do not touch any other line, statement, or sid comment.",
        "- Hole code is sequential local computation only. It must not introduce",
        "  any synchronization, thread, channel, atomic, unsafe, or `cir_trace`",
        "  construct, and no new `use`.",
        "- Return a JSON object mapping each hole id to the exact Rust expression",
        "  or statement block to substitute.",
        "",
        f"Specification:\n{requirements}",
        "",
    ]
    for hole in holes_of(root):
        hid = hole["id"]
        # locate the line containing this hole
        idx = next((i for i, l in enumerate(source) if f"HOLE({hid})" in l), None)
        if idx is None:
            continue
        lo, hi = max(0, idx - context_lines), min(len(source), idx + context_lines + 1)
        parts.append(f"### hole {hid} (expected {hole['expected']})")
        parts.append("```rust")
        parts.extend(f"{n+1:>4}: {source[n]}" for n in range(lo, hi))
        parts.append("```")
        parts.append("")
    parts.append("Reply with only the JSON object, e.g. "
                 '{"h1": "x + 1", "h2": "if flag { 1 } else { 0 }"}.')
    return "\n".join(parts)


def parse_fill(text: str) -> dict[str, str]:
    """Extract ``{hole_id: code}`` from a model reply (JSON object, fenced ok)."""

    stripped = text.strip()
    if "```" in stripped:
        parts = stripped.split("```")
        stripped = parts[1] if len(parts) > 1 else stripped
        if stripped.lstrip().lower().startswith("json"):
            stripped = stripped.lstrip()[4:]
    try:
        data = json.loads(stripped)
    except json.JSONDecodeError:
        return {}
    if not isinstance(data, dict):
        return {}
    return {str(k): str(v) for k, v in data.items()}


def fill_holes(skeleton_dir: Path | str, filled_dir: Path | str, fills: dict[str, str]) -> dict[str, Any]:
    """Substitute hole codes into a copy of the skeleton (no LLM here)."""

    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if filled.exists():
        shutil.rmtree(filled)
    shutil.copytree(skeleton, filled)
    applied = []
    for hole in holes_of(skeleton):
        hid = hole["id"]
        if hid not in fills:
            continue
        for name in ("src/main.rs",):
            path = filled / name
            text = path.read_text(encoding="utf-8")
            text = text.replace(
                f"/* HOLE({hid}) expected: {hole['expected']} */ Default::default()",
                fills[hid])
            path.write_text(text, encoding="utf-8")
        applied.append(hid)
    return {"applied": applied, "missing": [h["id"] for h in holes_of(skeleton)
                                            if h["id"] not in fills]}


def lint_filled(skeleton_dir: Path | str, filled_dir: Path | str) -> dict[str, Any]:
    """Line-level diff: only HOLE placeholders may change; no forbidden code."""

    skel = Path(skeleton_dir).expanduser().resolve() / "src/main.rs"
    fill = Path(filled_dir).expanduser().resolve() / "src/main.rs"
    s_lines = skel.read_text(encoding="utf-8").splitlines()
    f_lines = fill.read_text(encoding="utf-8").splitlines()
    violations: list[dict[str, Any]] = []
    if len(s_lines) != len(f_lines):
        violations.append({"kind": "line_count",
                           "detail": f"{len(s_lines)} -> {len(f_lines)}"})
    for i in range(min(len(s_lines), len(f_lines))):
        s, f = s_lines[i], f_lines[i]
        if s == f:
            continue
        match = PLACEHOLDER_RE.search(s)
        if match:
            # the only allowed change is the placeholder replacement
            prefix = s[:match.start()]
            suffix = s[match.end():]
            if f.startswith(prefix) and f.endswith(suffix) and len(f) >= len(prefix) + len(suffix):
                body = f[len(prefix):len(f) - len(suffix) if suffix else len(f)]
                if FORBIDDEN.search(body):
                    violations.append({"kind": "forbidden_in_hole", "line": i + 1,
                                       "detail": body.strip()})
            else:
                violations.append({"kind": "hole_line_rewritten", "line": i + 1,
                                   "detail": f})
        else:
            violations.append({"kind": "changed_outside_hole", "line": i + 1,
                               "detail": f})
    # the multiset of sid annotations must be unchanged from the skeleton
    sid_re = re.compile(r"// @cir (\S+)")

    def sid_multiset(lines: list[str]) -> dict[str, int]:
        counts: dict[str, int] = {}
        for line in lines:
            m = sid_re.search(line)
            if m:
                counts[m.group(1)] = counts.get(m.group(1), 0) + 1
        return counts

    if sid_multiset(s_lines) != sid_multiset(f_lines):
        violations.append({"kind": "sid_count",
                           "detail": {"skeleton": sid_multiset(s_lines),
                                      "filled": sid_multiset(f_lines)}})
    return {"ok": not violations, "violations": violations}


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


def _sha(text: str) -> str:
    import hashlib

    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _sha_bytes(data: bytes) -> str:
    import hashlib

    return hashlib.sha256(data).hexdigest()


def _git_rev(binary: Path) -> str:
    try:
        repo = binary.resolve().parents[2]
        out = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                             capture_output=True, text=True, timeout=10)
        if out.returncode == 0:
            return out.stdout.strip()
    except Exception:  # noqa: BLE001
        pass
    return "unknown"


def collect_traces(filled_dir: Path | str, *, native_runs: int = 50,
                   miri_seeds: int = 64, timeout_s: float = 10.0,
                   calls_dir: Path | str | None = None,
                   run_miri: bool = True) -> list[TraceRun]:
    """Build the filled project, run it N times plus oneMiri many-seeds pass."""

    import time

    root = Path(filled_dir).expanduser().resolve()
    calls = Path(calls_dir) if calls_dir else root / "calls"
    calls.mkdir(parents=True, exist_ok=True)
    build = _run(["cargo", "build", "--offline", "--quiet"], cwd=root, timeout=300.0)
    (calls / "build.stdout.txt").write_text(build.stdout, encoding="utf-8")
    (calls / "build.stderr.txt").write_text(build.stderr, encoding="utf-8")
    if build.returncode != 0:
        raise RuntimeError("filled project failed to build")

    runs: list[TraceRun] = []
    binary = root / "target/debug/cir_generated"
    for i in range(native_runs):
        trace = calls / f"native-{i:03d}.jsonl"
        started = time.monotonic()
        try:
            proc = _run([str(binary)], cwd=root, timeout=timeout_s,
                        env={"CIR_TRACE_OUT": str(trace)})
            hang = False
            code = proc.returncode
            out, err = proc.stdout, proc.stderr
        except subprocess.TimeoutExpired:
            hang = True
            code = None
            out, err = "", ""
        wall = int((time.monotonic() - started) * 1000)
        runs.append(TraceRun(
            kind="native", index=i, trace_path=str(trace) if trace.is_file() else None,
            hang=hang, exit_code=code, wall_ms=wall,
            stdout_sha256=_sha(out), stderr_sha256=_sha(err)))
    if run_miri:
        # One trace per seed: run each seed in its own process so the trace file
        # is not overwritten and each seed is an independent observation.
        for seed in range(miri_seeds):
            trace = calls / f"miri-{seed:03d}.jsonl"
            started = time.monotonic()
            # Miri caches the build/run environment; a clean before each seed is
            # what makes `CIR_TRACE_OUT` take effect per seed.
            _run(["cargo", "clean"], cwd=root, timeout=120.0)
            try:
                proc = _run(["cargo", "miri", "run", "--offline", "--quiet"], cwd=root,
                            timeout=max(timeout_s, 120.0),
                            env={"CIR_TRACE_OUT": str(trace),
                                 "MIRIFLAGS": f"-Zmiri-seed={seed} "
                                              "-Zmiri-preemption-rate=0.5 "
                                              "-Zmiri-disable-isolation"})
                hang = False
                code = proc.returncode
                out, err = proc.stdout, proc.stderr
            except subprocess.TimeoutExpired:
                hang = True
                code = None
                out, err = "", ""
            wall = int((time.monotonic() - started) * 1000)
            runs.append(TraceRun(
                kind="miri", index=seed,
                trace_path=str(trace) if trace.is_file() else None,
                hang=hang, exit_code=code, wall_ms=wall,
                stdout_sha256=_sha(out), stderr_sha256=_sha(err),
                hang_suspect=hang))
    return runs


def run_conformance_smoke(cases: list[tuple[str, Path | str]], out_dir: Path | str, *,
                          binary: Path | str | None = None, native_runs: int = 50,
                          miri_seeds: int = 64) -> dict[str, Any]:
    """Generate + build + trace + conform for each (label, program) case.

    Cases with holes are recorded as ``needs_fill`` (the offline runner cannot
    fill them); no-hole cases must reach 100% conformance.
    """

    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    binary_path = _binary(binary)
    binary_sha = _sha_bytes(binary_path.read_bytes())
    git_rev = _git_rev(binary_path)
    records = []
    for label, program in cases:
        program = Path(program).resolve()
        case_dir = out / label
        skeleton = case_dir / "skeleton"
        record: dict[str, Any] = {"case": label, "program": str(program),
                                  "program_sha256": _sha(program.read_text(encoding="utf-8")),
                                  "binary_sha256": binary_sha, "git_rev": git_rev}
        try:
            codegen(program, skeleton, binary=binary)
            holes = holes_of(skeleton)
            record["holes"] = [h["id"] for h in holes]
            if holes:
                record["status"] = "needs_fill"
                records.append(record)
                continue
            lint = lint_filled(skeleton, skeleton)
            record["lint"] = lint
            traces = collect_traces(skeleton, native_runs=native_runs,
                                    miri_seeds=miri_seeds,
                                    calls_dir=case_dir / "calls")
            aggregate = conform_all(program, traces, binary=binary)
            record["traces"] = aggregate
            record["traces_raw"] = [t.__dict__ for t in traces]
            expected = native_runs + miri_seeds
            record["status"] = (
                "ready" if aggregate["conformant"] == expected
                and aggregate["violation"] == 0 and lint["ok"] else "review")
        except Exception as exc:  # noqa: BLE001
            record["status"] = "error"
            record["error"] = f"{type(exc).__name__}: {exc}"
        records.append(record)
    payload = {"schema_version": "conformance-v1", "binary_sha256": binary_sha,
               "git_rev": git_rev, "cases": records}
    (out / "CONFORMANCE.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (out / "CONFORMANCE.md").write_text(render_conformance_md(payload), encoding="utf-8")
    return payload


def render_conformance_md(payload: dict[str, Any]) -> str:
    lines = ["# Conformance smoke", "",
             "| case | status | holes | lint | traces | conformant | violation | timeout | hang_suspect | coverage |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for r in payload["cases"]:
        t = r.get("traces") or {}
        cov = t.get("coverage") or {}
        lines.append(
            f"| {r['case']} | {r.get('status')} | {len(r.get('holes', []))} | "
            f"{(r.get('lint') or {}).get('ok')} | {t.get('traces_total')} | "
            f"{t.get('conformant')} | {t.get('violation')} | {t.get('timeout')} | "
            f"{t.get('hang_suspect')} | {cov.get('sids_seen')}/{cov.get('sids_total')} |")
    lines += ["", "A conformant trace means every observed concurrency step was a step",
              "the verified model could take. It is not a claim that the code is correct."]
    return "\n".join(lines) + "\n"


def conform_all(program: Path | str, traces: list[TraceRun], *,
                binary: Path | str | None = None,
                conform_runner: Callable[[Path, Path], dict] | None = None) -> dict[str, Any]:
    """Replay every trace and aggregate conformance."""

    def default_runner(program_path: Path, trace_path: Path) -> dict:
        result = _run([str(_binary(binary)), "conform", str(program_path),
                       str(trace_path)], timeout=120.0)
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
            details.append({"kind": run.kind, "index": run.index, "status": "timeout",
                            "hang_suspect": run.hang_suspect})
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
        details.append({"kind": run.kind, "index": run.index, "status": status,
                        "event_index": result.get("event_index")})
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
