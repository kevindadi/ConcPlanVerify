#!/usr/bin/env python3
"""strong-link-v27 DeepSeek ablation launcher (paid; separate budgets).

Runs the pre-registered ablation configurations sequentially, each 24 tasks x
3 reps = 72 G3 cells, DeepSeek Flash direct. `without_code_correspondence` is NOT
implemented and is deliberately excluded.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow.env import load_dotenv  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v27")
OUT = REPO / "experiments/strong-link-v27-ablation"
CONFIGS = [
    ("full", {}),
    ("without_explore", {"without_explore": True}),
    ("without_goal_gate", {"without_goal_gate": True}),
    ("without_diagnostics", {"without_diagnostics": True}),
]


def main() -> int:
    load_dotenv(REPO / ".env")
    manifest = json.loads((REPO / "benchmarks/GENERATION_MANIFEST.json").read_text())
    tasks = [{"family": e["family"], "task": e["task"]} for e in manifest["tasks"]]
    for name, ablation in CONFIGS:
        cfg = {
            "status": f"v27_ablation_{name}",
            "real_requests_authorized": True,
            "note": ("DeepSeek Flash direct G3-only ablation. Only the pre-registered "
                     "component is disabled; model, tasks, public prompt, thinking, "
                     "stage budget, tools and the independent evaluator are fixed."),
            "models": [{
                "display_name": "DeepSeek Flash", "model_id": "deepseek-flash",
                "channel": "deepseek-direct", "surface": "chat",
                "max_output_tokens": None,
                "max_output_policy": "provider default; token usage recorded",
                "request_timeout_seconds": 180, "hard_timeout_seconds": 300,
                "thinking": {"thinking": {"type": "enabled"}},
                "thinking_status": "to_confirm_by_preflight"}],
            "tasks": tasks,
            "arms": ["G3_concir"],
            "reps": [0, 1, 2],
            "cell_caps": {"G3_concir": 12},
            "rounds": {"G3_concir": 3},
            "g3_code_rounds": 3,
            "matrix_cap": 864, "preflight_cap": 2, "global_physical_cap": 866,
            "wall_seconds": 172800, "concurrency": 3,
            "ablation": ablation,
            "run_id": f"v27-ablation-{name}",
        }
        cfg_path = NOTES / f"ABLATION_CONFIG_{name}.json"
        cfg_path.write_text(json.dumps(cfg, indent=2) + "\n")
        out_dir = OUT / name
        out_dir.mkdir(parents=True, exist_ok=True)
        env = {"PYTHONPATH": "python:.", "V23_OUT": str(out_dir),
               "PATH": __import__("os").environ["PATH"]}
        env.update({k: v for k, v in __import__("os").environ.items()
                    if k.endswith("_API_KEY")})
        print(f"=== ablation {name} ===", flush=True)
        subprocess.run([sys.executable, str(REPO / "scripts/v23_live.py"), str(cfg_path)],
                       cwd=str(REPO), env=env, check=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
