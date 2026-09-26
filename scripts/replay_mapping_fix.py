#!/usr/bin/env python3
"""Phase A: offline replay of the resource-mapping fix on frozen samples.

No model calls, no source edits. For each cell it recomputes the runtime->CIR
mapping from the saved instrument resources, rewrites the saved native traces,
and re-runs operation-bound conformance. Reports before/after per cell.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import generation  # noqa: E402


def _accepted_cir(cell: Path) -> Path | None:
    revs = sorted(cell.glob("cir/revision-*.cir.json"),
                  key=lambda p: int(p.stem.split("-")[1].split(".")[0]))
    return revs[-1] if revs else None


def _rounds(cell: Path) -> list[Path]:
    return sorted((p for p in cell.glob("code/round-*") if p.is_dir()),
                  key=lambda p: int(p.name.split("-")[1]))


def replay(cell: Path, binary: Path) -> dict:
    cir_path = _accepted_cir(cell)
    if not cir_path:
        return {"cell": cell.name, "error": "no cir"}
    rounds = _rounds(cell)
    if not rounds:
        return {"cell": cell.name, "error": "no code round"}
    rnd = rounds[-1]
    res_path = rnd / "instrument" / "resources.json"
    traces = rnd / "traces"
    if not res_path.is_file() or not traces.is_dir():
        return {"cell": cell.name, "error": "no instrument/traces"}
    resources = json.loads(res_path.read_text())["resources"]
    cir = json.loads(cir_path.read_text())
    mapping, prov = generation.mapping_to_cir(resources, cir)
    out = rnd / "conform-traces-replay"
    wrapper_names = {r["name"] for r in resources if r.get("kind") == "ChannelWrapper"}
    generation._rewrite_traces(traces, out, mapping, generation._DROP_OPS, wrapper_names)
    conform = generation._conform_all_op_resource(binary, cir_path, out)
    return {"cell": str(cell.relative_to(cell.parents[3])),
            "rules": prov.get("rules"),
            "ambiguous": prov.get("ambiguous"),
            "mapping": mapping,
            "conform_statuses": conform["statuses"],
            "conformant": conform["conformant"],
            "traces": conform["traces"]}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch", required=True)
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    batch = Path(args.batch)
    if not batch.is_absolute():
        batch = REPO / batch
    binary = Path(args.binary)
    results = []
    for cell in sorted(batch.glob("*/*/rep*")):
        if not (cell / "code").is_dir():
            continue
        results.append(replay(cell, binary))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "MAPPING_REPLAY.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    fixed = sum(1 for r in results if r.get("conformant") and not r.get("ambiguous"))
    print(f"cells={len(results)} mapping-fixed-conformant={fixed}")
    for r in results:
        if r.get("ambiguous"):
            print("  still ambiguous:", r["cell"], r["ambiguous"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
