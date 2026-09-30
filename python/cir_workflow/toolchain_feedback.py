"""Feedback taken only from the candidate evaluator's tool results.

The independent scoring oracle is not an input. Model-check PASS on the frozen
CIR is not reported as a CVN counterexample of the Rust program.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any


def _sha(path: str | None) -> str | None:
    if not path:
        return None
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return None


def _model_properties(path: str | None) -> list[str] | None:
    if not path:
        return None
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    names = []
    for prop in data.get("properties") or []:
        if isinstance(prop, dict) and prop.get("id"):
            names.append(str(prop["id"]).replace("preserved: ", ""))
    return names or None


def toolchain_items(result: dict) -> list[dict[str, Any]]:
    ledger = result.get("ledger") or {}
    items: list[dict[str, Any]] = []
    model = ledger.get("model") or {}
    items.append({
        "layer": "cir_model_check",
        "tool": "concir-backend explore",
        "status": "PASS" if model.get("verified") else "not_verified",
        "source_sha256": result.get("source_sha256"),
        "cir_sha256": result.get("cir_sha256"),
        "artifact": _artifact(result, "model_check"),
        "property": _model_properties(_artifact(result, "model_check")),
        "resource": None, "event_index": None,
        "note": "frozen CIR exploration; not a Rust counterexample",
    })
    trace = ledger.get("trace") or {}
    violation = (trace.get("violations") or [None])[0]
    if trace.get("state") == "observed_violation" and violation:
        items.append({
            "layer": "bounded_trace_deviation",
            "tool": "concir-backend conform",
            "status": "violation",
            "source_sha256": result.get("source_sha256"),
            "cir_sha256": result.get("cir_sha256"),
            "artifact": _artifact(result, "conform_check"),
            "property": None,
            "resource": violation.get("resource"),
            "event_index": violation.get("event_index"),
            "got": violation.get("got"),
            "checker_next": violation.get("expected"),
            "detail": violation.get("detail"),
        })
    elif trace.get("state") == "observed_conformant":
        items.append({
            "layer": "bounded_trace_deviation",
            "tool": "concir-backend conform",
            "status": "conformant",
            "source_sha256": result.get("source_sha256"),
            "cir_sha256": result.get("cir_sha256"),
            "artifact": _artifact(result, "conform_check"),
            "property": None, "resource": None, "event_index": None,
        })
    run = ledger.get("run") or {}
    if run.get("state"):
        items.append({
            "layer": "execution",
            "tool": "run_native",
            "status": run.get("state"),
            "source_sha256": result.get("source_sha256"),
            "cir_sha256": result.get("cir_sha256"),
            "artifact": _artifact(result, "execution"),
            "property": None, "resource": None, "event_index": None,
            "runs_started": run.get("started"),
            "runs_completed": run.get("completed"),
            "hang": bool(run.get("hang")),
        })
    if ledger.get("current_evaluation") == "source_build_failed":
        log_path = result.get("source_build_log")
        log = ""
        if log_path and Path(log_path).is_file():
            from .generation import _compiler_errors
            log = _compiler_errors(Path(log_path).read_text(encoding="utf-8"))
        items.append({
            "layer": "source_build",
            "tool": "cargo",
            "status": "failed",
            "source_sha256": result.get("source_sha256"),
            "cir_sha256": result.get("cir_sha256"),
            "artifact": log_path,
            "property": None, "resource": None, "event_index": None,
            "compiler_log": log or None,
        })
    functional = result.get("functional") or {}
    if functional.get("status") in {"pass", "fail"}:
        items.append({
            "layer": "external_functional",
            "tool": "run_functional_check",
            "status": functional.get("status"),
            "source_sha256": result.get("source_sha256"),
            "cir_sha256": result.get("cir_sha256"),
            "artifact": functional.get("evidence_path"),
            "artifact_sha256": _sha(functional.get("evidence_path")),
            "property": None, "resource": None, "event_index": None,
            "observed_stdout": functional.get("raw_stdout"),
            "returncode": functional.get("returncode"),
            "timed_out": functional.get("timed_out"),
            "reason": functional.get("reason"),
        })
    return items


def _artifact(result: dict, role: str) -> str | None:
    for art in result.get("artifacts") or []:
        if art.get("role") == role:
            return art.get("path")
    return None


def feedback_from_toolchain(arm: str, result: dict) -> tuple[str, list[dict]]:
    items = toolchain_items(result)
    for item in items:
        item["status_in_prompt"] = True
        item["detail_in_prompt"] = False
    lines = [f"{item['layer']}={item['status']}" for item in items]
    if arm == "counterexample":
        for item in items:
            if item["layer"] == "bounded_trace_deviation" and item["status"] == "violation":
                item["detail_in_prompt"] = True
                lines.append(
                    f"conform.event_index={item.get('event_index')} "
                    f"got={item.get('got')} resource={item.get('resource')} "
                    f"checker_next={item.get('checker_next')}")
            if item["layer"] == "external_functional" and item["status"] == "fail":
                item["detail_in_prompt"] = True
                lines.append(
                    f"functional.observed_stdout={item.get('observed_stdout')!r} "
                    f"returncode={item.get('returncode')} timed_out={item.get('timed_out')}")
            if item["layer"] == "source_build" and item["status"] == "failed" and item.get("compiler_log"):
                item["detail_in_prompt"] = True
                lines.append(f"cargo.compiler_log={item.get('compiler_log')}")
            if item["layer"] == "execution" and item["status"] == "timeout":
                item["detail_in_prompt"] = True
                lines.append(
                    f"execution.started={item.get('runs_started')} "
                    f"completed={item.get('runs_completed')} hang={item.get('hang')}")
    if arm not in {"verdict_only", "counterexample"}:
        raise KeyError(arm)
    return "\n".join(lines) + "\n", items


def has_defect_signal(result: dict) -> bool:
    ledger = result.get("ledger") or {}
    trace = (ledger.get("trace") or {}).get("state")
    run = (ledger.get("run") or {}).get("state")
    functional = (result.get("functional") or {}).get("status")
    verdict = ledger.get("current_evaluation")
    if verdict in {"tool_error", "instrument_failed", "instrument_build_failed"}:
        return False
    return (trace == "observed_violation" or functional == "fail"
            or run in {"timeout", "runtime_crash", "partial"}
            or verdict in {"explicit_failure", "functional_failure", "requirement_failure",
                           "timeout", "runtime_crash", "source_build_failed"})
