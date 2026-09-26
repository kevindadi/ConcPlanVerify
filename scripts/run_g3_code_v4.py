#!/usr/bin/env python3
"""Code-stage rerun on frozen verified CIRs. Does not overwrite g3-v3."""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow.cursor_harness import StagedCursorClient  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import load_gen_tasks, run_g3, run_llmcode_from_cir  # noqa: E402
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402
from cir_workflow.prompts import (  # noqa: E402
    concir_generation_v3_system_prompt, PROMPT_ASSET_DIR,
)

OLD = REPO / "experiments/evidence-20260925/g3-v3"
OUT = REPO / "experiments/evidence-20260925/g3-code-v4"
MODELS = ("deepseek-flash", "composer-2.5", "grok-4.7")
MAX_REQUESTS = 180


def cir_of(record: dict) -> Path | None:
    stage = record.get("cir_stage") or {}
    if not stage.get("accepted"):
        return None
    raw = record.get("cir_path") or (record.get("code_stage") or {}).get("cir_path")
    # combined record stores cir path on the cir subdir via rounds; g3-v3 RECORD
    # from run_g3_v2 puts cir_path on code_stage or top-level only if accepted.
    if not raw:
        return None
    path = Path(raw)
    return path if path.is_file() else None


def load_old(model: str) -> dict[str, dict]:
    path = OLD / model / "CELLS.jsonl"
    if not path.is_file():
        return {}
    out = {}
    root = OLD / model / "cells"
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        rec_path = root / row["task"].replace("/", "__") / "RECORD.json"
        rec = json.loads(rec_path.read_text(encoding="utf-8")) if rec_path.is_file() else {}
        # cir file: accepted cir lives at cells/.../cir/revision-N
        cir = None
        if (rec.get("cir_stage") or {}).get("accepted"):
            candidates = sorted((root / row["task"].replace("/", "__") / "cir").glob("revision-*.cir.json"))
            # the accepting round is the last cir round whose decision is accepted
            rounds = [r for r in rec.get("rounds") or [] if r.get("stage") == "cir" and r.get("decision") == "accepted"]
            if rounds:
                cir = root / row["task"].replace("/", "__") / "cir" / f"revision-{rounds[-1]['round']}.cir.json"
            elif candidates:
                cir = candidates[-1]
        out[row["task"]] = {"row": row, "record": rec, "cir": cir if cir and cir.is_file() else None}
    return out


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, choices=MODELS)
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    binary = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
    rust_prompt = (PROMPT_ASSET_DIR / "rust_from_cir_v2.md").read_text(encoding="utf-8")
    cir_prompt = concir_generation_v3_system_prompt()
    out = OUT / args.model
    out.mkdir(parents=True, exist_ok=True)
    (out / "PROTOCOL.json").write_text(json.dumps({
        "rust_prompt_sha256": hashlib.sha256(rust_prompt.encode()).hexdigest(),
        "cir_prompt_sha256": hashlib.sha256(cir_prompt.encode()).hexdigest(),
        "k_code": 3, "k_cir": 4, "max_requests": MAX_REQUESTS,
        "native_runs": 32, "run_timeout_s": 10,
        "backend": str(binary),
        "note": "code stage resends full system+user; CIR reused when already verified",
    }, indent=2) + "\n", encoding="utf-8")
    budget = LiveBudget(out / "budget.json", max_requests=MAX_REQUESTS, max_seconds=12 * 3600)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    old = load_old(args.model)
    done = set()
    cells_path = out / "CELLS.jsonl"
    if cells_path.is_file():
        for line in cells_path.read_text().splitlines():
            if line.strip():
                done.add(json.loads(line)["task"])
    n = 0
    with cells_path.open("a", encoding="utf-8") as fh:
        for task_id, task in tasks.items():
            if task_id in done:
                continue
            if args.limit and n >= args.limit:
                return 0
            prior = old.get(task_id, {})
            cir = prior.get("cir")
            restarted = False
            if cir is None and args.model == "grok-4.7" and task_id not in old:
                if budget.exhausted():
                    fh.write(json.dumps({"task": task_id, "status": "budget_exhausted"}) + "\n")
                    continue
                inbox = out / "inbox" / task_id.replace("/", "__")
                inbox.mkdir(parents=True, exist_ok=True)
                (inbox / "REQUIREMENTS.md").write_text(task.requirements_md, encoding="utf-8")
                client = StagedCursorClient(args.model, os.environ["CURSOR_API_KEY"], budget, inbox)
                client.set_stage("cir")
                cell = out / "cells" / task_id.replace("/", "__")
                try:
                    rec = run_g3(client, binary, task, cell / "cir", k=4, with_codegen=False)
                finally:
                    client.close()
                restarted = True
                if rec.get("accepted") and rec.get("cir_path"):
                    cir = Path(rec["cir_path"])
                (cell / "CIR_RECORD.json").write_text(json.dumps(rec, default=str) + "\n", encoding="utf-8")
            elif cir is None:
                fh.write(json.dumps({
                    "task": task_id, "model": args.model, "status": "no_verified_cir",
                    "restarted_cir": False,
                }) + "\n")
                fh.flush()
                n += 1
                print(f"{args.model} {task_id} no_verified_cir", flush=True)
                continue
            if cir is None or not Path(cir).is_file():
                fh.write(json.dumps({"task": task_id, "model": args.model, "status": "cir_failed",
                                     "restarted_cir": restarted}) + "\n")
                print(f"{args.model} {task_id} cir_failed", flush=True)
                continue
            if budget.exhausted():
                fh.write(json.dumps({"task": task_id, "status": "budget_exhausted"}) + "\n")
                continue
            inbox = out / "inbox" / task_id.replace("/", "__")
            inbox.mkdir(parents=True, exist_ok=True)
            (inbox / "REQUIREMENTS.md").write_text(task.requirements_md, encoding="utf-8")
            cell = out / "cells" / task_id.replace("/", "__")
            session = None
            if args.model == "deepseek-flash":
                client = DeepSeekFlashClient(
                    api_key=os.environ["DEEPSEEK_API_KEY"], budget=budget,
                    evidence_dir=out / "llm" / task_id.replace("/", "__"),
                    timeout=90.0, max_tokens=4096)
            else:
                session = StagedCursorClient(args.model, os.environ["CURSOR_API_KEY"], budget, inbox)
                session.set_stage("code")
                client = session
            started = time.time()
            try:
                code = run_llmcode_from_cir(client, binary, task, Path(cir), cell / "code", k_code=3)
            finally:
                if session is not None:
                    session.close()
            row = {
                "model": args.model, "task": task_id, "arm": "G3_code",
                "cir_sha256": hashlib.sha256(Path(cir).read_bytes()).hexdigest(),
                "restarted_cir": restarted,
                "accepted": bool(code.get("accepted")),
                "status": code.get("status"),
                "decision": ((code.get("rounds") or [{}])[-1]).get("decision"),
                "reasons": ((code.get("rounds") or [{}])[-1]).get("reasons"),
                "rf": (code.get("coverage") or {}).get("rf") if isinstance(code.get("coverage"), dict) else None,
                "requests_after": budget.requests_used,
                "elapsed_ms": int((time.time() - started) * 1000),
                "old_accepted": bool((prior.get("row") or {}).get("accepted")),
            }
            (cell / "CODE_RECORD.json").write_text(json.dumps(code, default=str) + "\n", encoding="utf-8")
            fh.write(json.dumps(row) + "\n")
            fh.flush()
            n += 1
            print(f"{args.model} {task_id} accepted={row['accepted']} {row['decision']} req={budget.requests_used}",
                  flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
