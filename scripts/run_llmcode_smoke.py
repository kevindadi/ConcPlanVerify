#!/usr/bin/env python3
"""§1.3 smoke: LLM code from the 45 verified CIRs of flash-gen-main-v1.

Skips the CIR stage and runs steps 2-5 of §1.1 (LLM Rust -> instrument ->
build -> conform --op-resource -> monitor -> behavior) on the CIRs the old
tool-codegen pipeline could not all land. Budget <= 180 DeepSeek Flash requests.
Writes `experiments/gen-llmcode-smoke-v1/{SUMMARY.json,SUMMARY.md}`.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks, run_llmcode_from_cir  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

G3_BATCH = "experiments/flash-gen-main-v1/run-20260922T015338"


def accepted_cirs() -> list[dict]:
    out = []
    root = REPO / G3_BATCH
    for cell in sorted(glob.glob(f"{root}/**/G3_concir/CELL.json", recursive=True)):
        d = json.loads(Path(cell).read_text(encoding="utf-8"))
        if d.get("accepted") and d.get("cir_path") and Path(d["cir_path"]).is_file():
            out.append({"task": d["task"], "cir": d["cir_path"],
                        "codegen_error": d.get("codegen_error")})
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-requests", type=int, default=180)
    parser.add_argument("--k-code", type=int, default=3)
    parser.add_argument("--out", default="experiments/gen-llmcode-smoke-v1")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "concir/target/release/concir-backend"))
    args = parser.parse_args()

    load_dotenv(REPO / ".env")
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("DEEPSEEK_API_KEY missing", file=sys.stderr)
        return 2
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    binary = Path(args.binary)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    cirs = accepted_cirs()

    out = REPO / args.out
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=6 * 3600)
    cells = []
    stop = None
    for entry in cirs:
        reason = budget.exhausted()
        if reason:
            cells.append({"task": entry["task"], "status": "not_run", "reason": reason})
            stop = reason
            continue
        task = tasks.get(entry["task"])
        cell_dir = batch / entry["task"].replace("/", "__")
        cell_dir.mkdir(parents=True, exist_ok=True)
        try:
            client = DeepSeekFlashClient(api_key=api_key, budget=budget,
                                         evidence_dir=batch / "llm", timeout=90.0,
                                         max_tokens=4096)
            record = run_llmcode_from_cir(client, binary, task, Path(entry["cir"]),
                                          cell_dir, k_code=args.k_code)
        except Exception as exc:  # noqa: BLE001
            record = {"task": entry["task"], "status": "error",
                      "error": f"{type(exc).__name__}: {exc}", "rounds": []}
        (cell_dir / "CELL.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        rounds = record.get("rounds", [])
        decisions = Counter(r.get("decision") for r in rounds)
        cell = {
            "task": entry["task"], "accepted": record.get("accepted"),
            "accepted_with_proof": record.get("accepted_with_proof"),
            "rounds": len(rounds), "codegen_error_before": bool(entry["codegen_error"]),
            "conform_ok": (rounds and rounds[-1].get("decision") == "accepted"
                           and rounds[-1].get("conform", {}).get("conformant") ==
                           rounds[-1].get("conform", {}).get("traces")),
            "monitor_status": rounds[-1].get("monitor_status") if rounds else None,
            "monitor_fail": rounds[-1].get("monitor_fail") if rounds else None,
            "instrument_limit": record.get("instrument_limit"),
            "decisions": dict(decisions), "status": record.get("status"),
        }
        cells.append(cell)
        (batch / "SUMMARY.json").write_text(
            json.dumps({"cells": cells, "stop_reason": stop}, indent=2) + "\n",
            encoding="utf-8")
        print(f"{entry['task']} accepted={cell['accepted']} rounds={cell['rounds']} "
              f"reqs={budget.requests_used}", flush=True)

    ran = [c for c in cells if c.get("status") != "not_run"]
    accepted = [c for c in ran if c.get("accepted")]
    conform_ok = [c for c in ran if c.get("conform_ok")]
    built = [c for c in ran if c.get("rounds")]
    lines = [
        "# gen-llmcode-smoke-v1 — LLM code from verified CIR (§1.3)",
        "",
        f"LLM generates Rust from the {len(cirs)} accepted CIRs of "
        "`flash-gen-main-v1`; instrument v2 -> build -> `conform --op-resource` -> "
        "`monitor` -> behavior, with conform/monitor feedback (K_code=3).",
        "",
        f"- cells: {len(ran)} (not_run {len(cells) - len(ran)}), requests {budget.requests_used}",
        f"- build rate: {len(built)}/{len(ran)}",
        f"- conform PASS rate: **{len(conform_ok)}/{len(ran)}**",
        f"- accepted (build ∧ conform ∧ monitor ∧ behavior): {len(accepted)}/{len(ran)}",
        f"- accepted-with-proof: {sum(1 for c in ran if c.get('accepted_with_proof'))}/{len(ran)}",
        f"- of the 23 CIRs the tool codegen could not build: "
        f"{sum(1 for c in ran if c.get('codegen_error_before') and c.get('conform_ok'))}/"
        f"{sum(1 for c in ran if c.get('codegen_error_before'))} now pass conform",
        "",
        "| task | accepted | rounds | conform_ok | monitor | codegen_err_before | decisions |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for c in cells:
        lines.append(f"| {c['task']} | {c.get('accepted')} | {c.get('rounds')} | "
                     f"{c.get('conform_ok')} | {c.get('monitor_status')} | "
                     f"{c.get('codegen_error_before')} | {c.get('decisions')} |")
    (batch / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    out.mkdir(parents=True, exist_ok=True)
    (out / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"done: {budget.requests_used} requests; conform {len(conform_ok)}/{len(ran)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
