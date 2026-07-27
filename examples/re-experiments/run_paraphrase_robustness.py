#!/usr/bin/env python3
"""Run a post-hoc natural-language robustness supplement for Reviewer 1."""

from __future__ import annotations

import argparse
import json
import platform
import sys
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python"))

from cir_workflow.env import load_dotenv  # noqa: E402
from cir_workflow.json_utils import extract_json  # noqa: E402
from cir_workflow.llm import create_llm_client, default_base_url  # noqa: E402
from cir_workflow.models import ModelConfig  # noqa: E402
from cir_workflow.prompts import (  # noqa: E402
    generation_retry_prompt,
    generation_system_prompt,
    generation_user_prompt,
    verification_feedback,
)
from cir_workflow.rust_cli import RustCli  # noqa: E402


SPECS = {
    "two_mutex_deadlock": [
        (
            "A main routine starts workers A and B and waits for both. Two shared "
            "mutexes protect independent resources. A locks mutex 1 and, without "
            "releasing it, requests mutex 2. B does the symmetric sequence: mutex 2 "
            "then mutex 1. On successful acquisition each worker releases in reverse "
            "order and returns. Model this intentionally flawed protocol, including "
            "the inconsistent lock order that permits circular wait."
        ),
        (
            "Represent this lock-order failure rather than correcting it. The process "
            "has two locks and two concurrent jobs. One job enters L then tries R; the "
            "other enters R then tries L. Each retains its first lock while waiting "
            "for the second. The parent spawns both jobs, joins them, and returns only "
            "after they finish. Include the normal unlock and return steps."
        ),
        (
            "Concurrent requirement: exactly two worker threads share two mutexes. "
            "Worker left acquires them in the order first/second, while worker right "
            "acquires the same mutexes in second/first order. Neither drops its first "
            "mutex before attempting the next. Both would release both locks and "
            "complete if acquisition succeeds; main launches and joins both. Preserve "
            "the deliberate deadlock risk in the model."
        ),
    ],
    "channel_mutex_block": [
        (
            "Model an intentionally blocking channel protocol. Main spawns a sender "
            "and receiver and joins both. They share one mutex and one rendezvous-style "
            "channel. The sender locks the mutex, attempts to send one integer while "
            "still holding it, then unlocks. The receiver must lock that same mutex "
            "before it can receive, then unlocks. Do not repair this circular blocking "
            "dependency in the generated model."
        ),
        (
            "A producer and consumer use a shared lock plus a synchronous message "
            "handoff. The producer enters the critical section before sending value 42 "
            "and leaves it only after the send. The consumer's code enters the same "
            "critical section before executing receive. A coordinator launches both "
            "tasks and waits for them. Capture the flawed ordering exactly, so the "
            "send/receive rendezvous can be blocked by the mutex."
        ),
        (
            "There are two child threads, one mutex, and one channel. Thread S performs "
            "lock, channel-send, unlock, return. Thread R performs lock, channel-receive, "
            "unlock, return. The channel operation requires a matching peer. The entry "
            "function starts S and R and joins each. This is a bug model, not a request "
            "to improve the protocol."
        ),
    ],
    "semaphore_baseline": [
        (
            "A parent starts three independent workers and joins all of them. A shared "
            "counting semaphore begins with two permits. Every worker acquires one "
            "permit, performs its abstract work, releases the permit, and returns. "
            "Model the safe throttling protocol without introducing other resources."
        ),
        (
            "Represent a concurrency limit of two for three parallel jobs. The jobs "
            "all use the same semaphore: take a permit before work and give it back "
            "afterward. Main launches the three jobs, waits for every one, then exits. "
            "The intended model is a bug-free semaphore baseline."
        ),
        (
            "System requirement: run workers A, B, and C concurrently but allow no "
            "more than two inside the work region. Use one semaphore initialized to 2; "
            "each worker's sequence is acquire, work, release, return. The entry routine "
            "spawns and joins all three. Preserve this safe behavior in CIR."
        ),
    ],
}


