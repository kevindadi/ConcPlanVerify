"""Frozen development-set cases for the feedback-mechanism pilot.

Sources are hand-authored in this repository. They are not Kimi 2.7 Code
outputs and must not be labelled as such.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .pilot_oracle import (
    _build, design_join, design_lock_order, design_sync_matches_cir,
    requirement_hold, requirement_lock_order, requirement_stdout,
    requirement_terminates, support_of, _spawn_join,
)

FIXTURE_ROOT = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "pilot_v7"

CASES = (
    "lock_order",
    "early_release",
    "omitted_lock",
    "compute_value",
    "compute_label",
    "no_join",
)


def case_dir(name: str, root: Path | None = None) -> Path:
    return (root or FIXTURE_ROOT) / name


def load_case(name: str, root: Path | None = None) -> dict[str, Any]:
    directory = case_dir(name, root)
    cir = json.loads((directory / "design.cir.json").read_text(encoding="utf-8"))
    contract = json.loads((directory / "contract.json").read_text(encoding="utf-8"))
    return {
        "id": name,
        "class": (directory / "class.txt").read_text(encoding="utf-8").strip(),
        "directory": directory,
        "requirements": (directory / "requirements.md").read_text(encoding="utf-8"),
        "defect": (directory / "defect.rs").read_text(encoding="utf-8"),
        "control": (directory / "control.rs").read_text(encoding="utf-8"),
        "cir": cir,
        "contract": contract,
        "origin": "hand-authored fixture; not a model generation",
    }


def _bundle(parts: list[dict], oracle: str) -> dict[str, Any]:
    statuses = [part["status"] for part in parts]
    if any(status == "fail" for status in statuses):
        overall = "fail"
    elif any(status == "tool_error" for status in statuses):
        overall = "tool_error"
    elif any(status == "unknown" for status in statuses):
        overall = "unknown"
    elif statuses and all(status == "pass" for status in statuses):
        overall = "pass"
    else:
        overall = "unknown"
    return {"oracle": oracle, "status": overall,
            "observed": [part.get("observed") for part in parts],
            "expected": None, "checks": parts,
            "uncovered": [part.get("id") or part.get("oracle")
                          for part in parts if part["status"] == "unknown"]}


def _spawn_check(source: str, build_ok: bool) -> dict[str, Any]:
    parsed = _spawn_join(source)
    if parsed["status"] != "ok":
        return {"id": "spawn_join", "oracle": "spawn_join", "status": "unknown",
                "observed": None, "reason": parsed.get("reason"), "expected": None}
    ok = parsed["joined"] and "w" in parsed["calls"]
    status = "pass" if ok else "fail"
    if status == "pass" and not build_ok:
        status = "fail"
    return {"id": "spawn_join", "oracle": "spawn_join", "status": status,
            "observed": {"joined": parsed["joined"], "calls": parsed["calls"],
                         "handles": parsed["handles"]},
            "expected": None}


def evaluate_role(case: dict[str, Any], source: str, work: Path) -> dict[str, Any]:
    name = case["id"]
    cir = case["cir"]
    built, _log = _build(source, work / "oracle-build")
    if name == "lock_order":
        requirement = _bundle([
            requirement_lock_order(source, "w", ["a", "b"], build_ok=built),
            _spawn_check(source, built),
        ], "requirement_lock_order")
        design = _bundle([
            design_lock_order(source, cir, "w", build_ok=built),
            design_join(source, cir, build_ok=built),
        ], "design_lock_order")
    elif name == "early_release":
        hold = requirement_hold(source, "w", "a", "b", build_ok=built)
        requirement = _bundle([hold, _spawn_check(source, built)], "requirement_hold")
        design = _bundle([
            {**hold, "id": "design_hold", "oracle": "design_hold"},
            design_join(source, cir, build_ok=built),
        ], "design_hold")
    elif name == "omitted_lock":
        requirement = _bundle([
            requirement_lock_order(source, "w", ["a", "b"], build_ok=built),
            _spawn_check(source, built),
        ], "requirement_locks_present")
        design = _bundle([
            design_lock_order(source, cir, "w", build_ok=built),
            design_join(source, cir, build_ok=built),
        ], "design_lock_order")
    elif name in {"compute_value", "compute_label"}:
        stdout = requirement_stdout(source, work / "stdout", "DONE done=6")
        requirement = _bundle([
            {**stdout, "id": "stdout"},
        ], "requirement_stdout")
        requirement["uncovered"] = []
        design = design_sync_matches_cir(source, cir, "w")
        if not built:
            if design["status"] == "pass":
                design["status"] = "fail"
                design["reason"] = "build_failed"
    elif name == "no_join":
        term = requirement_terminates(source, work / "run", timeout=2.0)
        requirement = _bundle([
            {**term, "id": "terminates"},
            _spawn_check(source, built),
        ], "requirement_terminates")
        design = design_join(source, cir, build_ok=built)
    else:
        raise KeyError(name)
    support = support_of(source)
    return {"requirement": requirement, "design": design, "support": support,
            "requirement_error": requirement["status"] == "fail",
            "cir_design_deviation": design["status"] == "fail",
            "tool_error": requirement["status"] == "tool_error" or design["status"] == "tool_error",
            "capability_gap": not support["supported"],
            "build_ok": built}


