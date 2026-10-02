#!/usr/bin/env python3
"""strong-link-v29 unified offline scoring for one batch directory.

Scores each cell's delivered candidate with the shared independent requirement
oracle (same scorer as v28 PHASE3_RESULTS, so numbers are comparable) and
records run status, stage loss, tokens, calls, and per-requirement outcomes.
Read-only over the batch; results go to a separate directory.

Usage:
  v29_phase3_eval.py --batch experiments/strong-link-v29-online \
                     --out experiments/strong-link-v29-online/scored
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

ARMS = ["G3_concir"]
MODELS = ["gpt-6-luna", "glm-5.3-flash"]


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
    audit_events = 0
    attempts = set()
    inp = out = reason = 0
    missing = 0
    if audit.is_file():
        for line in audit.read_text(encoding="utf-8").splitlines():
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            audit_events += 1
            aid = ev.get("attempt_id")
            if aid:
                attempts.add(aid)
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
    return {"calls": len(attempts), "audit_events": audit_events,
            "input_tokens": inp, "output_tokens": out,
            "reasoning_tokens": reason, "missing_token_fields": missing}


def stage_loss(record_status: str | None, run_status: str) -> str:
    if run_status == "transport_error":
        return "external_transport"
    if run_status == "budget_exhausted":
        return "external_budget"
    if record_status in {"cir_generation_failed", "generation_failed"}:
        return "cir_generation_failed"
    if record_status == "unsupported":
        return "cir_unsupported"
    if record_status == "instrumented_build_failed":
        return "tool_instrumentation"
    if record_status == "capability_gap":
        return "checker_capability_gap"
    if record_status in {"candidate_error", "source_build_failed"}:
        return "code_candidate_error"
    if record_status in {"accepted", "satisfied_bounded"}:
        return "delivered"
    return record_status or "unknown"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--batch", required=True)
    ap.add_argument("--out", default="")
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/v28-verified/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/v28-verified/release/concir-instrument"))
    ap.add_argument("--binding", default=str(REPO.parent / "ConcIR/target/v28-verified/release/bind_check"))
    args = ap.parse_args()
    batch = (REPO / args.batch).resolve() if not Path(args.batch).is_absolute() else Path(args.batch)
    out = (REPO / args.out).resolve() if args.out and not Path(args.out).is_absolute() else (
        Path(args.out) if args.out else batch / "scored")
    binary, instrument = Path(args.binary), Path(args.instrument)
    binding = Path(args.binding)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    summary = json.loads((batch / "SUMMARY.json").read_text())
    records = []
    for c in summary["cells"]:
        cell = c["cell"]
        task = tasks[cell["task"]]
        cdir = batch / cell["task"].replace("/", "__") / cell["model_id"] / cell["arm"] / f"rep{cell['rep']}"
        cand = last_candidate(cdir, cell["arm"])
        record_status = c.get("record_status")
        row = {"model": cell["model_id"], "task": cell["task"], "arm": cell["arm"],
               "run_status": c["status"], "accepted": c.get("accepted"),
               "record_status": record_status,
               "stage_loss": stage_loss(record_status, c["status"]),
               "candidate": str(cand) if cand else None,
               "candidate_sha256": sha(cand) if cand else None,
               **tokens_for(cdir)}
        if cand is None:
            row["evaluable"] = False
        else:
            try:
                r = rust_oracle.evaluate(cand.read_text(encoding="utf-8"), task.contract_path,
                                         task.reference_cir_path,
                                         out / "work" / cell["task"].replace("/", "__")
                                         / cell["model_id"] / cell["arm"],
                                         n_native=32, miri_seeds=0, run_timeout=10.0,
                                         binary=binary, instrument_binary=instrument,
                                         binding_binary=binding)
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
            evaluable = [r for r in rows if r.get("rf") is not None]
            delivered = [r for r in evaluable if r.get("accepted") is True]
            agg[f"{model}/{arm}"] = {
                "planned": planned,
                "executed": sum(1 for r in rows if r["run_status"] == "executed"),
                "budget_exhausted": sum(1 for r in rows if r["run_status"] == "budget_exhausted"),
                "transport_error": sum(1 for r in rows if r["run_status"] == "transport_error"),
                "accepted": sum(1 for r in rows if r.get("accepted")),
                "delivered_evaluable": len(delivered),
                "delivered_unevaluable": sum(1 for r in rows if r.get("accepted") and r.get("rf") is None),
                "not_delivered": sum(1 for r in rows if not r.get("accepted")),
                "delivery_rate": round(len(delivered) / planned, 4) if planned else None,
                "build_fail": sum(1 for r in rows if r.get("built") is False),
                "RF_delivered_all": round(sum(r["rf"] for r in delivered) / planned, 4) if planned else None,
                "RF_delivered_acc": round(sum(r["rf"] for r in delivered) / len(delivered), 4)
                                    if delivered else None,
                "RF_last_generated_all": round(sum(r["rf"] for r in evaluable) / planned, 4) if planned else None,
                "RF_last_generated_eval": round(sum(r["rf"] for r in evaluable) / len(evaluable), 4)
                                          if evaluable else None,
                "calls": sum(r["calls"] for r in rows),
                "input_tokens_known": sum(r["input_tokens"] for r in rows),
                "output_tokens_known": sum(r["output_tokens"] for r in rows),
                "reasoning_tokens_known": sum(r["reasoning_tokens"] for r in rows),
                "token_fields_missing": sum(r.get("missing_token_fields", 0) for r in rows),
                "stage_loss": dict(collections.Counter(r["stage_loss"] for r in rows)),
            }
    out.mkdir(parents=True, exist_ok=True)
    (out / "PHASE3_RESULTS.json").write_text(json.dumps({
        "batch": str(batch), "using": str(batch),
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "binding_sha256": sha(binding),
        "selection_rule": "last emitted round per cell",
        "scorer": "rust_oracle.evaluate against the task reference CIR (same as v28)",
        "records": records, "model_arm": agg}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8")
    print(json.dumps(agg, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
