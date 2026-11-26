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


from ..core.toolchain import find_concir_backend, resolve_binary_path, resolve_exe_in_dir


def _binary(explicit: Path | str | None = None) -> Path:
    if explicit:
        return resolve_binary_path(explicit)
    found = find_concir_backend()
    if found is not None:
        return found
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


def holes_of(skeleton_dir: Path | str) -> list[dict[str, Any]]:
    map_path = Path(skeleton_dir) / "codegen.json"
    if not map_path.is_file():
        return []
    data = json.loads(map_path.read_text(encoding="utf-8"))
    return list(data.get("holes", []))


def type_holes_of(skeleton_dir: Path | str) -> list[dict[str, Any]]:
    map_path = Path(skeleton_dir) / "codegen.json"
    if not map_path.is_file():
        return []
    data = json.loads(map_path.read_text(encoding="utf-8"))
    return list(data.get("type_holes", []))


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
    """Build the hole-filling prompt (multi-file SeqHoles + TypeHoles with context)."""

    root = Path(skeleton_dir)
    file_cache: dict[str, list[str]] = {}

    def _read_lines(rel_file: str) -> list[str]:
        if rel_file not in file_cache:
            path = root / rel_file
            file_cache[rel_file] = (
                path.read_text(encoding="utf-8").splitlines() if path.is_file() else []
            )
        return file_cache[rel_file]

    parts = [
        "You are filling only the marked holes in a Rust skeleton.",
        "Rules:",
        "- Replace ONLY the text inside a `/* HOLE(id) expected: T */` marker's",
        "  placeholder (or `/* TYPE_HOLE(id) ... */` struct in `src/domain.rs`);",
        "  do not touch any other line, statement, or `// @cir` sid comment.",
        "- SeqHole code is sequential local computation only. It must not introduce",
        "  any synchronization, thread, channel, atomic, unsafe, `cir_trace`,",
        "  unbounded loop, or new `use`.",
        "- TypeHole code in `src/domain.rs` must define a pure `pub struct <Name>`",
        "  implementing `Send + Sync + Clone + Default + 'static` (pure `impl`",
        "  methods and bounded `for` loops allowed; no sync, threads, atomics,",
        "  unsafe, `cir_trace`, or unbounded `loop`/`while`).",
        "- Return a JSON object mapping each hole / type_hole id to the exact Rust",
        "  code to substitute.",
        "",
        f"Specification:\n{requirements}",
        "",
    ]
    for th in type_holes_of(root):
        th_id = th["id"]
        rel_file = th.get("file") or "src/domain.rs"
        source = _read_lines(rel_file)
        idx = next((i for i, l in enumerate(source) if f"TYPE_HOLE({th_id})" in l), None)
        if idx is None:
            continue
        lo, hi = max(0, idx - context_lines), min(len(source), idx + context_lines + 1)
        type_name = th.get("type_name") or ""
        parts.append(f"### type_hole {th_id} (type {type_name} in {rel_file})")
        parts.append("```rust")
        parts.extend(f"{n+1:>4}: {source[n]}" for n in range(lo, hi))
        parts.append("```")
        parts.append("")
    for hole in holes_of(root):
        hid = hole["id"]
        rel_file = hole.get("file") or "src/main.rs"
        source = _read_lines(rel_file)
        idx = next((i for i, l in enumerate(source) if f"HOLE({hid})" in l), None)
        if idx is None:
            continue
        lo, hi = max(0, idx - context_lines), min(len(source), idx + context_lines + 1)
        parts.append(f"### hole {hid} (expected {hole['expected']} in {rel_file})")
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
    """Substitute SeqHole and TypeHole code into a copy of the skeleton (no LLM here)."""

    from .static_verify import TYPE_HOLE_PLACEHOLDER_RE, _list_src_rs_files

    skeleton = Path(skeleton_dir).expanduser().resolve()
    filled = Path(filled_dir).expanduser().resolve()
    if filled.exists():
        shutil.rmtree(filled)
    shutil.copytree(skeleton, filled)

    rs_files = _list_src_rs_files(filled)
    applied: list[str] = []
    missing: list[str] = []
    for hole in holes_of(skeleton):
        hid = hole["id"]
        if hid not in fills:
            missing.append(hid)
            continue
        preferred = str(hole.get("file") or "src/main.rs")
        candidate_files = [preferred] + [f for f in rs_files if f != preferred]
        placeholder = f"/* HOLE({hid}) expected: {hole['expected']} */ Default::default()"
        pattern = re.compile(
            rf"/\* HOLE\({re.escape(hid)}\) expected: .*? \*/ Default::default\(\)"
        )
        matched = False
        for rel in candidate_files:
            path = filled / rel
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
            if placeholder in text:
                path.write_text(text.replace(placeholder, fills[hid], 1), encoding="utf-8")
                applied.append(hid)
                matched = True
                break
            new_text, count = pattern.subn(lambda _m: fills[hid], text, count=1)
            if count:
                path.write_text(new_text, encoding="utf-8")
                applied.append(hid)
                matched = True
                break
        if not matched:
            missing.append(hid)

    known_type_holes = type_holes_of(skeleton)
    if not known_type_holes:
        domain_rs = filled / "src/domain.rs"
        if domain_rs.is_file():
            for m in TYPE_HOLE_PLACEHOLDER_RE.finditer(domain_rs.read_text(encoding="utf-8")):
                known_type_holes.append({
                    "id": m.group("id"),
                    "type_name": m.group("type_name"),
                    "file": "src/domain.rs",
                })

    applied_type_holes: list[str] = []
    missing_type_holes: list[str] = []
    for th in known_type_holes:
        th_id = str(th.get("id") or "")
        if not th_id:
            continue
        if th_id not in fills:
            missing_type_holes.append(th_id)
            continue
        rel_file = str(th.get("file") or "src/domain.rs")
        target_path = filled / rel_file
        if not target_path.is_file():
            missing_type_holes.append(th_id)
            continue
        text = target_path.read_text(encoding="utf-8")
        pattern = re.compile(
            rf"/\* TYPE_HOLE\({re.escape(th_id)}\).*? \*/"
            r"\s*(?:#\[[^\]]*\]\s*)*"
            r"pub\s+struct\s+[A-Za-z_][A-Za-z0-9_]*\s*\{\s*\}",
            re.DOTALL,
        )
        new_text, count = pattern.subn(lambda _m: fills[th_id].strip("\r\n"), text, count=1)
        if count:
            target_path.write_text(new_text, encoding="utf-8")
            applied_type_holes.append(th_id)
        else:
            missing_type_holes.append(th_id)

    return {
        "applied": applied,
        "missing": missing,
        "applied_type_holes": applied_type_holes,
        "missing_type_holes": missing_type_holes,
    }


