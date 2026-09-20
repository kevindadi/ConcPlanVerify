"""Extraction v6 (§6): rerun the harness-fixable v5 cells with a stronger
label prompt and the panic-free BIN_MAIN.

Selected cells: stage == "labels" (model dropped the labels) and stage ==
"codegen" whose reason is a backend panic (now an E-code). Offline replay of the
rest is done by replay_extraction_v5.
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

V5 = REPO / "experiments/extraction-v5"
OUT = REPO / "experiments/extraction-v6"
MAX_REQUESTS = int(os.environ.get("EXTRACT6_MAX_REQUESTS", "40"))


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    v5_run = sorted(V5.glob("run-*"))[-1]
    payload = json.loads((V5 / "CELLS.json").read_text(encoding="utf-8"))
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    OUT.mkdir(parents=True, exist_ok=True)
    run = OUT / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    run.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(budget_path=run / "budget.json", max_requests=MAX_REQUESTS)
    system = (REPO / "prompts/rust_to_cir_extract_v6.md").read_text(encoding="utf-8")
    schema = schema_text(binary)
    updated = 0
    for cell in payload["cells"]:
        if cell.get("stage") == "labels":
            pass
        elif cell.get("stage") == "codegen" and str(cell.get("reason", "")).startswith("panic"):
            pass
        else:
            continue
        if budget.exhausted():
            break
        key = f"{cell['task']}__{cell['arm']}__rep{cell['rep']}"
        src_dir = v5_run / key / "instrument"
        annotated = src_dir / "annotated.rs"
        labels_file = src_dir / "labels.json"
        if not (annotated.is_file() and labels_file.is_file()):
            continue
        labels = [l["label"] for l in json.loads(
            labels_file.read_text(encoding="utf-8")).get("labels", [])]
        labels_md = labels_prompt_section(json.loads(labels_file.read_text(encoding="utf-8")))
        source = (src_dir / "input.rs").read_text(encoding="utf-8")
        task = tasks.get(cell["task"])
        contract = task.resolve(REPO / "benchmarks", task.contract)
        cell_dir = run / key
        cell_dir.mkdir(parents=True, exist_ok=True)
        (cell_dir / "instrument").mkdir(exist_ok=True)
        for name in ("annotated.rs", "labels.json", "input.rs"):
            (cell_dir / "instrument" / name).write_text(
                (src_dir / name).read_text(encoding="utf-8"), encoding="utf-8")
        llm = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                  evidence_dir=run / "llm", timeout=90.0, max_tokens=4096)
        prompt = extraction_prompt_v4(schema, source, labels_md)
        result = None
        for attempt in (1, 2):
            if budget.exhausted():
                break
            try:
                outcome = llm.complete(system, prompt)
            except Exception as exc:  # noqa: BLE001
                result = {"stage": "parse", "reason": str(exc)}
                break
            parsed = parse_cir_only(outcome.text)
            if not parsed:
                result = write_extraction_result(cell_dir, {"stage": "parse",
                                                            "reason": "not a CIR object"})
                break
            result = validate_extraction(parsed["cir"],
                                         (cell_dir / "instrument/annotated.rs").read_text(),
                                         contract, cell_dir, binary=binary,
                                         native_runs=20, miri_seeds=8,
                                         lenient_unlock=True, attempt_events=True,
                                         required_labels=labels)
            if result.get("stage") == "labels" and attempt == 1:
                prompt = (extraction_prompt_v4(schema, source, labels_md)
                          + "\n\nRejected: missing " + str(result.get("missing_labels"))
                          + ". Emit every label exactly once as a sid.")
                continue
            break
        cell["stage"] = (result or {}).get("stage")
        cell["model_verdict"] = (result or {}).get("model_verdict")
        cell["reason"] = (result or {}).get("reason", "")
        cell["validated"] = bool(cell["stage"] == "explore"
                                 and cell["model_verdict"] in ("PASS", "FAIL"))
        cell["contract_invalid"] = bool(cell["stage"] == "explore"
                                        and cell["model_verdict"] not in ("PASS", "FAIL"))
        cell["extract_validated"] = cell["validated"]
        updated += 1
    (V5 / "CELLS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    payload["batch"] = str(V5.relative_to(REPO))
    payload["v6_rerun"] = {"run": str(run.relative_to(REPO)), "cells": updated,
                           "requests_used": budget.requests_used,
                           "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    (OUT / "SUMMARY.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    stages: dict[str, int] = {}
    for c in payload["cells"]:
        stages[c.get("stage", "?")] = stages.get(c.get("stage", "?"), 0) + 1
    validated = sum(1 for c in payload["cells"] if c.get("validated"))
    lines = ["# Extraction v6 — SUMMARY", "",
             f"- rerun cells: {updated}; requests: {budget.requests_used}/{MAX_REQUESTS}",
             f"- binary sha256: `{hashlib.sha256(binary.read_bytes()).hexdigest()}`",
             f"- prompt sha256: `{hashlib.sha256((REPO / 'prompts/rust_to_cir_extract_v6.md').read_bytes()).hexdigest()}`",
             f"- validated: **{validated}** / {len(payload['cells'])}",
             f"- stage distribution: `{stages}`", "",
             "| task | arm | rep | stage | validated | verdict | reason |",
             "| --- | --- | --- | --- | --- | --- | --- |"]
    for c in payload["cells"]:
        lines.append(f"| {c['task']} | {c['arm']} | {c['rep']} | {c.get('stage')} | "
                     f"{c.get('validated')} | {c.get('model_verdict')} | {c.get('reason','')[:60]} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"rerun": updated, "requests_used": budget.requests_used,
                      "validated": validated, "stages": stages}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
