"""Rebuild TRACKD.json with 16-seed Miri for the Rust-reference tasks (§1.3).

Reuses the detection-v3 ConcIR records; re-runs the Rust arm (Miri 16 seeds +
Lockbud) for tasks that have `rust/{buggy,fixed}.rs`. Tasks without a Rust
reference keep ConcIR-only rows (rendered in the CIR-only appendix).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.experiments_v2 import load_manifest  # noqa: E402
from cir_workflow.rust_arm import RustArmProject, lockbud_path  # noqa: E402


def _miri(rec: dict | None) -> dict | None:
    if not rec:
        return None
    runs = list(rec.get("miri") or [])
    ext = rec.get("miri_extended")
    if isinstance(ext, dict):
        runs.extend(ext.get("runs", []) if "runs" in ext else [ext])
    statuses = [r.get("extra", {}).get("status", "unknown") for r in runs]
    detected = any(r.get("extra", {}).get("detected") for r in runs)
    leak = any(r.get("extra", {}).get("thread_leak") for r in runs)
    return {"seeds": len(runs), "detected": detected, "thread_leak": leak,
            "statuses": statuses}


def _lockbud(rec: dict | None) -> dict | None:
    if not rec:
        return None
    lb = rec.get("lockbud") or {}
    extra = lb.get("extra") or {}
    return {"status": lb.get("status") or extra.get("status"),
            "detected": extra.get("detected") or []}


def main() -> int:
    out = REPO / "experiments/detection-v3"
    det = json.loads((out / "DETECTION.json").read_text(encoding="utf-8"))
    concir_by_task = {r["task"]: r.get("concir") for r in det["records"]}
    manifest = load_manifest(REPO / "benchmarks/MANIFEST.json")
    root = REPO / "benchmarks"
    work = out / "rust16"
    tasks = []
    only = sys.argv[1:] or None
    for task in manifest:
        if only and task.id not in only:
            continue
        concir = concir_by_task.get(task.id) or {}
        entry = {"task": task.id}
        for side, rel in (("buggy", task.buggy_rs), ("fixed", task.fixed_rs)):
            c = concir.get(side) or {}
            base = {"concir_petri": (c.get("petri") or {}).get("outcome"),
                    "concir_interp": (c.get("interp") or {}).get("outcome"),
                    "concir_petri_ms": (c.get("petri") or {}).get("wall_ms")}
            if rel and (root / rel).is_file():
                src = (root / rel).read_text(encoding="utf-8")
                project = RustArmProject(work / f"{task.id.replace('/', '_')}_{side}",
                                         src, name="probe")
                rec = project.analyze(run_miri=True, run_lockbud=True, timeout_s=30.0,
                                      run_miri_many_seeds=False, miri_seed_count=16)
                base["miri"] = _miri(rec)
                base["lockbud"] = _lockbud(rec)
            else:
                base["miri"] = None
                base["lockbud"] = None
            entry[side] = base
        tasks.append(entry)
    lockbud = {"available": lockbud_path() is not None,
               "path": str(lockbud_path()) if lockbud_path() else None}
    payload = {"schema_version": "trackd-v2", "binary_sha256": det.get("binary_sha256"),
               "miri_seeds": 16, "lockbud": lockbud, "tasks": tasks}
    (out / "TRACKD.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({"tasks": len(tasks), "rust_tasks": sum(
        1 for t in tasks if (t["buggy"].get("miri") or t["fixed"].get("miri")))},
        indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
