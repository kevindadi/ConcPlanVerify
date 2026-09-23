"""Extraction v5 over every accepted A0/A1/A2 cell of flash-repair-main-v1 (§4.1).

Tool instruments the Rust; the model writes only CIR; `labels.json` is a
mandatory checklist (each label exactly once as a sid) enforced before codegen,
with one retry that names the missing/duplicate labels.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.extract import (  # noqa: E402
    extraction_prompt_v4, parse_cir_only, schema_text, validate_extraction,
    write_extraction_result)
from cir_workflow.instrument import instrument, labels_prompt_section  # noqa: E402
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
RUST_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml")
MAX_REQUESTS = int(os.environ.get("EXTRACT_MAX_REQUESTS", "120"))


def main() -> int:
    out = REPO / "experiments/extraction-v5"
    out.mkdir(parents=True, exist_ok=True)
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    run = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(budget_path=run / "budget.json", max_requests=MAX_REQUESTS)
    system = (REPO / "prompts/rust_to_cir_extract_v5.md").read_text(encoding="utf-8")
    schema = schema_text(binary)
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    cells = []
    for rep in summary["reps"]:
        for task in rep["tasks"]:
            tid = task["task"]
            task_dir = tasks.get(tid)
            if not task_dir:
                continue
            contract = task_dir.resolve(REPO / "benchmarks", task_dir.contract)
            for arm in RUST_ARMS:
                rec = (task.get("arms") or {}).get(arm) or {}
                if not rec.get("accepted"):
                    continue
                source_path = rec.get("final_artifact_path")
                if not source_path or not Path(source_path).is_file():
                    cells.append({"task": tid, "arm": arm, "rep": rep["rep"],
                                  "stage": "no_artifact", "extract_validated": False})
                    continue
                if budget.exhausted():
                    cells.append({"task": tid, "arm": arm, "rep": rep["rep"],
                                  "stage": "budget"})
                    continue
                source = Path(source_path).read_text(encoding="utf-8")
                key = f"{tid}__{arm}__rep{rep['rep']}"
                cell_dir = run / key
                try:
                    inst = instrument(source, cell_dir / "instrument")
                except Exception as exc:  # noqa: BLE001
                    cells.append({"task": tid, "arm": arm, "rep": rep["rep"],
                                  "stage": "harness", "reason": str(exc),
                                  "harness_error": True})
                    continue
                labels = [l["label"] for l in inst["labels"].get("labels", [])]
                labels_md = labels_prompt_section(inst["labels"])
                llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=run / "llm",
                    timeout=float(os.environ.get("EXTRACT_TIMEOUT", "90")),
                    max_tokens=int(os.environ.get("EXTRACT_MAX_TOKENS", "4096")))
                result = None
                prompt = extraction_prompt_v4(schema, source, labels_md)
                for attempt in (1, 2):
                    if budget.exhausted():
                        result = {"stage": "budget"}
                        break
                    try:
                        outcome = llm.complete(system, prompt)
                    except Exception as exc:  # noqa: BLE001
                        result = {"stage": "parse", "reason": str(exc)}
                        break
                    parsed = parse_cir_only(outcome.text)
                    if not parsed:
                        result = write_extraction_result(
                            cell_dir, {"stage": "parse", "reason": "not a CIR object"})
                        break
                    result = validate_extraction(
                        parsed["cir"], inst["annotated"], contract, cell_dir,
                        binary=binary, native_runs=20, miri_seeds=8,
                        lenient_unlock=True, attempt_events=True,
                        required_labels=labels)
                    if result.get("stage") == "labels" and attempt == 1:
                        prompt = (extraction_prompt_v4(schema, source, labels_md)
                                  + "\n\nYour previous CIR was rejected: missing labels "
                                  + str(result.get("missing_labels"))
                                  + ", duplicate labels "
                                  + str(result.get("duplicate_labels"))
                                  + ". Every label must appear exactly once as a sid.")
                        continue
                    break
                oracle = rec.get("oracle") or {}
                cells.append({
                    "task": tid, "arm": arm, "rep": rep["rep"],
                    "stage": (result or {}).get("stage"),
                    "extract_validated": bool((result or {}).get("extract_validated")),
                    "model_verdict": (result or {}).get("model_verdict"),
                    "behavior": oracle.get("behavior_status"),
                    "miri": "detected" if oracle.get("miri_detected") else "clean",
                    "normalizations": (result or {}).get("normalizations", []),
                    "missing_labels": (result or {}).get("missing_labels"),
                })
    stages: dict[str, int] = {}
    for c in cells:
        stages[c.get("stage", "?")] = stages.get(c.get("stage", "?"), 0) + 1
    validated = [c for c in cells if c.get("extract_validated")]
    payload = {
        "batch": str(BATCH.relative_to(REPO)),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "prompt_sha256": hashlib.sha256(
            (REPO / "prompts/rust_to_cir_extract_v5.md").read_bytes()).hexdigest(),
        "requests_used": budget.requests_used,
        "max_requests": MAX_REQUESTS,
        "validated": len(validated),
        "harness_errors": sum(1 for c in cells if c.get("harness_error")),
        "stage_distribution": stages,
        "cells": cells,
    }
    (out / "CELLS.json").write_text(json.dumps(payload, indent=1) + "\n",
                                    encoding="utf-8")
    lines = ["# Extraction v5 — SUMMARY", "",
             f"- batch: `{payload['batch']}`",
             f"- binary sha256: `{payload['binary_sha256']}`",
             f"- prompt sha256: `{payload['prompt_sha256']}`",
             f"- requests: {payload['requests_used']}/{MAX_REQUESTS}",
             f"- validated: **{payload['validated']}** / {len(cells)}",
             f"- harness errors: {payload['harness_errors']}",
             f"- stage distribution: `{stages}`", "",
             "| task | arm | rep | stage | validated | verdict | behavior | miri |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for c in cells:
        lines.append(f"| {c['task']} | {c['arm']} | {c.get('rep')} | {c.get('stage')} | "
                     f"{c.get('extract_validated')} | {c.get('model_verdict')} | "
                     f"{c.get('behavior')} | {c.get('miri')} |")
    (out / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({k: payload[k] for k in ("requests_used", "validated",
                                              "harness_errors", "stage_distribution")},
                     indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
