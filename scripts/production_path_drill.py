#!/usr/bin/env python3
"""Fake inner client, real AuditedClient, real local tools. No network model call."""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.channels import AuditedClient  # noqa: E402
from cir_workflow.feedback_runner import load_frozen, run_arm, validate_config  # noqa: E402
from cir_workflow.transport import build_registry, require_experiment_model  # noqa: E402
from scripts.feedback_pilot import freeze_inputs  # noqa: E402


class _Inner:
    def __init__(self, text: str, model_id: str) -> None:
        self.text = text
        self.model_id = model_id
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        return SimpleNamespace(text=self.text, usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, response_model=self.model_id, request_id="fake")


def main() -> int:
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    config, rows, _oracle = freeze_inputs()
    plan = validate_config(config, rows)
    if not plan["ok"]:
        print(json.dumps(plan["errors"], ensure_ascii=False, indent=2))
        return 2
    spec = require_experiment_model(build_registry(), "DeepSeek Flash")
    cells = []
    provenance = []
    started = time.perf_counter()
    for row in rows:
        case = load_frozen(row, config["tools"])
        fenced = "```rust\n" + case["control"] + "\n```"
        for arm in ("verdict_only", "counterexample"):
            dest = out / "cells" / case["id"] / arm
            inner = _Inner(fenced, spec.model_id)
            audit = AuditLog(dest / "events.jsonl")
            client = AuditedClient(
                inner, audit=audit, run_id="v8-drill", cell_id=f"{case['id']}/{arm}",
                spec=spec, arm=arm, task_id=case["id"], replicate=0, stage="repair")
            result = run_arm(case, spec, arm, client, dest, max_repairs=2)
            state = json.loads((dest / "state.json").read_text(encoding="utf-8"))
            feedback = "\n".join(item.get("feedback") or "" for item in state["requests"])
            events = read_events(audit.path)
            record = {
                "case": case["id"], "arm": arm, "model": spec.model_id,
                "stop": result["stop"],
                "fake_inner_calls": inner.calls,
                "real_model_requests": 0,
                "first_requirement_round": result["first_requirement_round"],
                "first_design_round": result["first_design_round"],
                "tool_time_s": result.get("tool_time_s"),
                "identity_confirmed": [item.get("identity_confirmed")
                                       for item in (result.get("identity") or [])],
                "feedback_has_scoring_oracle": "requirement_oracle=" in feedback
                or "design_oracle=" in feedback,
                "feedback_has_expected_field": "expected=" in feedback,
                "audit_events": len(events),
                "provenance_layers": sorted({item.get("layer")
                                            for item in result.get("feedback_provenance") or []}),
            }
            cells.append(record)
            for item in result.get("feedback_provenance") or []:
                provenance.append({"case": case["id"], **item})
            print(case["id"], arm, result["stop"], "calls", inner.calls, flush=True)
    ok = all(cell["stop"] == "requirement_pass" and cell["fake_inner_calls"] == 1
             and cell["real_model_requests"] == 0
             and not cell["feedback_has_scoring_oracle"]
             and not cell["feedback_has_expected_field"]
             and cell["audit_events"] == 1
             and cell["tool_time_s"] and cell["tool_time_s"] > 0
             for cell in cells)
    summary = {
        "path": "fake inner -> AuditedClient -> run_arm -> evaluate_candidate",
        "real_model_requests": 0,
        "elapsed_s": round(time.perf_counter() - started, 3),
        "ok": ok,
        "cells": cells,
    }
    (out / "PRODUCTION_PATH_DRILL.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (out / "FEEDBACK_PROVENANCE.jsonl").write_text(
        "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in provenance),
        encoding="utf-8")
    print("ok", ok, "cells", len(cells))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
