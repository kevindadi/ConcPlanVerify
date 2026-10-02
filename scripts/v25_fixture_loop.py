#!/usr/bin/env python3
"""strong-link-v25: manual-fixture repair loop for an internal-value goal.

This is NOT a new LLM result. Both responses are hand-written fixtures:
  1. a wrong candidate whose counter ends at 1;
  2. the corrected candidate (compare-and-swap retry) written by hand.
The loop shows that the machine-readable var_eq diagnostic is specific enough to
drive a repair, and that the corrected candidate re-verifies.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow import rust_oracle  # noqa: E402

OUT = REPO / "experiments/strong-link-v25-value"
ATOMIC = REPO / "benchmarks/families/atomic-data/atomic_lost_update"
WRONG = OUT / "controls/atomic_neg_print.rs"
CORRECT = REPO / ("experiments/strong-link-v23-thinking-2/atomic-data__atomic_lost_update"
                  "/gpt-6-luna/G3_concir/rep0/code/round-1.rs")
PROP = "main::c == 2"


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def evaluate(source: str, name: str, binary: Path, instrument: Path) -> dict:
    r = rust_oracle.evaluate(source, ATOMIC / "contract.json", ATOMIC / "fixed.cir.json",
                             OUT / "fixture-loop" / name, n_native=32, miri_seeds=0,
                             run_timeout=10.0, binary=binary, instrument_binary=instrument)
    prop = next((p for p in (r.get("monitor") or {}).get("properties", [])
                 if p.get("id") == PROP), None)
    return {"built": r.get("built"), "status": prop.get("status") if prop else None,
            "detail": prop.get("detail") if prop else None}


def main() -> int:
    binary = Path(REPO.parent / "ConcIR/target/v25-value/release/concir-backend")
    instrument = Path(REPO.parent / "ConcIR/target/v25-value/release/concir-instrument")
    wrong_src = WRONG.read_text()
    correct_src = CORRECT.read_text()
    round1 = evaluate(wrong_src, "round-1-wrong", binary, instrument)
    # The diagnostic carries resource, expected value and observed values+sids.
    feedback = (f"requirement goal {PROP} not satisfied. {round1['detail']}. "
                "Use a compare-and-swap retry in every worker so the final counter is 2.")
    round2 = evaluate(correct_src, "round-2-correct", binary, instrument)
    record = {
        "manual_fixture": True,
        "not_an_llm_result": True,
        "task": "atomic-data/atomic_lost_update",
        "goal": PROP,
        "rounds": [
            {"round": 1, "response_source": "human fixture", "source_sha256": sha(WRONG), **round1},
            {"round": 2, "response_source": "human fixture", "source_sha256": sha(CORRECT), **round2},
        ],
        "feedback_after_round1": feedback,
        "accepted_after_round2": round2["status"] == "PASS_bounded",
        "binary_sha256": sha(binary), "instrument_sha256": sha(instrument),
    }
    (OUT / "FIXTURE_LOOP.json").write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                                           encoding="utf-8")
    print(json.dumps(record, ensure_ascii=False, indent=2))
    return 0 if record["accepted_after_round2"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
