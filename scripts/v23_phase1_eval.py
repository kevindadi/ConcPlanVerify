#!/usr/bin/env python3
"""strong-link-v23 phase 1: zero-real-request consolidation of frozen candidates.

Scores the frozen v22 three-model 72-cell matrix and the 24 historical DeepSeek
cells with ONE independent Rust requirement oracle (``rust_oracle.evaluate``:
same contract, same instrumentation, 32 native runs, no Miri) and reports
planned/executed/accepted/evaluable, RC, RF_all, RF_acc, and a sensitivity range
over three pre-registered candidate classes.

No model call, no network, no source edit. Derived records only; historical
directories are read, never written.

Candidate classes (pre-registered, never "highest-RF round"):

  delivered        the artifact the arm would deliver: the accepted candidate
                   under that arm's own acceptance rule, else none.
  last_generated   the final candidate the model emitted, accepted or not.
  last_build_ok    the last candidate that instruments and builds.

Arm acceptance rules (reconstructed from persisted evidence only):

  G0_direct        accepts the first candidate iff it builds.
  G1_self_iter     accepts on a NO_ISSUES claim the most recent build_ok round.
  G2_tools_iter    accepts the round whose tools are green (last emitted round).
  G3_concir        accepts the last code round iff the CIR stage passed and the
                   code stage accepted (persisted accepted=true).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402

V22 = REPO / "experiments/strong-link-v22-three-model"
DS = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345"
TASKS = ["lock-order/abba_2lock", "condvar/bare_wait_no_predicate",
         "channel/bounded_backpressure_lock_held",
         "semaphore/acquire_twice_no_release", "atomic-data/atomic_lost_update",
         "structure/finite_call_loop"]
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]
MODELS = ["qwen3.8-flash", "gpt-6-luna", "glm-5.3-flash"]


def sha(p: Path | None) -> str | None:
    try:
        return hashlib.sha256(Path(p).read_bytes()).hexdigest()
    except (OSError, TypeError):
        return None


def round_no(path: Path) -> int:
    name = path.parent.name if path.name == "candidate.rs" else path.stem
    return int(name.split("-")[1])


# ───────────────────────────── cell discovery ─────────────────────────────

def _rounds(pattern_dir: Path, glob: str) -> list[Path]:
    return sorted(pattern_dir.glob(glob), key=round_no)


def v22_cells() -> list[dict]:
    summary = json.loads((V22 / "SUMMARY.json").read_text(encoding="utf-8"))
    out = []
    for c in summary["cells"]:
        cell = c["cell"]
        cdir = (V22 / cell["task"].replace("/", "__") / cell["model_id"]
                / cell["arm"] / f"rep{cell['rep']}")
        out.append({"source": "v22", "model_id": cell["model_id"],
                    "task": cell["task"], "arm": cell["arm"], "rep": cell["rep"],
                    "cell_dir": cdir, "accepted": bool(c.get("accepted")),
                    "record_status": c.get("record_status"),
                    "cached": bool(c.get("cached")), "calls": c.get("calls")})
    return out


def deepseek_cells() -> list[dict]:
    summary = json.loads((DS / "SUMMARY.json").read_text(encoding="utf-8"))
    out = []
    for c in summary["cells"]:
        if c["rep"] != 0 or c["task"] not in TASKS or c["arm"] not in ARMS:
            continue
        rec = c.get("record") or {}
        out.append({"source": "deepseek-history", "model_id": "deepseek-flash-hist",
                    "task": c["task"], "arm": c["arm"], "rep": 0,
                    "cell_dir": None, "accepted": bool(c.get("accepted")),
                    "record_status": c.get("status"),
                    "cached": False, "calls": rec.get("llm_calls"),
                    "hist_rc": c.get("rc"), "hist_rf": c.get("rf"),
                    "final_artifact": rec.get("final_artifact_path"),
                    "final_rust": rec.get("final_rust"),
                    "cir_path": rec.get("cir_path")})
    return out


def candidates(cell: dict) -> dict[str, list[Path]]:
    """Return {arm_class: [round paths]} for a cell (newest last)."""
    if cell["source"] == "v22":
        d = cell["cell_dir"]
        if cell["arm"] == "G3_concir":
            return {"code": _rounds(d / "code", "round-*.rs")}
        return {"rust": _rounds(d, "run-*/round-*/candidate.rs")}
    # deepseek history
    if cell["arm"] == "G3_concir":
        cir = cell.get("cir_path")
        d = Path(cir).parent.parent if cir else None
        return {"code": _rounds(d / "code", "round-*.rs") if d else []}
    final = cell.get("final_artifact")
    if not final:
        return {"rust": []}
    arm_dir = Path(final).parent.parent.parent
    return {"rust": _rounds(arm_dir, "run-*/round-*/candidate.rs")}


def delivered_path(cell: dict, rounds: list[Path], evals: dict[str, dict]) -> Path | None:
    """The arm's delivered candidate, reconstructed from persisted evidence."""
    if not cell["accepted"]:
        return None
    if cell["arm"] == "G0_direct":
        return rounds[0] if rounds else None
    if cell["arm"] == "G1_self_iter":
        built = [p for p in rounds if (evals.get(str(p), {}).get("built"))]
        return built[-1] if built else None
    # G2 and G3 accept on the last emitted round
    return rounds[-1] if rounds else None


