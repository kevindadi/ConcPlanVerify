#!/usr/bin/env python3
"""Offline common-oracle re-evaluation of frozen generation candidates.

Does not regenerate or edit candidates. Writes a new evidence directory.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
# Cursor's sandbox redirects Cargo output; the oracle expects target/ inside the work dir.
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.project_template import cargo_toml  # noqa: E402
from cir_workflow import rust_oracle  # noqa: E402

SUMMARY = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345/SUMMARY.json"


def sha256_file(path: Path | None) -> str | None:
    if path is None or not path.is_file():
        return None
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def tool_sha(path: Path) -> str | None:
    return sha256_file(path) if path.is_file() else None


def locate_rust(cell: dict) -> Path | None:
    rec = cell.get("record") or {}
    for key in ("final_artifact_path", "final_rust"):
        raw = rec.get(key)
        if raw and Path(raw).is_file():
            return Path(raw)
    code = rec.get("code_stage") or {}
    raw = code.get("final_rust")
    if raw and Path(raw).is_file():
        return Path(raw)
    cir = rec.get("cir_path") or code.get("cir_path")
    if cir:
        rust = Path(cir).parent / "rust" / "src" / "main.rs"
        if rust.is_file():
            return rust
    return None


def locate_cir(cell: dict) -> Path | None:
    rec = cell.get("record") or {}
    raw = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
    if raw and Path(raw).is_file():
        return Path(raw)
    return None


def original_build(source: str, dest: Path) -> tuple[bool, str]:
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "src").mkdir(exist_ok=True)
    (dest / "Cargo.toml").write_text(cargo_toml("probe"), encoding="utf-8")
    (dest / "src" / "main.rs").write_text(source, encoding="utf-8")
    return rust_oracle.cargo_build(dest, offline=True, timeout=180)


def classify(row: dict) -> str:
    if row.get("rust_path") is None:
        return "no_artifact"
    if row.get("original_build") is False:
        return "original_build_error"
    if row.get("instrumentation") == "error":
        return "instrument_error"
    if row.get("instrumentation") == "ok" and row.get("instrumented_build") is False:
        limits = row.get("limitations") or []
        if limits:
            return "unsupported"
        return "instrumented_build_error"
    if row.get("evaluable"):
        return "evaluable"
    if row.get("hang"):
        return "run_timeout"
    return "unevaluable"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="experiments/evidence-20260925/reeval")
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()
    out = REPO / args.out
    out.mkdir(parents=True, exist_ok=True)
    from cir_workflow.instrument import find_instrument_binary

    instrument = find_instrument_binary(None)
    backend = Path(os.environ.get(
        "CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    cells = summary["cells"]
    manifest_path = out / "INPUT_MANIFEST.jsonl"
    results_path = out / "CELLS.jsonl"
    done = set()
    if results_path.is_file():
        for line in results_path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                done.add((row["rep"], row["task"], row["arm"]))
    if not manifest_path.is_file():
        with manifest_path.open("w", encoding="utf-8") as fh:
            for cell in cells:
                rust = locate_rust(cell)
                cir = locate_cir(cell)
                task = tasks[cell["task"]]
                rec = cell.get("record") or {}
                fh.write(json.dumps({
                    "rep": cell["rep"], "task": cell["task"], "arm": cell["arm"],
                    "tier": cell.get("tier"),
                    "source_run": cell.get("source_run"),
                    "generation_accepted": bool(cell.get("accepted")),
                    "historical_status": cell.get("status"),
                    "historical_rf": cell.get("rf"),
                    "historical_rc": cell.get("rc"),
                    "historical_oracle_status": (rec.get("oracle") or {}).get("status"),
                    "rust_path": str(rust) if rust else None,
                    "rust_sha256": sha256_file(rust),
                    "cir_path": str(cir) if cir else None,
                    "cir_sha256": sha256_file(cir),
                    "contract_path": str(task.contract_path),
                    "contract_sha256": sha256_file(task.contract_path),
                    "reference_cir_sha256": sha256_file(task.reference_cir_path),
                }, ensure_ascii=False) + "\n")
    meta = {
        "summary": str(SUMMARY),
        "instrument_sha256": tool_sha(instrument),
        "backend_sha256": tool_sha(backend),
        "instrument": str(instrument),
        "backend": str(backend),
        "n_native": 32,
        "run_timeout_s": 10,
        "protocol": "instrument --wrappers; cargo build --offline; 32 native runs; bounded monitor",
    }
    (out / "TOOLS.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")

    processed = 0
    with results_path.open("a", encoding="utf-8") as fh:
        for cell in cells:
            key = (cell["rep"], cell["task"], cell["arm"])
            if key in done:
                continue
            if args.limit and processed >= args.limit:
                break
            rust = locate_rust(cell)
            task = tasks[cell["task"]]
            row = {
                "rep": cell["rep"], "task": cell["task"], "arm": cell["arm"],
                "generation_accepted": bool(cell.get("accepted")),
                "historical_rf": cell.get("rf"),
                "historical_oracle_status": ((cell.get("record") or {}).get("oracle") or {}).get("status"),
                "rust_path": str(rust) if rust else None,
                "rust_sha256": sha256_file(rust),
                "original_build": None,
                "instrumentation": None,
                "instrumented_build": None,
                "evaluable": False,
                "monitor": None,
                "conformance": None,
                "model": None,
                "ri": None,
                "rf": None,
                "rc": None,
                "hang": None,
                "limitations": [],
                "failure_class": None,
            }
            work = out / "work" / f"rep{cell['rep']}" / cell["task"].replace("/", "__") / cell["arm"]
            if rust is None:
                row["failure_class"] = "no_artifact"
                fh.write(json.dumps(row) + "\n")
                fh.flush()
                processed += 1
                print(f"skip no_artifact {key}", flush=True)
                continue
            source = rust.read_text(encoding="utf-8")
            try:
                ob, olog = original_build(source, work / "original")
                row["original_build"] = ob
                (work / "original_build.log").write_text(olog, encoding="utf-8")
            except Exception as exc:  # noqa: BLE001
                row["original_build"] = False
                (work / "original_build.log").write_text(str(exc), encoding="utf-8")
            try:
                result = rust_oracle.evaluate(
                    source, task.contract_path, task.reference_cir_path, work / "oracle",
                    n_native=32, miri_seeds=0, run_timeout=10.0, binary=backend,
                    instrument_binary=instrument)
                row["instrumentation"] = "ok"
                row["instrumented_build"] = bool(result.get("built"))
                row["limitations"] = result.get("limitations") or []
                row["hang"] = result.get("hang")
                monitor = result.get("monitor") or {}
                row["monitor"] = monitor.get("status")
                cov = result.get("coverage") or {}
                row["rf"] = cov.get("rf")
                row["rc"] = cov.get("rc")
                row["ri"] = cov.get("statuses")
                row["evaluable"] = row["rf"] is not None
                row["model"] = None
                if not result.get("built"):
                    (work / "instrument_build.log").write_text(
                        result.get("build_log_tail") or "", encoding="utf-8")
            except Exception as exc:  # noqa: BLE001
                row["instrumentation"] = "error"
                row["instrument_error"] = str(exc)[:500]
            row["failure_class"] = classify(row)
            fh.write(json.dumps(row, ensure_ascii=False) + "\n")
            fh.flush()
            processed += 1
            print(f"{row['failure_class']} {cell['arm']} r{cell['rep']} {cell['task']} "
                  f"rf={row['rf']}", flush=True)
    print(f"processed {processed}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
