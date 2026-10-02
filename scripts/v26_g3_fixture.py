#!/usr/bin/env python3
"""strong-link-v26: end-to-end G3-orchestration fixture (manual, not an LLM run).

Runs the real `candidate_eval.evaluate_candidate` entry: a wrong candidate must
yield a specific requirement repair feedback; the hand-corrected candidate must
re-verify. Also checks that the conformance projection drops `value` while the
monitor projection keeps it.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow import candidate_eval, generation  # noqa: E402

OUT = REPO / "experiments/strong-link-v26-value"
TASKS = {
    "atomic": REPO / "benchmarks/families/atomic-data/atomic_lost_update",
    "bare": REPO / "benchmarks/families/condvar/bare_wait_no_predicate",
}
WRONG = {"atomic": REPO / "experiments/strong-link-v25-value/controls/atomic_neg_print.rs",
         "bare": REPO / "experiments/strong-link-v25-value/controls/bare_neg_print.rs"}
CORRECT = {
    "atomic": REPO / ("experiments/strong-link-v23-thinking-2/atomic-data__atomic_lost_update"
                      "/gpt-6-luna/G3_concir/rep0/code/round-1.rs"),
    "bare": REPO / ("experiments/strong-link-v23-thinking-2/condvar__bare_wait_no_predicate"
                    "/gpt-6-luna/G3_concir/rep0/code/round-1.rs"),
}


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def has_op(path: Path, op: str) -> bool:
    for f in sorted(path.glob("*.jsonl")):
        for line in f.read_text(encoding="utf-8").splitlines():
            if f'"op":"{op}"' in line.replace(" ", ""):
                return True
    return False


def run_one(name: str, source: str, binary: Path, instrument: Path, binding: Path) -> dict:
    tdir = TASKS[name]
    cir, contract = tdir / "fixed.cir.json", tdir / "contract.json"
    props, complete = generation._cir_props_for(binary, cir, contract)
    out = OUT / "g3-fixture" / name
    res = candidate_eval.evaluate_candidate(
        source, cir, contract, out, binary=binary, instrument=instrument, n_runs=32,
        accepted=True, cir_props=props, cir_complete=complete,
        cell_id=f"fixture/{name}", candidate_kind="online", round_no=1,
        binding_binary=binding)
    follow = res.get("followup") or {}
    return {
        "action": follow.get("action"), "category": follow.get("category"),
        "status": follow.get("status"), "feedback": (follow.get("feedback") or "")[:400],
        "ledger_reasons": (res.get("ledger") or {}).get("reasons"),
        "monitor": res.get("monitor"),
        "conform": res.get("conform"),
        "monitor_has_value": has_op(out / "monitor-traces", "value"),
        "conform_has_value": has_op(out / "conform-traces", "value"),
        "source_sha256": res.get("source_sha256"), "cir_sha256": res.get("cir_sha256"),
        "contract_sha256": res.get("contract_sha256"),
        "backend_sha256": res.get("backend_sha256"), "instrument_sha256": res.get("instrument_sha256"),
        "evidence_path": follow.get("evidence_path"),
    }


def main() -> int:
    binary = Path(REPO.parent / "ConcIR/target/v25-value/release/concir-backend")
    instrument = Path(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument")
    binding = Path(REPO.parent / "ConcIR/target/v25-value/release/bind_check")
    record = {"manual_fixture": True, "not_an_llm_result": True, "rounds": {}}
    for name in ("atomic", "bare"):
        wrong = run_one(name, WRONG[name].read_text(), binary, instrument, binding)
        correct = run_one(name, CORRECT[name].read_text(), binary, instrument, binding)
        record["rounds"][name] = {
            "round1_wrong": wrong, "round2_correct": correct,
            "repaired": wrong["action"] in ("repair", "repair_protocol")
                        and correct["action"] == "accept",
            "value_kept_in_monitor": wrong["monitor_has_value"],
            "value_dropped_from_conform": not wrong["conform_has_value"],
        }
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "G3_FIXTURE_LOOP.json").write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(record, ensure_ascii=False, indent=2))
    ok = all(v["repaired"] and v["value_kept_in_monitor"] and v["value_dropped_from_conform"]
             for v in record["rounds"].values())
    return 0 if ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
