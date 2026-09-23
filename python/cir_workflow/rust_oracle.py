"""Bounded Rust-arm oracle: instrument -> build -> run -> monitor (§1).

Pipeline per artifact:

1. ``concir-instrument --wrappers`` rewrites free std-only Rust onto
   ``cir_trace::sync`` wrapper types and emits ``resources.json``.
2. the rewritten program is built as a std-only cargo project;
3. native runs (and optionally Miri seeds) each write one ``cir_trace``
   stream under ``CIR_TRACE_OUT``;
4. ``concir-backend monitor`` checks the contract against the observed states;
5. resource names are auto-mapped to contract FQNs by kind and declaration
   order (reference CIR), then requirement coverage is computed.

The result is a *bounded* verdict; the model arm (`G3_concir`) stays the
exhaustive one.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

from . import bounded_monitor
from .instrument import find_instrument_binary

KINDS = ("Mutex", "Condvar", "Semaphore", "Channel")
CONTAINER = """[package]
name = "probe"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "probe"
path = "src/main.rs"
"""


def _collect_contract_refs(value: Any, resources: list[str], functions: list[str]) -> None:
    if isinstance(value, dict):
        if isinstance(value.get("resource"), str) and value["resource"] not in resources:
            resources.append(value["resource"])
        if isinstance(value.get("function"), str) and value["function"] not in functions:
            functions.append(value["function"])
        for item in value.get("resources", []) if isinstance(value.get("resources"), list) else []:
            if isinstance(item, str) and item not in resources:
                resources.append(item)
        for child in value.values():
            _collect_contract_refs(child, resources, functions)
    elif isinstance(value, list):
        for child in value:
            _collect_contract_refs(child, resources, functions)


def reference_kinds(cir: dict[str, Any]) -> dict[str, str]:
    """FQN -> resource type for a reference CIR, in declaration order."""
    kinds: dict[str, str] = {}
    for module in cir.get("modules", []):
        for res in module.get("resources", []):
            kinds[f"{module['name']}::{res['name']}"] = res.get("type", "")
    return kinds


def auto_mapping(
    rust_resources: list[dict[str, str]],
    contract: dict[str, Any],
    cir: dict[str, Any],
) -> tuple[dict[str, str], dict[str, Any]]:
    """Map runtime resource names to contract FQNs by kind and declaration order."""
    refs: list[str] = []
    functions: list[str] = []
    _collect_contract_refs(contract, refs, functions)
    kinds = reference_kinds(cir)
    mapping: dict[str, str] = {}
    provenance: dict[str, Any] = {"by_kind": {}, "spawns": []}
    for kind in KINDS:
        targets = [f for f in refs if kinds.get(f) == kind]
        sources = [r["name"] for r in rust_resources if r.get("kind") == kind]
        pair = list(zip(sources, targets))
        for src, dst in pair:
            mapping[src] = dst
        if pair:
            provenance["by_kind"][kind] = pair
    spawn_sources = [r["name"] for r in rust_resources if r.get("kind") == "Spawn"]
    worker_targets = [f for f in functions if f.split("::")[-1] != "main"]
    for src, dst in zip(spawn_sources, worker_targets):
        mapping[src] = dst
        provenance["spawns"].append((src, dst))
    return mapping, provenance


def instrument_wrappers(
    source: str, out_dir: Path, *, binary: Path | str | None = None,
    timeout: float = 120.0,
) -> dict[str, Any]:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    source_path = out / "input.rs"
    source_path.write_text(source, encoding="utf-8")
    proc = subprocess.run(
        [str(find_instrument_binary(binary)), str(source_path), "--out", str(out),
         "--wrappers"],
        capture_output=True, text=True, timeout=timeout)
    if proc.returncode != 0:
        raise RuntimeError(f"concir-instrument --wrappers failed: {proc.stderr.strip()}")
    resources = json.loads((out / "resources.json").read_text(encoding="utf-8"))
    sync_path = out / "concir_sync.rs"
    return {
        "annotated": (out / "annotated.rs").read_text(encoding="utf-8"),
        "runtime": (out / "cir_trace.rs").read_text(encoding="utf-8"),
        "sync_runtime": sync_path.read_text(encoding="utf-8") if sync_path.is_file() else None,
        "resources": resources.get("resources", []),
        "limitations": resources.get("limitations", []),
    }


def prepare_project(work_dir: Path, annotated: str, runtime: str,
                    sync_runtime: str | None = None) -> Path:
    work = Path(work_dir)
    src = work / "src"
    src.mkdir(parents=True, exist_ok=True)
    (work / "Cargo.toml").write_text(CONTAINER, encoding="utf-8")
    (src / "main.rs").write_text(annotated, encoding="utf-8")
    (src / "cir_trace.rs").write_text(runtime, encoding="utf-8")
    if sync_runtime is not None:
        (src / "concir_sync.rs").write_text(sync_runtime, encoding="utf-8")
    return work


def cargo_build(work_dir: Path, *, timeout: float = 600.0,
                offline: bool = True) -> tuple[bool, str]:
    argv = ["cargo", "build"] + (["--offline"] if offline else [])
    proc = subprocess.run(argv, cwd=work_dir, capture_output=True, text=True,
                          timeout=timeout)
    log = (proc.stdout + proc.stderr)[-4000:]
    return proc.returncode == 0, log


def run_native(work_dir: Path, traces_dir: Path, *, n: int = 32,
               timeout: float = 30.0) -> list[dict[str, Any]]:
    traces_dir.mkdir(parents=True, exist_ok=True)
    binary = work_dir / "target/debug/probe"
    runs: list[dict[str, Any]] = []
    for index in range(n):
        trace = traces_dir / f"native-{index:03d}.jsonl"
        env = dict(os.environ, CIR_TRACE_OUT=str(trace))
        try:
            proc = subprocess.run([str(binary)], cwd=work_dir, env=env,
                                  capture_output=True, text=True, timeout=timeout)
            completed = proc.returncode == 0
            timed_out = False
        except subprocess.TimeoutExpired:
            completed, timed_out = False, True
        runs.append({"index": index, "source": "native", "completed": completed,
                     "timed_out": timed_out, "trace": str(trace),
                     "trace_exists": trace.is_file()})
        if timed_out:
            # A hang is already conclusive for behavior; one run is enough.
            break
    return runs


def run_miri(work_dir: Path, traces_dir: Path, *, seeds: int = 16,
             toolchain: str | None = None, timeout: float = 300.0) -> list[dict[str, Any]]:
    traces_dir.mkdir(parents=True, exist_ok=True)
    runs: list[dict[str, Any]] = []
    base = ["cargo"] + ([f"+{toolchain}"] if toolchain else []) + ["miri", "run", "--offline"]
    for seed in range(seeds):
        trace = traces_dir / f"miri-{seed:03d}.jsonl"
        env = dict(os.environ, CIR_TRACE_OUT=str(trace),
                   MIRIFLAGS=f"-Zmiri-seed={seed} -Zmiri-disable-isolation")
        try:
            proc = subprocess.run(base, cwd=work_dir, env=env, capture_output=True,
                                  text=True, timeout=timeout)
            completed = proc.returncode == 0
            timed_out = False
        except subprocess.TimeoutExpired:
            completed, timed_out = False, True
        runs.append({"index": seed, "source": "miri", "completed": completed,
                     "timed_out": timed_out, "trace": str(trace),
                     "trace_exists": trace.is_file()})
    return runs


def evaluate(
    source: str,
    contract_path: Path,
    reference_cir_path: Path,
    work_dir: Path,
    *,
    n_native: int = 32,
    miri_seeds: int = 0,
    miri_toolchain: str | None = None,
    binary: Path | str | None = None,
    instrument_binary: Path | str | None = None,
    run_timeout: float = 30.0,
) -> dict[str, Any]:
    contract = json.loads(Path(contract_path).read_text(encoding="utf-8"))
    cir = json.loads(Path(reference_cir_path).read_text(encoding="utf-8"))
    wrapped = instrument_wrappers(source, work_dir / "instrument",
                                  binary=instrument_binary)
    prepare_project(work_dir, wrapped["annotated"], wrapped["runtime"])
    built, build_log = cargo_build(work_dir)
    result: dict[str, Any] = {
        "built": built, "build_log_tail": build_log[-1200:],
        "resources": wrapped["resources"], "limitations": wrapped["limitations"],
    }
    if not built:
        result["status"] = "build_failed"
        return result

    traces_dir = work_dir / "traces"
    runs = run_native(work_dir, traces_dir, n=n_native, timeout=run_timeout)
    if miri_seeds:
        runs += run_miri(work_dir, traces_dir, seeds=miri_seeds,
                         toolchain=miri_toolchain, timeout=max(run_timeout, 120.0))
    result["runs"] = runs
    behavior_ok = all(run["completed"] for run in runs) if runs else False
    hang = any(run["timed_out"] for run in runs)
    result["behavior_ok"] = behavior_ok
    result["hang"] = hang

    mapping, provenance = auto_mapping(wrapped["resources"], contract, cir)
    (work_dir / "mapping.json").write_text(
        json.dumps({"mapping": mapping, "provenance": provenance}, indent=2) + "\n",
        encoding="utf-8")
    report = bounded_monitor.run_monitor(
        contract_path, traces_dir, resources=work_dir / "instrument/resources.json",
        mapping=work_dir / "mapping.json", binary=binary)
    result["monitor"] = report
    result["mapping"] = mapping
    result["mapping_provenance"] = provenance
    requirements_path = Path(contract_path).parent / "generation_input/requirements.json"
    if requirements_path.is_file():
        reqjson = json.loads(requirements_path.read_text(encoding="utf-8"))
        cov = bounded_monitor.coverage(contract, report, reqjson["requirements"],
                                       reqjson["unverifiable"], behavior_ok=behavior_ok)
        result["coverage"] = cov.as_dict()
    result["status"] = "ok"
    return result