def deepseek_delivered(cell: dict, rounds: list[Path]) -> Path | None:
    if not cell["accepted"]:
        return None
    for key in ("final_rust", "final_artifact"):
        p = cell.get(key)
        if p and Path(p).is_file():
            return Path(p)
    return rounds[-1] if rounds else None


# ───────────────────────────── evaluation ─────────────────────────────

class EvalCache:
    def __init__(self, out: Path, binary: Path, instrument: Path, tasks: dict):
        self.root = out / "work"
        self.root.mkdir(parents=True, exist_ok=True)
        self.binary = binary
        self.instrument = instrument
        self.tasks = tasks
        self.cache: dict[str, dict] = {}

    def get(self, path: Path, task) -> dict:
        key = str(path.resolve())
        if key in self.cache:
            return self.cache[key]
        work = self.root / f"{sha(path)[:12]}-{path.stem}"
        try:
            r = rust_oracle.evaluate(
                path.read_text(encoding="utf-8"), task.contract_path,
                task.reference_cir_path, work, n_native=32, miri_seeds=0,
                run_timeout=10.0, binary=self.binary, instrument_binary=self.instrument)
        except Exception as exc:  # noqa: BLE001 - one candidate must not stop the run
            r = {"built": False, "status": f"oracle_error:{type(exc).__name__}",
                 "coverage": None, "hang": None, "behavior_ok": None}
        cov = r.get("coverage")
        rec = {"rust_sha256": sha(path), "status": r.get("status"),
               "built": bool(r.get("built")), "hang": r.get("hang"),
               "behavior_ok": r.get("behavior_ok"), "coverage": cov,
               "limitations": r.get("limitations"),
               "failure": ("build_failed_after_instrument"
                           if not r.get("built") else
                           (None if cov else "not_evaluable"))}
        self.cache[key] = rec
        return rec


def evaluate_cell(cell: dict, cache: EvalCache) -> dict:
    task = cache.tasks[cell["task"]]
    cand = candidates(cell)
    flat = [p for v in cand.values() for p in v]
    # a delivered artifact may sit outside the globbed round dirs (historically
    # consolidated batches); include it in the evaluation set explicitly.
    extra = []
    if cell["source"] == "deepseek-history":
        for key in ("final_rust", "final_artifact"):
            p = cell.get(key)
            if p and Path(p).is_file():
                extra.append(Path(p))
    flat = list(dict.fromkeys(flat + extra))
    evals = {str(p): cache.get(p, task) for p in flat}
    if cell["source"] == "v22":
        delivered = delivered_path(cell, flat, evals)
    else:
        delivered = deepseek_delivered(cell, flat)
    if delivered is not None and str(delivered) not in evals:
        evals[str(delivered)] = cache.get(delivered, task)
    last = flat[-1] if flat else None
    built = [p for p in flat if evals.get(str(p), {}).get("built")]
    last_build = built[-1] if built else None
    row = {"source": cell["source"], "model_id": cell["model_id"], "task": cell["task"],
           "arm": cell["arm"], "rep": cell["rep"], "accepted": cell["accepted"],
           "record_status": cell.get("record_status"), "cached": cell.get("cached"),
           "planned_rounds": len(flat),
           "rounds": [{"round": round_no(p), "path": str(p), **evals[str(p)]}
                      for p in flat],
           "candidates": {
               "delivered": str(delivered) if delivered else None,
               "delivered_sha256": sha(delivered),
               "last_generated": str(last) if last else None,
               "last_generated_sha256": sha(last),
               "last_build_ok": str(last_build) if last_build else None,
               "last_build_ok_sha256": sha(last_build),
           },
           "hist_rc": cell.get("hist_rc"), "hist_rf": cell.get("hist_rf")}
    for kind, p in (("delivered", delivered), ("last_generated", last),
                    ("last_build_ok", last_build)):
        e = evals.get(str(p)) if p else None
        cov = (e or {}).get("coverage")
        row[f"{kind}_coverage"] = cov
        row[f"{kind}_rf"] = (cov or {}).get("rf")
        row[f"{kind}_rc"] = (cov or {}).get("rc")
    return row


