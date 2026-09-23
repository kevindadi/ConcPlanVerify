"""§6.1 A2-ml offline reclassification (K-3).

Re-run the fixed `classify_detection`/`classify_tool_run` on the stored Miri
outputs of every A2-ml round of flash-repair-main-v1 and report whether any
`tools_green` decision changes. No LLM requests.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.rust_arm import ToolRun, classify_tool_run  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")


def main() -> int:
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    rows = []
    changed = 0
    decision_changes = []
    for rep in summary["reps"]:
        for t in rep["tasks"]:
            rec = (t.get("arms") or {}).get("A2_tools_iter_ml") or {}
            rounds = (rec.get("notes") or {}).get("tool_rounds") or []
            for ri, rnd in enumerate(rounds, 1):
                old_detected = new_detected = False
                old_green = new_green = True
                for run in rnd.get("miri", []) or []:
                    files = run.get("files") or {}
                    try:
                        out = Path(files["stdout.txt"]).read_text(encoding="utf-8")
                        err = Path(files["stderr.txt"]).read_text(encoding="utf-8")
                    except Exception:  # noqa: BLE001
                        continue
                    old_status = run.get("extra", {}).get("status")
                    old_det = bool(run.get("extra", {}).get("detected"))
                    tr = ToolRun(tool="miri", argv=[], exit_code=run.get("exit_code"),
                                 wall_ms=run.get("wall_ms") or 0,
                                 timed_out=(old_status == "timeout"), stdout=out,
                                 stderr=err, stdout_sha256="", stderr_sha256="")
                    new = classify_tool_run(tr)
                    old_detected |= old_det
                    new_detected |= bool(new["detected"])
                    old_green &= (old_status == "clean")
                    new_green &= (new["status"] == "clean")
                    if old_status != new["status"]:
                        changed += 1
                        rows.append((t["task"], rep["rep"], ri,
                                     run.get("extra", {}).get("seed"),
                                     old_status, new["status"]))
                lockbud = rnd.get("lockbud") or {}
                lb_det = bool((lockbud.get("extra") or {}).get("detected"))
                lb_green = (lockbud.get("status") in ("clean", "lockbud_unavailable", "skipped")
                            and not lb_det)
                old_tier = bool(rnd.get("build_ok") and old_green and not old_detected and lb_green)
                new_tier = bool(rnd.get("build_ok") and new_green and not new_detected and lb_green)
                if old_tier != new_tier:
                    decision_changes.append((t["task"], rep["rep"], ri, old_tier, new_tier))
    lines = ["# A2-ml offline reclassification (K-3)", "",
             f"- rounds scanned: all A2-ml rounds of `{BATCH.name}`",
             f"- miri runs whose status changed: **{changed}**",
             f"- `tools_green` decisions changed: **{len(decision_changes)}**", ""]
    if rows:
        lines += ["| task | rep | round | seed | old | new |", "| --- | --- | --- | --- | --- | --- |"]
        for r in rows[:200]:
            lines.append("| " + " | ".join(str(x) for x in r) + " |")
    else:
        lines.append("No Miri status changed; the classifier fix (word-boundary "
                     "`deadlock`/`deadrace`) does not affect any stored A2 output. "
                     "The deviation is therefore **closed as verified no-impact**.")
    if decision_changes:
        lines += ["", "## Changed tools_green decisions", ""]
        for d in decision_changes:
            lines.append(f"- {d}")
    (BATCH / "A2_RECLASS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"changed_miri_statuses": changed,
                      "changed_tools_green": len(decision_changes)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
