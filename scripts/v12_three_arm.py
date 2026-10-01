#!/usr/bin/env python3
"""One-task three-arm repair pilot. Without --confirm-paid-run it only freezes."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.channels import AuditedClient, ChannelUnavailable, build_client, key_for  # noqa: E402
from cir_workflow.conditional_arms import arm_order, plan_row, run_repairs  # noqa: E402
from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.live import LiveBudget  # noqa: E402
from cir_workflow.task_score import score_for_task  # noqa: E402
from cir_workflow.transport import CHANNELS, build_registry, require_experiment_model  # noqa: E402

BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")
BIND = BIN.with_name("bind_check")
MODELS = ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "GLM 5.3 Flash"]
MODEL_STOPS = {"identity_mismatch", "identity_unconfirmed", "channel_unavailable",
               "cell_error", "request_error"}


def _sha(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def freeze_paths(records: list[dict]) -> dict[str, str]:
    """Files whose bytes must match the frozen protocol before any send."""

    paths = []
    for record in records:
        paths.extend([
            Path(record["source_path"]),
            Path(record["requirements_path"]),
            Path(record["cir_path"]),
            Path(record["contract_path"]),
        ])
    paths.extend([
        REPO / "python/cir_workflow/conditional_arms.py",
        REPO / "python/cir_workflow/send_holding_requirements.py",
        REPO / "python/cir_workflow/condvar_requirements.py",
        REPO / "python/cir_workflow/task_score.py",
        REPO / "python/cir_workflow/evidence_v2.py",
        REPO / "python/cir_workflow/candidate_eval.py",
        REPO / "python/cir_workflow/feedback_runner.py",
        REPO / "python/cir_workflow/audit.py",
        REPO / "python/cir_workflow/live.py",
        REPO / "python/cir_workflow/channels.py",
        REPO / "python/cir_workflow/transport.py",
        REPO / "python/cir_workflow/opencode_go.py",
        REPO / "python/cir_workflow/direct.py",
        REPO / "python/cir_workflow/env.py",
        REPO / "python/cir_workflow/rust_oracle.py",
        REPO / "scripts/v12_three_arm.py",
        REPO / "runtime/concir_sync/src/lib.rs",
        REPO / "runtime/concir_sync/README.md",
        BIN, INS, BIND,
    ])
    out = {}
    for path in paths:
        digest = _sha(path)
        if digest is None:
            raise FileNotFoundError(path)
        out[str(path)] = digest
    return out


def build_protocol(records: list[dict], concurrency: int = 3, prior_probes: int = 0) -> dict:
    orders = arm_order(["deepseek-flash", "qwen3.8-flash", "gpt-6-luna", "glm-5.3-flash"], seed=12)
    cells = len(records) * len(MODELS) * 3
    return {
        "seed": 12,
        "arm_order": orders,
        "max_repairs": 2,
        "max_physical": cells * 2 - int(prior_probes),
        "prior_probes": int(prior_probes),
        "concurrency": concurrency,
        "schedule": "all first rounds, then all second rounds",
        "cell_key": "task/model/arm",
        "max_seconds": 6 * 3600,
        "models": {
            "deepseek-flash": {"temperature": 0, "channel": "deepseek-direct"},
            "qwen3.8-flash": {"temperature": 0, "channel": "dashscope-direct", "enable_thinking": False},
            "gpt-6-luna": {"temperature": None, "channel": "opencode-go", "surface": "responses"},
            "glm-5.3-flash": {
                "display": "GLM 5.3 Flash", "channel": "opencode-go",
                "surface": "chat", "temperature": 0, "max_tokens": 16384,
                "reasoning_effort": "low", "thinking_field": "not_sent",
                "thinking_field_reason": "OpenCode Go returned 400 unknown field thinking",
                "parameter_source": "https://docs.z.ai/guides/llm/glm-5.3",
            },
        },
        "inputs": [{
            "task": record["task"],
            "source_path": record["source_path"],
            "requirements_path": record["requirements_path"],
            "cir_path": record["cir_path"],
            "contract_path": record["contract_path"],
        } for record in records],
        "freeze_files": freeze_paths(records),
        "note": "Sampling parameters are per model. They are the same across arms of that model.",
    }


def _acquire_coordinator(out: Path):
    """One process may send for this run. The handle is held until exit."""

    handle = (out / "coordinator.lock").open("a", encoding="utf-8")
    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        handle.close()
        return None
    return handle


def _reject(out: Path, reason: str, detail: dict) -> None:
    """Record this launch's refusal without touching frozen history."""

    path = out / "RUN_REJECTION.json"
    prior = []
    if path.is_file():
        prior = json.loads(path.read_text(encoding="utf-8"))
    prior.append({"reason": reason, **detail})
    path.write_text(json.dumps(prior, ensure_ascii=False, indent=2) + "\n")


