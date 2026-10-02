#!/usr/bin/env python3
"""strong-link-v26 phase 3: unified offline scoring of the completed v26 batch.

Scores each cell's delivered candidate with the shared independent requirement
oracle and records run status, tokens, calls, and per-requirement outcomes.
Read-only over the batch; derived rows in a new directory.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402

BATCH = REPO / "experiments/strong-link-v26-batch-4"
OUT = REPO / "experiments/strong-link-v26-value"
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]
MODELS = ["deepseek-flash", "gpt-6-luna", "glm-5.3-flash"]


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def last_candidate(cell: Path, arm: str) -> Path | None:
    rounds = (sorted(cell.glob("code/round-*.rs"), key=lambda p: int(p.stem.split("-")[1]))
              if arm == "G3_concir" else
              sorted(cell.glob("run-*/round-*/candidate.rs"),
                     key=lambda p: int(p.parent.name.split("-")[1])))
    return rounds[-1] if rounds else None


def tokens_for(cell: Path) -> dict:
    audit = cell / "audit.jsonl"
    calls = 0
    inp = out = reason = 0
    missing = 0
    if audit.is_file():
        for line in audit.read_text(encoding="utf-8").splitlines():
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            calls += 1
            u = ev.get("usage") or {}
            for key, acc in (("input_tokens", "in"), ("output_tokens", "out"),
                             ("reasoning_tokens", "reason")):
                v = u.get(key)
                if v is None:
                    missing += 1
                elif acc == "in":
                    inp += int(v)
                elif acc == "out":
                    out += int(v)
                else:
                    reason += int(v)
    return {"calls": calls, "input_tokens": inp, "output_tokens": out,
            "reasoning_tokens": reason, "missing_token_fields": missing}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument"))
    args = ap.parse_args()
    binary, instrument = Path(args.binary), Path(args.instrument)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    summary = json.loads((BATCH / "SUMMARY.json").read_text())
    records = []
    for c in summary["cells"]:
        cell = c["cell"]
        task = tasks[cell["task"]]
        cdir = BATCH / cell["task"].replace("/", "__") / cell["model_id"] / cell["arm"] / f"rep{cell['rep']}"
        cand = last_candidate(cdir, cell["arm"])
        row = {"model": cell["model_id"], "task": cell["task"], "arm": cell["arm"],
               "run_status": c["status"], "accepted": c.get("accepted"),
               "record_status": c.get("record_status"),
               "candidate": str(cand) if cand else None,
               "candidate_sha256": sha(cand) if cand else None,
               **tokens_for(cdir)}
        if cand is None:
            row["evaluable"] = False
        else:
            try:
                r = rust_oracle.evaluate(cand.read_text(encoding="utf-8"), task.contract_path,
                                         task.reference_cir_path, OUT / "phase3-work"
                                         / cell["task"].replace("/", "__") / cell["model_id"]
                                         / cell["arm"], n_native=32, miri_seeds=0, run_timeout=10.0,
                                         binary=binary, instrument_binary=instrument)
                cov = r.get("coverage") or {}
                row.update({"built": r.get("built"), "behavior_ok": r.get("behavior_ok"),
                            "hang": r.get("hang"), "evaluable": cov.get("total") is not None,
                            "rc": cov.get("rc"), "rf": cov.get("rf"),
                            "statuses": cov.get("statuses"),
                            "properties": {p.get("id"): p.get("status")
                                           for p in (r.get("monitor") or {}).get("properties", [])}})
            except Exception as exc:  # noqa: BLE001
                row.update({"evaluable": False, "error": f"{type(exc).__name__}: {exc}"[:200]})
        records.append(row)
    agg = {}
    for model in MODELS:
        for arm in ARMS:
            rows = [r for r in records if r["model"] == model and r["arm"] == arm]
            planned = len(rows)
            ev = [r for r in rows if r.get("rf") is not None]
            rf_all = sum(r["rf"] for r in ev) / planned if planned else None
            agg[f"{model}/{arm}"] = {
                "planned": planned,
                "executed": sum(1 for r in rows if r["run_status"] == "executed"),
                "budget_exhausted": sum(1 for r in rows if r["run_status"] == "budget_exhausted"),
                "transport_error": sum(1 for r in rows if r["run_status"] == "transport_error"),
                "accepted": sum(1 for r in rows if r.get("accepted")),
                "evaluable": len(ev),
                "build_fail": sum(1 for r in rows if r.get("built") is False),
                "unsupported": sum(1 for r in rows if (r.get("statuses") or {}).values()
                                   and "unsupported" in (r.get("statuses") or {}).values()),
                "rc_all": round(sum((r.get("rc") or 0.0) for r in rows) / planned, 4) if planned else None,
                "rf_all": round(rf_all, 4) if rf_all is not None else None,
                "rf_acc": round(sum(r["rf"] for r in ev) / len(ev), 4) if ev else None,
                "calls": sum(r["calls"] for r in rows),
                "input_tokens": sum(r["input_tokens"] for r in rows),
                "output_tokens": sum(r["output_tokens"] for r in rows),
                "reasoning_tokens": sum(r["reasoning_tokens"] for r in rows),
            }
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "PHASE3_RESULTS.json").write_text(json.dumps({
        "batch": str(BATCH), "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "selection_rule": "last emitted round per cell", "records": records,
        "model_arm": agg}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(agg, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
