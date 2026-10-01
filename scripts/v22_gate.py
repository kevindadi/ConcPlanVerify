#!/usr/bin/env python3
"""Offline gate for the v22 four-system pilot. No network."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import load_gen_tasks, run_g3_v2  # noqa: E402
from cir_workflow.multi_gen import execute_matrix, pilot_tasks  # noqa: E402

BINARY = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-backend")
INSTRUMENT = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-instrument")
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v22")
MODELS = [
    {"display_name": "DeepSeek Flash", "model_id": "deepseek-flash", "max_tokens": 4096, "temperature": 0},
    {"display_name": "Qwen", "model_id": "qwen3.8-flash", "max_tokens": 4096, "temperature": 0,
     "extra_body": {"enable_thinking": False}},
    {"display_name": "GPT 6 Luna", "model_id": "gpt-6-luna", "max_tokens": 4096, "temperature": 0},
    {"display_name": "GLM 5.3 Flash", "model_id": "glm-5.3-flash", "max_tokens": 16384, "temperature": 0,
     "reasoning_effort": "low"},
]


class MockModel:
    def __init__(self, spec, budget, params):
        self.model = spec.model_id
        self.max_tokens = params.get("max_tokens", 4096)
        self.temperature = params.get("temperature", 0)
        self.reasoning_effort = params.get("reasoning_effort")
        self.extra_body = params.get("extra_body")
        self.base_url = "mock://local"
        self.budget = budget
        self.stage = "code"

    def set_stage(self, stage: str) -> None:
        self.stage = stage

    def complete(self, system, user):
        self.budget.reserve({"cell_id": self.model, "logical_attempt": self.stage})
        if hasattr(self.budget, "mark_attempt"):
            self.budget.mark_attempt("returned")
        text = "fn main() {}\n" if self.stage != "cir" else "not-json"
        return SimpleNamespace(
            text=text, response_model=self.model,
            usage={"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            wall_ms=1, finish_reason="stop", request_id="mock", transport_attempt=1,
            prompt_sha256="mock")


def main() -> int:
    chosen = pilot_tasks(REPO / "benchmarks/GENERATION_MANIFEST.json")
    config = {
        "status": "mock_gate",
        "real_requests_authorized": False,
        "models": MODELS,
        "tasks": chosen,
        "arms": ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"],
        "reps": [0],
        "cell_caps": {"G0_direct": 2, "G1_self_iter": 8, "G2_tools_iter": 8, "G3_concir": 14},
        "global_physical_cap": 800,
        "preflight_cap": 8,
        "matrix_cap": 768,
        "wall_seconds": 12 * 3600,
        "concurrency": 3,
        "timeouts_seconds": {"deepseek-direct": 180, "opencode-go": 180, "dashscope-direct": 300},
        "run_id": "v22-mock",
    }
    NOTES.mkdir(parents=True, exist_ok=True)
    config_path = NOTES / "FROZEN_CONFIG.json"
    config_path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    # load_config stamps the file hash after the file exists
    from cir_workflow.multi_gen import load_config
    config = load_config(config_path)
    tasks = {task.id: task for task in load_gen_tasks(REPO)}
    params = {item["model_id"]: item for item in MODELS}

    def factory(spec, cell, budget, out_dir):
        return MockModel(spec, budget, params[spec.model_id])

    mock_out = REPO / "experiments/strong-link-v22-mock"
    result = execute_matrix(config, mock_out, client_factory=factory, tasks_by_id=tasks,
                            binary=BINARY, instrument=INSTRUMENT, live=False)
    summary = {
        "real_requests": 0,
        "cells": len(result["cells"]),
        "calls": result["calls"],
        "stop": result["stop"],
        "by_arm": {},
        "errors": [item["error"] for item in result["cells"] if item["error"]][:12],
    }
    for item in result["cells"]:
        arm = item["cell"]["arm"]
        summary["by_arm"].setdefault(arm, {"executed": 0, "other": 0})
        if item["status"] == "executed":
            summary["by_arm"][arm]["executed"] += 1
        else:
            summary["by_arm"][arm]["other"] += 1
    (NOTES / "DRY_RUN_MATRIX.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))
    ok = (summary["cells"] == 96 and summary["by_arm"].get("G0_direct", {}).get("executed") == 24
          and summary["by_arm"].get("G1_self_iter", {}).get("executed") == 24
          and summary["by_arm"].get("G2_tools_iter", {}).get("executed") == 24
          and summary["by_arm"].get("G3_concir", {}).get("executed") == 24
          and not summary["errors"])
    return 0 if ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
