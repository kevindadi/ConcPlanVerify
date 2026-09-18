"""Track D: detection-capability comparison (no LLM).

Runs the local detectors on the frozen benchmark inputs:

- ConcIR ``explore`` (petri and interp) on the human-written CIR ground truth;
- Miri on the Rust reference programs with the frozen seed/preemption table;
- Lockbud when available (currently absent on this machine).

The output is a per-task, per-detector record of detection / miss / false
positive and wall-clock. Miri results are explicitly labelled dynamic: a miss is
never reported as "no bug". The ConcIR arm consumes CIR, not source, so the
comparison is capability-level, not same-input.
"""

from __future__ import annotations

import json
import os
import tempfile
import time
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient
from .experiments_v2 import MIRI_COMBOS, load_manifest, sha256_file
from .rust_arm import RustArmProject, lockbud_available


def _default_binary() -> Path | None:
    env = os.environ.get("CONCIR_BACKEND")
    candidates = [Path(env)] if env else []
    repo = Path(__file__).resolve().parents[2]
    candidates.append(repo.parent / "ConcIR/target/release/concir-backend")
    candidates.append(repo.parent / "ConcIR/target/debug/concir-backend")
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def _cir_arm(root: Path, task, client: ConcirClient) -> dict[str, Any]:
    record: dict[str, Any] = {"arm": "concir", "input_form": "human-written modular CIR"}
    for side, rel in (("buggy", task.buggy_cir), ("fixed", task.fixed_cir)):
        if not rel:
            record[side] = None
            continue
        path = root / rel
        contract = root / task.contract
        entry: dict[str, Any] = {}
        for engine in ("petri", "interp"):
            result = client.explore(path, contract, engine)
            entry[engine] = {
                "outcome": result.outcome,
                "complete": result.complete,
                "states_explored": (result.payload or {}).get("states_explored"),
                "transitions_explored": (result.payload or {}).get("transitions_explored"),
                "wall_ms": result.wall_ms,
                "status": result.status,
                "kind": result.kind,
                "input_sha256": sha256_file(path),
            }
        record[side] = entry
    return record


def _rust_arm(root: Path, task, *, run_miri: bool, timeout_s: float,
              work_root: Path) -> dict[str, Any]:
    record: dict[str, Any] = {"arm": "rust", "input_form": "Rust source"}
    for side, rel in (("buggy", task.buggy_rs), ("fixed", task.fixed_rs)):
        if not rel:
            record[side] = None
            continue
        src = (root / rel).read_text(encoding="utf-8")
        project = RustArmProject(work_root / f"_rust_{task.id}_{side}", src, name="probe")
        rec = project.analyze(run_miri=run_miri, run_lockbud=True, timeout_s=timeout_s)
        rec["input_sha256"] = sha256_file(root / rel)
        record[side] = rec
    return record


def _miri_detected(rust_record: dict[str, Any] | None) -> bool | None:
    if not rust_record or not rust_record.get("miri"):
        return None
    for run in rust_record["miri"]:
        if run.get("extra", {}).get("detected"):
            return True
    return False


def run_detection(manifest_path: Path | str, out_dir: Path | str, *,
                  run_miri: bool = True, timeout_s: float = 120.0) -> dict[str, Any]:
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    binary = _default_binary()
    client = None
    if binary is not None:
        client = ConcirClient(binary, workdir=out / "calls", timeout=60.0)

    tasks = load_manifest(manifest_path)
    records = []
    for task in tasks:
        if task.status != "ready":
            records.append({"task": task.id, "status": "skipped_to_author"})
            continue
        entry: dict[str, Any] = {"task": task.id, "status": "ready",
                                 "ground_truth": task.ground_truth}
        if client is not None and task.buggy_cir and task.contract:
            entry["concir"] = _cir_arm(root, task, client)
        else:
            entry["concir"] = {"skipped": "no CIR/contract or no backend"}
        if task.buggy_rs:
            entry["rust"] = _rust_arm(root, task, run_miri=run_miri,
                                      timeout_s=timeout_s,
                                      work_root=out / "rust_projects")
        else:
            entry["rust"] = {"skipped": "no rust reference"}
        records.append(entry)

    result = {
        "schema_version": "detection-v1",
        "binary_sha256": sha256_file(binary) if binary else None,
        "binary": str(binary) if binary else None,
        "miri_combos": [{"seed": s, "preemption_rate": r} for s, r in MIRI_COMBOS],
        "lockbud_available": lockbud_available(),
        "run_miri": run_miri,
        "records": records,
        "caveats": [
            "Miri is dynamic and schedule-dependent: a miss is not proof of "
            "absence, and is not evidence of safety.",
            "The deadlock fixtures are lock-order bugs that only manifest on some "
            "interleavings; each Miri seed runs one schedule, so a miss here is "
            "expected rather than surprising.",
            "The ConcIR arm consumes human-written CIR, not Rust source; the "
            "comparison is capability-level, not same-input.",
            "Lockbud is unavailable on this machine unless installed and pinned.",
        ],
    }
    (out / "DETECTION.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return result


def render_markdown(result: dict[str, Any]) -> str:
    lines = ["# Track D — detection capability (no LLM)", "",
             f"- ConcIR binary sha256: `{result['binary_sha256']}`",
             f"- Miri combos: {result['miri_combos']}",
             f"- Lockbud available: {result['lockbud_available']}", "",
             "## Per task", "",
             "| task | CIR buggy | CIR fixed | Miri buggy | Miri fixed | notes |",
             "| --- | --- | --- | --- | --- | --- |"]
    for rec in result["records"]:
        if rec.get("status") != "ready":
            lines.append(f"| {rec['task']} | — | — | — | — | {rec['status']} |")
            continue
        cir = rec.get("concir") or {}
        buggy = (cir.get("buggy") or {}).get("petri", {}) if isinstance(cir.get("buggy"), dict) else {}
        fixed = (cir.get("fixed") or {}).get("petri", {}) if isinstance(cir.get("fixed"), dict) else {}
        rust = rec.get("rust") or {}
        notes = []
        if buggy.get("status") not in (None, "pass", "unsupported"):
            notes.append("CIR detects")
        if fixed.get("outcome") not in (None, "PASS"):
            notes.append("CIR false positive")
        lines.append(
            f"| {rec['task']} | {buggy.get('outcome')} | {fixed.get('outcome')} | "
            f"{_miri_detected(rust.get('buggy'))} | {_miri_detected(rust.get('fixed'))} | "
            f"{', '.join(notes)} |")
    lines += ["", "## Caveats", ""]
    lines += [f"- {c}" for c in result["caveats"]]
    return "\n".join(lines) + "\n"