def _snapshot(out: Path) -> dict[str, str | None]:
    names = ("PROTOCOL.json", "RESULTS.json", "SUMMARY.json", "budget.json", "REQUEST_EVENTS.jsonl",
             "batch_state.json")
    return {name: _sha(out / name) if (out / name).is_file() else None for name in names}


def _write_table(out: Path, results: list, budget: dict | None, audit_count: int) -> None:
    summary = {"results": results, "audit_events": audit_count, "budget": budget,
               "real_model_requests_this_process": None}
    lock_path = out / ".results.lock"
    with lock_path.open("a", encoding="utf-8") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _atomic_json(out / "RESULTS.json", results)
        _atomic_json(out / "SUMMARY.json", summary)


def _atomic_json(path: Path, payload) -> None:
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def _cell_key(task: str, model_id: str, arm: str) -> str:
    return f"{task}/{model_id}/{arm}"


def _cell_dir(out: Path, task: str, model_id: str, arm: str) -> Path:
    return out / task.replace("/", "__") / model_id / arm


def _plan(records: list[dict], orders: dict) -> list[dict]:
    registry = build_registry()
    rows = []
    for record in records:
        for name in MODELS:
            spec = require_experiment_model(registry, name)
            for arm in orders[spec.model_id]:
                rows.append(plan_row(
                    {"model_id": spec.model_id, "case": record["task"], "task": record["task"],
                     "arm": arm, "cell_key": _cell_key(record["task"], spec.model_id, arm)},
                    global_stop=None, model_stop=None, executed=None))
    return rows


