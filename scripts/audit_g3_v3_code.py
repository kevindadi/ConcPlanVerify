#!/usr/bin/env python3
"""Offline audit of g3-v3 Rust. Does not modify those records."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
os.environ.pop("CARGO_TARGET_DIR", None)

from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import (  # noqa: E402
    _conform_all_op_resource, _rewrite_traces, mapping_to_cir,
)
from cir_workflow.project_template import cargo_toml  # noqa: E402

SRC = REPO / "experiments/evidence-20260925/g3-v3"
OUT = REPO / "experiments/evidence-20260925/g3-code-v4/audit"
BACKEND = Path(os.environ.get("CONCIR_BACKEND", REPO.parent / "ConcIR/target/release/concir-backend"))


def compile_source(source: str, dest: Path) -> tuple[bool, str]:
    dest.mkdir(parents=True, exist_ok=True)
    (dest / "src").mkdir(exist_ok=True)
    (dest / "Cargo.toml").write_text(cargo_toml("probe"), encoding="utf-8")
    (dest / "src" / "main.rs").write_text(source, encoding="utf-8")
    return rust_oracle.cargo_build(dest)


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    for model_dir in sorted(p for p in SRC.iterdir() if p.is_dir()):
        cells = model_dir / "cells"
        if not cells.is_dir():
            continue
        for rec_path in sorted(cells.glob("*/RECORD.json")):
            rec = json.loads(rec_path.read_text(encoding="utf-8"))
            code = rec_path.parent / "code"
            rusts = sorted(code.glob("round-*.rs"))
            if not rusts:
                continue
            rust_path = rusts[-1]
            if rec.get("accepted") and rec.get("final_rust"):
                final = Path(str(rec["final_rust"]))
                # final_rust may point at this tree; prefer the last round file we have
                if final.is_file():
                    rust_path = final
            source = rust_path.read_text(encoding="utf-8")
            work = OUT / "work" / model_dir.name / rec_path.parent.name
            row = {
                "model": model_dir.name, "task": rec.get("task"),
                "rust": str(rust_path),
                "old_accepted": bool(rec.get("accepted")),
                "source_build": None, "instrument": None, "instrumented_build": None,
                "scope_limited": False, "ambiguous": None, "conform": None,
                "cause": "pending",
            }
            ok, log = compile_source(source, work / "source")
            row["source_build"] = ok
            if not ok:
                row["cause"] = "source_error"
                rows.append(row)
                print(row["model"], row["task"], row["cause"], flush=True)
                continue
            try:
                wrapped = rust_oracle.instrument_wrappers(source, work / "instrument")
                row["instrument"] = "ok"
                row["limitations"] = wrapped["limitations"]
                row["scope_limited"] = any("thread::scope" in x for x in wrapped["limitations"])
            except Exception as exc:  # noqa: BLE001
                row["instrument"] = "error"
                row["cause"] = "instrument_error"
                row["detail"] = str(exc)[:300]
                rows.append(row)
                print(row["model"], row["task"], row["cause"], flush=True)
                continue
            rust_oracle.prepare_project(work / "proj", wrapped["annotated"], wrapped["runtime"])
            built, blog = rust_oracle.cargo_build(work / "proj")
            row["instrumented_build"] = built
            if not built:
                row["cause"] = "instrument_introduced_error" if ok else "source_error"
                row["detail"] = "\n".join(ln for ln in blog.splitlines() if ln.startswith("error"))[:500]
                rows.append(row)
                print(row["model"], row["task"], row["cause"], flush=True)
                continue
            if row["scope_limited"]:
                row["cause"] = "instrument_unsupported_scope"
                rows.append(row)
                print(row["model"], row["task"], row["cause"], flush=True)
                continue
            cir = rec.get("cir_path") or (rec.get("code_stage") or {}).get("cir_path")
            if not cir or not Path(cir).is_file():
                row["cause"] = "no_cir"
                rows.append(row)
                continue
            mapping, prov = mapping_to_cir(wrapped["resources"], json.loads(Path(cir).read_text()))
            row["ambiguous"] = prov.get("ambiguous")
            if prov.get("ambiguous"):
                row["cause"] = "mapping_ambiguous"
                rows.append(row)
                print(row["model"], row["task"], row["cause"], flush=True)
                continue
            traces = work / "traces"
            rust_oracle.run_native(work / "proj", traces, n=4, timeout=8.0)
            conform_dir = work / "conform"
            from cir_workflow.generation import _DROP_OPS
            wrappers = {r["name"] for r in wrapped["resources"] if r.get("kind") == "ChannelWrapper"}
            _rewrite_traces(traces, conform_dir, mapping, _DROP_OPS, wrappers)
            conform = _conform_all_op_resource(BACKEND, Path(cir), conform_dir)
            row["conform"] = {"conformant": conform["conformant"], "traces": conform["traces"],
                              "statuses": conform["statuses"]}
            if conform["traces"] and conform["conformant"] == conform["traces"]:
                row["cause"] = "conformant_under_new_mapping"
            elif any(k == "violation" for k in (conform["statuses"] or {})):
                row["cause"] = "real_conformance_violation"
            else:
                row["cause"] = "undetermined"
            rows.append(row)
            print(row["model"], row["task"], row["cause"], flush=True)
            (OUT / "AUDIT.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    (OUT / "AUDIT.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    print("wrote", len(rows))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
