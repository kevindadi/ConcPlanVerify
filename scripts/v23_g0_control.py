#!/usr/bin/env python3
"""strong-link-v23 G0/G2 local control: one real toolchain pass where the Rust
arms are scored by the SAME independent requirement oracle as G3. No model
network."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.multi_gen import run_one_cell  # noqa: E402

BIN = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-backend")
INSTRUMENT = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-instrument")
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v23")
TASK_ID = "lock-order/abba_2lock"


class FixtureModel:
    def __init__(self, text: str):
        self.text = text
        self.stage = "code"
        self.calls = 0
        self.max_tokens = 1
        self.temperature = 0
        self.model = "local-fixture"

    def set_stage(self, stage: str) -> None:
        self.stage = stage

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(text=self.text, response_model="local-fixture",
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", request_id="fixture",
                               transport_attempt=1, prompt_sha256="fixture")


def main() -> int:
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    task = tasks[TASK_ID]
    rust_text = (task.directory / "rust/fixed.rs").read_text(encoding="utf-8")
    out = REPO / "experiments/strong-link-v23-g0-control"
    cell = {"arm": "G0_direct", "model_id": "local-fixture", "task": TASK_ID,
            "rep": 0, "cell_cap": 2, "round_cap": 1, "g3_code_cap": 3}
    record = run_one_cell(cell, FixtureModel(rust_text), task, out,
                          binary=BIN, instrument=INSTRUMENT)
    rounds = record.get("rounds") or []
    covered = [bool(r.get("requirement_coverage")) for r in rounds]
    summary = {
        "real_requests": 0,
        "task": TASK_ID,
        "arm": "G0_direct",
        "accepted": record.get("accepted"),
        "rounds": [{"round": r.get("round"), "decision": r.get("decision"),
                    "oracle_status": r.get("oracle_status"),
                    "requirement_coverage": r.get("requirement_coverage"),
                    "oracle_error": r.get("oracle_error")} for r in rounds],
        "requirement_coverage_present": covered,
        "model_pass_is_not_rf": True,
    }
    NOTES.mkdir(parents=True, exist_ok=True)
    (NOTES / "G0_TOOLCHAIN_CONTROL.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    # The control passes only if the G0 candidate was scored by the requirement oracle.
    if not rounds or not any(covered):
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
