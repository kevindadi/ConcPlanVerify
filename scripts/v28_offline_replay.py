#!/usr/bin/env python3
"""strong-link-v28 step 1: offline old->new toolchain replay of v27 G3 candidates.

Same frozen source/CIR/contract; only the toolchain changes. Uses the real
candidate_eval entry for both toolchains and records the per-cell diff. No model
requests. This is replay, not a new online generation result.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow import candidate_eval, generation  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.toolchain import from_dir  # noqa: E402

OUT = REPO / "experiments/strong-link-v28"
OLD = from_dir(REPO.parent / "ConcIR/target/release", label="legacy-release")
NEW = from_dir(REPO.parent / "ConcIR/target/v28-verified/release", label="v28-verified")
BATCHES = ["experiments/strong-link-v27-main", "experiments/strong-link-v27-ablation/full"]


def last_round(cell: Path) -> Path | None:
    rounds = sorted(cell.glob("code/round-*.rs"), key=lambda p: int(p.stem.split("-")[1]))
    return rounds[-1] if rounds else None


def cells() -> list[tuple[str, str, Path]]:
    out = []
    for batch in BATCHES:
        base = REPO / batch
        for cell in sorted(base.glob("*/*/G3_concir/rep*")):
            cand = last_round(cell)
            if cand:
                parts = cell.relative_to(base).parts
                out.append((batch, f"{parts[0]}/{parts[1]}/{parts[2]}", cand))
    return out


def evaluate(tc, source, task, cir, contract, out_dir, props, complete) -> dict:
    res = candidate_eval.evaluate_candidate(
        source, cir, contract, out_dir, binary=tc.backend, instrument=tc.instrument,
        n_runs=32, accepted=True, cir_props=props, cir_complete=complete,
        cell_id="replay", candidate_kind="replay", binding_binary=tc.bind_check)
    led = res.get("ledger") or {}
    return {
        "instrument_sha256": res.get("instrument_sha256"),
        "binding_sha256": res.get("binding_sha256"),
        "ledger_verdict": led.get("current_evaluation"),
        "delivery_status": led.get("delivery_status"),
        "monitor": res.get("monitor"),
        "binding_mapping_keys": sorted((res.get("binding") or {}).get("mapping", {}).keys()),
        "binding_ambiguous": [a.get("rust") for a in (res.get("binding") or {}).get("ambiguous", [])],
        "conform": {k: (res.get("conform") or {}).get(k) for k in ("traces", "conformant", "violation")},
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(OUT))
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    records = []
    all_cells = cells()
    if args.limit:
        all_cells = all_cells[:args.limit]
    for batch, label, cand in all_cells:
        task_id = label.split("/")[0].replace("__", "/")
        task = tasks[task_id]
        source = cand.read_text(encoding="utf-8")
        props, complete = generation._cir_props_for(OLD.backend, task.reference_cir_path,
                                                    task.contract_path)
        old = evaluate(OLD, source, task, task.reference_cir_path, task.contract_path,
                       out / "replay-work/old" / label.replace("/", "__"), props, complete)
        new = evaluate(NEW, source, task, task.reference_cir_path, task.contract_path,
                       out / "replay-work/new" / label.replace("/", "__"), props, complete)
        changed = (old["ledger_verdict"] != new["ledger_verdict"]
                   or old["monitor"] != new["monitor"]
                   or old["binding_mapping_keys"] != new["binding_mapping_keys"])
        records.append({"batch": batch, "cell": label, "candidate": str(cand),
                        "changed": changed, "old": old, "new": new})
    summary = {
        "cells": len(records),
        "changed": sum(1 for r in records if r["changed"]),
        "verdict_changes": sum(1 for r in records
                               if r["old"]["ledger_verdict"] != r["new"]["ledger_verdict"]),
        "monitor_changes": sum(1 for r in records if r["old"]["monitor"] != r["new"]["monitor"]),
        "binding_changes": sum(1 for r in records
                               if r["old"]["binding_mapping_keys"] != r["new"]["binding_mapping_keys"]),
        "old_instrument": OLD.fingerprint()["instrument_sha256"],
        "new_instrument": NEW.fingerprint()["instrument_sha256"],
    }
    (out / "OFFLINE_REPLAY_DIFF.json").write_text(json.dumps(
        {"summary": summary, "records": records}, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
