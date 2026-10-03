#!/usr/bin/env python3
"""Apply operation-bound mutations to a freshly code-generated CIR implementation."""
import argparse
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from cir_workflow.conformance import codegen, collect_traces, conform_all
from cir_workflow.mutation_operators import mutants


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--program", required=True); p.add_argument("--out", required=True)
    p.add_argument("--native-runs", type=int, default=8); p.add_argument("--miri", action="store_true")
    a = p.parse_args()
    if a.native_runs < 1: p.error("--native-runs must be positive")
    program = Path(a.program).resolve()
    out = Path(a.out).resolve(); out.mkdir(parents=True, exist_ok=False)
    binary = os.environ["CONCIR_BACKEND"]
    codegen(program, out / "control", binary=binary)
    source = (out / "control/src/main.rs").read_text()
    variants = [("control", "unmodified generated implementation", source), *mutants(source)]
    rows = []
    for op, description, source in variants:
        target = out / op
        if op != "control":
            shutil.copytree(out / "control", target, ignore=shutil.ignore_patterns("target", "traces"))
            (target / "src/main.rs").write_text(source, encoding="utf-8")
        row = {"operator": op, "description": description}
        try:
            traces = collect_traces(target, native_runs=a.native_runs, miri_seeds=16,
                                    run_miri=a.miri, calls_dir=target / "traces", timeout_s=8)
            native = [t for t in traces if t.kind == "native"]
            row.update(build_ok=True, conformance=conform_all(program, native, binary=binary),
                       traces=[t.__dict__ for t in traces])
        except Exception as exc:
            row.update(build_ok=False, error=f"{type(exc).__name__}: {exc}")
        rows.append(row)
        (out / "MUTATION.json").write_text(json.dumps({
            "program_sha256": hashlib.sha256(program.read_bytes()).hexdigest(), "runs": rows}, indent=2)+"\n")
    print(json.dumps({"applicable_mutants": len(variants)-1, "output": str(out / "MUTATION.json")}))
    control = rows[0]
    if not control.get("build_ok") or control["conformance"]["conformant"] != a.native_runs:
        raise RuntimeError("unmodified control failed; do not interpret mutants as detection evidence")


if __name__ == "__main__": main()
