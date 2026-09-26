#!/usr/bin/env python3
"""Rerun G3 only under the v3 CIR prompt and concrete checker feedback.

Does not regenerate G0/G1/G2. Each model has its own 180-request budget.
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
from cir_workflow.generation import load_gen_tasks, run_g3_v2  # noqa: E402
from cir_workflow.live import DeepSeekFlashClient, LiveBudget  # noqa: E402
from cir_workflow.prompts import concir_generation_v3_system_prompt  # noqa: E402

OUT = REPO / "experiments/evidence-20260925/g3-v3"
MODELS = ("deepseek-flash", "composer-2.5", "grok-4.7")
MAX_REQUESTS = 180


class CursorSession:
    """One Cursor agent per cell. Each send is one harness request, not one API call."""

    def __init__(self, model: str, api_key: str, budget: LiveBudget, inbox: Path) -> None:
        from cursor_sdk import Agent, AgentOptions, LocalAgentOptions
        self.model = model
        self.api_key = api_key
        self.budget = budget
        self.inbox = inbox
        self.Agent = Agent
        self.Options = AgentOptions
        self.Local = LocalAgentOptions
        self.agent = None
        self.calls: list[dict] = []

    def close(self) -> None:
        if self.agent is not None:
            self.agent.close()
            self.agent = None

    def complete(self, system: str, user: str):
        reason = self.budget.exhausted()
        if reason:
            raise RuntimeError(reason)
        self.budget.reserve()
        started = time.time()
        if self.agent is None:
            self.agent = self.Agent.create(self.Options(
                api_key=self.api_key, model=self.model, tools=[],
                local=self.Local(cwd=str(self.inbox)),
            ))
            message = system.strip() + "\n\n" + user.strip()
        else:
            message = user.strip()
        result = self.agent.send(message).wait()
        returned = None
        if result.model is not None:
            returned = getattr(result.model, "id", None)
        if returned is not None and returned != self.model:
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
            response_model=returned,
            usage=usage,
            wall_ms=int((time.time() - started) * 1000),
            prompt_sha256=hashlib.sha256(message.encode()).hexdigest(),
            transport_attempt=1,
        )
        self.calls.append({
            "request_id": outcome.request_id,
            "requested_model": self.model,
            "response_model": returned,
            "identity_confirmed": returned == self.model,
            "usage": usage,
            "usage_available": usage is not None,
            "harness_request": 1,
            "api_requests_visible": None,
        })
        return outcome


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, choices=MODELS)
    parser.add_argument("--limit", type=int, default=0)
    args = parser.parse_args()
    load_dotenv(REPO / ".env")
    binary = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))
    prompt = concir_generation_v3_system_prompt()
    out = OUT / args.model
    out.mkdir(parents=True, exist_ok=True)
    (out / "PROMPT.md").write_text(prompt, encoding="utf-8")
    (out / "PROTOCOL.json").write_text(json.dumps({
        "prompt": "prompts/concir_generation_v3.md",
        "prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest(),
        "k_cir": 4, "k_code": 3, "max_requests": MAX_REQUESTS,
        "max_tokens": 4096,
        "model": args.model,
        "g0_g2": "not rerun",
        "backend": str(binary),
    }, indent=2) + "\n", encoding="utf-8")
    budget = LiveBudget(out / "budget.json", max_requests=MAX_REQUESTS, max_seconds=10 * 3600)
    done = set()
    cells_path = out / "CELLS.jsonl"
    if cells_path.is_file():
        for line in cells_path.read_text().splitlines():
            if line.strip():
                done.add(json.loads(line)["task"])
    n = 0
    with cells_path.open("a", encoding="utf-8") as fh:
        for task in load_gen_tasks(REPO):
            if task.id in done:
                continue
            if args.limit and n >= args.limit:
                return 0
            if budget.exhausted():
                fh.write(json.dumps({"task": task.id, "arm": "G3_concir", "model": args.model,
                                     "status": "budget_exhausted"}) + "\n")
                continue
            inbox = out / "inbox" / task.id.replace("/", "__")
            inbox.mkdir(parents=True, exist_ok=True)
            (inbox / "REQUIREMENTS.md").write_text(task.requirements_md, encoding="utf-8")
            cell = out / "cells" / task.id.replace("/", "__")
            session = None
            started = time.time()
            try:
                if args.model == "deepseek-flash":
                    client = DeepSeekFlashClient(
                        api_key=os.environ["DEEPSEEK_API_KEY"], budget=budget,
                        evidence_dir=out / "llm" / task.id.replace("/", "__"),
                        timeout=90.0, max_tokens=4096)
                else:
                    session = CursorSession(args.model, os.environ["CURSOR_API_KEY"], budget, inbox)
                    client = session
                record = run_g3_v2(client, binary, task, cell, k_cir=4, k_code=3)
            except Exception as exc:  # noqa: BLE001
                record = {"status": "error", "error": f"{type(exc).__name__}: {exc}", "accepted": False, "rounds": []}
            finally:
                if session is not None:
                    session.close()
            cov = record.get("coverage") or {}
            calls = getattr(client, "calls", None) or record.get("llm_calls") or []
            row = {
                "model": args.model, "task": task.id, "arm": "G3_concir", "rep": 0,
                "accepted": bool(record.get("accepted")),
                "status": record.get("status"),
                "rf": cov.get("rf") if isinstance(cov, dict) else None,
                "cir_rounds": record.get("cir_rounds"),
                "requests_after": budget.requests_used,
                "elapsed_ms": int((time.time() - started) * 1000),
                "response_models": [c.get("response_model") for c in calls if isinstance(c, dict)],
            }
            (cell / "RECORD.json").write_text(
                json.dumps(record, ensure_ascii=False, indent=2, default=str) + "\n", encoding="utf-8")
            fh.write(json.dumps(row) + "\n")
            fh.flush()
            n += 1
            print(f"{args.model} {task.id} accepted={row['accepted']} status={row['status']} req={budget.requests_used}",
                  flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
