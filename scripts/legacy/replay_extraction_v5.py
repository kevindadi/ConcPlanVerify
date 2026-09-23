"""Offline replay of extraction-v5 validation with the improved normalizer.

No LLM requests: each cell's stored `extracted.cir.json` and instrumented
`instrument/annotated.rs` are re-validated (re-normalize + mandatory labels +
codegen/build/trace/conform). Updates CELLS.json / SUMMARY.md in place.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.extract import validate_extraction  # noqa: E402

OUT = REPO / "experiments/extraction-v5"


def main() -> int:
    run = sorted(OUT.glob("run-*"))[-1]
    cells_file = OUT / "CELLS.json"
    payload = json.loads(cells_file.read_text(encoding="utf-8"))
    binary = Path(sys.argv[1]) if len(sys.argv) > 1 else (
        REPO.parent / "ConcIR/target/release/concir-backend")
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    updated = 0
    for cell in payload["cells"]:
        key = f"{cell['task']}__{cell['arm']}__rep{cell['rep']}"
        cell_dir = run / key
        program_path = cell_dir / "extracted.cir.json"
        annotated = cell_dir / "instrument" / "annotated.rs"
        labels_file = cell_dir / "instrument" / "labels.json"
        if not (program_path.is_file() and annotated.is_file() and labels_file.is_file()):
            continue
        task = tasks.get(cell["task"])
        contract = task.resolve(REPO / "benchmarks", task.contract)
        cir = json.loads(program_path.read_text(encoding="utf-8"))
        labels = [l["label"] for l in json.loads(
            labels_file.read_text(encoding="utf-8")).get("labels", [])]
        result = validate_extraction(
            cir, annotated.read_text(encoding="utf-8"), contract, cell_dir,
            binary=binary, native_runs=20, miri_seeds=8,
            lenient_unlock=True, attempt_events=True, required_labels=labels)
        cell["stage"] = result.get("stage")
        cell["extract_validated"] = bool(result.get("extract_validated"))
        cell["model_verdict"] = result.get("model_verdict")
        cell["normalizations"] = result.get("normalizations", [])
        cell["missing_labels"] = result.get("missing_labels")
        updated += 1
    stages: dict[str, int] = {}
    for c in payload["cells"]:
        stages[c.get("stage", "?")] = stages.get(c.get("stage", "?"), 0) + 1
    payload["validated"] = sum(1 for c in payload["cells"] if c.get("extract_validated"))
    payload["stage_distribution"] = stages
    payload["replay"] = True
    payload["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
    cells_file.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    lines = ["# Extraction v5 — SUMMARY", "",
             f"- batch: `{payload['batch']}`",
             f"- binary sha256: `{payload['binary_sha256']}`",
             f"- prompt sha256: `{payload['prompt_sha256']}`",
             f"- requests: {payload['requests_used']}/{payload['max_requests']}",
             f"- validated: **{payload['validated']}** / {len(payload['cells'])}",
             f"- harness errors: {payload['harness_errors']}",
             f"- stage distribution: `{stages}`",
             "- note: validation replay is offline; native schedules are random so "
             "the validated count is not perfectly reproducible.",
             "", "| task | arm | rep | stage | validated | verdict | behavior | miri |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for c in payload["cells"]:
        lines.append(f"| {c['task']} | {c['arm']} | {c.get('rep')} | {c.get('stage')} | "
                     f"{c.get('extract_validated')} | {c.get('model_verdict')} | "
                     f"{c.get('behavior')} | {c.get('miri')} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"replayed": updated, "validated": payload["validated"],
                      "stage_distribution": stages}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
