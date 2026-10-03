#!/usr/bin/env python3
"""Portable source-only artifact runner. Live commands incur API charges."""
from __future__ import annotations
import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "python"))
TOOLCHAIN = "nightly-2026-09-04"


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def run(argv, **kwargs):
    subprocess.run([str(x) for x in argv], check=True, **kwargs)


def tools():
    from cir_workflow.toolchain import resolve
    tc = resolve(os.environ.get("CONCIR_TOOLCHAIN") or ROOT / ".build/backend/debug")
    tc.validate()
    os.environ.update(tc.as_env())
    os.environ["CONCIR_TOOLCHAIN"] = str(tc.backend.parent)
    os.environ.pop("CARGO_TARGET_DIR", None)
    os.environ.setdefault("RUSTUP_TOOLCHAIN", TOOLCHAIN)
    return tc


def verify(args):
    count = 0
    for task in read(ROOT / "benchmarks/MANIFEST.json")["tasks"]:
        for name, expected in task.get("files", {}).items():
            path = ROOT / "benchmarks" / name
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                raise RuntimeError(f"benchmark hash mismatch: {name}")
            count += 1
    from cir_workflow.generation import load_gen_tasks
    tasks = load_gen_tasks(ROOT)
    for t in tasks:
        for path in (t.contract_path, t.reference_cir_path):
            if not path.is_file(): raise RuntimeError(f"missing benchmark input: {path}")
    hashes = ROOT / "SOURCE_SHA256.json"
    if hashes.exists():
        for name, expected in read(hashes).items():
            path = ROOT / name
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
                raise RuntimeError(f"source hash mismatch: {name}")
    print(json.dumps({"benchmark_files_checked": count, "generation_tasks": len(tasks),
                      "tiers": dict(Counter(t.tier for t in tasks))}, indent=2))


def build(args):
    argv = ["cargo", f"+{TOOLCHAIN}", "build", "--locked", "--bins", "--manifest-path",
            ROOT / "ConcIR/Cargo.toml", "--target-dir", ROOT / ".build/backend"]
    if args.offline: argv.append("--offline")
    run(argv, cwd=ROOT)
    tools()


def plan(args):
    from cir_workflow.multi_gen import load_config, matrix_from_config
    cfg = load_config(args.config)
    cells = matrix_from_config(cfg)
    valid = {t["task"] for t in read(ROOT / "benchmarks/GENERATION_MANIFEST.json")["tasks"]}
    if any(c["task"] not in valid for c in cells): raise RuntimeError("unknown generation task")
    print(json.dumps({"models": [m["model_id"] for m in cfg["models"]], "tasks": len(cfg["tasks"]),
                      "repetitions": cfg["reps"], "arms": cfg["arms"], "arm_cells": len(cells),
                      "concurrency": cfg["concurrency"], "rounds": cfg["rounds"],
                      "maximum_physical_requests": cfg["global_physical_cap"],
                      "network_requests": 0}, indent=2))


def baseline_tools():
    from cir_workflow.rust_arm import lockbud_path
    if lockbud_path() is None: raise RuntimeError("G2 requires Lockbud; see README.md and LOCKBUD_BIN")
    run(["cargo", f"+{TOOLCHAIN}", "miri", "--version"], cwd=ROOT)


def generate(args):
    from cir_workflow.multi_gen import load_config
    from cir_workflow.env import load_dotenv
    from cir_workflow.transport import CHANNELS
    from cir_workflow.v23_specs import config_spec
    load_dotenv(ROOT / ".env")
    cfg = load_config(args.config)
    tc = tools()
    if "G2_tools_iter" in cfg["arms"]: baseline_tools()
    if importlib.util.find_spec("openai") is None: raise RuntimeError("Install python/ first (see README.md)")
    for entry in cfg["models"]:
        name = CHANNELS[config_spec(entry).channel].api_key_env
        if not os.environ.get(name): raise RuntimeError(f"missing credential: {name}")
    run([sys.executable, ROOT / "scripts/v23_live.py", Path(args.config).resolve(),
         "--toolchain-dir", tc.backend.parent], cwd=ROOT,
        env=dict(os.environ, V23_OUT=str(Path(args.out).resolve())))


