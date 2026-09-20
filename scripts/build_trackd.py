"""Build experiments/detection-v3/TRACKD.json from DETECTION.json (§6)."""

from __future__ import annotations

import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DET = REPO / "experiments/detection-v3/DETECTION.json"


def _miri(entry: dict) -> dict:
    runs = list(entry.get("miri", []) or [])
    ext = entry.get("miri_extended")
    if isinstance(ext, dict):
        runs.extend(ext.get("runs", []) if "runs" in ext else [ext])
    statuses = [r.get("extra", {}).get("status") for r in runs]
    detected = any(r.get("extra", {}).get("detected") for r in runs)
    return {"seeds": len(runs), "detected": detected,
            "statuses": sorted(set(s for s in statuses if s))}


def _lockbud(entry: dict) -> dict:
    lb = entry.get("lockbud") or {}
    extra = lb.get("extra") or {}
    return {"status": lb.get("status") or extra.get("status"),
            "detected": extra.get("detected") or []}


def main() -> int:
    data = json.loads(DET.read_text(encoding="utf-8"))
    tasks = []
    for rec in data["records"]:
        rust = rec.get("rust") or {}
        concir = rec.get("concir") or {}
        entry = {"task": rec["task"], "family": (rec.get("ground_truth") or {}).get("defect_family")}
        for side in ("buggy", "fixed"):
            c = concir.get(side) or {}
            r = rust.get(side) or {}
            entry[side] = {
                "concir_petri": (c.get("petri") or {}).get("outcome"),
                "concir_interp": (c.get("interp") or {}).get("outcome"),
                "concir_petri_ms": (c.get("petri") or {}).get("wall_ms"),
                "miri": _miri(r) if r else None,
                "lockbud": _lockbud(r) if r else None,
            }
        tasks.append(entry)
    payload = {"schema_version": "trackd-v1",
               "binary_sha256": data.get("binary_sha256"),
               "lockbud": data.get("lockbud"),
               "miri_combos": data.get("miri_combos"),
               "caveats": data.get("caveats"),
               "tasks": tasks}
    out = REPO / "experiments/detection-v3/TRACKD.json"
    out.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {out}: {len(tasks)} tasks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
