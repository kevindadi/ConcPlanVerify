#!/usr/bin/env python3
"""§2 offline replay: re-score stored LLM Rust on the fixed toolchain (0 LLM).

For every `flash-gen-main-v2` G3 code cell, take the last round's Rust and
re-run instrument -> build -> conform --op-resource -> monitor. Reports, per
cell, the freeze-5 verdict vs the fixed verdict and the violation kind; and the
harness-gap share among the failures. Also replays the 45 smoke programs.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import sys
import tempfile
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import bounded_monitor, rust_oracle  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    _DROP_OPS, _conform_all_op_resource, _conform_kind, _rewrite_traces,
    load_gen_tasks, mapping_to_cir,
)

BATCH = REPO / "experiments/flash-gen-main-v2/run-20260923T005232"
SMOKE = REPO / "experiments/gen-llmcode-smoke-v1"
OUT = REPO / "experiments/gen-code-replay-v1"


def _score(rust: str, cir_path: Path, contract_path: Path, binary: Path,
           td: Path) -> dict:
    wrapped = rust_oracle.instrument_wrappers(rust, td / "inst")
    rust_oracle.prepare_project(td / "proj", wrapped["annotated"], wrapped["runtime"])
    built, log = rust_oracle.cargo_build(td / "proj")
    if not built:
        return {"built": False, "detail": log[-300:]}
    rust_oracle.run_native(td / "proj", td / "traces", n=4, timeout=10)
    cir = json.loads(Path(cir_path).read_text(encoding="utf-8"))
    mapping, _ = mapping_to_cir(wrapped["resources"], cir)
    _rewrite_traces(td / "traces", td / "conform", mapping, _DROP_OPS)
    conform = _conform_all_op_resource(binary, cir_path, td / "conform")
    report = bounded_monitor.run_monitor(contract_path, td / "traces",
                                         resources=td / "inst/resources.json",
                                         binary=binary)
    return {"built": True, "conformant": conform["conformant"],
            "traces": conform["traces"],
            "kind": _conform_kind(conform["first_violation"]),
            "monitor": report.get("status")}


def _replay_cells(binary: Path, tasks: dict) -> list[dict]:
    records = []
    for cell in sorted(glob.glob(f"{BATCH}/**/G3_concir", recursive=True)):
        cell_dir = Path(cell)
        cells_json = cell_dir / "CELL.json"
        rounds = sorted(cell_dir.glob("code/round-*.rs"),
                        key=lambda p: int(p.stem.split("-")[1]))
        if not cells_json.is_file() or not rounds:
            continue
        record = json.loads(cells_json.read_text(encoding="utf-8"))
        task_id = record.get("task")
        cir_path = Path(record.get("cir_path")
                        or (record.get("code_stage") or {}).get("cir_path") or "")
        if not cir_path.is_file():
            continue
        before = [r for r in (record.get("code_stage") or {}).get("rounds", [])]
        before_ok = bool(before and before[-1].get("conform", {}).get("conformant", 0)
                         == before[-1].get("conform", {}).get("traces", -1))
        with tempfile.TemporaryDirectory() as td:
            after = _score(rounds[-1].read_text(encoding="utf-8"), cir_path,
                           tasks[task_id].contract_path, binary, Path(td))
        records.append({"task": task_id, "before_ok": before_ok, "after": after,
                        "family": task_id.split("/")[0]})
    return records


def _replay_smoke(binary: Path, tasks: dict) -> dict:
    ok = 0
    total = 0
    runs = sorted((SMOKE).glob("run-*"))
    pattern = f"{runs[-1]}/**/round-*.rs" if runs else f"{SMOKE}/run-*/*/round-*.rs"
    for rs in sorted(glob.glob(pattern, recursive=True)):
        cell = Path(rs)
        cell_json = cell.parent / "CELL.json"
        if not cell_json.is_file():
            continue
        record = json.loads(cell_json.read_text(encoding="utf-8"))
        cir_path = Path(record.get("cir_path") or "")
        task_id = record.get("task")
        if not cir_path.is_file() or task_id not in tasks:
            continue
        total += 1
        with tempfile.TemporaryDirectory() as td:
            after = _score(cell.read_text(encoding="utf-8"), cir_path,
                           tasks[task_id].contract_path, binary, Path(td))
        if after.get("conformant") == after.get("traces") and after.get("traces"):
            ok += 1
    return {"smoke_total": total, "smoke_conform_ok": ok}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "/Users/kevin/local-repos/ConcIR/target/release/concir-backend"))
    parser.add_argument("--skip-smoke", action="store_true")
    args = parser.parse_args()
    binary = Path(args.binary)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    records = _replay_cells(binary, tasks)
    smoke = {} if args.skip_smoke else _replay_smoke(binary, tasks)

    failed_before = [r for r in records if not r["before_ok"]]
    fixed = [r for r in failed_before if r["after"].get("conformant") == r["after"].get("traces") and r["after"].get("traces")]
    accepted_before = [r for r in records if r["before_ok"]]
    regressions = [r for r in accepted_before
                   if r["after"].get("conformant") != r["after"].get("traces")]
    kinds = Counter(r["after"].get("kind") for r in failed_before
                    if r["after"].get("conformant") != r["after"].get("traces"))
    out = OUT / "SUMMARY.md"
    OUT.mkdir(parents=True, exist_ok=True)
    lines = [
        "# gen-code-replay-v1 — re-scoring stored LLM Rust (fixed toolchain)",
        "",
        f"Batch `{BATCH.name}`; {len(records)} G3 code cells; replays the last "
        "round Rust of each cell. 0 LLM requests.",
        "",
        f"- failures at freeze-5: **{len(failed_before)}**",
        f"- fixed to conformant now: **{len(fixed)}/{len(failed_before)}** "
        f"({round(100*len(fixed)/len(failed_before),1) if failed_before else 0}% harness gap)",
        f"- still failing: **{len(failed_before)-len(fixed)}**",
        f"- freeze-5 accepted cells replayed: {len(accepted_before)}; regressions: "
        f"**{len(regressions)}**",
        f"- remaining violation kinds: {dict(kinds)}",
        f"- smoke: conform {smoke.get('smoke_conform_ok')}/{smoke.get('smoke_total')}",
        "",
        "| task | family | before | after conformant | kind |",
        "| --- | --- | --- | --- | --- |",
    ]
    for r in sorted(records, key=lambda x: (x["family"], x["task"])):
        a = r["after"]
        lines.append(f"| {r['task']} | {r['family']} | "
                     f"{'PASS' if r['before_ok'] else 'violation'} | "
                     f"{a.get('conformant')}/{a.get('traces')} | {a.get('kind')} |")
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT / "RESULTS.json").write_text(
        json.dumps({"records": records, "smoke": smoke}, indent=2) + "\n",
        encoding="utf-8")
    print(f"failures={len(failed_before)} fixed={len(fixed)} "
          f"regressions={len(regressions)} kinds={dict(kinds)} smoke={smoke}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