EXPECTED = {
    "two_mutex_deadlock": {
        "resource_types": {"Mutex": 2},
        "function_count": 3,
        "operations": {"spawn": 2, "join": 2, "lock": 4, "drop": 4},
        "status": "verified_unsafe",
        "bug_kind": "Deadlock",
    },
    "channel_mutex_block": {
        "resource_types": {"Mutex": 1, "Channel": 1},
        "function_count": 3,
        "operations": {
            "spawn": 2,
            "join": 2,
            "lock": 2,
            "drop": 2,
            "send": 1,
            "recv": 1,
        },
        "status": "verified_unsafe",
        "bug_kind": "ChannelBlock",
    },
    "semaphore_baseline": {
        "resource_types": {"Semaphore": 1},
        "semaphore_count": 2,
        "function_count": 4,
        "operations": {"spawn": 3, "join": 3, "acquire": 3, "release": 3},
        "status": "verified_safe",
        "bug_kind": None,
    },
}

MODEL_DISPLAY_NAME = "DeepSeek V4 Flash"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "paper/rebuttal-experiments/paraphrase-robustness.json",
    )
    parser.add_argument("--patterns", nargs="+", choices=tuple(SPECS), default=list(SPECS))
    parser.add_argument("--max-rounds", type=int, default=5)
    parser.add_argument("--max-transport-errors", type=int, default=8)
    parser.add_argument("--max-tokens", type=int, default=4096)
    parser.add_argument("--timeout", type=float, default=240.0)
    parser.add_argument("--model-id", default="deepseek-v4-pro")
    parser.add_argument("--resume", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    load_dotenv(ROOT / ".env")
    model = ModelConfig(
        name=f"{MODEL_DISPLAY_NAME} paraphrase supplement",
        provider="deepseek",
        model_id=args.model_id,
        api_key_env="DEEPSEEK_API_KEY",
        base_url=default_base_url("deepseek"),
        reasoning_effort="high",
        thinking_enabled=True,
    )
    client = create_llm_client(model, timeout=args.timeout, max_retries=3)
    rust = RustCli(
        repo_root=ROOT,
        binary=ROOT / "target/release/cir2cvn",
        build_if_missing=True,
    )
    results = load_or_create(args, model)
    completed = {
        (task["pattern"], task["paraphrase"])
        for task in results.get("tasks", [])
        if completed_without_infrastructure_failure(task)
    }

    for pattern in args.patterns:
        for index, requirements in enumerate(SPECS[pattern], 1):
            key = (pattern, index)
            if key in completed:
                print(f"skip completed: {pattern}/p{index}", flush=True)
                continue
            print(f"run: {pattern}/p{index}", flush=True)
            task = run_task(
                client,
                rust,
                pattern=pattern,
                paraphrase=index,
                requirements=requirements,
                max_rounds=args.max_rounds,
                max_transport_errors=args.max_transport_errors,
                max_tokens=args.max_tokens,
            )
            results["tasks"] = [
                old
                for old in results.get("tasks", [])
                if (old.get("pattern"), old.get("paraphrase")) != key
            ]
            results["tasks"].append(task)
            results["summary"] = summarize(results["tasks"])
            write_json(args.output, results)
            print(
                f"done: {pattern}/p{index}: generated={task['generation_success']}, "
                f"fidelity={task['semantic_fidelity']}",
                flush=True,
            )

    results["summary"] = summarize(results["tasks"])
    results["completed_at"] = now()
    write_json(args.output, results)
    return 0


def load_or_create(args: argparse.Namespace, model: ModelConfig) -> dict[str, Any]:
    if args.resume and args.output.exists():
        results = json.loads(args.output.read_text(encoding="utf-8"))
        configuration = results.setdefault("configuration", {})
        configuration["max_generation_rounds"] = args.max_rounds
        configuration["max_transport_errors"] = args.max_transport_errors
        configuration["max_output_tokens"] = args.max_tokens
        return results
    return {
        "experiment": "post-hoc natural-language paraphrase robustness",
        "scope": (
            "Three newly authored specifications per three representative patterns, "
            "one DeepSeek run per specification. The original experiment prompts were "
            "not preserved, so these are fixed post-hoc variants, not paraphrases of "
            "a recoverable original prompt."
        ),
        "started_at": now(),
        "configuration": {
            "model_name": MODEL_DISPLAY_NAME,
            "model": model.model_id,
            "provider": model.provider,
            "temperature": 0.0,
            "reasoning_effort": model.reasoning_effort,
            "thinking_enabled": model.thinking_enabled,
            "max_generation_rounds": args.max_rounds,
            "max_transport_errors": args.max_transport_errors,
            "max_output_tokens": args.max_tokens,
            "static_validity": "CIR parser and validator accept the artifact",
            "semantic_fidelity": (
                "The generated model matches a name-independent structural oracle "
                "and the exhaustive verifier returns the expected status/primary kind."
            ),
            "platform": platform.platform(),
            "python": platform.python_version(),
        },
        "specifications": SPECS,
        "expected_oracles": EXPECTED,
        "tasks": [],
        "summary": {},
    }


def run_task(
    client: Any,
    rust: RustCli,
    *,
    pattern: str,
    paraphrase: int,
    requirements: str,
    max_rounds: int,
    max_transport_errors: int,
    max_tokens: int,
) -> dict[str, Any]:
    task: dict[str, Any] = {
        "pattern": pattern,
        "paraphrase": paraphrase,
        "requirements": requirements,
        "rounds": [],
        "generation_success": False,
        "generation_round": None,
        "semantic_fidelity": False,
        "transport_errors": 0,
        "infrastructure_failure": False,
        "complete": False,
    }
    prompt = generation_user_prompt(requirements)

    round_number = 1
    request_attempt = 0
    while round_number <= max_rounds:
        request_attempt += 1
        started = time.perf_counter()
        record: dict[str, Any] = {
            "round": round_number,
            "request_attempt": request_attempt,
            "prompt": prompt,
        }
        try:
            response, usage = client.chat(
                generation_system_prompt(),
                prompt,
                temperature=0.0,
                max_tokens=max_tokens,
            )
            record["response"] = response
            record["usage"] = usage
        except Exception as error:
            record.update({"llm_error": str(error), "duration_ms": elapsed_ms(started)})
            task["rounds"].append(record)
            task["transport_errors"] += 1
            if task["transport_errors"] >= max_transport_errors:
                task["infrastructure_failure"] = True
                break
            prompt = generation_retry_prompt(
                requirements,
                issue=f"The model request failed. Generate the complete CIR again: {error}",
            )
            continue

        candidate_text = extract_json(response)
        try:
            candidate = json.loads(candidate_text)
            if not isinstance(candidate, dict):
                raise ValueError("CIR root is not an object")
        except (json.JSONDecodeError, ValueError) as error:
            record.update(
                {
                    "candidate_text": candidate_text,
                    "parse_error": str(error),
                    "duration_ms": elapsed_ms(started),
                }
            )
            task["rounds"].append(record)
            prompt = generation_retry_prompt(
                requirements,
                issue=f"The previous answer was not valid JSON: {error}",
                current_cir=candidate_text,
            )
            round_number += 1
            continue

        canonical = pretty(candidate)
        validation = rust.validate(canonical)
        record["candidate_cir"] = candidate
        record["validation"] = result_dict(validation)
        record["duration_ms"] = elapsed_ms(started)
        record["accepted"] = validation.valid
        task["rounds"].append(record)
        if not validation.valid:
            prompt = generation_retry_prompt(
                requirements,
                issue=(
                    "The CIR validator rejected the candidate. Fix every issue.\n\n"
                    + verification_feedback(validation.payload, validation.error or validation.stderr)
                ),
                current_cir=canonical,
            )
            round_number += 1
            continue

        task["generation_success"] = True
        task["generation_round"] = round_number
        verification = rust.analyze(canonical)
        oracle_passed, oracle_checks = semantic_oracle(
            candidate, verification.payload, EXPECTED[pattern]
        )
        task["verification"] = result_dict(verification)
        task["oracle_checks"] = oracle_checks
        task["semantic_fidelity"] = oracle_passed
        break

    task["complete"] = True
    return task


def completed_without_infrastructure_failure(task: dict[str, Any]) -> bool:
    if not task.get("complete") or task.get("infrastructure_failure"):
        return False
    if "infrastructure_failure" not in task:
        return not any(record.get("llm_error") for record in task.get("rounds", []))
    return True


def semantic_oracle(
    program: dict[str, Any],
    payload: dict[str, Any] | None,
    expected: dict[str, Any],
) -> tuple[bool, dict[str, Any]]:
    resources = Counter(
        resource.get("type")
        for resource in program.get("resources", [])
        if isinstance(resource, dict)
    )
    operations = operation_counts(program)
    observed_kind = primary_bug_kind(payload)
    checks: dict[str, Any] = {
        "resource_types": {
            "expected": expected["resource_types"],
            "observed": dict(resources),
            "passed": resources == Counter(expected["resource_types"]),
        },
        "function_count": {
            "expected": expected["function_count"],
            "observed": len(program.get("functions", [])),
            "passed": len(program.get("functions", [])) == expected["function_count"],
        },
        "operations": {
            "expected": expected["operations"],
            "observed": dict(operations),
            "passed": all(
                operations[name] == count
                for name, count in expected["operations"].items()
            ),
        },
        "verification_status": {
            "expected": expected["status"],
            "observed": (payload or {}).get("status"),
            "passed": (payload or {}).get("status") == expected["status"],
        },
        "primary_bug_kind": {
            "expected": expected["bug_kind"],
            "observed": observed_kind,
            "passed": observed_kind == expected["bug_kind"],
        },
    }
    if "semaphore_count" in expected:
        counts = [
            resource.get("count")
            for resource in program.get("resources", [])
            if resource.get("type") == "Semaphore"
        ]
        checks["semaphore_count"] = {
            "expected": expected["semaphore_count"],
            "observed": counts,
            "passed": counts == [expected["semaphore_count"]],
        }
    return all(check["passed"] for check in checks.values()), checks


def operation_counts(program: dict[str, Any]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for function in program.get("functions", []):
        for statement in function.get("body", []):
            operation = statement.get("op")
            if isinstance(operation, list) and operation:
                if operation[0] == "res_op" and len(operation) >= 3:
                    counts[str(operation[2])] += 1
                else:
                    counts[str(operation[0])] += 1
            elif isinstance(operation, str):
                counts[operation] += 1
    return counts


def primary_bug_kind(payload: dict[str, Any] | None) -> str | None:
    bugs = (payload or {}).get("bugs", [])
    if not bugs:
        return None
    kind = bugs[0].get("kind")
    if isinstance(kind, dict) and kind:
        return next(iter(kind))
    return str(kind) if kind else None


def summarize(tasks: list[dict[str, Any]]) -> dict[str, Any]:
    complete = [task for task in tasks if task.get("complete")]
    evaluated = [task for task in complete if not task.get("infrastructure_failure")]
    generated = [task for task in evaluated if task.get("generation_success")]
    faithful = [task for task in evaluated if task.get("semantic_fidelity")]
    per_pattern = {}
    for pattern in SPECS:
        selected = [task for task in evaluated if task.get("pattern") == pattern]
        if selected:
            per_pattern[pattern] = {
                "tasks": len(selected),
                "static_valid": sum(bool(task.get("generation_success")) for task in selected),
                "semantic_fidelity": sum(bool(task.get("semantic_fidelity")) for task in selected),
                "generation_rounds": [task.get("generation_round") for task in selected],
            }
    usage = Counter()
    for task in complete:
        for record in task.get("rounds", []):
            for key, value in record.get("usage", {}).items():
                if isinstance(value, (int, float)):
                    usage[key] += value
    return {
        "tasks": len(evaluated),
        "infrastructure_failures": len(complete) - len(evaluated),
        "static_valid": len(generated),
        "semantic_fidelity": len(faithful),
        "generation_rounds": [task.get("generation_round") for task in evaluated],
        "per_pattern": per_pattern,
        "usage": dict(usage),
    }


def result_dict(result: Any) -> dict[str, Any]:
    return {
        "mode": result.mode,
        "exit_code": result.exit_code,
        "status": result.status,
        "payload": result.payload,
        "stderr": result.stderr,
        "error": result.error,
        "duration_ms": result.duration_ms,
    }


def pretty(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2)


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(pretty(value) + "\n", encoding="utf-8")
    temporary.replace(path)


def elapsed_ms(started: float) -> float:
    return (time.perf_counter() - started) * 1000.0


def now() -> str:
    return datetime.now(timezone.utc).isoformat()


if __name__ == "__main__":
    raise SystemExit(main())
