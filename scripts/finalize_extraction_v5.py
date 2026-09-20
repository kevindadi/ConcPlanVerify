"""Finalize extraction-v5 CELLS.json (J-1): reasons, validated definition.

- `validated` = stage == "explore" AND contract evaluation succeeded AND
  model_verdict in {PASS, FAIL}. A verdict of `INVALID` (contract could not be
  evaluated) is counted separately as `contract_invalid`, never validated.
- every stage=codegen/labels/conform cell gets a non-empty `reason` (first
  `E\\d{3}` / panic line from the stored ConcIR stderr or the harness record).
No LLM requests.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUT = REPO / "experiments/extraction-v5"


def _short_reason(text: str) -> str:
    if not text:
        return ""
    m = re.search(r"panicked at ([^\s:]+:\d+)", text)
    if m:
        return f"panic at {m.group(1)}"
    m = re.search(r"\[(E\d{3})\]:?\s*([^\n]*)", text)
    if m:
        return f"{m.group(1)}: {m.group(2).strip()[:120]}"
    m = re.search(r"(invalid type: [^\n,]*|missing field `[^`]*`)[^\n]*", text)
    if m:
        return m.group(1)[:120]
    return text.strip().splitlines()[0][:120] if text.strip() else ""


def main() -> int:
    run = sorted(OUT.glob("run-*"))[-1]
    payload = json.loads((OUT / "CELLS.json").read_text(encoding="utf-8"))
    for cell in payload["cells"]:
        key = f"{cell['task']}__{cell['arm']}__rep{cell['rep']}"
        cell_dir = run / key
        result_path = cell_dir / "extraction_result.json"
        reason = ""
        if result_path.is_file():
            data = json.loads(result_path.read_text(encoding="utf-8"))
            reason = _short_reason(data.get("reason", ""))
            if not reason:
                reason = _short_reason(str(data.get("detail", "")))
            cell["stage"] = data.get("stage", cell.get("stage"))
            cell["model_verdict"] = data.get("model_verdict")
        if not reason:
            reason = "unclassified"
        cell["reason"] = reason
        verdict = cell.get("model_verdict")
        cell["validated"] = bool(cell.get("stage") == "explore" and verdict in ("PASS", "FAIL"))
        cell["contract_invalid"] = bool(cell.get("stage") == "explore"
                                        and verdict not in ("PASS", "FAIL"))
        cell["extract_validated"] = cell["validated"]
    stages: dict[str, int] = {}
    reasons: dict[str, int] = {}
    for cell in payload["cells"]:
        stages[cell.get("stage", "?")] = stages.get(cell.get("stage", "?"), 0) + 1
        if cell.get("stage") in ("codegen", "labels", "conform"):
            reasons[cell["reason"]] = reasons.get(cell["reason"], 0) + 1
    payload["validated"] = sum(1 for c in payload["cells"] if c["validated"])
    payload["contract_invalid"] = sum(1 for c in payload["cells"] if c["contract_invalid"])
    payload["stage_distribution"] = stages
    payload["reason_distribution"] = reasons
    payload["validated_definition"] = ("stage=explore AND contract evaluation "
                                       "succeeded AND verdict in {PASS,FAIL}")
    (OUT / "CELLS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")

    lines = ["# Extraction v5 — SUMMARY (from CELLS.json)", "",
             f"- batch: `{payload['batch']}`",
             f"- binary sha256: `{payload['binary_sha256']}`",
             f"- prompt sha256: `{payload['prompt_sha256']}`",
             f"- requests: {payload['requests_used']}/{payload['max_requests']}",
             f"- **validated: {payload['validated']}** / {len(payload['cells'])} "
             f"(definition: {payload['validated_definition']})",
             f"- contract_invalid (explore, verdict INVALID): {payload['contract_invalid']}",
             f"- harness errors: {payload['harness_errors']}",
             f"- stage distribution: `{stages}`",
             f"- failure reason distribution: `{reasons}`", "",
             "| task | arm | rep | stage | validated | verdict | reason |",
             "| --- | --- | --- | --- | --- | --- | --- |"]
    for c in payload["cells"]:
        lines.append(f"| {c['task']} | {c['arm']} | {c['rep']} | {c.get('stage')} | "
                     f"{c.get('validated')} | {c.get('model_verdict')} | "
                     f"{c.get('reason','')} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"validated": payload["validated"],
                      "contract_invalid": payload["contract_invalid"],
                      "stages": stages, "reasons": reasons}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
