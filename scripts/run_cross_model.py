#!/usr/bin/env python3
"""Composer 2.5 and Grok 4.7 extension. One repeat. Same Cursor harness, no tools.

The model receives only the arm prompt. Contract, reference solutions, and other
models' outputs are not in the prompt or the working directory. Each Agent.prompt
counts as one harness request. Internal Cursor calls are recorded from usage when
the SDK reports them, and are otherwise unavailable. A session is not treated as
one API request.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
import time
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    load_gen_tasks, run_g3_v2, run_rust_generation, rust_arm_oracle,
)

OUT = REPO / "experiments/evidence-20260925/cross-model"
MODELS = ("composer-2.5", "grok-4.7")
ARMS = ("G0_direct", "G2_tools_iter", "G3_concir")
MAX_REQUESTS = 320
# One task per family, frozen before the rest of the matrix.
PILOT_TASKS = (
    "lock-order/abba_2lock",
    "condvar/bare_wait_no_predicate",
    "channel/rendezvous_both_send",
    "semaphore/acquire_twice_no_release",
    "atomic-data/bounded_counter_invariant",
    "structure/nested_scope_lock_order",
)


class Budget:
    def __init__(self, path: Path, limit: int) -> None:
        self.path = path
        self.limit = limit
        self.used = 0
        if path.is_file():
            self.used = int(json.loads(path.read_text()).get("used", 0))

    def save(self) -> None:
        self.path.write_text(json.dumps({"limit": self.limit, "used": self.used}) + "\n")

    def take(self) -> bool:
        if self.used >= self.limit:
            return False
        self.used += 1
        self.save()
        return True


class CursorHarnessClient:
    def __init__(self, model: str, api_key: str, budget: Budget, inbox: Path) -> None:
        from cursor_sdk import Agent, AgentOptions, LocalAgentOptions
        self.Agent = Agent
        self.Options = AgentOptions
        self.Local = LocalAgentOptions
        self.model = model
        self.api_key = api_key
        self.budget = budget
        self.inbox = inbox
        self.calls = []

    def complete(self, system: str, user: str):
        if not self.budget.take():
            raise RuntimeError("budget_exhausted")
        prompt = system.strip() + "\n\n" + user.strip()
        started = time.time()
        result = self.Agent.prompt(
            prompt,
            self.Options(
                api_key=self.api_key,
                model=self.model,
                tools=[],
                local=self.Local(cwd=str(self.inbox)),
            ),
        )
        returned = None
        if result.model is not None:
            returned = getattr(result.model, "id", None) or str(result.model)
        if returned not in (None, self.model):
            raise RuntimeError(f"model mismatch: requested {self.model}, got {returned}")
        usage = None
        if result.usage is not None:
            usage = {
                "prompt_tokens": result.usage.input_tokens,
                "completion_tokens": result.usage.output_tokens,
                "total_tokens": result.usage.total_tokens,
            }
        outcome = SimpleNamespace(
            text=result.result or "",
            request_id=result.id,
            response_model=returned or self.model,
            usage=usage,
            wall_ms=int((time.time() - started) * 1000),
            prompt_sha256=hashlib.sha256(prompt.encode()).hexdigest(),
            transport_attempt=1,
            status=str(result.status),
            usage_available=usage is not None,
            harness="cursor-sdk-no-tools",
        )
        self.calls.append({
            "request_id": outcome.request_id,
            "response_model": outcome.response_model,
            "usage": usage,
            "status": outcome.status,
        })
        (self.inbox / "last_prompt_sha.txt").write_text(outcome.prompt_sha256 + "\n")
        return outcome


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, choices=MODELS)
    parser.add_argument("--pilot-only", action="store_true")
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    api_key = os.environ.get("CURSOR_API_KEY", "")
    if not api_key:
        print("CURSOR_API_KEY missing", file=sys.stderr)
        return 2
    binary = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
    tasks = load_gen_tasks(REPO)
    if args.pilot_only:
        tasks = [t for t in tasks if t.id in PILOT_TASKS]
    out = OUT / args.model
    out.mkdir(parents=True, exist_ok=True)
    budget = Budget(out / "budget.json", MAX_REQUESTS)
    done = set()
    cells_path = out / "CELLS.jsonl"
    if cells_path.is_file():
        for line in cells_path.read_text().splitlines():
            if line.strip():
                row = json.loads(line)
                done.add((row["task"], row["arm"]))
    n = 0
    with cells_path.open("a") as fh:
        for task in tasks:
            for arm in ARMS:
                if (task.id, arm) in done:
                    continue
                if args.limit and n >= args.limit:
                    print("limit", flush=True)
                    return 0
                if budget.used >= budget.limit:
                    fh.write(json.dumps({"task": task.id, "arm": arm, "status": "budget_exhausted"}) + "\n")
                    print("budget", task.id, arm, flush=True)
                    continue
                inbox = out / "inbox" / task.id.replace("/", "__") / arm
                inbox.mkdir(parents=True, exist_ok=True)
                (inbox / "REQUIREMENTS.md").write_text(task.requirements_md, encoding="utf-8")
                cell = out / "cells" / task.id.replace("/", "__") / arm
                client = CursorHarnessClient(args.model, api_key, budget, inbox)
                started = time.time()
                try:
                    if arm == "G3_concir":
                        record = run_g3_v2(client, binary, task, cell, k_cir=4, k_code=3)
                    else:
                        _run, record = run_rust_generation(
                            client, arm, task, cell, k=1 if arm == "G0_direct" else 4, miri_seeds=16)
                        if _run.final_artifact_path:
                            oracle = rust_arm_oracle(
                                Path(_run.final_artifact_path), task, cell / "oracle", binary=binary)
                            record["oracle"] = {
                                "status": oracle.get("status"),
                                "rf": (oracle.get("coverage") or {}).get("rf"),
                            }
                            record["coverage"] = oracle.get("coverage")
                except Exception as exc:  # noqa: BLE001
                    record = {"status": "error", "error": f"{type(exc).__name__}: {exc}", "accepted": False}
                cov = record.get("coverage") or {}
                row = {
                    "model": args.model, "harness": "cursor-sdk-no-tools",
                    "task": task.id, "arm": arm, "rep": 0,
                    "accepted": bool(record.get("accepted")),
                    "status": record.get("status"),
                    "rf": cov.get("rf") if isinstance(cov, dict) else None,
                    "requests_after": budget.used,
                    "elapsed_ms": int((time.time() - started) * 1000),
                    "response_models": [c.get("response_model") for c in client.calls],
                    "usage_available": [c.get("usage") is not None for c in client.calls],
                }
                (cell / "RECORD.json").write_text(json.dumps(record, default=str)[:200000], encoding="utf-8")
                fh.write(json.dumps(row) + "\n")
                fh.flush()
                n += 1
                print(f"{args.model} {task.id} {arm} accepted={row['accepted']} req={budget.used}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