def _eval(source, case, work):
    # run_arm passes the arm case, whose id is the task. The planner case uses task.
    cell = case.get("task") or case.get("id")
    return evaluate_candidate(
        source, Path(case["cir_path"]), Path(case["contract_path"]), Path(work),
        binary=BIN, instrument=INS, n_runs=1, run_timeout=6, cell_id=cell)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--selected", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--confirm-paid-run", action="store_true")
    parser.add_argument("--concurrency", type=int, default=3)
    parser.add_argument("--prior-probes", type=int, default=0)
    args = parser.parse_args()
    selected = json.loads(Path(args.selected).read_text(encoding="utf-8"))
    records = list(selected["selected_records"])
    if not records:
        print("no selected records")
        return 2
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    protocol_path = out / "PROTOCOL.json"
    state_path = out / "batch_state.json"
    try:
        expected = build_protocol(records, concurrency=args.concurrency,
                                  prior_probes=args.prior_probes)
    except FileNotFoundError as exc:
        print(f"freeze input missing: {exc}")
        return 2
    orders = expected["arm_order"]
    existing_results = out / "RESULTS.json"
    if not args.confirm_paid_run:
        if protocol_path.is_file() or existing_results.is_file():
            stored = json.loads(protocol_path.read_text(encoding="utf-8")) if protocol_path.is_file() else None
            _reject(out, "dry_run_existing_directory", {
                "protocol_matches": stored == expected,
            })
            print("existing run directory; not rewriting")
            return 0 if stored == expected else 2
        results = _plan(records, orders)
        protocol_path.write_text(json.dumps(expected, ensure_ascii=False, indent=2) + "\n")
        _write_table(out, results, None, 0)
        print("frozen; not executing")
        return 0
    coordinator = _acquire_coordinator(out)
    if coordinator is None:
        print("another coordinator holds this run; not sending")
        return 2
    if not protocol_path.is_file():
        coordinator.close()
        _reject(out, "missing_freeze", {})
        print("missing freeze; not executing")
        return 2
    stored = json.loads(protocol_path.read_text(encoding="utf-8"))
    if stored != expected:
        coordinator.close()
        _reject(out, "frozen_protocol_mismatch", {})
        print("frozen protocol mismatch; not executing")
        return 2
    if existing_results.is_file():
        results = json.loads(existing_results.read_text(encoding="utf-8"))
    else:
        results = _plan(records, orders)
    load_dotenv(REPO / ".env", override=True)
    audit = AuditLog(out / "REQUEST_EVENTS.jsonl")
    budget = LiveBudget(out / "budget.json", max_requests=int(stored["max_physical"]),
                        max_seconds=int(stored["max_seconds"]))
    batch = {"stopped": {}, "global_stop": None}
    if state_path.is_file():
        batch = json.loads(state_path.read_text(encoding="utf-8"))
    stopped = dict(batch.get("stopped") or {})
    global_stop = batch.get("global_stop")
    registry = build_registry()
    cases = []
    for record in records:
        cases.append({"task": record["task"],
                      "defect": Path(record["source_path"]).read_text(encoding="utf-8"),
                      "requirements": Path(record["requirements_path"]).read_text(encoding="utf-8"),
                      "cir_text": Path(record["cir_path"]).read_text(encoding="utf-8"),
                      "cir_path": record["cir_path"], "contract_path": record["contract_path"]})
    _write_table(out, results, {"requests_used": budget.requests_used, "max_requests": budget.max_requests}, 0)
    state_lock = threading.Lock()
    peak = {"current": 0, "max": 0}
    model_locks: dict[str, threading.Lock] = {}
    concurrency = int(stored.get("concurrency") or 3)
    assignments = []
    cursor = 0
    for case in cases:
        for name in MODELS:
            spec = require_experiment_model(registry, name)
            for arm in orders[spec.model_id]:
                assignments.append((cursor, case, spec, arm))
                cursor += 1

    def _persist() -> None:
        batch = {"stopped": stopped, "global_stop": global_stop, "concurrency_peak": peak["max"]}
        state_path.write_text(json.dumps(batch, ensure_ascii=False, indent=2) + "\n")
        events = read_events(audit.path) if audit.path.is_file() else []
        used = json.loads((out / "budget.json").read_text()) if (out / "budget.json").is_file() else None
        _write_table(out, results, used, len(events))

    def _run_one(item: tuple, stop_after: int) -> None:
        nonlocal global_stop
        index, case, spec, arm = item
        queued = time.perf_counter()
        with model_locks.setdefault(spec.model_id, threading.Lock()):
            _run_one_locked(index, case, spec, arm, stop_after, queued)

    def _run_one_locked(index, case, spec, arm, stop_after, queued) -> None:
        nonlocal global_stop
        with state_lock:
            peak["current"] += 1
            peak["max"] = max(peak["max"], peak["current"])
            cell = results[index]
            if cell.get("status") in {"executed", "blocked"}:
                peak["current"] -= 1
                return
            if global_stop:
                cell["status"] = "not_started"
                cell["stop"] = global_stop
                peak["current"] -= 1
                return
            if spec.model_id in stopped:
                cell["status"] = "blocked"
                cell["stop"] = stopped[spec.model_id]
                peak["current"] -= 1
                return
        cell["queue_s"] = round(time.perf_counter() - queued, 6)
        dest = _cell_dir(out, case["task"], spec.model_id, arm)
        channel = CHANNELS[spec.channel]
        try:
            api_key = key_for(spec, dict(os.environ), channel.api_key_env)
            model_budget = (stored.get("models") or {}).get(spec.model_id) or {}
            inner = build_client(spec, budget=budget, evidence_dir=dest / "llm", api_key=api_key,
                                 timeout=120.0, max_tokens=int(model_budget.get("max_tokens") or 4096))
            client = AuditedClient(inner, audit=audit, run_id=out.name,
                                   cell_id=_cell_key(case["task"], spec.model_id, arm),
                                   spec=spec, arm=arm, task_id=case["task"], replicate=0, stage="repair")
            result = run_repairs(
                case, arm, client, dest, evaluate_candidate=_eval,
                score=lambda source, work, task=case["task"]: score_for_task(task, source, work),
                freeze_files=stored["freeze_files"], stop_after_round=stop_after)
            result.update({"model_id": spec.model_id, "case": case["task"], "task": case["task"],
                           "arm": arm, "cell_key": _cell_key(case["task"], spec.model_id, arm),
                           "queue_s": cell.get("queue_s"),
                           "status": "round1_done" if result.get("stop") == "phase_complete" and stop_after == 1
                           else "executed"})
        except ChannelUnavailable as exc:
            text = str(exc)
            stop = "identity_unconfirmed" if "identity_unconfirmed" in text else "channel_unavailable"
            result = {**cell, "status": "blocked", "stop": stop, "error": text}
        except Exception as exc:  # noqa: BLE001
            result = {**cell, "status": "executed", "stop": "cell_error",
                      "error": f"{type(exc).__name__}: {exc}"}
        with state_lock:
            results[index] = result
            if result.get("stop") in {"global_request_budget_exhausted", "global_time_budget_exhausted"}:
                global_stop = result["stop"]
            elif result.get("stop") in MODEL_STOPS:
                stopped[spec.model_id] = result["stop"]
            peak["current"] -= 1
            _persist()
        print(case["task"], spec.model_id, arm, result.get("stop"), flush=True)

    def _phase(indices: list[int], stop_after: int) -> None:
        if not indices:
            return
        workers = max(1, min(concurrency, len(indices)))
        with ThreadPoolExecutor(max_workers=workers) as pool:
            list(pool.map(lambda item: _run_one(item, stop_after), [assignments[i] for i in indices]))

    phase1 = [index for index, row in enumerate(results)
              if row.get("status") not in {"executed", "blocked", "round1_done"}]
    _phase(phase1, 1)
    phase2 = [index for index, row in enumerate(results) if row.get("status") == "round1_done"]
    _phase(phase2, 2)
    events = read_events(audit.path) if audit.path.is_file() else []
    _write_table(out, results, json.loads((out / "budget.json").read_text()) if (out / "budget.json").is_file() else None,
                 len(events))
    print("cells", len(results), "budget", (json.loads((out / "budget.json").read_text()).get("requests_used")
                                            if (out / "budget.json").is_file() else None))
    coordinator.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
