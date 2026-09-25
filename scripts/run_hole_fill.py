#!/usr/bin/env python3
"""Mechanical skeleton plus Flash hole-fill, paired to frozen accepted CIRs.

Does not replace the main method. Holes may contain only sequential code;
lint_filled rejects edits outside the hole. Budget is a hard cap including retries.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow.conformance import (  # noqa: E402
    codegen, fill_holes, fill_prompt, holes_of, lint_filled, parse_fill,
)
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.live import (  # noqa: E402
    ALLOWED_PROVIDER, DeepSeekFlashClient, LiveBudget, assert_allowed_model,
)

SUMMARY = REPO / "experiments/flash-gen-main-v5-code/run-20260923T211345/SUMMARY.json"
OUT = REPO / "experiments/evidence-20260925/hole-fill"
BACKEND = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))


def sha_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    load_dotenv(REPO / ".env")
    key = os.environ.get("DEEPSEEK_API_KEY", "")
    if not key:
        print("DEEPSEEK_API_KEY missing", file=sys.stderr)
        return 2
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    OUT.mkdir(parents=True, exist_ok=True)
    budget = LiveBudget(OUT / "budget.json", max_requests=180, max_seconds=6 * 3600)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    cells = json.loads(SUMMARY.read_text(encoding="utf-8"))["cells"]
    done = {}
    results_path = OUT / "CELLS.jsonl"
    if results_path.is_file():
        for line in results_path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                done[(row["rep"], row["task"])] = row
    client = DeepSeekFlashClient(api_key=key, budget=budget, evidence_dir=OUT / "llm",
                                 timeout=90.0, max_tokens=4096)
    selected = []
    for c in cells:
        if c["arm"] != "G3_concir":
            continue
        rec = c.get("record") or {}
        cir_stage = rec.get("cir_stage") or {}
        if isinstance(cir_stage, dict) and cir_stage.get("accepted") is False:
            continue
        cir = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
        if not cir or not Path(cir).is_file():
            continue
        # Accepted Rust cells, plus the one code-stage miss that still has a CIR.
        if c.get("accepted") or rec.get("arm") == "G3_concir_llmcode":
            selected.append((c, Path(cir)))
    print(f"selected {len(selected)} CIR cells", flush=True)
    with results_path.open("a", encoding="utf-8") as fh:
        for c, cir in selected:
            key = (c["rep"], c["task"])
            if key in done:
                continue
            reason = budget.exhausted()
            row = {"rep": c["rep"], "task": c["task"], "cir_sha256": sha_file(cir),
                   "generation_accepted_llm_rust": bool(c.get("accepted"))}
            if reason:
                row["status"] = "budget_exhausted"
                row["reason"] = reason
                fh.write(json.dumps(row) + "\n")
                print("budget", key, flush=True)
                continue
            work = OUT / "work" / f"rep{c['rep']}" / c["task"].replace("/", "__")
            skel = work / "skeleton"
            try:
                codegen(cir, skel, binary=BACKEND)
                holes = holes_of(skel)
            except Exception as exc:  # noqa: BLE001
                row["status"] = "codegen_failed"
                row["error"] = str(exc)[:300]
                fh.write(json.dumps(row) + "\n")
                fh.flush()
                print("codegen_failed", key, flush=True)
                continue
            row["n_holes"] = len(holes)
            if not holes:
                row["status"] = "no_holes"
                fh.write(json.dumps(row) + "\n")
                fh.flush()
                print("no_holes", c["task"], flush=True)
                continue
            task = tasks[c["task"]]
            prompt = fill_prompt(skel, task.requirements_md)
            fills = {}
            rounds = 0
            for _ in range(3):
                if budget.exhausted():
                    row["status"] = "budget_exhausted"
                    break
                rounds += 1
                outcome = client.complete(
                    "Fill only sequential holes. Return JSON mapping hole ids to Rust.",
                    prompt)
                fills = parse_fill(outcome.text)
                filled = work / f"filled-{rounds}"
                applied = fill_holes(skel, filled, fills)
                lint = lint_filled(skel, filled)
                row["rounds"] = rounds
                row["applied"] = applied["applied"]
                row["missing_holes"] = applied["missing"]
                row["lint_ok"] = lint["ok"]
                if lint["ok"] and not applied["missing"]:
                    row["status"] = "filled"
                    row["filled_main"] = str(filled / "src/main.rs")
                    break
                prompt = fill_prompt(skel, task.requirements_md) + "\nPrevious fill was rejected: " + json.dumps(lint["violations"][:4])
            else:
                row.setdefault("status", "lint_rejected")
            fh.write(json.dumps(row) + "\n")
            fh.flush()
            print(row["status"], c["task"], "r"+str(c["rep"]), "holes", len(holes), "reqs", budget.requests_used, flush=True)
    print("requests", budget.requests_used)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
