#!/usr/bin/env python3
"""Run the Reviewer 1 hard diagnostic-feedback repair ablation.

The experiment intentionally uses one model and fixed initial CIR artifacts.
Every model response, candidate, verifier payload, and usage record is written
after each task so an interrupted run remains auditable.
"""

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
    repair_system_prompt,
    verification_feedback,
)
from cir_workflow.rust_cli import RustCli  # noqa: E402


FIXTURE_DIR = ROOT / "paper/rebuttal-experiments/diagnostic-hard-fixtures"
PATTERNS = {
    name: {
        "initial": FIXTURE_DIR / f"{name}-initial.json",
        "reference": FIXTURE_DIR / f"{name}-reference.json",
    }
    for name in (
        "dense_lock_graph",
        "partial_semaphore_goals",
        "staged_lock_cycles",
        "mixed_three_stage",
    )
}
CONDITIONS = ("self_repair", "coarse", "structured")
MODEL_DISPLAY_NAME = "DeepSeek V4 Flash"
GENERIC_HINTS = {
    "Deadlock": (
        "Remove the circular wait, for example by imposing a consistent lock "
        "acquisition order, while preserving all modeled operations."
    ),
    "ChannelBlock": (
        "Make the send and receive able to rendezvous without being prevented "
        "by lock ownership, while preserving all modeled operations."
    ),
    "SignalLoss": (
        "Repair the ordering protocol so a notification cannot be lost before "
        "the corresponding waiter is ready, while preserving all operations."
    ),
    "DeadTransition": (
        "Restore reachability of the required operation without deleting any "
        "modeled behavior."
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "paper/rebuttal-experiments/diagnostic-ablation-hard.json",
    )
    parser.add_argument(
        "--patterns",
        nargs="+",
        choices=tuple(PATTERNS),
        default=list(PATTERNS),
    )
    parser.add_argument(
        "--conditions",
        nargs="+",
        choices=CONDITIONS,
        default=list(CONDITIONS),
    )
    parser.add_argument("--max-rounds", type=int, default=5)
    parser.add_argument("--max-tokens", type=int, default=16384)
    parser.add_argument("--max-transport-errors", type=int, default=8)
    parser.add_argument("--timeout", type=float, default=240.0)
    parser.add_argument("--model-id", default="deepseek-v4-pro")
    parser.add_argument("--resume", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    load_dotenv(ROOT / ".env")
    model = ModelConfig(
        name=f"{MODEL_DISPLAY_NAME} supplementary rebuttal run",
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

    results = load_or_create_results(args, model)
    assert_resume_configuration(results, args, model)
    completed = {
        (task["pattern"], task["condition"])
        for task in results.get("tasks", [])
        if task.get("complete")
    }

    for pattern in args.patterns:
        for condition in args.conditions:
            key = (pattern, condition)
            if key in completed:
                print(f"skip completed: {pattern}/{condition}", flush=True)
                continue
            print(f"run: {pattern}/{condition}", flush=True)
            task = run_task(
                client,
                rust,
                pattern=pattern,
                condition=condition,
                max_rounds=args.max_rounds,
                max_tokens=args.max_tokens,
                max_transport_errors=args.max_transport_errors,
            )
            results["tasks"] = [
                old
                for old in results.get("tasks", [])
                if (old.get("pattern"), old.get("condition")) != key
            ]
            results["tasks"].append(task)
            results["summary"] = summarize(results["tasks"])
            write_json(args.output, results)
            print(
                f"done: {pattern}/{condition}: "
                f"accepted={task['accepted']}, rounds={len(task['rounds'])}",
                flush=True,
            )

    results["summary"] = summarize(results["tasks"])
    results["completed_at"] = now()
    write_json(args.output, results)
    return 0


def load_or_create_results(
    args: argparse.Namespace, model: ModelConfig
) -> dict[str, Any]:
    if args.resume and args.output.exists():
        return json.loads(args.output.read_text(encoding="utf-8"))
    return {
        "experiment": "hard diagnostic-feedback ablation",
        "scope": (
            "Pre-registered single-model supplementary experiment on four harder "
            "multi-fault or nonlocal-repair tasks; one run per condition and task. "
            "The earlier three-case pilot remains separate."
        ),
        "started_at": now(),
        "configuration": {
            "model_name": MODEL_DISPLAY_NAME,
            "model": model.model_id,
            "provider": model.provider,
            "temperature": 0.0,
            "reasoning_effort": model.reasoning_effort,
            "thinking_enabled": model.thinking_enabled,
            "max_rounds": args.max_rounds,
            "max_output_tokens": args.max_tokens,
            "max_transport_errors_per_cell": args.max_transport_errors,
            "conditions": {
                "self_repair": (
                    "Unguided: the model is told that the CIR needs a concurrency "
                    "repair, but receives no problem class, location, trace, state, "
                    "CIR slice, or repair hint."
                ),
                "coarse": (
                    "LLM-only issue report: current primary problem class plus a "
                    "generic class-level description; no SID, witness, state summary, "
                    "resource/function localization, or CIR slice."
                ),
                "structured": (
                    "Complete current Rust feedback: status, bug kind, witness, "
                    "state summary, resources/functions, CIR slice, constraints, hint."
                ),
            },
            "acceptance": (
                "Rust status verified_safe AND strict preservation oracle passed"
            ),
            "strict_preservation_oracle": (
                "Exact preservation of resources, protection entries, function "
                "names/kinds, per-function SID sets, goals, and per-function operation "
                "multisets. Reordering and transfer rewiring are permitted."
            ),
            "transport_policy": (
                "Provider/transport errors are recorded separately and retried "
                "without consuming one of the five model rounds."
            ),
            "platform": platform.platform(),
            "python": platform.python_version(),
        },
        "tasks": [],
        "summary": {},
    }


def assert_resume_configuration(
    results: dict[str, Any], args: argparse.Namespace, model: ModelConfig
) -> None:
    configuration = results.get("configuration", {})
    expected = {
        "model": model.model_id,
        "max_rounds": args.max_rounds,
        "max_output_tokens": args.max_tokens,
        "max_transport_errors_per_cell": args.max_transport_errors,
    }
    mismatches = {
        key: (configuration.get(key), value)
        for key, value in expected.items()
        if configuration.get(key) != value
    }
    if mismatches:
        raise ValueError(f"resume configuration mismatch: {mismatches}")


def run_task(
    client: Any,
    rust: RustCli,
    *,
    pattern: str,
    condition: str,
    max_rounds: int,
    max_tokens: int,
    max_transport_errors: int,
) -> dict[str, Any]:
    fixture = PATTERNS[pattern]
    initial_text = fixture["initial"].read_text(encoding="utf-8")
    initial = json.loads(initial_text)
    reference = json.loads(fixture["reference"].read_text(encoding="utf-8"))
    current = pretty(initial)
    initial_verification = rust.analyze(current)
    reference_verification = rust.analyze(pretty(reference))
    reference_preserved, reference_deviations = preservation_check(
        initial, reference
    )
    precheck_errors = []
    if initial_verification.status != "verified_unsafe":
        precheck_errors.append(
            f"initial status is {initial_verification.status}, expected verified_unsafe"
        )
    if reference_verification.status != "verified_safe":
        precheck_errors.append(
            "reference status is "
            f"{reference_verification.status}, expected verified_safe"
        )
    if not reference_preserved:
        precheck_errors.append(
            f"reference violates preservation oracle: {reference_deviations}"
        )
    if precheck_errors:
        raise RuntimeError(f"fixture precheck failed for {pattern}: {precheck_errors}")

    initial_kind = primary_bug_kind(initial_verification.payload)
    task: dict[str, Any] = {
        "pattern": pattern,
        "condition": condition,
        "fixture": str(fixture["initial"].relative_to(ROOT)),
        "reference_fixture": str(fixture["reference"].relative_to(ROOT)),
        "fixture_metadata": fixture_metadata(initial, initial_verification.payload),
        "initial_cir": initial,
        "initial_verification": result_dict(initial_verification),
        "reference_precheck": {
            "verification": result_dict(reference_verification),
            "preservation_passed": reference_preserved,
            "preservation_deviations": reference_deviations,
        },
        "initial_bug_kind": initial_kind,
        "initial_bug_kinds": bug_kinds(initial_verification.payload),
        "initial_unmet_goals": unmet_goal_ids(initial_verification.payload),
        "rounds": [],
        "transport_errors": [],
        "evaluable": True,
        "verifier_accepted": False,
        "accepted": False,
        "accepted_round": None,
        "preservation_passed": None,
        "terminal_status": None,
        "remaining_bug_kinds": bug_kinds(initial_verification.payload),
        "remaining_unmet_goals": unmet_goal_ids(initial_verification.payload),
        "complete": False,
    }

    verification = initial_verification
    round_number = 1
    while round_number <= max_rounds:
        prompt = user_prompt(
            condition,
            current,
            verification.payload,
            round_number=round_number,
        )
        started = time.perf_counter()
        round_record: dict[str, Any] = {
            "round": round_number,
            "prompt": prompt,
        }
        try:
            response, usage = client.chat(
                repair_system_prompt(),
                prompt,
                temperature=0.0,
                max_tokens=max_tokens,
            )
            round_record["response"] = response
            round_record["usage"] = usage
        except Exception as error:
            task["transport_errors"].append(
                {
                    "for_round": round_number,
                    "prompt": prompt,
                    "error": str(error),
                    "duration_ms": elapsed_ms(started),
                    "occurred_at": now(),
                }
            )
            if len(task["transport_errors"]) >= max_transport_errors:
                task.update(
                    {
                        "evaluable": False,
                        "terminal_status": "transport_error_limit",
                        "complete": True,
                    }
                )
                return task
            continue

        candidate_text = extract_json(response)
        try:
            candidate = json.loads(candidate_text)
            if not isinstance(candidate, dict):
                raise ValueError("CIR root is not an object")
        except (json.JSONDecodeError, ValueError) as error:
            round_record.update(
                {
                    "candidate_text": candidate_text,
                    "parse_error": str(error),
                    "duration_ms": elapsed_ms(started),
                    "accepted": False,
                }
            )
            task["rounds"].append(round_record)
            round_number += 1
            continue

        canonical = pretty(candidate)
        verification = rust.analyze(canonical)
        preservation_passed, preservation_deviations = preservation_check(
            initial, candidate
        )
        verifier_accepted = verification.status == "verified_safe"
        accepted = verifier_accepted and preservation_passed
        next_kind = primary_bug_kind(verification.payload)
        remaining_bug_kinds = bug_kinds(verification.payload)
        remaining_unmet_goals = unmet_goal_ids(verification.payload)
        round_record.update(
            {
                "candidate_cir": candidate,
                "verification": result_dict(verification),
                "primary_bug_kind": next_kind,
                "bug_kind_drift": bool(
                    initial_kind and next_kind and initial_kind != next_kind
                ),
                "remaining_bug_kinds": remaining_bug_kinds,
                "remaining_unmet_goals": remaining_unmet_goals,
                "verifier_accepted": verifier_accepted,
                "preservation_passed": preservation_passed,
                "preservation_deviations": preservation_deviations,
                "accepted": accepted,
                "duration_ms": elapsed_ms(started),
            }
        )
        task["rounds"].append(round_record)
        current = canonical
        task["verifier_accepted"] = task["verifier_accepted"] or verifier_accepted
        task["remaining_bug_kinds"] = remaining_bug_kinds
        task["remaining_unmet_goals"] = remaining_unmet_goals
        if accepted:
            task["accepted"] = True
            task["accepted_round"] = round_number
            task["preservation_passed"] = True
            task["terminal_status"] = "strict_success"
            break
        round_number += 1

    task["complete"] = True
    if not task["accepted"] and task["rounds"]:
        last = task["rounds"][-1]
        task["preservation_passed"] = last.get("preservation_passed")
        task["terminal_status"] = "round_budget_exhausted"
    return task


def user_prompt(
    condition: str,
    cir_json: str,
    payload: dict[str, Any] | None,
    *,
    round_number: int,
) -> str:
    common = (
        "Repair the concurrency protocol in the complete CIR below. Preserve every "
        "resource and function, every existing SID, every business goal, and the "
        "multiset of modeled operations within each function. Reordering operations "
        "and changing control-flow transfers are allowed; deleting behavior is not."
    )
    if condition == "self_repair":
        evidence = (
            "No external diagnostic is available. Independently inspect the CIR and "
            f"reason about all thread interleavings. This is self-review round {round_number}."
        )
    elif condition == "coarse":
        kind = primary_bug_kind(payload) or status_name(payload)
        hint = GENERIC_HINTS.get(kind, "Repair the concurrency fault without deleting behavior.")
        evidence = (
            "LLM-only issue report:\n"
            f"- Problem class: {kind}\n"
            f"- Generic description: {hint}"
        )
    elif condition == "structured":
        evidence = "Structured verifier feedback:\n\n" + verification_feedback(payload)
    else:
        raise ValueError(condition)
    return (
        "# CIR Repair Request\n\n"
        f"{common}\n\n{evidence}\n\n"
        "## Current CIR\n\n"
        f"```json\n{cir_json}\n```\n\n"
        "Return the complete revised CIR JSON object only."
    )


def preservation_check(
    original: dict[str, Any], candidate: dict[str, Any]
) -> tuple[bool, list[str]]:
    deviations: list[str] = []
    if original.get("entry") != candidate.get("entry"):
        deviations.append("changed entry")
    for key in ("resources", "protection", "fn_summaries", "goals"):
        if normalized(original.get(key, [])) != normalized(candidate.get(key, [])):
            deviations.append(f"changed {key}")

    original_functions = function_map(original)
    candidate_functions = function_map(candidate)
    if set(original_functions) != set(candidate_functions):
        deviations.append("changed function names")
        return False, deviations

    for name, original_function in original_functions.items():
        candidate_function = candidate_functions[name]
        if original_function.get("kind") != candidate_function.get("kind"):
            deviations.append(f"changed function kind: {name}")
        original_body = original_function.get("body", [])
        candidate_body = candidate_function.get("body", [])
        if {item.get("sid") for item in original_body} != {
            item.get("sid") for item in candidate_body
        }:
            deviations.append(f"changed SID set: {name}")
        if operation_multiset(original_body) != operation_multiset(candidate_body):
            deviations.append(f"changed operation multiset: {name}")
    return not deviations, deviations


def function_map(program: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        function.get("name", ""): function
        for function in program.get("functions", [])
        if isinstance(function, dict)
    }


def operation_multiset(body: list[dict[str, Any]]) -> Counter[str]:
    return Counter(
        json.dumps(statement.get("op"), sort_keys=True, separators=(",", ":"))
        for statement in body
    )


def normalized(value: Any) -> str:
    return json.dumps(value or [], sort_keys=True, separators=(",", ":"))


def primary_bug_kind(payload: dict[str, Any] | None) -> str | None:
    if not payload:
        return None
    bugs = payload.get("bugs", [])
    if not bugs:
        return "GoalUnreachable" if payload.get("unmet_goals") else None
    kind = bugs[0].get("kind")
    if isinstance(kind, dict) and kind:
        return next(iter(kind))
    return str(kind) if kind else None


def bug_kinds(payload: dict[str, Any] | None) -> list[str]:
    kinds = []
    for bug in (payload or {}).get("bugs", []):
        kind = bug.get("kind")
        if isinstance(kind, dict) and kind:
            kinds.append(str(next(iter(kind))))
        elif kind:
            kinds.append(str(kind))
    return kinds


def unmet_goal_ids(payload: dict[str, Any] | None) -> list[str]:
    return [
        str(item.get("goal", {}).get("id", "?"))
        for item in (payload or {}).get("unmet_goals", [])
    ]


def fixture_metadata(
    program: dict[str, Any], payload: dict[str, Any] | None
) -> dict[str, Any]:
    functions = program.get("functions", [])
    return {
        "resource_count": len(program.get("resources", [])),
        "function_count": len(functions),
        "statement_count": sum(
            len(function.get("body", []))
            for function in functions
            if isinstance(function, dict)
        ),
        "goal_count": len(program.get("goals", [])),
        "initial_state_count": (payload or {}).get("state_count"),
        "initial_bug_count": len((payload or {}).get("bugs", [])),
        "initial_unmet_goal_count": len((payload or {}).get("unmet_goals", [])),
    }


def status_name(payload: dict[str, Any] | None) -> str:
    return str((payload or {}).get("status", "unknown"))


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


def summarize(tasks: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "completed_cells": sum(bool(task.get("complete")) for task in tasks),
        "evaluable_cells": sum(bool(task.get("evaluable")) for task in tasks),
        "by_condition": {},
    }
    for condition in CONDITIONS:
        selected = [task for task in tasks if task.get("condition") == condition]
        if not selected:
            continue
        evaluable = [task for task in selected if task.get("evaluable")]
        accepted = [task for task in evaluable if task.get("accepted")]
        verifier_accepted = [
            task for task in evaluable if task.get("verifier_accepted")
        ]
        rounds = [task["accepted_round"] for task in accepted]
        regressions = Counter()
        usage = Counter()
        remaining = Counter()
        model_rounds = 0
        transport_errors = 0
        for task in evaluable:
            model_rounds += len(task.get("rounds", []))
            transport_errors += len(task.get("transport_errors", []))
            if not task.get("accepted"):
                remaining.update(task.get("remaining_bug_kinds", []))
                if task.get("remaining_unmet_goals"):
                    remaining["GoalUnreachable"] += len(
                        task["remaining_unmet_goals"]
                    )
            for item in task.get("rounds", []):
                verification = item.get("verification", {})
                status = verification.get("status")
                if item.get("parse_error"):
                    regressions["parse"] += 1
                elif status in {"invalid_model", "invalid_json"}:
                    regressions["static"] += 1
                elif status == "translation_failed":
                    regressions["translation"] += 1
                if item.get("bug_kind_drift"):
                    regressions["bug_kind_drift"] += 1
                if item.get("verifier_accepted") and not item.get(
                    "preservation_passed", True
                ):
                    regressions["verifier_safe_but_behavior_dropping"] += 1
                for key, value in item.get("usage", {}).items():
                    if isinstance(value, (int, float)):
                        usage[key] += value
        summary["by_condition"][condition] = {
            "cells": len(selected),
            "evaluable": len(evaluable),
            "verifier_accepted": len(verifier_accepted),
            "strictly_accepted": len(accepted),
            "accepted_rounds": rounds,
            "mean_rounds_among_accepted": (
                sum(rounds) / len(rounds) if rounds else None
            ),
            "model_rounds": model_rounds,
            "transport_errors": transport_errors,
            "remaining_issue_counts": dict(remaining),
            "regressions": dict(regressions),
            "usage": dict(usage),
        }
    return summary


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
