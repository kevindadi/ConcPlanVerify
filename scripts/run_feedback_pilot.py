#!/usr/bin/env python3
"""Future paid entry for the six-case feedback pilot.

Without ``--confirm-paid-run`` this process validates the freeze and exits.
strong-link-v8 does not pass that flag. A cell failure is written and stops
that model; it does not discard results already saved for other cells.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.channels import AuditedClient, build_client, key_for  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.feedback_runner import load_frozen, validate_config, run_arm  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, build_registry, require_experiment_model  # noqa: E402

_STOP_MODEL = {
    "identity_mismatch", "identity_unconfirmed", "request_error",
    "outcome_unknown", "budget_exhausted", "cell_error",
}


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
    results = []
    events = read_events(audit.path) if audit.path.is_file() else []
    stopped_models: set[str] = set()

    def _flush() -> None:
        (out / "RESULTS.json").write_text(
            json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    for name in config["models"]:
        spec = require_experiment_model(registry, name)
        if spec.model_id in stopped_models:
            continue
        channel = CHANNELS[spec.channel]
        api_key = key_for(spec, dict(os.environ), channel.api_key_env)
        for row in rows:
            if spec.model_id in stopped_models:
                break
            try:
                case = load_frozen(row, config.get("tools"))
            except (OSError, ValueError, KeyError) as exc:
                results.append({"case": row.get("id"), "model": spec.model_id,
                                "stop": "cell_error", "error": f"{type(exc).__name__}: {exc}",
                                "actual_model_requests_this_run": 0})
                stopped_models.add(spec.model_id or "")
                _flush()
                break
            for arm in config["arms"]:
                cell_id = f"{spec.model_id}/{row['id']}/{arm}"
                cell = out / spec.model_id / row["id"] / arm
                cell_events = [event for event in events
                               if event.get("cell_id") == cell_id and event.get("arm") == arm]
                try:
                    inner = build_client(spec, budget=budget, evidence_dir=out / "llm",
                                         api_key=api_key, timeout=120.0, max_tokens=4096)
                    client = AuditedClient(
                        inner, audit=audit, run_id=out.name, cell_id=cell_id,
                        spec=spec, arm=arm, task_id=row["id"], replicate=0, stage="repair")
                    result = run_arm(case, spec, arm, client, cell, max_repairs=2,
                                     audit_events=cell_events)
                except Exception as exc:  # noqa: BLE001
                    result = {"case": row["id"], "arm": arm, "model": spec.model_id,
                              "stop": "cell_error", "error": f"{type(exc).__name__}: {exc}",
                              "actual_model_requests_this_run": None}
                results.append(result)
                _flush()
                if result.get("stop") in _STOP_MODEL:
                    stopped_models.add(spec.model_id or "")
                    break
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
