#!/usr/bin/env python3
"""strong-link-v23 local G3 toolchain control. One real local ConcIR pass over a
canned CIR and Rust reply. No model network."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import load_gen_tasks, run_g3_v2  # noqa: E402

BIN = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-backend")
INSTRUMENT = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-instrument")
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v23")
TASK_ID = "lock-order/abba_2lock"


class FixtureModel:
    def __init__(self, cir_text: str, rust_text: str):
        self.cir_text = cir_text
        self.rust_text = rust_text
        self.stage = "cir"
        self.calls = 0
        self.max_tokens = 1
        self.temperature = 0
        self.model = "local-fixture"

    def set_stage(self, stage: str) -> None:
        self.stage = stage

    def complete(self, system, user):
        self.calls += 1
        text = self.cir_text if self.stage == "cir" else self.rust_text
        return SimpleNamespace(text=text, response_model="local-fixture",
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", request_id="fixture",
                               transport_attempt=1, prompt_sha256="fixture")


def main() -> int:
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    task = tasks[TASK_ID]
    cir_text = task.reference_cir_path.read_text(encoding="utf-8")
    rust_text = "fn main() {\n    println!(\"fixture\");\n}\n"
    client = FixtureModel(cir_text, rust_text)
    out = REPO / "experiments/strong-link-v23-g3-control"
    record = run_g3_v2(client, BIN, task, out, k_cir=1, k_code=1, instrument_binary=INSTRUMENT)
    stages = []
    for rnd in record.get("rounds") or []:
        stages.append({"stage": rnd.get("stage"), "decision": rnd.get("decision"),
                       "check": rnd.get("check"),
                       "explore": (rnd.get("explore") or {}).get("outcome")
                       if isinstance(rnd.get("explore"), dict) else rnd.get("explore")})
    summary = {"real_requests": 0, "fixture_calls": client.calls, "task": TASK_ID,
               "reference_cir": str(task.reference_cir_path),
               "cir_accepted": (record.get("cir_stage") or {}).get("accepted"),
               "code_stage_present": record.get("code_stage") is not None,
               "stages": stages, "status": record.get("status"),
               "model_pass_is_not_rf": True}
    NOTES.mkdir(parents=True, exist_ok=True)
    (NOTES / "G3_TOOLCHAIN_CONTROL.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    if not stages:
        return 2
    if stages[0].get("check") is None and stages[0].get("decision") == "parse_error":
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
