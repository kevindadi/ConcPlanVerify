#!/usr/bin/env python3
"""§2 replay v2: re-score the v3-code G3 cells on the round-q toolchain.

For every cell with stored code rounds, re-run instrument -> build -> conform
--op-resource -> monitor. Classify each freeze-6 failure as build-fixed
(P-1), conform-fixed (P-2), or still failing; check accepted cells for
regressions and for `unmapped` drop. 0 LLM requests.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import sys
import tempfile
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import bounded_monitor, rust_oracle  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    _DROP_OPS, _conform_all_op_resource, _conform_kind, _rewrite_traces,
    load_gen_tasks, mapping_to_cir,
)

DEFAULT_BATCH = REPO / "experiments/flash-gen-main-v3-code/run-20260923T182939"


def _score(rust, cir_path, contract_path, binary, td):
    wrapped = rust_oracle.instrument_wrappers(rust, td / "inst")
    rust_oracle.prepare_project(td / "proj", wrapped["annotated"], wrapped["runtime"])
    built, log = rust_oracle.cargo_build(td / "proj")
    if not built:
        return {"built": False, "detail": log[-300:], "harness_notes": wrapped["harness_notes"]}
    rust_oracle.run_native(td / "proj", td / "traces", n=4, timeout=10)
    cir = json.loads(Path(cir_path).read_text(encoding="utf-8"))
    mapping, _ = mapping_to_cir(wrapped["resources"], cir)
    _rewrite_traces(td / "traces", td / "conform", mapping, _DROP_OPS)
    conform = _conform_all_op_resource(binary, cir_path, td / "conform")
    report = bounded_monitor.run_monitor(contract_path, td / "traces",
                                         resources=td / "inst/resources.json",
                                         binary=binary)
    return {"built": True, "conformant": conform["conformant"],
            "traces": conform["traces"], "kind": _conform_kind(conform["first_violation"]),
            "first_violation": conform["first_violation"], "monitor": report.get("status"),
            "unmapped": report.get("unmapped_resources", []),
            "harness_notes": wrapped["harness_notes"]}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch", default=str(DEFAULT_BATCH))
    parser.add_argument("--out", default="experiments/gen-code-replay-v2")
    parser.add_argument("--binary", default=os.environ.get(
        "CONCIR_BACKEND", "/Users/kevin/local-repos/ConcIR/target/release/concir-backend"))
    args = parser.parse_args()
    binary = Path(args.binary)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    records = []
    for cell_json in sorted(glob.glob(f"{Path(args.batch)}/**/CELL.json", recursive=True)):
        cell = Path(cell_json).parent
        rounds = sorted((cell / "code").glob("round-*.rs"),
                        key=lambda p: int(p.stem.split("-")[1])) if (cell / "code").is_dir() else []
        if not rounds:
            continue
        rec = json.loads(Path(cell_json).read_text(encoding="utf-8"))
        task_id = rec.get("task")
        cir_path = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
        if not cir_path or not Path(cir_path).is_file() or task_id not in tasks:
            continue
        before = bool(rec.get("accepted"))
        before_build = any(r.get("decision") != "build_failed" for r in rec.get("rounds", [])) or before
        with tempfile.TemporaryDirectory() as td:
            after = _score(rounds[-1].read_text(encoding="utf-8"), cir_path,
                           tasks[task_id].contract_path, binary, Path(td))
        after_ok = after.get("conformant") == after.get("traces") and after.get("traces")
        records.append({"task": task_id, "before_accepted": before,
                        "before_build": before_build, "after_ok": bool(after_ok),
                        "after": after})

    fails = [r for r in records if not r["before_accepted"]]
    fixed_build = [r for r in fails if not r["before_build"] and r["after_ok"]]
    fixed_conform = [r for r in fails if r["before_build"] and not r["after_ok"]][:]
    fixed_conform = [r for r in fails if r["before_build"] and r["after_ok"]]
    still = [r for r in fails if not r["after_ok"]]
    accepted = [r for r in records if r["before_accepted"]]
    regressions = [r for r in accepted if not r["after_ok"]]
    kinds = Counter(r["after"].get("kind") for r in still)
    unmap_before = sum(1 for r in accepted if r["after"].get("unmapped"))
    out = REPO / args.out
    out.mkdir(parents=True, exist_ok=True)
    def line(r):
        a = r["after"]
        return (f"| {r['task']} | {'accept' if r['before_accepted'] else ('build' if not r['before_build'] else 'conform')} | "
                f"{a.get('built')} | {a.get('conformant')}/{a.get('traces')} | {a.get('kind')} |")
    lines = ["# gen-code-replay-v2 — v3-code cells on the round-q toolchain", "",
             f"Batch `{Path(args.batch).name}`; cells replayed: {len(records)}. 0 LLM requests.", "",
             f"- freeze-6 failures: **{len(fails)}**",
             f"  - fixed by build/P-1: **{len(fixed_build)}**",
             f"  - fixed by conform/P-2: **{len(fixed_conform)}**",
             f"  - still failing: **{len(still)}** {dict(kinds)}",
             f"- freeze-6 accepted cells: {len(accepted)}; regressions: **{len(regressions)}**",
             f"- accepted cells with monitor `unmapped`: {unmap_before}/{len(accepted)}",
             "", "| task | before | built | conformant | kind |",
             "| --- | --- | --- | --- | --- |"]
    lines += [line(r) for r in sorted(records, key=lambda x: x["task"])]
    (out / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (out / "RESULTS.json").write_text(json.dumps({"records": records}, indent=2) + "\n",
                                      encoding="utf-8")
    print(f"failures={len(fails)} fixed_build={len(fixed_build)} "
          f"fixed_conform={len(fixed_conform)} still={len(still)} kinds={dict(kinds)} "
          f"regressions={len(regressions)} unmapped={unmap_before}/{len(accepted)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
