#!/usr/bin/env python3
"""Minimal connectivity + usage-integrity smoke across the available channels.

One fixed request per available model. Records a unified audit event per call
(request/response model, usage, latency, prompt/response hashes) and a cell row.
This is a transport validation, not the main comparison.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog  # noqa: E402
from cir_workflow.channels import AuditedClient, ChannelUnavailable, build_client  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.transport import CHANNELS, available_models, blocked_models  # noqa: E402

SYSTEM = "You are a connectivity probe. Follow the instruction exactly."
USER = "Reply with exactly this token and nothing else: PROBE_OK"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="experiments/multimodel-v1")
    parser.add_argument("--max-requests", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=90.0)
    args = parser.parse_args()

    load_dotenv(REPO / ".env", override=True)
    env = dict(os.environ)
    out = REPO / args.out
    batch = out / f"smoke-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    audit = AuditLog(batch / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(batch / "budget.json", max_requests=args.max_requests,
                        max_seconds=1800.0)

    specs = available_models()
    blocked = blocked_models()
    rows = []
    print(f"available: {[s.display_name for s in specs]}")
    print(f"blocked:   {[(s.display_name, s.blocked_reason) for s in blocked]}")

    for spec in specs:
        ch = CHANNELS[spec.channel]
        api_key = env.get(ch.api_key_env, "")
        row = {"model": spec.display_name, "provider": spec.provider,
               "transport": spec.channel, "requested_model": spec.model_id,
               "returned_model": None, "status": None, "identity_confirmed": None,
               "input_tokens": None, "output_tokens": None, "total_tokens": None,
               "usage_source": None, "latency_ms": None, "error": None}
        if not api_key:
            row["status"] = "blocked"
            row["error"] = f"missing {ch.api_key_env}"
            rows.append(row)
            continue
        cell_id = f"smoke/{spec.display_name}"
        try:
            inner = build_client(spec, budget=budget, evidence_dir=batch / "llm",
                                 api_key=api_key, timeout=args.timeout, max_tokens=64)
            client = AuditedClient(inner, audit=audit, run_id=batch.name,
                                   cell_id=cell_id, spec=spec, arm="smoke",
                                   task_id="connectivity", replicate=0,
                                   stage="connectivity")
            started = time.time()
            outcome = client.complete(SYSTEM, USER)
            row.update({
                "status": "ok",
                "returned_model": getattr(outcome, "response_model", None),
                "identity_confirmed": getattr(outcome, "response_model", None) == spec.model_id,
                "latency_ms": int((time.time() - started) * 1000),
            })
            usage = getattr(outcome, "usage", None) or {}
            row["input_tokens"] = usage.get("prompt_tokens", usage.get("input_tokens"))
            row["output_tokens"] = usage.get("completion_tokens", usage.get("output_tokens"))
            row["total_tokens"] = usage.get("total_tokens")
            row["usage_source"] = "server" if usage else "unknown"
        except ChannelUnavailable as exc:
            row["status"] = "blocked"
            row["error"] = str(exc)
        except Exception as exc:  # noqa: BLE001
            row["status"] = "error"
            row["error"] = f"{type(exc).__name__}: {exc}"
        rows.append(row)
        print(f"  {spec.display_name:14s} {row['status']:7s} "
              f"returned={row['returned_model']} "
              f"tok={row['input_tokens']}/{row['output_tokens']}")

    fields = list(rows[0].keys())
    with (batch / "CELL_RESULTS.csv").open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)

    summary = {
        "batch": str(batch),
        "kind": "connectivity-smoke",
        "available": [s.display_name for s in specs],
        "blocked": [{"model": s.display_name, "reason": s.blocked_reason}
                    for s in blocked],
        "requests_used": budget.requests_used,
        "rows": rows,
    }
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {batch}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
