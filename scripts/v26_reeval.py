#!/usr/bin/env python3
"""strong-link-v26 phase 2: unified offline re-eval of the three target models
(DeepSeek Flash history, GPT 6 Luna, GLM 5.3 Flash) over six tasks x four arms.

Read-only over history; derived rows in a new directory. Candidate selection is
pre-registered: the LAST emitted round of each cell (never the highest-scoring).
Missing cells keep a registration row.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402

OUT = REPO / "experiments/strong-link-v26-value"
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]
NEW_MODELS = ["gpt-6-luna", "glm-5.3-flash"]
SIX_TASKS = ["lock-order/abba_2lock", "condvar/bare_wait_no_predicate",
             "channel/bounded_backpressure_lock_held",
             "semaphore/acquire_twice_no_release", "atomic-data/atomic_lost_update",
             "structure/finite_call_loop"]
BATCHES = [REPO / "experiments/strong-link-v23-thinking-2",
           REPO / "experiments/strong-link-v22-three-model"]
DS_SUMMARY = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345/SUMMARY.json"


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def last_new_candidate(cell: Path, arm: str) -> Path | None:
    rounds = (sorted(cell.glob("code/round-*.rs"), key=lambda p: int(p.stem.split("-")[1]))
              if arm == "G3_concir" else
              sorted(cell.glob("run-*/round-*/candidate.rs"),
                     key=lambda p: int(p.parent.name.split("-")[1])))
    return rounds[-1] if rounds else None


def deepseek_cells(tasks: list[str]) -> dict:
    out = {}
    if not DS_SUMMARY.is_file():
        return out
    for c in json.loads(DS_SUMMARY.read_text())["cells"]:
        if c["rep"] != 0 or c["task"] not in tasks or c["arm"] not in ARMS:
            continue
        rec = c.get("record") or {}
        cand = rec.get("final_rust") or rec.get("final_artifact_path")
        out[(c["task"], c["arm"])] = {
            "candidate": cand, "arm_accepted": bool(c.get("accepted")),
            "hist_rc": c.get("rc"), "hist_rf": c.get("rf")}
    return out


def evaluate(cand: Path, task, binary: Path, instrument: Path) -> dict:
    try:
        r = rust_oracle.evaluate(cand.read_text(encoding="utf-8"), task.contract_path,
                                 task.reference_cir_path, OUT / "reeval-work"
                                 / task.id.replace("/", "__") / sha(cand)[:10],
                                 n_native=32, miri_seeds=0, run_timeout=10.0,
                                 binary=binary, instrument_binary=instrument)
        cov = r.get("coverage") or {}
        props = {p.get("id"): p.get("status") for p in (r.get("monitor") or {}).get("properties", [])}
        return {"built": r.get("built"), "behavior_ok": r.get("behavior_ok"),
                "hang": r.get("hang"), "rc": cov.get("rc"), "rf": cov.get("rf"),
                "statuses": cov.get("statuses"), "properties": props,
                "limitations": r.get("limitations")}
    except Exception as exc:  # noqa: BLE001
        return {"error": f"{type(exc).__name__}: {exc}"[:200]}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument"))
    args = ap.parse_args()
    binary, instrument = Path(args.binary), Path(args.instrument)
    tasks = {t.id: t for t in load_gen_tasks(REPO) if t.id in SIX_TASKS}
    ds = deepseek_cells(list(tasks))
    records = []
    for model in ["deepseek-flash-hist"] + NEW_MODELS:
        for task_id, task in tasks.items():
            for arm in ARMS:
                if model == "deepseek-flash-hist":
                    info = ds.get((task_id, arm))
                    cand = Path(info["candidate"]) if info and info.get("candidate") else None
                    accepted = info.get("arm_accepted") if info else None
                    batches = "flash-gen-main-v5"
                else:
                    cand, accepted, batches = None, None, None
                    for b in BATCHES:
                        c = b / task_id.replace("/", "__") / model / arm / "rep0"
                        if c.is_dir():
                            p = last_new_candidate(c, arm)
                            cache = c / "cache.json"
                            if p:
                                cand = p
                                batches = b.name
                                accepted = (json.loads(cache.read_text()).get("accepted")
                                            if cache.is_file() else None)
                row = {"model": model, "task": task_id, "arm": arm, "batch": batches,
                       "candidate": str(cand) if cand else None,
                       "candidate_sha256": sha(cand) if cand else None,
                       "arm_accepted": accepted}
                if cand is None:
                    row["state"] = "missing_candidate"
                else:
                    row.update(evaluate(cand, task, binary, instrument))
                records.append(row)
    (OUT / "REEVAL_V26.json").write_text(json.dumps({
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "selection_rule": "last emitted round per cell; missing keeps a row",
        "records": records}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    # aggregate per model/arm
    agg = {}
    for model in ["deepseek-flash-hist"] + NEW_MODELS:
        for arm in ARMS:
            rows = [r for r in records if r["model"] == model and r["arm"] == arm]
            planned = len(rows)
            ev = [r for r in rows if r.get("rf") is not None]
            rf_all = sum((r["rf"] or 0.0) for r in ev) / planned if planned else None
            rf_acc = (sum(r["rf"] for r in ev) / len(ev)) if ev else None
            agg[f"{model}/{arm}"] = {
                "planned": planned, "evaluable": len(ev),
                "missing": sum(1 for r in rows if r.get("state") == "missing_candidate"),
                "build_fail": sum(1 for r in rows if r.get("built") is False),
                "unsupported": sum(1 for r in rows if (r.get("statuses") or {}).values()
                                   and "unsupported" in (r.get("statuses") or {}).values()),
                "rf_all": round(rf_all, 4) if rf_all is not None else None,
                "rf_acc": round(rf_acc, 4) if rf_acc is not None else None}
    (OUT / "REEVAL_V26_SUMMARY.json").write_text(json.dumps(agg, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(agg, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