def lint_filled(skeleton_dir: Path | str, filled_dir: Path | str) -> dict[str, Any]:
    """Multi-file, multi-line structural + purity diff across ``src/*.rs`` and ``TypeHole``s."""

    from .static_verify import (
        _FORBIDDEN_RULES,
        _TYPE_HOLE_FORBIDDEN_RULES,
        HOLE_MARKER_RE,
        SID_ANNOTATION_RE,
        TYPE_HOLE_MARKER_RE,
        TYPE_HOLE_PLACEHOLDER_RE,
        _align_filled_to_skeleton,
        _align_filled_to_type_hole_skeleton,
        _first_diff_line,
        _is_explicitly_bounded_loop,
        _list_src_rs_files,
        _offset_to_line_range,
        _sid_multiset,
        _unwrap_hole_markers,
        _unwrap_type_hole_markers,
        strip_rust_comments_and_literals,
    )

    skel_root = Path(skeleton_dir).expanduser().resolve()
    fill_root = Path(filled_dir).expanduser().resolve()
    skel_files = _list_src_rs_files(skel_root)
    fill_files = _list_src_rs_files(fill_root)
    violations: list[dict[str, Any]] = []

    for extra in sorted(set(fill_files) - set(skel_files)):
        violations.append({
            "kind": "changed_outside_hole",
            "file": extra,
            "line": 1,
            "detail": f"unexpected source file added outside skeleton: {extra}",
        })
    for missing_f in sorted(set(skel_files) - set(fill_files)):
        violations.append({
            "kind": "changed_outside_hole",
            "file": missing_f,
            "line": 1,
            "detail": f"skeleton source file removed: {missing_f}",
        })

    for rel in skel_files:
        if rel not in fill_files:
            continue
        skel_text = (skel_root / rel).read_text(encoding="utf-8")
        fill_text = (fill_root / rel).read_text(encoding="utf-8")

        s_sids = _sid_multiset(skel_text)
        f_sids = _sid_multiset(fill_text)
        if s_sids != f_sids:
            violations.append({
                "kind": "sid_count",
                "file": rel,
                "detail": {"skeleton": s_sids, "filled": f_sids},
            })

        has_type_holes = bool(TYPE_HOLE_PLACEHOLDER_RE.search(skel_text))
        has_seq_holes = bool(PLACEHOLDER_RE.search(skel_text))

        if has_type_holes:
            _th_ok, extracted_th, th_errs = _align_filled_to_type_hole_skeleton(
                skel_text, fill_text
            )
            for err in th_errs:
                violations.append({
                    "kind": "changed_outside_hole",
                    "file": rel,
                    "line": err["line"],
                    "detail": err["message"],
                })
            th_to_scan = list(extracted_th)
            if not th_to_scan and TYPE_HOLE_MARKER_RE.search(fill_text):
                for m in TYPE_HOLE_MARKER_RE.finditer(fill_text):
                    s, e = _offset_to_line_range(fill_text, m.start(), m.end())
                    th_to_scan.append({
                        "id": m.group("id"),
                        "body": m.group(0),
                        "start_line": s,
                        "end_line": e,
                    })
            for item in th_to_scan:
                th_id = item["id"]
                inner_body = _unwrap_type_hole_markers(th_id, item["body"])
                start_line = int(item["start_line"])
                if SID_ANNOTATION_RE.search(inner_body):
                    violations.append({
                        "kind": "forbidden_in_hole",
                        "file": rel,
                        "hole_id": th_id,
                        "line": start_line,
                        "detail": f"type hole '{th_id}' must not contain // @cir annotations",
                    })
                stripped = strip_rust_comments_and_literals(inner_body)
                for kind, _desc, pattern in _TYPE_HOLE_FORBIDDEN_RULES:
                    for m in pattern.finditer(stripped):
                        rel_line = stripped.count("\n", 0, m.start())
                        if kind == "loop_in_type_hole" and _is_explicitly_bounded_loop(inner_body, rel_line):
                            continue
                        violations.append({
                            "kind": "forbidden_in_hole",
                            "file": rel,
                            "hole_id": th_id,
                            "line": start_line + rel_line,
                            "detail": item["body"].strip(),
                        })
                        break
                    else:
                        continue
                    break

        if has_seq_holes:
            _ok, extracted_holes, structural_errors = _align_filled_to_skeleton(
                skel_text, fill_text
            )
            for err in structural_errors:
                violations.append({
                    "kind": "changed_outside_hole",
                    "file": rel,
                    "line": err["line"],
                    "detail": err["message"],
                })
            holes_to_scan = list(extracted_holes)
            if not holes_to_scan and HOLE_MARKER_RE.search(fill_text):
                for m in HOLE_MARKER_RE.finditer(fill_text):
                    s, e = _offset_to_line_range(fill_text, m.start(), m.end())
                    holes_to_scan.append({
                        "id": m.group("id"),
                        "body": m.group(0),
                        "start_line": s,
                        "end_line": e,
                    })
            for item in holes_to_scan:
                hid = item["id"]
                inner_body = _unwrap_hole_markers(hid, item["body"])
                start_line = int(item["start_line"])
                if SID_ANNOTATION_RE.search(inner_body):
                    violations.append({
                        "kind": "forbidden_in_hole",
                        "file": rel,
                        "hole_id": hid,
                        "line": start_line,
                        "detail": f"hole '{hid}' must not contain // @cir annotations",
                    })
                stripped = strip_rust_comments_and_literals(inner_body)
                for _kind, _desc, pattern in _FORBIDDEN_RULES:
                    m = pattern.search(stripped)
                    if m:
                        rel_line = stripped.count("\n", 0, m.start())
                        violations.append({
                            "kind": "forbidden_in_hole",
                            "file": rel,
                            "hole_id": hid,
                            "line": start_line + rel_line,
                            "detail": inner_body.strip(),
                        })
                        break

        if not has_type_holes and not has_seq_holes and fill_text != skel_text:
            diff_line = _first_diff_line(skel_text, fill_text)
            violations.append({
                "kind": "changed_outside_hole",
                "file": rel,
                "line": diff_line,
                "detail": f"{rel} modified outside holes near line {diff_line}",
            })

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
    # Resolve so CIR_TRACE_OUT is absolute: the child runs with cwd=root, so a
    # relative trace path would be written under the skeleton instead.
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
            type_holes = type_holes_of(skeleton)
            record["holes"] = [h["id"] for h in holes]
            record["type_holes"] = [th["id"] for th in type_holes]
            if holes or type_holes:
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
                conform_runner: Callable[[Path, Path], dict] | None = None,
                lenient_unlock: bool = False,
                attempt_events: bool = False) -> dict[str, Any]:
    """Replay every trace and aggregate conformance.

    ``lenient_unlock`` is the extraction-mode relaxation: a model
    ``mutex_unlock`` statement may be taken silently (Rust guard drop). It is
    never used for codegen-mode conformance.
    """

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
