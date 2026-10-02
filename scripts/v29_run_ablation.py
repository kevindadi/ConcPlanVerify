#!/usr/bin/env python3
"""strong-link-v29 ablation launcher (GPT 6 Luna + GLM 5.3 Flash, paid).

Runs the four pre-registered G3 ablation configurations sequentially, each
24 tasks x 2 models x rep0 = 48 G3 cells: full, without_explore,
without_goal_gate, without_diagnostics. Only the pre-registered component is
disabled; the model, tasks, public prompts, thinking params, stage budget,
toolchain and the independent evaluator are fixed. `without_code_correspondence`
is not implemented and is deliberately excluded. DeepSeek is not used.

The preflight records of the online G3 diagnostic (same two models) are reused
so the ablation spends no extra probe requests.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
from cir_workflow.env import load_dotenv  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v29")
OUT = REPO / "experiments/strong-link-v29-ablation"
TOOLCHAIN = "/Users/kevin/local-repos/ConcIR/target/v28-verified/release"
ONLINE_PREFLIGHT = REPO / "experiments/strong-link-v29-online/PREFLIGHT.json"
CONFIGS = [
    ("full", {}),
    ("without_explore", {"without_explore": True}),
    ("without_goal_gate", {"without_goal_gate": True}),
    ("without_diagnostics", {"without_diagnostics": True}),
]


def base_config() -> dict:
    frozen = json.loads((NOTES / "FROZEN_CONFIG.json").read_text(encoding="utf-8"))
    return frozen


def main() -> int:
    load_dotenv(REPO / ".env")
    frozen = base_config()
    for name, ablation in CONFIGS:
        cfg = dict(frozen)
        cfg["status"] = f"v29_ablation_{name}"
        cfg["real_requests_authorized"] = True
        cfg["ablation"] = ablation
        cfg["run_id"] = f"v29-ablation-{name}"
        cfg["note"] = (
            "GPT 6 Luna + GLM 5.3 Flash G3-only ablation, rep0. Only the "
            "pre-registered component is disabled; model, tasks, public prompts, "
            "thinking, stage budget, toolchain and independent evaluator are fixed."
        )
        cfg_path = NOTES / f"ABLATION_CONFIG_{name}.json"
        cfg_path.write_text(json.dumps(cfg, indent=2, ensure_ascii=False) + "\n",
                            encoding="utf-8")
        out_dir = OUT / name
        out_dir.mkdir(parents=True, exist_ok=True)
        # Reuse the diagnostic preflight (same models, already verified).
        if ONLINE_PREFLIGHT.is_file() and not (out_dir / "PREFLIGHT.json").is_file():
            shutil.copyfile(ONLINE_PREFLIGHT, out_dir / "PREFLIGHT.json")
        env = {"PYTHONPATH": "python:.", "V23_OUT": str(out_dir),
               "CONCIR_TOOLCHAIN": TOOLCHAIN,
               "PATH": os.environ["PATH"]}
        env.update({k: v for k, v in os.environ.items() if k.endswith("_API_KEY")})
        print(f"=== ablation {name} ===", flush=True)
        with (out_dir / "live.log").open("w", encoding="utf-8") as log:
            subprocess.run(
                [sys.executable, str(REPO / "scripts/v23_live.py"), str(cfg_path),
                 "--toolchain-dir", TOOLCHAIN, "--skip-preflight"],
                cwd=str(REPO), env=env, stdout=log, stderr=subprocess.STDOUT, check=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
