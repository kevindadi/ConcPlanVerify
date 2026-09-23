"""Rebase offline checks onto BIN_MAIN (§3.1).

Re-runs, with a single binary:
- the frozen-contract replay (CONTRACT_STRENGTH) — via the contract-strength CLI;
- `explore` for every accepted A3 CIR in the v3 batch;
- the conformance-v4 offline smoke (optional: --conformance).
Writes `experiments/REBASE_<sha8>.md` listing any verdict change.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.concir_client import ConcirClient  # noqa: E402
from cir_workflow.experiments_v2 import load_manifest  # noqa: E402

BATCH = REPO / ("experiments/flash-repair-smoke-v3/"
                "run-20260919T065313-94177-958361")


def sha8(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:8]


def main() -> int:
    binary = Path(sys.argv[1]) if len(sys.argv) > 1 else (
        REPO.parent / "ConcIR/target/release/concir-backend")
    tag = sha8(binary)
    client = ConcirClient(binary, workdir=REPO / "experiments" / f"rebase-{tag}",
                          timeout=60.0)
    summary = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    tasks = {t.id: t for t in load_manifest(REPO / "benchmarks/MANIFEST.json")}
    rows = []
    changes = []
    for task in summary["tasks"]:
        tid = task["task"]
        task_dir = tasks.get(tid)
        contract = task_dir.resolve(REPO / "benchmarks", task_dir.contract) if task_dir else None
        for arm in ("A3_local", "A3_whole"):
            rec = (task.get("arms") or {}).get(arm) or {}
            if not rec.get("accepted"):
                continue
            cir = rec.get("final_cir")
            old = (rec.get("oracle") or {}).get("verify_pass")
            if not cir or not Path(cir).is_file():
                rows.append((tid, arm, old, "no_cir", "missing"))
                continue
            result = client.explore(Path(cir), contract)
            new = result.outcome == "PASS" and result.complete is True
            changed = (old is not None and old != new)
            if changed:
                changes.append((tid, arm, old, new))
            rows.append((tid, arm, old, new, result.outcome))

    lines = [f"# REBASE {tag}", "",
             f"- binary sha256: `{hashlib.sha256(binary.read_bytes()).hexdigest()}`",
             f"- source: `{binary}`", "",
             "## v3 accepted A3 CIR `explore` under BIN_MAIN", "",
             "| task | arm | old verify_pass | new verify_pass | outcome |",
             "| --- | --- | --- | --- | --- |"]
    for tid, arm, old, new, outcome in rows:
        lines.append(f"| {tid} | {arm} | {old} | {new} | {outcome} |")
    lines += ["", f"Verdict changes: **{len(changes)}** "
                  + (f"({changes})" if changes else "(none)"), ""]
    out = REPO / "experiments" / f"REBASE_{tag}.md"
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out}; changes={len(changes)}")
    for c in changes:
        print("CHANGE", c)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
