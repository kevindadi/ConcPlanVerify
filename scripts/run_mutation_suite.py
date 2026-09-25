#!/usr/bin/env python3
"""M1–M5 on frozen G3 Rust. Does not call an LLM. Does not overwrite r1/r2."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow import bounded_monitor, rust_oracle  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    _DROP_OPS, _conform_all_op_resource, _rewrite_traces, mapping_to_cir,
)
from cir_workflow.instrument import find_instrument_binary  # noqa: E402
from cir_workflow.mutation_protocol import classify  # noqa: E402
from cir_workflow.project_template import cargo_toml  # noqa: E402

SUMMARY = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345/SUMMARY.json"
OUT = REPO / "experiments/evidence-20260925/mutation-m15"
BACKEND = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
N_DEFAULT = 8
REVIEW = "machine_proposed"


def sha_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def cargo_src(source: str, dest: Path) -> tuple[bool, str]:
    if dest.exists():
        import shutil
        shutil.rmtree(dest)
    (dest / "src").mkdir(parents=True)
    (dest / "Cargo.toml").write_text(cargo_toml("probe"), encoding="utf-8")
    (dest / "src" / "main.rs").write_text(source, encoding="utf-8")
    return rust_oracle.cargo_build(dest, offline=True, timeout=180)


def src_mutate(op: str, index: int, path: Path, instrument: Path) -> dict:
    proc = subprocess.run(
        [str(instrument), "--src-mutate", op, str(index), str(path)],
        capture_output=True, text=True, timeout=60)
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"applicable": False, "reason": proc.stderr[-200:], "source": ""}


def cir_has_notify(cir: Path | None) -> bool:
    if cir is None or not cir.is_file():
        return False
    return "condvar_notify" in cir.read_text(encoding="utf-8")


def label_for(kind: str, source: str, cir: Path | None) -> dict:
    if kind in {"comment", "rename-local"}:
        return {"role": "negative", "design_deviation": "no", "requirement_violation": "no",
                "reason": "comment or scoped rename does not change synchronization calls",
                "evidence": "source_diff", "review": REVIEW}
    if kind == "omit-notify":
        left = source.count("notify_one") + source.count("notify_all")
        if left == 0 and cir_has_notify(cir):
            return {"role": "mutant", "design_deviation": "yes", "requirement_violation": "uncertain",
                    "reason": "CIR contains condvar_notify and the Rust file no longer has a notify call",
                    "evidence": "cir_text_and_source", "review": REVIEW}
        return {"role": "mutant", "design_deviation": "uncertain", "requirement_violation": "uncertain",
                "reason": "a notify was removed but another notify remains or the CIR has no notify obligation",
                "evidence": "insufficient", "review": REVIEW}
    if kind == "swap-locks":
        return {"role": "mutant", "design_deviation": "uncertain", "requirement_violation": "uncertain",
                "reason": "swapping two lock statements is not by itself a semantic ground truth",
                "evidence": "syntax_only", "review": REVIEW}
    return {"role": "invalid", "design_deviation": "uncertain", "requirement_violation": "uncertain",
            "reason": "edit is expected to be ill-typed (dangling guard or moved Permit)",
            "evidence": "language_rules", "review": REVIEW}


def pool(cells: list[dict]) -> tuple[list[dict], list[dict]]:
    groups: dict[str, dict] = {}
    for cell in cells:
        if cell.get("arm") != "G3_concir" or not cell.get("accepted"):
            continue
        rust = (cell.get("record") or {}).get("final_rust")
        if not rust or not Path(rust).is_file():
            continue
        text = Path(rust).read_text(encoding="utf-8")
        digest = sha_text(text)
        groups.setdefault(digest, {"rust_sha256": digest, "rust_path": rust, "text": text, "cells": []})
        groups[digest]["cells"].append({
            "task": cell["task"], "rep": cell["rep"], "family": cell["task"].split("/")[0],
            "cir_path": (cell.get("record") or {}).get("cir_path"),
        })
    uniques = []
    for item in groups.values():
        item["cells"].sort(key=lambda c: (c["task"], c["rep"]))
        rep = item["cells"][0]
        uniques.append({**rep, "rust_sha256": item["rust_sha256"], "rust_path": item["rust_path"],
                        "paired_cells": item["cells"], "n_paired": len(item["cells"])})
    uniques.sort(key=lambda r: (r["family"], r["task"], r["rep"]))
    return uniques, []


def run_path(source: str, cir: Path, contract: Path, work: Path, n: int) -> dict:
    row: dict = {"instrumentation": None, "instrumented_build": None, "hang": False,
                 "conform_statuses": {}, "n_traces": 0, "monitor_fail": [], "monitor": None}
    try:
        wrapped = rust_oracle.instrument_wrappers(source, work / "instrument")
        row["instrumentation"] = "ok"
    except Exception as exc:  # noqa: BLE001
        row["instrumentation"] = "error"
        row["instrument_error"] = str(exc)[:300]
        return row
    rust_oracle.prepare_project(work / "proj", wrapped["annotated"], wrapped["runtime"])
    built, log = rust_oracle.cargo_build(work / "proj", offline=True, timeout=180)
    row["instrumented_build"] = built
    if not built:
        (work / "inst_build.log").write_text(log[-1500:], encoding="utf-8")
        return row
    traces = work / "traces"
    runs = rust_oracle.run_native(work / "proj", traces, n=n, timeout=8.0)
    row["hang"] = any(r["timed_out"] for r in runs)
    row["runs_completed"] = sum(1 for r in runs if r["completed"])
    if not cir.is_file():
        row["instrumentation"] = "error"
        row["instrument_error"] = "missing cir"
        return row
    cir_obj = json.loads(cir.read_text(encoding="utf-8"))
    mapping, _ = mapping_to_cir(wrapped["resources"], cir_obj)
    mapping_path = work / "mapping.json"
    mapping_path.write_text(json.dumps({"mapping": mapping}) + "\n", encoding="utf-8")
    conform_dir, monitor_dir = work / "conform-traces", work / "monitor-traces"
    wrappers = {r["name"] for r in wrapped["resources"] if r.get("kind") == "ChannelWrapper"}
    _rewrite_traces(traces, conform_dir, mapping, _DROP_OPS, wrappers)
    _rewrite_traces(traces, monitor_dir, {}, set(), wrappers)
    conform = _conform_all_op_resource(BACKEND, cir, conform_dir)
    row["conform_statuses"] = conform["statuses"]
    row["n_traces"] = conform["traces"]
    report = bounded_monitor.run_monitor(
        contract, monitor_dir, resources=work / "instrument/resources.json",
        mapping=mapping_path, binary=BACKEND)
    row["monitor"] = report.get("status")
    row["monitor_fail"] = [p["id"] for p in report.get("properties", []) if p.get("status") == "FAIL"]
    row["run_only_fail"] = bool(row["hang"]) or row["runs_completed"] == 0
    row["conform_only"] = int(conform["statuses"].get("violation", 0)) > 0
    row["monitor_only"] = bool(row["monitor_fail"])
    return row


def proposals(text: str, instrument: Path, src_path: Path) -> list[dict]:
    out = [{"kind": "comment", "index": 0, "source": text + ("\n" if text.endswith("\n") else "\n") + "// kept-comment\n"}]
    for index in range(3):
        renamed = src_mutate("rename-local", index, src_path, instrument)
        if renamed.get("applicable"):
            out.append({"kind": "rename-local", "index": index, "source": renamed["source"]})
        swapped = src_mutate("swap-locks", index, src_path, instrument)
        if swapped.get("applicable"):
            out.append({"kind": "swap-locks", "index": index, "source": swapped["source"]})
        omitted = src_mutate("omit-notify", index, src_path, instrument)
        if omitted.get("applicable"):
            out.append({"kind": "omit-notify", "index": index, "source": omitted["source"]})
    lines = text.splitlines(keepends=True)
    lock_at = next((i for i, line in enumerate(lines) if ".lock()" in line), None)
    if lock_at is not None:
        deleted = "".join(line for i, line in enumerate(lines) if i != lock_at)
        out.append({"kind": "delete-lock-line", "index": 0, "source": deleted})
    if ".release()" in text:
        out.append({"kind": "double-release", "index": 0,
                    "source": text.replace(".release()", ".release(); /*again*/ 0.release()", 1)})
    return out


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    instrument = find_instrument_binary(None)
    cells = json.loads(SUMMARY.read_text(encoding="utf-8"))["cells"]
    uniques, _ = pool(cells)
    (OUT / "POOL.json").write_text(json.dumps({
        "rule": "all accepted G3 Rust SHA values; operators applied at source positions 0..2 before looking at checker output",
        "n_accepted_cells": sum(1 for c in cells if c.get("arm") == "G3_concir" and c.get("accepted")),
        "n_unique_sha": len(uniques),
        "programs": [{k: v for k, v in u.items() if k != "text"} for u in uniques],
        "instrument_sha256": hashlib.sha256(instrument.read_bytes()).hexdigest(),
        "review": REVIEW,
        "dev_set": "mutation-g3 and mutation-g3-r2 were the development runs; this directory is the frozen re-evaluation",
    }, indent=2) + "\n", encoding="utf-8")
    done = set()
    results = OUT / "RESULTS.jsonl"
    if results.is_file():
        for line in results.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                done.add((row["rust_sha256"], row["kind"], row["index"]))
    families = REPO / "benchmarks/families"
    with results.open("a", encoding="utf-8") as fh:
        for prog in uniques:
            src_path = Path(prog["rust_path"])
            text = src_path.read_text(encoding="utf-8")
            cir = Path(prog["cir_path"]) if prog.get("cir_path") else Path()
            contract = families / prog["task"] / "contract.json"
            for mut in proposals(text, instrument, src_path):
                key = (prog["rust_sha256"], mut["kind"], mut["index"])
                if key in done:
                    continue
                labels = label_for(mut["kind"], mut["source"], cir if cir.is_file() else None)
                work = OUT / "work" / prog["rust_sha256"][:12] / f"{mut['kind']}-{mut['index']}"
                work.mkdir(parents=True, exist_ok=True)
                (work / "mutant.rs").write_text(mut["source"], encoding="utf-8")
                source_build, slog = cargo_src(mut["source"], work / "source-build")
                (work / "source_build.log").write_text(slog[-800:], encoding="utf-8")
                observed = {}
                if source_build:
                    observed = run_path(mut["source"], cir, contract, work, N_DEFAULT)
                row = {
                    "rust_sha256": prog["rust_sha256"], "task": prog["task"], "rep": prog["rep"],
                    "family": prog["family"], "paired_tasks": [c["task"] for c in prog["paired_cells"]],
                    "kind": mut["kind"], "index": mut["index"], "source_build": source_build,
                    **labels, **observed,
                }
                row.update(classify(row))
                fh.write(json.dumps(row) + "\n")
                fh.flush()
                print(f"{prog['family']} {mut['kind']}#{mut['index']} {row['call']}", flush=True)
    print("done", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
