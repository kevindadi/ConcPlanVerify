#!/usr/bin/env python3
"""strong-link-v23 live thinking batch (3 models x 6 tasks x 4 arms, rep0).

Uses the config-resolved OpenCode channel and send parameters (thinking, max
output), a hard wall-clock timeout per call, the shared persistent budget, and
single-coordinator/resume guards. Keys come from .env and are never printed.

Run only after the offline gate is green."""

from __future__ import annotations

import json
import os
import sys
import threading
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog  # noqa: E402
from cir_workflow.channels import build_client, key_for  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.hard_timeout import HardTimeoutClient  # noqa: E402
from cir_workflow.live import BudgetExhausted, LiveBudget  # noqa: E402
from cir_workflow.multi_gen import SharedCounterBudget, execute_matrix, load_config  # noqa: E402
from cir_workflow.transport import CHANNELS, ModelSpec  # noqa: E402
from cir_workflow.v23_specs import config_spec  # noqa: E402

BINARY = Path(os.environ.get("CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend"))).resolve()
INSTRUMENT = Path(os.environ.get("CONCIR_INSTRUMENT", str(REPO.parent / "ConcIR/target/release/concir-instrument"))).resolve()
OUT = Path(os.environ.get("V23_OUT", str(REPO / "experiments/strong-link-v23-thinking"))).resolve()
# Deviation D-1: the OpenCode gateway reset concurrent Qwen thinking connections,
# so Qwen calls are serialized per model; global concurrency stays 3.
V23_MODEL_CONCURRENCY = {"qwen3.8-flash": 1}


def build_inner(entry: dict, spec: ModelSpec, budget, evidence_dir: Path, *, timeout: float,
                max_tokens: int):
    key = key_for(spec, os.environ, CHANNELS[spec.channel].api_key_env)
    inner = build_client(spec, budget=budget, evidence_dir=evidence_dir, api_key=key,
                         timeout=timeout, max_tokens=max_tokens,
                         thinking=entry.get("thinking"))
    return HardTimeoutClient(inner, seconds=float(entry.get("hard_timeout_seconds", 300)))


def preflight(config: dict, out_dir: Path) -> list[str]:
    """At most preflight_cap probes: identity, thinking params, usage fields."""
    budget = LiveBudget(out_dir / "budget.json", max_requests=int(config["global_physical_cap"]),
                        max_seconds=float(config["wall_seconds"]))
    counter = [0]
    lock = threading.Lock()
    blocked, records = [], []
    for entry in config["models"]:
        spec = config_spec(entry)
        if counter[0] >= int(config["preflight_cap"]):
            blocked.append(spec.model_id)
            continue
        capped = SharedCounterBudget(budget, int(config["preflight_cap"]), counter, lock, "preflight")
        try:
            client = build_inner(entry, spec, capped,
                                 out_dir / "preflight" / spec.model_id,
                                 timeout=180, max_tokens=int(entry["max_output_tokens"]))
            outcome = client.complete("Reply with the single word pong.", "pong")
            record = {
                "model_id": spec.model_id, "channel": spec.channel,
                "surface": spec.surface, "returned_model": getattr(outcome, "response_model", None),
                "request_params": getattr(client, "last_request_params", None),
                "thinking": entry.get("thinking"), "max_output_tokens": entry["max_output_tokens"],
                "usage": getattr(outcome, "usage", None),
                "reasoning_tokens": _reasoning(getattr(outcome, "usage", None)),
                "ok": getattr(outcome, "response_model", None) == spec.model_id,
            }
        except BudgetExhausted:
            blocked.append(spec.model_id)
            break
        except Exception as exc:  # noqa: BLE001 - block this system only
            record = {"model_id": spec.model_id, "channel": spec.channel,
                      "ok": False, "error_type": type(exc).__name__,
                      "error": str(exc)[:300]}
        records.append(record)
        if not record.get("ok"):
            blocked.append(spec.model_id)
    (out_dir / "PREFLIGHT.json").write_text(json.dumps(
        {"blocked_models": blocked, "records": records, "requests_note": "see budget.json"},
        indent=2) + "\n", encoding="utf-8")
    return blocked


def _reasoning(usage):
    if not isinstance(usage, dict):
        return None
    details = usage.get("completion_tokens_details") or usage.get("output_tokens_details") or {}
    return usage.get("reasoning_tokens") if isinstance(details, dict) else None


