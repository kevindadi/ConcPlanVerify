#!/usr/bin/env python3
"""One-task three-arm repair pilot. Without --confirm-paid-run it only freezes."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.channels import AuditedClient, build_client, key_for  # noqa: E402
from cir_workflow.conditional_arms import arm_order, run_repairs  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.send_holding_requirements import evaluate_requirements  # noqa: E402
from cir_workflow.transport import CHANNELS, build_registry, require_experiment_model  # noqa: E402

BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")
MODELS = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "Kimi 2.7 Code"]


def _eval(source, case, work):
    return evaluate_candidate(
        source, Path(case["cir_path"]), Path(case["contract_path"]), Path(work),
        binary=BIN, instrument=INS, n_runs=1, run_timeout=6, cell_id=case["task"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--selected", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--confirm-paid-run", action="store_true")
    args = parser.parse_args()
    selected = json.loads(Path(args.selected).read_text(encoding="utf-8"))
    record = selected["selected_records"][0]
    requirements = Path(record["requirements_path"]).read_text(encoding="utf-8")
    cir_text = Path(record["cir_path"]).read_text(encoding="utf-8")
    case = {"task": record["task"], "defect": Path(record["source_path"]).read_text(encoding="utf-8"),
            "requirements": requirements, "cir_text": cir_text,
            "cir_path": record["cir_path"], "contract_path": record["contract_path"]}
    orders = arm_order(["deepseek-flash", "qwen3.8-flash", "gpt-6-luna", "kimi-k2.7-code"], seed=12)
    protocol = {
        "seed": 12, "arm_order": orders, "max_repairs": 2, "max_physical": 24,
        "models": {
            "deepseek-flash": {"temperature": 0, "channel": "deepseek-direct"},
            "qwen3.8-flash": {"temperature": 0, "channel": "dashscope-direct", "enable_thinking": False},
            "gpt-6-luna": {"temperature": None, "channel": "opencode-go", "surface": "responses"},
            "kimi-k2.7-code": {"temperature": 1, "channel": "opencode-go"},
        },
        "note": "Sampling parameters are per model. They are the same across arms of that model.",
    }
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "PROTOCOL.json").write_text(json.dumps(protocol, ensure_ascii=False, indent=2) + "\n")
    if not args.confirm_paid_run:
        print("frozen; not executing")
        return 0
    load_dotenv(REPO / ".env", override=True)
    audit = AuditLog(out / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(out / "budget.json", max_requests=24, max_seconds=6 * 3600)
    registry = build_registry()
    results = []
    global_stop = None
    stopped: dict[str, str] = {}
    for name in MODELS:
        spec = require_experiment_model(registry, name)
        for arm in orders[spec.model_id]:
            cell = {"model_id": spec.model_id, "case": case["task"], "arm": arm}
            if global_stop:
                results.append({**cell, "status": "not_started", "stop": global_stop})
                continue
            if spec.model_id in stopped:
                results.append({**cell, "status": "blocked", "stop": stopped[spec.model_id]})
                (out / "RESULTS.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
                continue
            dest = out / spec.model_id / arm
            channel = CHANNELS[spec.channel]
            try:
                api_key = key_for(spec, dict(os.environ), channel.api_key_env)
                inner = build_client(spec, budget=budget, evidence_dir=dest / "llm", api_key=api_key,
                                     timeout=120.0, max_tokens=4096)
                if spec.model_id == "kimi-k2.7-code" and hasattr(inner, "temperature"):
                    inner.temperature = 1.0
                client = AuditedClient(inner, audit=audit, run_id=out.name, cell_id=f"{spec.model_id}/{arm}",
                                       spec=spec, arm=arm, task_id=case["task"], replicate=0, stage="repair")
                result = run_repairs(case, arm, client, dest, evaluate_candidate=_eval, score=evaluate_requirements)
                result.update(cell)
                result["status"] = "executed"
            except Exception as exc:  # noqa: BLE001
                result = {**cell, "status": "executed", "stop": "cell_error", "error": f"{type(exc).__name__}: {exc}"}
            results.append(result)
            (out / "RESULTS.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
            print(spec.model_id, arm, result.get("stop"), flush=True)
            if result.get("stop") in {"global_request_budget_exhausted", "global_time_budget_exhausted"}:
                global_stop = result["stop"]
            elif result.get("stop") in {"identity_mismatch", "cell_error", "request_error"}:
                stopped[spec.model_id] = result["stop"]
    events = read_events(audit.path) if audit.path.is_file() else []
    summary = {"results": results, "audit_events": len(events),
               "budget": json.loads((out / "budget.json").read_text())}
    (out / "SUMMARY.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print("cells", len(results), "budget", summary["budget"].get("requests_used"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
