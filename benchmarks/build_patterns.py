#!/usr/bin/env python3
"""Author the ``benchmarks/patterns/P1..P9`` task set from the legacy corpus.

This script is reproducible benchmark tooling, not experiment code. It:

- copies the legacy Rust reference programs into ``P<N>/buggy.rs`` / ``fixed.rs``
  (the baseline patterns P7/P8/P9 only have one side in the legacy corpus; that
  one side is used for both, since those patterns are bug-free by construction);
- writes ``ground_truth.json`` from the legacy manifest;
- writes ``P<N>/legacy_brief.md`` with the legacy canonical requirement, for
  tasks whose natural-language ``spec.md`` is not yet authored;
- authors the modular-CIR ground truth for the lock-order patterns it can build
  and validate (P1, P4);
- emits ``benchmarks/MANIFEST.json`` with sha256 for every frozen file and a
  ``status`` per task (``ready`` or ``to_author``).

Run from the repository root: ``python3 benchmarks/build_patterns.py``.
"""

from __future__ import annotations

import copy
import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEGACY = ROOT / "benchmarks/legacy-cir2cvn"
OUT = ROOT / "benchmarks/patterns"
FROZEN = ROOT / "experiments/deepseek-flash-repair-v1/frozen-inputs"

IDS = ["P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9"]
MAP = {
    "P1": ("mutex_deadlock", None),
    "P2": ("signal_loss", None),
    "P3": ("channel_deadlock", None),
    "P4": ("three_way_deadlock", "three_way_deadlock"),
    "P5": ("partial_deadlock", "partial_deadlock"),
    "P6": ("dual_condvar", "dual_condvar"),
    "P7": ("semaphore_throttle", "semaphore_throttle"),
    "P8": ("cas_race", "cas_race"),
    "P9": ("fn_summary_prop", "fn_summary_prop"),
}

