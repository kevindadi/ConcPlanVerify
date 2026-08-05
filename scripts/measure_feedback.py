"""Measure repair-prompt feedback size for a CIR case (offline, no LLM)."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))

from cir_workflow.prompts import repair_user_prompt, verification_feedback
from cir_workflow.rust_cli import RustCli

REPO_ROOT = Path(__file__).resolve().parents[1]

case = sys.argv[1] if len(sys.argv) > 1 else "benchmarks/cir/deep_lock_chain_4x3/buggy.json"
cir_json = Path(case).read_text(encoding="utf-8")
cli = RustCli(repo_root=REPO_ROOT)
result = cli.analyze(cir_json)
payload = result.payload

bugs = payload.get("bugs", [])
feedback = verification_feedback(payload)
prompt = repair_user_prompt(cir_json, feedback)
print(f"case: {case}")
print(f"bugs: {len(bugs)} counterexamples")
print(f"feedback chars: {len(feedback):,}")
print(f"full repair prompt chars: {len(prompt):,}")
print(f"~tokens (chars/3.5): {len(prompt) / 3.5:,.0f}")
