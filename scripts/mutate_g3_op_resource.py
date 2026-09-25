#!/usr/bin/env python3
"""Mutation and negative-control check on frozen G3 LLM Rust, via --op-resource.

Ground truth is an independent syntactic rule, not the conformer under test.
Uncertain or equivalent mutants are stored separately and are not recall
denominators. Suggested labels are machine_proposed, not human review.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow import bounded_monitor, rust_oracle  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    _conform_all_op_resource, _rewrite_traces, mapping_to_cir,
)
from cir_workflow.instrument import find_instrument_binary  # noqa: E402

SUMMARY = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345/SUMMARY.json"
OUT = REPO / "experiments/evidence-20260925/mutation-g3-r2"
N_TRACES = 8
# Frozen before execution: one accepted G3 program per family, smallest
# (task, rep) among unique rust SHAs. Sync mutations apply only when a site exists.

LOCK_RE = re.compile(r"(\b[A-Za-z_][A-Za-z0-9_]*)\.lock\(\)\.unwrap\(\)")
NOTIFY_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\.(notify_one|notify_all)\(\)")
RELEASE_RE = re.compile(r"(\b[A-Za-z_][A-Za-z0-9_]*)\.release\(\)")


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def select_samples(cells: list[dict]) -> list[dict]:
    chosen: dict[str, dict] = {}
    for cell in sorted(cells, key=lambda c: (c["task"], c["rep"])):
        if cell.get("arm") != "G3_concir" or not cell.get("accepted"):
            continue
        fam = cell["task"].split("/")[0]
        if fam in chosen:
            continue
        rust = Path(cell["record"]["final_rust"])
        if not rust.is_file():
            continue
        text = rust.read_text(encoding="utf-8")
        chosen[fam] = {
            "family": fam, "task": cell["task"], "rep": cell["rep"],
            "rust_path": str(rust), "rust_sha256": sha256_text(text),
            "cir_path": cell["record"].get("cir_path"),
        }
    return [chosen[k] for k in sorted(chosen)]


def _replace_nth(pattern: re.Pattern, text: str, n: int, repl) -> str | None:
    matches = list(pattern.finditer(text))
    if n >= len(matches):
        return None
    m = matches[n]
    return text[:m.start()] + repl(m) + text[m.end():]


def mutations_for(src: str) -> list[dict]:
    out = [{"kind": "identity", "role": "negative", "source": src,
            "expected_violation": "no", "semantic_validity": "preserved"}]
    renamed = src.replace("let tmp2", "let tmp2_renamed").replace("tmp2", "tmp2_renamed") if "tmp2" in src else None
    if renamed and renamed != src:
        out.append({"kind": "rename_local", "role": "negative", "source": renamed,
                    "expected_violation": "no", "semantic_validity": "preserved"})
    lines = src.splitlines(keepends=True)
    lock_lines = [i for i, line in enumerate(lines) if ".lock()" in line and not line.strip().startswith("//")]
    if lock_lines:
        kept = [line for i, line in enumerate(lines) if i != lock_lines[0]]
        deleted = "".join(kept)
        if deleted != src:
            out.append({"kind": "delete_lock_line", "role": "mutant", "source": deleted,
                        "expected_violation": "yes", "semantic_validity": "violating",
                        "note": "removed the first line that calls .lock(); independent of conform"})
    locks = list(LOCK_RE.finditer(src))
    if len(locks) >= 2 and locks[0].end() < locks[1].start():
        a, b = locks[0], locks[1]
        if "\n" in src[a.end():b.start()] and src[a.start():b.end()].count("\n") <= 3:
            swapped = src[:a.start()] + src[b.start():b.end()] + src[a.end():b.start()] + src[a.start():a.end()] + src[b.end():]
            if swapped != src:
                out.append({"kind": "swap_lock_order", "role": "mutant", "source": swapped,
                            "expected_violation": "uncertain",
                            "semantic_validity": "uncertain",
                            "note": "swapped two nearby lock calls; violation depends on whether the CIR orders them"})
    if len({m.group(1) for m in locks}) >= 2:
        wrong = _replace_nth(LOCK_RE, src, 1, lambda m: f"{locks[0].group(1)}.lock().unwrap()")
        if wrong and wrong != src:
            out.append({"kind": "wrong_resource", "role": "mutant", "source": wrong,
                        "expected_violation": "uncertain",
                        "semantic_validity": "uncertain",
                        "note": "rebound a lock call onto another mutex; may fail to compile if types differ"})
    notify_lines = [i for i, line in enumerate(lines) if NOTIFY_RE.search(line)]
    if notify_lines:
        omitted = "".join(line for i, line in enumerate(lines) if i != notify_lines[0])
        if omitted != src:
            out.append({"kind": "omit_notify", "role": "mutant", "source": omitted,
                        "expected_violation": "yes", "semantic_validity": "violating",
                        "note": "removed the first notify line"})
    wrong_kind = _replace_nth(
        NOTIFY_RE, src, 0,
        lambda m: f"{m.group(1)}.{'notify_all' if m.group(2)=='notify_one' else 'notify_one'}()")
    if wrong_kind and wrong_kind != src:
        out.append({"kind": "wrong_notify_kind", "role": "mutant", "source": wrong_kind,
                    "expected_violation": "uncertain", "semantic_validity": "uncertain",
                    "note": "notify_one/notify_all swap is not always a model violation"})
    doubled = _replace_nth(RELEASE_RE, src, 0, lambda m: f"{m.group(0)}; {m.group(0)}")
    if doubled:
        out.append({"kind": "double_release", "role": "mutant", "source": doubled,
                    "expected_violation": "uncertain", "semantic_validity": "uncertain",
                    "note": "second release may be a permit-count violation or a compile error"})
    return out


def run_path(source: str, cir_path: Path, contract_path: Path, work: Path, binary: Path) -> dict:
    work.mkdir(parents=True, exist_ok=True)
    (work / "input.rs").write_text(source, encoding="utf-8")
    row = {"syntactic_mutant": True, "build_valid": False, "tool_failure": None,
           "conform": None, "monitor": None, "hang": None}
    try:
        wrapped = rust_oracle.instrument_wrappers(source, work / "instrument")
    except Exception as exc:  # noqa: BLE001
        row["tool_failure"] = "instrument"
        row["detail"] = str(exc)[:400]
        return row
    project = work / "proj"
    rust_oracle.prepare_project(project, wrapped["annotated"], wrapped["runtime"])
    built, log = rust_oracle.cargo_build(project, offline=True, timeout=180)
    row["build_valid"] = built
    if not built:
        row["tool_failure"] = "compile"
        (work / "build.log").write_text(log, encoding="utf-8")
        return row
    traces = work / "traces"
    runs = rust_oracle.run_native(project, traces, n=N_TRACES, timeout=10.0)
    row["hang"] = any(r["timed_out"] for r in runs)
    row["runs_completed"] = sum(1 for r in runs if r["completed"])
    cir = json.loads(cir_path.read_text(encoding="utf-8"))
    mapping, _ = mapping_to_cir(wrapped["resources"], cir)
    mapping_path = work / "mapping.json"
    mapping_path.write_text(json.dumps({"mapping": mapping}, indent=2) + "\n", encoding="utf-8")
    conform_dir = work / "conform-traces"
    monitor_dir = work / "monitor-traces"
    wrapper_names = {r["name"] for r in wrapped["resources"] if r.get("kind") == "ChannelWrapper"}
    from cir_workflow.generation import _DROP_OPS
    _rewrite_traces(traces, conform_dir, mapping, _DROP_OPS, wrapper_names)
    _rewrite_traces(traces, monitor_dir, {}, set(), wrapper_names)
    conform = _conform_all_op_resource(binary, cir_path, conform_dir)
    row["conform"] = {"conformant": conform["conformant"], "traces": conform["traces"],
                      "statuses": conform["statuses"]}
    report = bounded_monitor.run_monitor(
        contract_path, monitor_dir, resources=work / "instrument/resources.json",
        mapping=mapping_path, binary=binary)
    row["monitor"] = report.get("status")
    row["monitor_fail"] = [p["id"] for p in report.get("properties", []) if p.get("status") == "FAIL"]
    if row["hang"]:
        row["tool_failure"] = "timeout"
    return row


def verdict(mut: dict, result: dict) -> str:
    """Independent of using conform as the definition of the mutant."""
    if result.get("tool_failure") == "instrument":
        return "tool_instrument_failure"
    if result.get("tool_failure") == "compile":
        return "compile_failure"
    if result.get("tool_failure") == "timeout":
        return "timeout_not_a_structural_proof"
    conformant = (result.get("conform") or {}).get("conformant")
    traces = (result.get("conform") or {}).get("traces") or 0
    violated = traces > 0 and conformant is not None and conformant < traces
    monitored = bool(result.get("monitor_fail"))
    detected = violated or monitored
    expected = mut["expected_violation"]
    if mut["role"] == "negative":
        return "fp" if detected else "tn"
    if expected == "uncertain":
        return "uncertain_detected" if detected else "uncertain_not_detected"
    if expected == "yes":
        return "tp" if detected else "fn"
    return "unclassified"


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    binary = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
    instrument = find_instrument_binary(None)
    cells = json.loads(SUMMARY.read_text(encoding="utf-8"))["cells"]
    accepted = [c for c in cells if c.get("arm") == "G3_concir" and c.get("accepted")]
    samples = select_samples(cells)
    (OUT / "SAMPLE_RULE.json").write_text(json.dumps({
        "universe": "G3_concir accepted cells in flash-gen-main-v5-code SUMMARY",
        "n_accepted_cells": len(accepted),
        "n_unique_sha": len({hashlib.sha256(Path(c['record']['final_rust']).read_bytes()).hexdigest()
                             for c in accepted if Path(c['record'].get('final_rust','')).is_file()}),
        "rule": "one program per family; smallest (task, rep); SHA-dedup by that choice",
        "n_selected": len(samples),
        "samples": samples,
        "n_traces": N_TRACES,
        "instrument_sha256": hashlib.sha256(instrument.read_bytes()).hexdigest(),
        "backend_sha256": hashlib.sha256(binary.read_bytes()).hexdigest() if binary.is_file() else None,
        "label_provenance": "machine_proposed",
    }, indent=2) + "\n", encoding="utf-8")
    rows = []
    families_root = REPO / "benchmarks/families"
    for sample in samples:
        src = Path(sample["rust_path"]).read_text(encoding="utf-8")
        contract = families_root / sample["task"] / "contract.json"
        cir = Path(sample["cir_path"])
        for mut in mutations_for(src):
            changed = mut["source"] != src
            if mut["kind"] != "identity" and not changed:
                continue
            work = OUT / "work" / sample["family"] / mut["kind"]
            result = run_path(mut["source"], cir, contract, work, binary)
            result.update({
                "family": sample["family"], "task": sample["task"], "rep": sample["rep"],
                "base_sha256": sample["rust_sha256"],
                "mutant_sha256": sha256_text(mut["source"]),
                "kind": mut["kind"], "role": mut["role"],
                "expected_violation": mut["expected_violation"],
                "semantic_validity": mut["semantic_validity"],
                "note": mut.get("note"),
                "changed_text": changed,
                "review": "machine_proposed",
            })
            result["observed"] = verdict(mut, result)
            rows.append({k: v for k, v in result.items()})
            print(f"{sample['family']} {mut['kind']} {result['observed']}", flush=True)
            (OUT / "RESULTS.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(rows)} rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