class MockModel:
    """Fake transport for the dry-run wiring check. No network."""

    def __init__(self, spec, budget):
        self.model = spec.model_id
        self.max_tokens = 4096
        self.temperature = 0
        self.budget = budget
        self.stage = "code"
        self.thinking = None
        self.calls = 0

    def set_stage(self, stage):
        self.stage = stage

    def complete(self, system, user):
        self.budget.reserve({"cell_id": self.model})
        self.budget.mark_attempt("returned")
        self.calls += 1
        return SimpleNamespace(text="fn main() {}\n", response_model=self.model,
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", request_id="mock",
                               transport_attempt=1)


def _dry_run(config: dict) -> int:
    out = OUT / "dry-run"
    out.mkdir(parents=True, exist_ok=True)
    entries = {m["model_id"]: m for m in config["models"]}
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    result = execute_matrix(
        config, out, client_factory=lambda spec, cell, budget, o: MockModel(spec, budget),
        tasks_by_id=tasks, binary=BINARY, instrument=INSTRUMENT, live=False,
        spec_resolver=lambda cell: config_spec(entries[cell["model_id"]]))
    by_arm: dict[str, int] = {}
    for item in result["cells"]:
        by_arm[item["cell"]["arm"]] = by_arm.get(item["cell"]["arm"], 0) + 1
    print(json.dumps({"dry_run": True, "cells": len(result["cells"]), "by_arm": by_arm,
                      "real_requests": 0, "stop": result["stop"]}, indent=2))
    return 0


def main() -> int:
    load_dotenv(REPO / ".env")
    config_path = Path(sys.argv[1]) if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else (
        Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v23/FROZEN_CONFIG.json"))
    config = load_config(config_path)
    if "--dry-run" in sys.argv:
        return _dry_run(config)
    from cir_workflow.binding import default_binary
    from cir_workflow.multi_gen import run_pin_error, workflow_fingerprint
    workflow = {"workflow_sha256": workflow_fingerprint(BINARY, INSTRUMENT, default_binary()),
                "config_sha256": config.get("_sha256")}
    pin_error = run_pin_error(OUT, workflow)
    if pin_error:
        print(json.dumps({"stop": "protocol_stop", "error": pin_error, "new_requests": 0}))
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    if "--skip-preflight" in sys.argv:
        pf = OUT / "PREFLIGHT.json"
        blocked = json.loads(pf.read_text())["blocked_models"] if pf.is_file() else []
    else:
        blocked = preflight(config, OUT)
    runnable = [m for m in config["models"] if m["model_id"] not in blocked]
    if not runnable:
        print(json.dumps({"stop": "all_models_blocked", "blocked": blocked}))
        return 2
    entries = {m["model_id"]: m for m in runnable}
    live_config = dict(config)
    live_config["models"] = runnable
    tasks = {t.id: t for t in load_gen_tasks(REPO)}

    def resolver(cell: dict) -> ModelSpec:
        return config_spec(entries[cell["model_id"]])

    def factory(spec, cell, budget, out_dir):
        entry = entries[spec.model_id]
        return build_inner(entry, spec, budget, out_dir / "transport" / spec.model_id,
                           timeout=float(entry.get("request_timeout_seconds", 180)),
                           max_tokens=int(entry["max_output_tokens"]))

    result = execute_matrix(live_config, OUT, client_factory=factory, tasks_by_id=tasks,
                            binary=BINARY, instrument=INSTRUMENT, live=True,
                            spec_resolver=resolver,
                            model_concurrency=V23_MODEL_CONCURRENCY)
    slim = [{"cell": item["cell"], "status": item["status"], "error": item["error"],
             "calls": item["calls"], "cached": item.get("cached"),
             "accepted": (item.get("record") or {}).get("accepted"),
             "record_status": (item.get("record") or {}).get("status")}
            for item in result["cells"]]
    from collections import Counter
    from cir_workflow.multi_gen import matrix_from_config
    planned_cells = len(matrix_from_config(config))
    unfinished = Counter(item["status"] for item in slim if item["status"] != "executed")
    complete = not unfinished and not blocked and len(slim) == planned_cells
    summary = {"real_requests": result["real_requests"], "stop": result["stop"],
               "blocked_models": blocked, "cells": slim, "matrix_complete": complete, "planned_cells": planned_cells,
               "unfinished_by_status": dict(unfinished)}
    (OUT / "SUMMARY.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"stop": result["stop"], "real_requests": result["real_requests"],
                      "executed": sum(1 for i in slim if i["status"] == "executed"),
                      "matrix_complete": complete, "unfinished_by_status": dict(unfinished),
                      "blocked": blocked}))
    return 0 if complete else 3


if __name__ == "__main__":
    raise SystemExit(main())
