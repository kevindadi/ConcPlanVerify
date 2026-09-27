#!/usr/bin/env python3
"""Real offline re-execution and identity binding for frozen Rust candidates.

Chain (all outputs in a fresh run directory; source cells are read-only):

    frozen Rust
      -> source build (original)              [stages.source_build]
      -> concir-instrument --wrappers         [instrument/]
      -> instrumented build                   [stages.instrumented_build]
      -> execute N times, save raw traces     [traces/]
      -> identity binding                     [binding.json]
      -> projected traces for op-resource     [conform-traces/]
      -> conform + monitor
      -> per-sample result                    [result.json]

The new resources.json is never mixed with old event namespaces: traces come
from executing the newly built program. Thread lifecycle (spawn/join/scope) is
recorded separately because op-resource conformance does not cover it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import bounded_monitor, generation, rust_oracle  # noqa: E402
from cir_workflow.evidence import load_cir_properties  # noqa: E402
from cir_workflow.evidence_v2 import evaluate_reexecution  # noqa: E402


def _sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def _count_events(directory: Path) -> int:
    total = 0
    for f in sorted(directory.glob("*.jsonl")):
        total += len([ln for ln in f.read_text(encoding="utf-8").splitlines() if ln.strip()])
    return total


def _count_ops(directory: Path, ops: set[str]) -> int:
    total = 0
    for f in sorted(directory.glob("*.jsonl")):
        for ln in f.read_text(encoding="utf-8").splitlines():
            if not ln.strip():
                continue
            try:
                if json.loads(ln).get("op") in ops:
                    total += 1
            except json.JSONDecodeError:
                pass
    return total


def _artifacts(out: Path, result: dict) -> list[dict]:
    """A role-tagged, verifiable manifest of every referenced artifact."""

    role_paths: list[tuple[str, str | None]] = [
        ("source", result.get("source_path")),
        ("cir", result.get("cir_path")),
        ("contract", result.get("contract_path")),
        ("model_check", str(out / "model-check.json")),
        ("binding", str(out / "binding.json")),
        ("monitor", str(out / "monitor.json")),
    ]
    artifacts = []
    for role, p in role_paths:
        if p and Path(p).is_file():
            artifacts.append({"role": role, "path": p, "sha256": _sha(Path(p))})
    for d, role in (("traces", "execution"), ("conform-traces", "conform")):
        if (out / d).is_dir():
            for p in sorted((out / d).glob("*.jsonl")):
                artifacts.append({"role": role, "path": str(p), "sha256": _sha(p)})
    f = result.get("functional") or {}
    if f.get("evidence_path") and Path(f["evidence_path"]).is_file():
        artifacts.append({"role": "functional", "path": f["evidence_path"],
                          "sha256": _sha(Path(f["evidence_path"]))})
    return artifacts


def _finalize(out: Path, result: dict, contract_path: Path, accepted: bool,
              cir_props: dict | None, cir_complete: bool | None) -> dict:
    """Every exit path (success or any failure) writes a result with a ledger."""

    try:
        contract = json.loads(Path(contract_path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        contract = {"properties": [], "preserved": []}
    result["artifacts"] = _artifacts(out, result)
    ledger = evaluate_reexecution(result, contract, accepted=accepted,
                                  cir_props=cir_props or {}, cir_complete=cir_complete)
    result["ledger"] = ledger.to_dict()
    result["evidence_path"] = str(out / "result.json")
    (out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
    return result


def reexecute(source: str, cir_path: Path, contract_path: Path, out: Path, *,
              binary: Path, instrument: Path, n_runs: int = 32,
              run_timeout: float = 10.0, accepted: bool = True, cell_id: str = '',
              cir_props: dict | None = None,
              cir_complete: bool | None = None,
              candidate_kind: str = 'final', round_no: int | None = None,
              manifest_path: Path | None = None,
              functional: dict | None = None) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    source_path = out / "source.rs"
    source_path.write_text(source, encoding="utf-8")
    result: dict = {
        "cell": cell_id,
        "source_path": str(source_path), "cir_path": str(cir_path),
        "contract_path": str(contract_path),
        "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
        "cir_sha256": _sha(cir_path), "contract_sha256": _sha(contract_path),
        "backend_sha256": _sha(binary), "instrument_sha256": _sha(instrument),
        "n_runs": n_runs, "stages": {}, "limitations": [],
        "candidate_kind": candidate_kind, "round_no": round_no,
        "cir_props": cir_props or {}, "cir_complete": cir_complete,
        "functional": functional,
    }
    (out / "model-check.json").write_text(json.dumps(
        {"cir_props": cir_props or {}, "cir_complete": cir_complete,
         "cir_path": str(cir_path), "cir_sha256": _sha(cir_path)}, indent=2) + "\n",
        encoding="utf-8")
    # 1. source build (original program, before instrumentation)
    src_proj = out / "source-proj"
    (src_proj / "src").mkdir(parents=True, exist_ok=True)
    (src_proj / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (src_proj / "src" / "main.rs").write_text(source, encoding="utf-8")
    src_ok, src_log = rust_oracle.cargo_build(src_proj)
    (out / "source-build.log").write_text(src_log, encoding="utf-8")
    result["stages"]["source_build"] = "ok" if src_ok else "failed"
    if not src_ok:
        return _finalize(out, result, contract_path, accepted, cir_props, cir_complete)

    # 2. instrument
    try:
        wrapped = rust_oracle.instrument_wrappers(source, out / "instrument",
                                                  binary=instrument)
    except Exception as exc:  # noqa: BLE001
        result["stages"]["instrument"] = f"error: {exc}"
        return _finalize(out, result, contract_path, accepted, cir_props, cir_complete)
    result["stages"]["instrument"] = "ok"
    result["limitations"] = wrapped["limitations"]
    result["resources"] = wrapped["resources"]

    # 3. instrumented build
    proj = out / "proj"
    rust_oracle.prepare_project(proj, wrapped["annotated"], wrapped["runtime"])
    built, build_log = rust_oracle.cargo_build(proj)
    (out / "instrumented-build.log").write_text(build_log, encoding="utf-8")
    result["stages"]["instrumented_build"] = "ok" if built else "failed"
    if not built:
        return _finalize(out, result, contract_path, accepted, cir_props, cir_complete)

    # 4. execute; raw traces
    runs = rust_oracle.run_native(proj, out / "traces", n=n_runs, timeout=run_timeout)
    result["runs_started"] = len(runs)
    result["runs_completed"] = sum(1 for r in runs if r["completed"])
    result["hang"] = any(r["timed_out"] for r in runs)
    result["raw_events"] = _count_events(out / "traces")
    result["thread_lifecycle"] = {
        "spawn": _count_ops(out / "traces", {"spawn"}),
        "join": _count_ops(out / "traces", {"join"}),
        "scope": _count_ops(out / "traces", {"scope"}),
    }

    # 5. binding via the ConcIR bind_check CLI (falls back to the Python
    #    structural mapping only if the Rust binary is unavailable).
    cir = json.loads(cir_path.read_text(encoding="utf-8"))
    from cir_workflow.binding import BindingUnavailable, bind as binding_bind
    try:
        rb = binding_bind(out / "instrument/resources.json", cir_path,
                          manifest_path=manifest_path)
    except BindingUnavailable as exc:
        # No silent fallback to a weaker name mapping.
        result["stages"]["binding"] = f"tool_error: {exc}"
        result["binding"] = {"mapping": {}, "ambiguous": [], "violated": {},
                             "source": "rust-cli-unavailable", "error": str(exc)}
        return _finalize(out, result, contract_path, accepted, cir_props, cir_complete)
    mapping = {name: v["cir"] for name, v in rb.get("verified", {}).items()}
    ambiguous = [{"rust": k, **v} for k, v in rb.get("unresolved", {}).items()]
    result["stages"]["binding"] = "ok"
    result["binding"] = {"mapping": mapping, "rules": {},
                         "ambiguous": ambiguous,
                         "violated": rb.get("violated", {}),
                         "source": "rust-cli"}
    (out / "binding.json").write_text(
        json.dumps({"mapping": mapping, "provenance": result["binding"]}, indent=2) + "\n",
        encoding="utf-8")

    # 6. projected traces for op-resource conformance (spawn/join/scope excluded
    #    from this check; they are a separate thread-lifecycle obligation).
    wrapper_names = {r["name"] for r in wrapped["resources"]
                     if r.get("kind") == "ChannelWrapper"}
    generation._rewrite_traces(out / "traces", out / "conform-traces", mapping,
                               generation._DROP_OPS, wrapper_names)
    result["projected_events"] = _count_events(out / "conform-traces")
    conform = generation._conform_all_op_resource(binary, cir_path, out / "conform-traces")
    result["conform"] = conform

    # 7. monitor on raw (wrapper-filtered) traces with the binding
    generation._rewrite_traces(out / "traces", out / "monitor-traces", {}, set(),
                               wrapper_names)
    report = bounded_monitor.run_monitor(contract_path, out / "monitor-traces",
                                         resources=out / "instrument/resources.json",
                                         mapping=out / "binding.json", binary=binary)
    (out / "monitor.json").write_text(json.dumps(report) + "\n", encoding="utf-8")
    result["monitor"] = {"status": report.get("status"),
                         "properties": [(p.get("id"), p.get("status"))
                                        for p in report.get("properties", [])]}
    return _finalize(out, result, contract_path, accepted, cir_props, cir_complete)


def _last_rust(cell: Path) -> Path | None:
    rusts = sorted(cell.glob("code/round-*.rs"),
                   key=lambda p: int(p.stem.split("-")[1]))
    return rusts[-1] if rusts else None


def _accepted_cir(cell: Path) -> Path | None:
    revs = sorted(cell.glob("cir/revision-*.cir.json"),
                  key=lambda p: int(p.stem.split("-")[1].split(".")[0]))
    return revs[-1] if revs else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--task", action="append", default=[],
                        help="restrict to these task ids")
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    parser.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    parser.add_argument("--n-runs", type=int, default=32)
    args = parser.parse_args()
    batch = Path(args.batch)
    if not batch.is_absolute():
        batch = REPO / batch
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    binary, instrument = Path(args.binary), Path(args.instrument)
    batch_summary = json.loads((batch / "SUMMARY.json").read_text())["cells"]
    accepted_map = {(c["model"], c["task"], c.get("replicate", 0)): bool(c.get("accepted"))
                    for c in batch_summary}
    acc_round_map = {(c["model"], c["task"], c.get("replicate", 0)):
                     c.get("first_code_accept_round") for c in batch_summary}
    summary = []
    for cell in sorted(batch.glob("*/*/rep*")):
        if not (cell / "code").is_dir():
            continue
        if args.task and not any(t.replace("/", "__") in str(cell) for t in args.task):
            continue
        rust = _last_rust(cell)
        cir = _accepted_cir(cell)
        if rust is None or cir is None:
            continue
        rel = cell.relative_to(batch)
        model = rel.parts[0]
        task_id = rel.parts[1].replace("__", "/")
        rep = int(rel.parts[2].replace("rep", ""))
        contract = REPO / "benchmarks/families" / task_id / "contract.json"
        cir_props, cir_complete = load_cir_properties(cell, cir)
        run_dir = out / rel
        accepted = accepted_map.get((model, task_id, rep), False)
        acc_round = acc_round_map.get((model, task_id, rep))
        last_round = int(rust.stem.split("-")[1])
        kind = "accepted" if (accepted and acc_round == last_round) else "final"
        try:
            res = reexecute(rust.read_text(encoding="utf-8"), cir, contract, run_dir,
                            binary=binary, instrument=instrument, n_runs=args.n_runs,
                            accepted=accepted, cir_props=cir_props,
                            cir_complete=cir_complete, cell_id=str(rel),
                            candidate_kind=kind, round_no=last_round)
        except Exception as exc:  # noqa: BLE001
            run_dir.mkdir(parents=True, exist_ok=True)
            res = {"cell": str(rel), "candidate_kind": kind, "round_no": last_round,
                   "stages": {"unexpected": f"{type(exc).__name__}: {exc}"},
                   "evidence_path": str(run_dir / "result.json"),
                   "ledger": {"current_evaluation": "tool_error"}}
            (run_dir / "result.json").write_text(json.dumps(res, indent=2) + "\n")
        summary.append(res)
        print(f"  {res['cell'][:60]:60s} stages={res['stages']} "
              f"raw={res.get('raw_events')} proj={res.get('projected_events')} "
              f"conform={res.get('conform',{}).get('statuses')}")
    (out / "SUMMARY.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
                                      encoding="utf-8")
    print(f"wrote {out} ({len(summary)} samples)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
