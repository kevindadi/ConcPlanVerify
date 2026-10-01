#!/usr/bin/env python3
"""Future paid entry for the six-case feedback pilot.

Without ``--confirm-paid-run`` this process validates the freeze and exits.
A cell that uses up its own repair rounds does not stop the rest of the model.
Identity or service failure stops that model only. A global budget stop records
every planned cell, including those that never started.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Callable

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.channels import AuditedClient, build_client, key_for  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.feedback_runner import load_frozen, validate_config, run_arm  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, build_registry, require_experiment_model  # noqa: E402

# A cell that merely used its two repair rounds must not stop its model.
STOP_MODEL = {
    "identity_mismatch", "identity_unconfirmed", "request_error", "cell_error",
}
GLOBAL_STOPS = {
    "global_request_budget_exhausted", "global_time_budget_exhausted",
}


def _blank(cell: dict, stop: str, status: str) -> dict[str, Any]:
    return {
        "model": cell["model"], "model_id": cell["model_id"], "case": cell["case"],
        "arm": cell["arm"], "stop": stop, "status": status,
        "actual_model_requests_this_run": 0, "semantic_repair_rounds": 0,
        "requests_recorded": 0, "usage": [], "in_denominator": True,
    }


def execute_batch(config: dict, rows: list[dict], out: Path, *,
                  client_factory: Callable, evaluate: Callable | None = None,
                  score: Callable | None = None,
                  audit_events: list[dict] | None = None,
                  max_repairs: int = 2,
                  execution_mode: str = "fake",
                  budget_path: Path | None = None) -> dict[str, Any]:
    """Run the frozen matrix. Always returns one row per planned cell."""

    plan = validate_config(config, rows)
    if not plan["ok"]:
        return {"ok": False, "errors": plan["errors"], "results": [], "planned_cells": 0}
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    by_id = {row["id"]: row for row in rows}
    events = list(audit_events or [])
    results: list[dict] = []
    stopped: dict[str, str] = {}
    global_reason = None
    for cell in plan["matrix"]:
        if global_reason:
            results.append(_blank(cell, global_reason, "not_started"))
            continue
        if cell["model_id"] in stopped:
            row = _blank(cell, stopped[cell["model_id"]], "blocked")
            row["blocked_by"] = stopped[cell["model_id"]]
            results.append(row)
            continue
        spec = require_experiment_model(build_registry(), cell["model"])
        cell_id = f"{spec.model_id}/{cell['case']}/{cell['arm']}"
        dest = out / spec.model_id / cell["case"] / cell["arm"]
        try:
            case = load_frozen(by_id[cell["case"]], config.get("tools"))
            client = client_factory(spec, cell_id, cell["arm"], cell["case"])
            cell_events = [event for event in events
                           if event.get("cell_id") == cell_id and event.get("arm") == cell["arm"]]
            result = run_arm(case, spec, cell["arm"], client, dest, max_repairs=max_repairs,
                             evaluate=evaluate, score=score, audit_events=cell_events)
            result["status"] = "executed"
            result["model_id"] = spec.model_id
        except Exception as exc:  # noqa: BLE001
            result = _blank(cell, "cell_error", "executed")
            result["error"] = f"{type(exc).__name__}: {exc}"
        results.append(result)
        print(f"{result.get('model_id')} {result.get('case')} {result.get('arm')} "
              f"{result.get('status')} {result.get('stop')} "
              f"phys={result.get('physical_attempts_this_run')}", flush=True)
        stop = result.get("stop")
        if stop in GLOBAL_STOPS:
            global_reason = stop
        elif stop in STOP_MODEL:
            stopped[spec.model_id] = stop
    physical = sum(int(item.get("physical_attempts_this_run") or 0) for item in results)
    logical = sum(int(item.get("logical_calls_this_run")
                      or item.get("actual_model_requests_this_run") or 0) for item in results)
    budget_used = None
    if budget_path and Path(budget_path).is_file():
        budget_used = json.loads(Path(budget_path).read_text(encoding="utf-8")).get("requests_used")
    payload = {
        "ok": True, "planned_cells": len(plan["matrix"]), "results": results,
        "execution_mode": execution_mode,
        "llm_calls": physical if execution_mode == "live" else 0,
        "physical_attempts": physical,
        "logical_calls": logical,
        "http_attempts": physical,
        "physical_attempts_unknown": sum(int(item.get("physical_attempts_unknown") or 0)
                                         for item in results),
        "semantic_repair_rounds": sum(int(item.get("semantic_repair_rounds") or 0)
                                      for item in results),
        "budget_requests_used": budget_used,
        "global_budget_stop": global_reason,
    }
    (out / "RESULTS.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
    return payload


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    parser.add_argument("--inputs", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--confirm-paid-run", action="store_true")
    args = parser.parse_args()
    config = json.loads(Path(args.config).read_text(encoding="utf-8"))
    rows = [json.loads(line) for line in Path(args.inputs).read_text(encoding="utf-8").splitlines()
            if line.strip()]
    plan = validate_config(config, rows)
    if not plan["ok"]:
        print(json.dumps(plan["errors"], ensure_ascii=False, indent=2))
        return 2
    upper = plan["requests_upper_bound"]
    print(f"validated cells={plan['cells']} request_upper_bound={upper} llm_calls=0")
    if not args.confirm_paid_run:
        print("not executing: pass --confirm-paid-run only for a future paid pilot")
        return 0
    load_dotenv(REPO / ".env", override=True)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    audit = AuditLog(out / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(out / "budget.json", max_requests=upper, max_seconds=6 * 3600)
    registry = build_registry()
    events = read_events(audit.path) if audit.path.is_file() else []

    def factory(spec, cell_id, arm, case_id):
        channel = CHANNELS[spec.channel]
        api_key = key_for(spec, dict(os.environ), channel.api_key_env)
        inner = build_client(spec, budget=budget,
                             evidence_dir=out / "llm" / cell_id.replace("/", "__"),
                             api_key=api_key, timeout=120.0, max_tokens=4096)
        return AuditedClient(inner, audit=audit, run_id=out.name, cell_id=cell_id,
                             spec=spec, arm=arm, task_id=case_id, replicate=0, stage="repair")

    mode = os.environ.get("CPV_EXECUTION_MODE", "live")
    payload = execute_batch(config, rows, out, client_factory=factory, audit_events=events,
                            execution_mode=mode, budget_path=out / "budget.json")
    print(f"planned_cells={payload['planned_cells']} execution_mode={payload['execution_mode']} "
          f"physical_attempts={payload['physical_attempts']} llm_calls={payload['llm_calls']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
