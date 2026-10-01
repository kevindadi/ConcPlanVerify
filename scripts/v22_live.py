#!/usr/bin/env python3
"""Live 96-cell four-system pilot. Loads keys from .env and does not print them."""

from __future__ import annotations

import json
import os
import sys
import threading
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SKELNET = Path("/Users/kevin/local-repos/SkelNet/python")
sys.path.insert(0, str(SKELNET))
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog  # noqa: E402
from cir_workflow.channels import AuditedClient, key_for  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.live import BudgetExhausted, LiveBudget  # noqa: E402
from cir_workflow.multi_gen import (  # noqa: E402
    SharedCounterBudget, execute_matrix, load_config, seed_executed_cache)
from cir_workflow.transport import CHANNELS  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v22")
BINARY = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-backend")
INSTRUMENT = Path("/Users/kevin/local-repos/ConcIR/target/release/concir-instrument")
OUT = REPO / "experiments/strong-link-v22-pilot"


def _skel_client(spec, budget, evidence_dir: Path, api_key: str, item: dict):
    """SkelNet's channel client. Token cap and reasoning stay the frozen config."""

    from skelnet.channels import build_client as skel_build
    from skelnet.params import params_for_model
    from skelnet.transport import build_registry

    from dataclasses import replace
    skel_spec = next(row for row in build_registry() if row.model_id == spec.model_id)
    if spec.model_id == "qwen3.8-flash":
        skel_spec = replace(skel_spec, channel="opencode-go", surface="chat",
                            stream=False, thinking=False, supports_seed=False)
    overrides = {
        "max_output_tokens": int(item["max_tokens"]),
        "seed_policy": "none",
    }
    if item.get("reasoning_effort"):
        overrides["reasoning_effort"] = item["reasoning_effort"]
    client = skel_build(
        skel_spec, params_for_model(skel_spec, **overrides),
        budget=budget, evidence_dir=evidence_dir, api_key=api_key, timeout=180)
    if spec.model_id == "qwen3.8-flash":
        # The OpenCode chat client does not read RunParams.thinking. Without
        # this body, Qwen thinks on a silent non-streaming socket until the
        # gateway drops it (APIConnectionError around the 180s client timeout).
        _send_qwen_thinking_off(client)
    return client


def _send_qwen_thinking_off(client):
    """Put ``enable_thinking: false`` on the wire and on the protocol record."""

    client.extra_body = {"enable_thinking": False}
    original = client._kwargs

    def _kwargs(messages, max_tokens, _orig=original):
        kwargs = _orig(messages, max_tokens)
        extra = dict(kwargs.get("extra_body") or {})
        extra.update(client.extra_body)
        kwargs["extra_body"] = extra
        return kwargs

    client._kwargs = _kwargs
    return client


def _api_key(spec) -> str:
    env_name = ("OPENCODE_API_KEY" if spec.model_id == "qwen3.8-flash"
                else CHANNELS[spec.channel].api_key_env)
    return key_for(spec, os.environ, env_name)


def _factory(params):
    def factory(spec, cell, budget, out_dir):
        return _skel_client(spec, budget, out_dir / "transport" / spec.model_id,
                            _api_key(spec), params[spec.model_id])
    return factory


def preflight(config, out_dir: Path) -> list[str]:
    budget = LiveBudget(out_dir / "budget.json", max_requests=int(config["global_physical_cap"]),
                        max_seconds=float(config["wall_seconds"]))
    counter = [0]
    lock = threading.Lock()
    blocked = []
    params = {item["model_id"]: item for item in config["models"]}
    for item in config["models"]:
        if counter[0] >= int(config["preflight_cap"]):
            blocked.append(item["model_id"])
            continue
        from cir_workflow.transport import require_experiment_model, build_registry
        spec = require_experiment_model(build_registry(), item["display_name"])
        capped = SharedCounterBudget(budget, int(config["preflight_cap"]), counter, lock, "preflight")
        try:
            raw = _skel_client(spec, capped, out_dir / "preflight" / spec.model_id,
                               _api_key(spec), params[spec.model_id])
            client = AuditedClient(
                raw, audit=AuditLog(out_dir / "preflight" / f"{spec.model_id}.jsonl"),
                run_id="v22-preflight", cell_id=f"preflight/{spec.model_id}", spec=spec,
                arm="preflight", task_id="preflight", replicate=0)
            client.complete("Reply with the single word pong.", "pong")
        except BudgetExhausted:
            blocked.append(item["model_id"])
            break
        except Exception as exc:  # noqa: BLE001 - block this system only
            blocked.append(item["model_id"])
            (out_dir / "preflight" / f"{spec.model_id}.error").write_text(
                type(exc).__name__ + "\n", encoding="utf-8")
    return blocked


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, default=NOTES / "FROZEN_CONFIG.json")
    parser.add_argument("--output", type=Path, default=OUT)
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    config = load_config(args.config)
    if not config.get("real_requests_authorized"):
        config["real_requests_authorized"] = True
    out = args.output
    out.mkdir(parents=True, exist_ok=True)
    resuming = (out / "budget.json").is_file() and (out / "SUMMARY.json").is_file()
    seeded = 0
    if resuming:
        previous = json.loads((out / "SUMMARY.json").read_text(encoding="utf-8"))
        blocked = list(previous.get("blocked_models") or [])
        seeded = seed_executed_cache(out, previous)
        print(json.dumps({"resume": True, "cached_cells": seeded,
                          "requests_used": json.loads((out / "budget.json").read_text()).get("requests_used")}),
              flush=True)
    else:
        blocked = preflight(config, out)
        (out / "PREFLIGHT.json").write_text(json.dumps({"blocked_models": blocked,
                                                       "requests_note": "see budget.json"}, indent=2) + "\n")
    runnable = [item for item in config["models"] if item["model_id"] not in blocked]
    if not runnable:
        print(json.dumps({"stop": "all_models_blocked", "blocked": blocked}))
        return 2
    live_config = dict(config)
    live_config["models"] = runnable
    live_config["run_id"] = "v22-pilot"
    tasks = {task.id: task for task in load_gen_tasks(REPO)}
    params = {item["model_id"]: item for item in runnable}
    result = execute_matrix(live_config, out, client_factory=_factory(params), tasks_by_id=tasks,
                            binary=BINARY, instrument=INSTRUMENT, live=True)
    slim = []
    for item in result["cells"]:
        slim.append({"cell": item["cell"], "status": item["status"], "error": item["error"],
                     "calls": item["calls"], "cached": bool(item.get("cached")),
                     "accepted": (item.get("record") or {}).get("accepted"),
                     "record_status": (item.get("record") or {}).get("status")})
    summary = {"real_requests": result["real_requests"], "stop": result["stop"],
               "blocked_models": blocked, "cells": slim}
    (out / "SUMMARY.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    (NOTES / "LIVE_SUMMARY.json").write_text(json.dumps({
        "real_requests": result["real_requests"], "stop": result["stop"],
        "blocked_models": blocked,
        "executed": sum(1 for item in slim if item["status"] == "executed"),
        "not_started": sum(1 for item in slim if item["status"] == "not_started"),
        "pilot_dir": str(out),
        "models": [item["model_id"] for item in runnable],
    }, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"stop": result["stop"], "real_requests": result["real_requests"],
                      "executed": sum(1 for item in slim if item["status"] == "executed")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