# ───────────────────────────── aggregation ─────────────────────────────

def _mean(xs):
    xs = [x for x in xs if x is not None]
    return round(sum(xs) / len(xs), 4) if xs else None


def aggregate(rows: list[dict], models: list[str], source: str | None = None) -> dict:
    report: dict[str, Any] = {}
    for model in models:
        per: dict[str, Any] = {}
        for arm in ARMS:
            cells = [r for r in rows if r["model_id"] == model and r["arm"] == arm
                     and (source is None or r["source"] == source)]
            n = len(cells)
            if not n:
                continue
            delivered = [c for c in cells if c["delivered_coverage"]]
            rf_all = sum((c["delivered_rf"] or 0.0) if c["delivered_coverage"] else 0.0
                         for c in cells) / n
            rf_acc = _mean([c["delivered_rf"] for c in delivered])
            rc_all = sum((c["delivered_coverage"] or {}).get("rc", 0.0) or 0.0
                         for c in cells) / n
            per[arm] = {
                "planned": n,
                "executed": sum(1 for c in cells if c["planned_rounds"] > 0),
                "accepted": sum(1 for c in cells if c["accepted"]),
                "delivered_evaluable": len(delivered),
                "not_delivered": sum(1 for c in cells if not c["accepted"]),
                "build_fail": sum(1 for c in cells if c["delivered_coverage"] is None
                                  and c["accepted"]),
                "rc_all": round(rc_all, 4),
                "rf_all": round(rf_all, 4),
                "rf_acc": rf_acc,
                "rf_last_generated": _mean([c["last_generated_rf"] for c in cells]),
                "rf_last_build_ok": _mean([c["last_build_ok_rf"] for c in cells]),
            }
        report[model] = per
    return report


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(REPO / "experiments/strong-link-v23-phase1"))
    ap.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    ap.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    args = ap.parse_args()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=True)
    binary, instrument = Path(args.binary), Path(args.instrument)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}

    cache = EvalCache(out, binary, instrument, tasks)
    rows = []
    for cell in v22_cells() + deepseek_cells():
        rows.append(evaluate_cell(cell, cache))

    with (out / "PER_CELL.jsonl").open("w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n")

    new = aggregate([r for r in rows if r["source"] == "v22"], MODELS, "v22")
    hist = aggregate([r for r in rows if r["source"] == "deepseek-history"],
                     ["deepseek-flash-hist"], "deepseek-history")
    manifest = {
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
        "evaluator": "rust_oracle.evaluate n_native=32 miri_seeds=0 run_timeout=10",
        "tasks": TASKS, "arms": ARMS, "models_new": MODELS,
        "deepseek_source": str(DS), "v22_source": str(V22),
        "cell_count": len(rows),
        "selection_rules": {
            "delivered": "arm acceptance rule; else none",
            "last_generated": "final emitted candidate",
            "last_build_ok": "last candidate that instruments and builds"},
    }
    (out / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (out / "UNIFIED_RESULTS.json").write_text(
        json.dumps({"per_model_arm": new, "deepseek_history": hist},
                   ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    print(json.dumps({"cells": len(rows), "out": str(out)}, indent=2))
    for model, per in new.items():
        for arm, a in per.items():
            print(f"{model:16} {arm:14} acc {a['accepted']}/{a['planned']} "
                  f"deliv {a['delivered_evaluable']} RC {a['rc_all']} "
                  f"RF_all {a['rf_all']} RF_acc {a['rf_acc']} "
                  f"RF_last {a['rf_last_generated']} RF_build {a['rf_last_build_ok']}")
    print("deepseek-history", json.dumps(hist, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
