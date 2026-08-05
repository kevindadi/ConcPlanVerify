"""Build the large-experiment summary table (todo section 4).

Joins every quantitative source into one flat table:

* ``results/scaling.json``        — parametric sweep points (verify leg)
* ``results/offline_v3.json``     — manifest cases under full analyze
* ``results/codegen.json``        — generated-code size + codegen tokens
* ``results/repair_8k.json`` and ``results/deep_repair.json`` — repair tokens
  (feedback_mode == full)

Output: ``results/summary_table.csv`` and ``results/summary_table.json``.
Each row: scaling factors x CIR metrics x CVN metrics x code size x tokens x
wall-clock, so state-explosion onset and codegen volume trends can be read
off a single table.
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
RESULTS = REPO / "results"

COLUMNS = [
    "source", "case", "pattern", "threads", "size",
    # CIR metrics
    "cir_statements", "cir_functions", "cir_resources", "cir_branches",
    "cir_spawns", "cir_bytes",
    # CVN metrics
    "cvn_places", "cvn_transitions", "cvn_arcs", "state_count",
    "analysis_complete", "status",
    # timings (ms)
    "analysis_ms", "verify_total_ms", "wall_ms",
    # generated-code metrics (codegen leg, when available)
    "code_loc", "code_bytes", "code_functions", "code_thread_spawns",
    "codegen_rounds", "codegen_input_tokens", "codegen_output_tokens",
    # repair tokens (feedback_mode == full, when available)
    "repair_input_tokens", "repair_output_tokens", "repair_rounds",
]


def load(name: str) -> dict | None:
    path = RESULTS / name
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def blank_row() -> dict:
    return {column: None for column in COLUMNS}


def cir_cols(row: dict, cir: dict | None) -> None:
    if not cir:
        return
    row["cir_statements"] = cir.get("statement_count")
    row["cir_functions"] = cir.get("function_count")
    row["cir_resources"] = cir.get("resource_count")
    row["cir_branches"] = cir.get("branch_count")
    row["cir_spawns"] = cir.get("spawn_count")
    row["cir_bytes"] = cir.get("cir_json_bytes")


def main() -> None:
    rows: list[dict] = []

    scaling = load("scaling.json")
    for point in (scaling or {}).get("points", []):
        row = blank_row()
        row.update(
            source="scaling_sweep",
            case=f"{point['pattern']}_{point['threads']}x{point['size']}",
            pattern=point["pattern"],
            threads=point["threads"],
            size=point["size"],
            cvn_places=point.get("places"),
            cvn_transitions=point.get("transitions"),
            cvn_arcs=(point.get("input_arcs") or 0) + (point.get("output_arcs") or 0),
            state_count=point.get("state_count"),
            analysis_complete=point.get("analysis_complete"),
            status=point.get("status"),
            analysis_ms=round(point.get("timings", {}).get("analysis_ms", 0), 3),
            verify_total_ms=round(point.get("timings", {}).get("total_ms", 0), 3),
            wall_ms=round(point.get("wall_ms", 0), 1),
        )
        cir_cols(row, point.get("cir_metrics"))
        rows.append(row)

    codegen_by_case: dict[str, dict] = {}
    codegen = load("codegen.json")
    for record in (codegen or {}).get("records", []):
        codegen_by_case[record["case_id"]] = record

    repair_by_case: dict[str, dict] = {}
    for name in ("repair_8k.json", "deep_repair.json"):
        data = load(name)
        for record in (data or {}).get("records", []):
            if record.get("feedback_mode") == "full":
                repair_by_case[record["case_id"]] = record

    offline = load("offline_v3.json")
    for record in (offline or {}).get("records", []):
        case_id = record["case_id"]
        cvn = record.get("cvn_metrics") or {}
        row = blank_row()
        row.update(
            source="benchmark",
            case=case_id,
            pattern=record.get("defect_type"),
            cvn_places=cvn.get("places"),
            cvn_transitions=cvn.get("transitions"),
            cvn_arcs=(cvn.get("input_arcs") or 0) + (cvn.get("output_arcs") or 0),
            state_count=cvn.get("state_count"),
            analysis_complete=cvn.get("analysis_complete"),
            status=(record.get("rust_cli") or {}).get("status"),
            analysis_ms=round(cvn.get("timings", {}).get("analysis_ms", 0), 3),
            verify_total_ms=round(cvn.get("timings", {}).get("total_ms", 0), 3),
            wall_ms=round(record.get("wall_ms", 0), 1),
        )
        cir_cols(row, record.get("cir_metrics"))

        generated = codegen_by_case.get(case_id)
        if generated:
            code = generated.get("code_metrics") or {}
            row.update(
                code_loc=code.get("loc"),
                code_bytes=code.get("bytes"),
                code_functions=code.get("functions"),
                code_thread_spawns=code.get("thread_spawns"),
                codegen_rounds=generated.get("codegen_rounds"),
                codegen_input_tokens=generated.get("total_input_tokens"),
                codegen_output_tokens=generated.get("total_output_tokens"),
            )

        repaired = repair_by_case.get(case_id)
        if repaired:
            row.update(
                repair_input_tokens=repaired.get("total_input_tokens"),
                repair_output_tokens=repaired.get("total_output_tokens"),
                repair_rounds=repaired.get("repair_rounds"),
            )
        rows.append(row)

    out_json = RESULTS / "summary_table.json"
    out_csv = RESULTS / "summary_table.csv"
    out_json.write_text(
        json.dumps({"columns": COLUMNS, "rows": rows}, ensure_ascii=False, indent=1),
        encoding="utf-8",
    )
    with out_csv.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=COLUMNS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"wrote {out_csv} and {out_json}: {len(rows)} rows")
    incomplete = [r["case"] for r in rows if r["analysis_complete"] is False]
    print("analysis_incomplete rows:", incomplete or "none")


if __name__ == "__main__":
    main()
