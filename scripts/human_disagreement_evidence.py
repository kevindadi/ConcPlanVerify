"""§A3: build HUMAN_DISAGREEMENT_EVIDENCE.md for the two human/auto
disagreements. No LLM requests; re-runs behavior + Miri to capture raw evidence.
The `owner_verdict:` / `owner_reason:` lines are left blank for the owner.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.rust_arm import RustArmProject  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
OUT = REPO / "experiments/flash-repair-main-v1/expert-labels/HUMAN_DISAGREEMENT_EVIDENCE.md"
TARGETS = [("semaphore/acquire_twice_no_release", "A0_direct",
            "f42afd77e7a90afba7225bfa4678f3e37ce918e4265b7dd7d815045ceee3270e"),
           ("lock-order/partial_deadlock_bystander", "A1_self_iter",
            "a08bd1a020fa")]
TIMEOUT_S = 10.0


def _find(summary, task, arm, sha12):
    for rep in summary["reps"]:
        for t in rep["tasks"]:
            if t["task"] != task:
                continue
            r = (t["arms"] or {}).get(arm) or {}
            p = r.get("final_artifact_path")
            if p and hashlib.sha256(Path(p).read_bytes()).hexdigest().startswith(sha12):
                return rep["rep"], r, p
    return None, None, None


def main() -> int:
    summary = json.loads((BATCH / "SUMMARY.json").read_text())
    expert = json.loads((REPO / "experiments/flash-repair-main-v1/expert-labels/"
                         "EXPERT_LABELS.json").read_text())
    labels = {x["sha256"][:12]: x for x in expert["labels"]}
    lines = ["# Human / automatic-oracle disagreement evidence", "",
             "Owner verdict/reason lines are **blank for the owner**.", ""]
    for task, arm, sha12 in TARGETS:
        rep, rec, path = _find(summary, task, arm, sha12)
        lines += [f"## {task} / {arm} (`{sha12}`)", ""]
        if not path:
            lines += ["candidate not found", ""]
            continue
        src = Path(path).read_text(encoding="utf-8")
        lines += [f"- rep {rep}; artifact `{Path(path).relative_to(REPO)}`; "
                  f"sha256 `{hashlib.sha256(src.encode()).hexdigest()}`", ""]
        # requirements terminal
        rt = json.loads((REPO / "benchmarks/families" / task /
                         "repair_task.json").read_text())
        req = (REPO / "benchmarks" / rt["requirements_file"]).read_text(encoding="utf-8")
        lines += ["### Requirements (terminal line)", "", "```", req.strip()[-500:], "```", ""]
        lines += ["### candidate.rs", "", "```rust"]
        lines += [f"{i:>4}  {l}" for i, l in enumerate(src.splitlines(), 1)]
        lines += ["```", ""]
        # behavior
        with __import__("tempfile").TemporaryDirectory() as tmp:
            project = RustArmProject(tmp, src, name="ev")
            project.build(timeout_s=120.0)
            bt = project.behavior_run(timeout_s=TIMEOUT_S)
            beh = {"status": "hang" if bt.timed_out else "terminated",
                   "exit_code": bt.exit_code, "stdout": bt.stdout[-400:],
                   "stderr": bt.stderr[-400:], "wall_ms": bt.wall_ms}
            build = project.analyze(run_miri=True, run_lockbud=False, timeout_s=30.0,
                                    run_miri_many_seeds=False, miri_seed_count=16)
        miri = [r.get("extra", {}) for r in build.get("miri", [])]
        lines += ["### behavior", "",
                  f"- timeout threshold: {TIMEOUT_S}s; observed: `{beh['status']}` "
                  f"exit {beh['exit_code']} in {beh['wall_ms']} ms",
                  f"- stdout: `{beh['stdout'].strip()!r}`",
                  f"- stderr: `{beh['stderr'].strip()!r}`", ""]
        lines += ["### Miri (16 seeds)", "",
                  f"- statuses: `{[r.get('status') for r in miri]}`",
                  f"- detected: `{any(r.get('detected') for r in miri)}`; "
                  f"thread_leak: `{any(r.get('thread_leak') for r in miri)}`", ""]
        lab = labels.get(sha12)
        if lab:
            lines += ["### agent-proxy evidence", "",
                      f"- bug_present `{lab.get('bug_present')}`, "
                      f"design_preserved `{lab.get('design_preserved')}`",
                      f"- evidence: {lab.get('evidence')}", ""]
        o = rec.get("oracle") or {}
        lines += ["### automatic oracle", "",
                  f"- `behavior_status = {o.get('behavior_status')}`, "
                  f"`build_ok = {o.get('build_ok')}`, "
                  f"`miri_detected = {o.get('miri_detected')}`",
                  "- rule: `bug_present = behavior in {hang} OR model FAIL OR "
                  "expert yes OR by_construction`; `false_accept = accepted AND "
                  "bug_present` (RESULTS §1).", ""]
        lines += ["```", "owner_verdict:", "owner_reason:", "```", ""]
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