SPECS = {
    "P1": (
        "Design a program with two mutexes A and B and two worker threads. Each "
        "worker must acquire both mutexes, perform its work inside the critical "
        "section, release both mutexes, and terminate. The main thread spawns "
        "both workers and joins them. Every interleaving must terminate without "
        "deadlock and both workers must complete."
    ),
    "P4": (
        "Design a program with three mutexes A, B, C and three worker threads. "
        "Worker 1 uses A and B, worker 2 uses B and C, worker 3 uses C and A. "
        "Each worker holds its two mutexes at the same time, releases them, and "
        "terminates. Main spawns all three and joins them. Every interleaving "
        "must terminate and all three workers must complete."
    ),
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def legacy_cases() -> dict:
    data = json.loads((LEGACY / "benchmarks/manifest.json").read_text(encoding="utf-8"))
    return {c["id"]: c for c in data["cases"]}


def build_p1_cir() -> dict:
    buggy = json.loads((FROZEN / "t2_abba_frozen.cir.json").read_text(encoding="utf-8"))
    fixed = copy.deepcopy(buggy)
    for fn in fixed["modules"][0]["functions"]:
        if fn["name"] == "t2":
            body = fn["body"]
            body[0]["resource"], body[1]["resource"] = "main::a", "main::b"
            body[2]["resource"], body[3]["resource"] = "main::b", "main::a"
    contract = json.loads((FROZEN / "t2_abba_contract.json").read_text(encoding="utf-8"))
    return {"buggy": buggy, "fixed": fixed, "contract": contract}


def _lock_worker(sid: str, first: str, second: str) -> list[dict]:
    return [
        {"sid": "s1", "kind": "mutex_lock", "resource": first},
        {"sid": "s2", "kind": "mutex_lock", "resource": second},
        {"sid": "s3", "kind": "mutex_unlock", "resource": second},
        {"sid": "s4", "kind": "mutex_unlock", "resource": first},
        {"sid": "s5", "kind": "return"},
    ]


def build_p4_cir() -> dict:
    def program(t3_first: str) -> dict:
        resources = [{"name": n, "kind": "sync", "type": "Mutex", "mode": "Sync"}
                     for n in ("a", "b", "c")]
        functions = [
            {"name": "main", "kind": "normal", "body": [
                {"sid": "s1", "kind": "scope",
                 "funcs": ["main::t1", "main::t2", "main::t3"]},
                {"sid": "s2", "kind": "return"},
            ]},
            {"name": "t1", "kind": "normal", "form": "closure",
             "body": _lock_worker("t1", "main::a", "main::b")},
            {"name": "t2", "kind": "normal", "form": "closure",
             "body": _lock_worker("t2", "main::b", "main::c")},
            {"name": "t3", "kind": "normal", "form": "closure",
             "body": _lock_worker("t3", t3_first, "main::c" if t3_first == "main::a"
                                  else "main::a")},
        ]
        return {
            "program": "three_way", "version": "3.5.0", "entry": "main::main",
            "modules": [{
                "name": "main",
                "provides": {"resources": ["a", "b", "c"],
                             "functions": ["main", "t1", "t2", "t3"]},
                "requires": {"resources": [], "functions": []},
                "resources": resources, "protection": [], "functions": functions,
            }],
        }

    preserved = [{"kind": "reachable", "description": f"main::{t} completes",
                  "goal": {"kind": "function_completed", "function": f"main::{t}"}}
                 for t in ("t1", "t2", "t3")]
    contract = {
        "name": "p4-three-lock",
        "properties": [{"kind": "deadlock_free", "id": "no-deadlock"}],
        "preserved": preserved,
        "assumptions": {"sequential_consistency": True, "no_spurious_wakeups": True},
        "bounds": {"max_threads": 8, "max_frames_per_thread": 8, "max_states": 50000,
                   "max_depth": 64, "max_boundary_events": 256},
        "allowed_scope": {"allow_lock_reorder": True},
    }
    # buggy: t3 takes c then a (circular); fixed: t3 takes a then c (global order)
    return {"buggy": program("main::c"), "fixed": program("main::a"), "contract": contract}


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    cases = legacy_cases()
    manifest_tasks = []

    for pid in IDS:
        legacy_id, _ = MAP[pid]
        case = cases[legacy_id]
        task_dir = OUT / pid
        task_dir.mkdir(parents=True, exist_ok=True)
        files: dict[str, str] = {}

        # Rust references (baselines reuse their single side for both).
        rust_dir = LEGACY / "benchmarks/rust" / legacy_id
        buggy_src = rust_dir / "buggy.rs"
        fixed_src = rust_dir / "fixed.rs"
        if buggy_src.is_file():
            shutil.copyfile(buggy_src, task_dir / "buggy.rs")
        if fixed_src.is_file():
            shutil.copyfile(fixed_src, task_dir / "fixed.rs")
        if not buggy_src.is_file() and fixed_src.is_file():
            shutil.copyfile(fixed_src, task_dir / "buggy.rs")
        if not fixed_src.is_file() and buggy_src.is_file():
            shutil.copyfile(buggy_src, task_dir / "fixed.rs")

        # ground truth
        gt = {"id": pid, "legacy_id": legacy_id,
              "defect_type": case.get("defect_type"),
              "expected": case.get("expected")}
        write_json(task_dir / "ground_truth.json", gt)

        # legacy brief (draft input until spec.md is authored)
        req = case.get("requirements", {})
        brief = [f"# {pid} legacy brief ({legacy_id})", "",
                 "Canonical requirement from the legacy corpus (draft; may reveal the",
                 "defect and therefore is NOT used as the live generation input):", "",
                 req.get("canonical", ""), ""]
        (task_dir / "legacy_brief.md").write_text("\n".join(brief), encoding="utf-8")

        status = "to_author"
        if pid in SPECS and pid in ("P1", "P4"):
            (task_dir / "spec.md").write_text(SPECS[pid] + "\n", encoding="utf-8")
            cir = build_p1_cir() if pid == "P1" else build_p4_cir()
            write_json(task_dir / "buggy.cir.json", cir["buggy"])
            write_json(task_dir / "fixed.cir.json", cir["fixed"])
            write_json(task_dir / "contract.json", cir["contract"])
            status = "ready"

        # collect hashes of files that exist
        for name in ("spec.md", "buggy.rs", "fixed.rs", "buggy.cir.json",
                     "fixed.cir.json", "contract.json", "ground_truth.json"):
            path = task_dir / name
            if path.is_file():
                files[f"patterns/{pid}/{name}"] = sha256(path)

        manifest_tasks.append({
            "id": pid, "status": status, "directory": f"patterns/{pid}",
            "spec": f"patterns/{pid}/spec.md" if (task_dir / "spec.md").is_file() else None,
            "contract": f"patterns/{pid}/contract.json" if status == "ready" else None,
            "buggy_cir": f"patterns/{pid}/buggy.cir.json" if status == "ready" else None,
            "fixed_cir": f"patterns/{pid}/fixed.cir.json" if status == "ready" else None,
            "buggy_rs": f"patterns/{pid}/buggy.rs",
            "fixed_rs": f"patterns/{pid}/fixed.rs",
            "visualized": status,
            "ground_truth": gt,
            "files": files,
        })

    # real cases already carry CIR + contract
    for case in ("rmw-zenoh-998", "dashmap-369"):
        base = ROOT / "benchmarks/real-cases/cases" / case
        files = {}
        for name in ("buggy.cir.json", "fixed.cir.json", "contract.json", "expected.json"):
            path = base / name
            if path.is_file():
                files[f"real-cases/cases/{case}/{name}"] = sha256(path)
        manifest_tasks.append({
            "id": case, "status": "ready", "directory": f"real-cases/cases/{case}",
            "spec": None, "contract": f"real-cases/cases/{case}/contract.json",
            "buggy_cir": f"real-cases/cases/{case}/buggy.cir.json",
            "fixed_cir": (f"real-cases/cases/{case}/fixed.cir.json"
                          if (base / "fixed.cir.json").is_file() else None),
            "buggy_rs": None, "fixed_rs": None, "visualized": "ready",
            "ground_truth": json.loads((base / "expected.json").read_text(encoding="utf-8")),
            "files": files,
        })

    manifest = {
        "version": 1,
        "description": "Frozen v2 benchmark tasks. Per-task status is ready or "
                       "to_author; to_author tasks are excluded from live runs.",
        "tasks": manifest_tasks,
    }
    write_json(ROOT / "benchmarks/MANIFEST.json", manifest)
    ready = sum(1 for t in manifest_tasks if t["status"] == "ready")
    print(f"wrote benchmarks/MANIFEST.json: {len(manifest_tasks)} tasks, {ready} ready")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