def smoke(args):
    from cir_workflow.concir_client import ConcirClient
    from cir_workflow.conformance import codegen, collect_traces, conform_all
    tc = tools()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=False)
    case = ROOT / "benchmarks/families/lock-order/abba_2lock"
    client = ConcirClient(tc.backend, workdir=out / "calls")
    for engine in ("petri", "interp"):
        for variant, expected in (("buggy", "FAIL"), ("fixed", "PASS")):
            result = client.explore(case / f"{variant}.cir.json", case / "contract.json", engine)
            if result.outcome != expected or not result.complete:
                raise RuntimeError(f"{engine}/{variant}: expected complete {expected}, got {result.outcome}")
    codegen(case / "fixed.cir.json", out / "skeleton", binary=tc.backend)
    traces = collect_traces(out / "skeleton", native_runs=4, run_miri=False, calls_dir=out / "traces")
    result = conform_all(case / "fixed.cir.json", traces, binary=tc.backend)
    write(out / "SMOKE.json", {"conformance": result, "llm_requests": 0})
    if result["conformant"] != 4 or result["violation"] or result["missing"] or result["timeout"]:
        raise RuntimeError("native codegen/conformance smoke failed; inspect SMOKE.json")
    print("PASS: both model engines, buggy/fixed fixtures, codegen, build, four native traces; no LLM calls")


def benchmark(args):
    from cir_workflow.concir_client import ConcirClient
    tc = tools()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=False)
    client = ConcirClient(tc.backend, workdir=out / "calls", timeout=120)
    rows = []
    for task in read(ROOT / "benchmarks/MANIFEST.json")["tasks"]:
        for variant in ("buggy", "fixed", "correct"):
            name = task.get(f"{variant}_cir")
            if not name: continue
            for engine in ("petri", "interp"):
                result = client.explore(ROOT / "benchmarks" / name,
                                        ROOT / "benchmarks" / task["contract"], engine)
                rows.append({"task": task["id"], "variant": variant, "engine": engine,
                             "outcome": result.outcome, "complete": result.complete,
                             "kind": result.kind, "status": result.status, "wall_ms": result.wall_ms,
                             "payload": result.payload})
    write(out / "BENCHMARK.json", {"runs": rows, "toolchain": tc.fingerprint()})
    print(json.dumps({"runs": len(rows), "outcomes": dict(Counter(r["outcome"] for r in rows))}))


def scale(args):
    from cir_workflow.scale import run_scale
    tools()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=False)
    kwargs = {"matrix": [(2, 2)], "max_states_values": [5000]} if args.quick else {}
    result = run_scale(out, **kwargs)
    print(json.dumps({"runs": len(result["runs"]), "output": str(out / "SCALE.json")}))


def tests(args):
    tools()
    run([sys.executable, "-m", "unittest", *read(ROOT / "configs/portable_tests.json")],
        cwd=ROOT / "python", env=dict(os.environ))
    argv = ["cargo", f"+{TOOLCHAIN}", "test", "--locked", "--manifest-path", ROOT / "ConcIR/Cargo.toml",
            "--target-dir", ROOT / ".build/backend"]
    if args.offline: argv.append("--offline")
    run(argv, cwd=ROOT)


def delegate(args):
    tools()
    run([sys.executable, ROOT / "scripts" / f"{args.command}.py", *args.arguments], cwd=ROOT)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    q = sub.add_parser("verify"); q.set_defaults(func=verify)
    q = sub.add_parser("build"); q.add_argument("--offline", action="store_true"); q.set_defaults(func=build)
    q = sub.add_parser("test"); q.add_argument("--offline", action="store_true"); q.set_defaults(func=tests)
    q = sub.add_parser("plan"); q.add_argument("--config", required=True); q.set_defaults(func=plan)
    q = sub.add_parser("generate", help="PAID: real LLM requests")
    q.add_argument("--config", required=True); q.add_argument("--out", required=True); q.set_defaults(func=generate)
    for name, fn in (("smoke", smoke), ("benchmark", benchmark), ("scale", scale)):
        q = sub.add_parser(name); q.add_argument("--out", required=True); q.set_defaults(func=fn)
        if name == "scale": q.add_argument("--quick", action="store_true")
    for name in ("score", "mutation", "repair"):
        q = sub.add_parser(name, add_help=False); q.set_defaults(func=delegate)
    args, extra = p.parse_known_args()
    if args.command in {"score", "repair", "mutation"}: args.arguments = extra
    elif extra: p.error("unrecognized arguments: " + " ".join(extra))
    try: args.func(args); return 0
    except (RuntimeError, ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr); return 2


if __name__ == "__main__":
    raise SystemExit(main())
