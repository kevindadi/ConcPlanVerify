#!/usr/bin/env python3
"""Build the versioned multi-model experiment manifest.

Reads the *real* arm definitions and task manifest; never re-defines an arm from
its name. Emits a single JSON used by the protocol document and the runs.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import transport  # noqa: E402
from cir_workflow.experiments_v2 import ARMS as REPAIR_ARMS  # noqa: E402

GEN_ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir", "G3_codegen"]
MANIFEST_VERSION = "multimodel-v1"


def _tasks() -> list[dict]:
    data = json.loads((REPO / "benchmarks/GENERATION_MANIFEST.json").read_text())
    tasks = data.get("tasks") if isinstance(data, dict) else data
    out = []
    for t in (tasks if isinstance(tasks, list) else tasks.values()):
        out.append({"id": t.get("id") or t.get("task"), "tier": t.get("tier"),
                    "family": (t.get("id") or t.get("task", "")).split("/")[0]})
    return sorted(out, key=lambda x: x["id"])


def build() -> dict:
    specs = transport.build_registry()
    return {
        "manifest_version": MANIFEST_VERSION,
        "built_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "channels": [
            {"name": c.name, "transport": c.transport, "provider": c.provider,
             "api_key_env": c.api_key_env, "base_url": c.base_url,
             "surface": c.surface, "notes": c.notes,
             "discovered_models": transport.DISCOVERED_MODELS.get(c.name, [])}
            for c in transport.CHANNELS.values()
        ],
        "models": [
            {"display_name": s.display_name, "model_id": s.model_id,
             "provider": s.provider, "channel": s.channel, "surface": s.surface,
             "role": s.role, "status": s.status,
             "blocked_reason": s.blocked_reason, "discovered": s.discovered,
             "aliases": list(s.aliases)}
            for s in specs
        ],
        "arms": {
            "generation": GEN_ARMS,
            "repair": list(REPAIR_ARMS),
            "note": "arms are read from arms.py / experiments_v2.py, not guessed",
        },
        "tasks": _tasks(),
        "budgets": {"K_cir": 4, "K_code": 3,
                    "reps_generation_main": 3, "reps_ablation": 1,
                    "first_candidate_round": 1},
        "acceptance": {
            "cir": "explore outcome == PASS and complete == True",
            "code": "builds, operation-bound conform has no violation and at "
                    "least one conformant trace, monitor has no FAIL",
            "awp": "model PASS and conform PASS and no monitor FAIL",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="experiments/multimodel-v1/MANIFEST.json")
    args = parser.parse_args()
    target = REPO / args.out
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(build(), ensure_ascii=False, indent=2) + "\n",
                      encoding="utf-8")
    print(f"wrote {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
