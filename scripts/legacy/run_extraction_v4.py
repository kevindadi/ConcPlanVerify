"""Run the tool-instrumented extraction (v4) on the 8 hard tasks.

For each task the Rust is instrumented by `concir-instrument` (no LLM edits
Rust); the pinned Flash model only maps labels to CIR. Conformance is checked
with `--lenient-unlock --attempt-events` (extraction mode).
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.experiments_v2 import load_manifest, sha256_file  # noqa: E402
from cir_workflow.flash_smoke import (  # noqa: E402
    SMOKE_V3_TASKS, _extract_labels_and_validate, assert_protocol_confirmed)
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402

BINARY = Path(os.environ.get(
    "CONCIR_BACKEND", str(Path.home() / "local-repos/ConcIR/target/release/concir-backend")))
MAX_REQUESTS = int(os.environ.get("EXTRACT_MAX_REQUESTS", "64"))


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else REPO / "experiments/extraction-v4"
    protocol = out / "PROTOCOL.md"
    expected = os.environ.get("EXTRACT_PROTOCOL_SHA256")
    if expected:
        assert_protocol_confirmed(protocol, expected)
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    run = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(budget_path=run / "budget.json", max_requests=MAX_REQUESTS)
    system = (REPO / "prompts/rust_to_cir_extract_v4.md").read_text(encoding="utf-8")
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    records = []
    for task_id in SMOKE_V3_TASKS:
        task = tasks[task_id]
        key = task_id.replace("/", "__")
        cell = run / key
        cell.mkdir(parents=True, exist_ok=True)
        if budget.exhausted():
            records.append({"task": task_id, "stage": "budget",
                            "reason": budget.exhausted()})
            continue
        rust = task.resolve(REPO / "benchmarks", task.fixed_rs).read_text(encoding="utf-8")
        contract = task.resolve(REPO / "benchmarks", task.contract)
        result = _extract_labels_and_validate(
            api_key, budget, run / "llm", system, rust, contract, cell,
            BINARY, None, float(os.environ.get("EXTRACT_TIMEOUT", "90")),
            int(os.environ.get("EXTRACT_MAX_TOKENS", "4096")))
        records.append({"task": task_id, **{k: v for k, v in result.items()
                                            if k != "normalizations"}})
    stages: dict[str, int] = {}
    for r in records:
        stages[r.get("stage", "?")] = stages.get(r.get("stage", "?"), 0) + 1
    summary = {
        "batch_dir": str(run),
        "binary_sha256": sha256_file(BINARY),
        "protocol_sha256": sha256_file(protocol) if protocol.exists() else None,
        "requests_used": budget.requests_used,
        "max_requests": MAX_REQUESTS,
        "validated": sum(1 for r in records if r.get("extract_validated")),
        "harness_errors": sum(1 for r in records if r.get("harness_error")),
        "stage_distribution": stages,
        "records": records,
    }
    (run / "SUMMARY.json").write_text(json.dumps(summary, indent=1) + "\n")
    lines = ["# Extraction v4 — SUMMARY", "",
             f"- binary sha256: `{summary['binary_sha256']}`",
             f"- protocol sha256: `{summary['protocol_sha256']}`",
             f"- requests: {summary['requests_used']}/{MAX_REQUESTS}",
             f"- validated: {summary['validated']}",
             f"- harness errors: {summary['harness_errors']}",
             f"- stage distribution: {stages}", "",
             "| task | stage | validated | model verdict |", "| --- | --- | --- | --- |"]
    for r in records:
        lines.append(f"| {r['task']} | {r.get('stage')} | {r.get('extract_validated')} | "
                     f"{r.get('model_verdict')} |")
    (run / "SUMMARY.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({k: summary[k] for k in ("batch_dir", "requests_used",
                                              "validated", "harness_errors",
                                              "stage_distribution")}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
