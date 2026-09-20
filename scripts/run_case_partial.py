"""Case study: partial_deadlock_bystander under the holds_all contract (§5.2).

A3_local and A3_whole, K=6, reps 0..2, budget 60, on BIN_MAIN (with the
holds_all repair hint).
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.flash_smoke import _run_task_arms  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402

TASK = "lock-order/partial_deadlock_bystander"
MAX_REQUESTS = int(os.environ.get("CASE_MAX_REQUESTS", "60"))


def main() -> int:
    api_key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not api_key:
        print("missing DEEPSEEK_API_KEY")
        return 2
    binary = Path(os.environ.get(
        "CONCIR_BACKEND", str(REPO.parent / "ConcIR/target/release/concir-backend")))
    out = REPO / "experiments/case-partial-deadlock-v1"
    out.mkdir(parents=True, exist_ok=True)
    batch = out / f"run-{time.strftime('%Y%m%dT%H%M%S')}"
    batch.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(batch / "budget.json", max_requests=MAX_REQUESTS)
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    task = tasks[TASK]
    rt = json.loads((REPO / "benchmarks" / task.directory / "repair_task.json").read_text())
    contract_path = REPO / "benchmarks" / rt["contract"]
    contract = json.loads(contract_path.read_text())
    spec = (REPO / "benchmarks" / rt["requirements_file"]).read_text()
    buggy_cir = REPO / "benchmarks" / rt["input_cir"]
    reps = []
    for rep in range(3):
        rep_dir = batch / f"rep-{rep}"
        rec = _run_task_arms(
            TASK, REPO / "benchmarks", task, contract_path, contract, spec, buggy_cir,
            None, rep_dir / TASK.replace("/", "__"), rep_dir / "llm", binary, api_key,
            budget, 6, 90.0, 4096, None, ("A3_local", "A3_whole"))
        rec["rep"] = rep
        reps.append(rec)
    payload = {"batch": str(batch), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
               "requests_used": budget.requests_used, "reps": reps}
    (batch / "SUMMARY.json").write_text(json.dumps(payload, indent=1) + "\n")
    print(json.dumps({"batch": str(batch), "requests_used": budget.requests_used}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
