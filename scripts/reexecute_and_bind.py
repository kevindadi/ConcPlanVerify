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

from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.evidence import load_cir_properties  # noqa: E402


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


def reexecute(source: str, cir_path: Path, contract_path: Path, out: Path, *,
              binary: Path, instrument: Path, n_runs: int = 32,
              run_timeout: float = 10.0, accepted: bool = True, cell_id: str = '',
              cir_props: dict | None = None,
              cir_complete: bool | None = None,
              candidate_kind: str = 'final', round_no: int | None = None,
              manifest_path: Path | None = None,
              functional: dict | None = None,
              functional_spec: dict | None = None) -> dict:
    """Offline selection entry. Evidence acquisition is ``evaluate_candidate``."""

    return evaluate_candidate(
        source, cir_path, contract_path, out, binary=binary, instrument=instrument,
        n_runs=n_runs, run_timeout=run_timeout, accepted=accepted, cell_id=cell_id,
        cir_props=cir_props, cir_complete=cir_complete, candidate_kind=candidate_kind,
        round_no=round_no, manifest_path=manifest_path, functional=functional,
        functional_spec=functional_spec)


def _last_rust(cell: Path) -> Path | None:
    rusts = sorted(cell.glob("code/round-*.rs"),
                   key=lambda p: int(p.stem.split("-")[1]))
    return rusts[-1] if rusts else None


def _round_rust(cell: Path, round_no) -> Path | None:
    if round_no is None:
        return None
    path = cell / "code" / f"round-{round_no}.rs"
    return path if path.is_file() else None


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
    # Iterate over EVERY expected cell (the full matrix), not only those that
    # reached the code stage: a missing candidate is a recorded terminal state,
    # never silently dropped from the denominator.
    rows = [c for c in batch_summary
            if not args.task or c["task"] in args.task]
    for row in rows:
        model, task_id, rep = row["model"], row["task"], row.get("replicate", 0)
        rel = Path(model) / task_id.replace("/", "__") / f"rep{rep}"
        cell = batch / rel
        final_rust = _last_rust(cell)
        cir = _accepted_cir(cell)
        contract = REPO / "benchmarks/families" / task_id / "contract.json"
        accepted = bool(row.get("accepted"))
        acc_round = row.get("first_code_accept_round")
        accepted_rust = _round_rust(cell, acc_round) if accepted else None
        rust = accepted_rust or final_rust
        run_dir = out / rel
        if rust is None or cir is None:
            # Classify why no candidate exists.
            if str(row.get("status", "")).startswith("error"):
                reason = "cir_unknown_or_raw_error"
            elif not row.get("cir_accepted"):
                reason = "cir_not_accepted"
            else:
                reason = "cir_accepted_no_rust"
            run_dir.mkdir(parents=True, exist_ok=True)
            res = {"cell": str(rel), "candidate_kind": "final", "round_no": None,
                   "stages": {"pre": reason},
                   "ledger": {"cell": str(rel), "historical_acceptance": accepted,
                              "current_evaluation": reason, "run": {"state": "not_run"},
                              "trace": {"state": "incomplete"}, "reasons": [reason]},
                   "evidence_path": str(run_dir / "result.json")}
            (run_dir / "result.json").write_text(json.dumps(res, ensure_ascii=False, indent=2) + "\n")
            summary.append(res)
            print(f"  {str(rel)[:60]:60s} {reason}")
            continue
        cir_props, cir_complete = load_cir_properties(cell, cir)
        final_round = int(final_rust.stem.split("-")[1]) if final_rust else None
        eval_round = int(rust.stem.split("-")[1])
        same = bool(final_rust and rust.resolve() == final_rust.resolve())
        kind = "accepted" if accepted and accepted_rust is not None else "final"
        try:
            res = reexecute(rust.read_text(encoding="utf-8"), cir, contract, run_dir,
                            binary=binary, instrument=instrument, n_runs=args.n_runs,
                            accepted=accepted, cir_props=cir_props,
                            cir_complete=cir_complete, cell_id=str(rel),
                            candidate_kind=kind, round_no=eval_round)
            res["accepted_round"] = acc_round if accepted else None
            res["final_round"] = final_round
            res["same_round"] = same
            if accepted and accepted_rust and final_rust and not same:
                final_res = reexecute(final_rust.read_text(encoding="utf-8"), cir, contract,
                                      run_dir / "final-only", binary=binary,
                                      instrument=instrument, n_runs=args.n_runs,
                                      accepted=False, cir_props=cir_props,
                                      cir_complete=cir_complete, cell_id=str(rel),
                                      candidate_kind="final", round_no=final_round)
                res["final_evidence_path"] = final_res["evidence_path"]
                res["final_evaluation"] = final_res["ledger"]["current_evaluation"]
            else:
                res["final_evidence_path"] = res["evidence_path"]
                res["final_evaluation"] = res["ledger"]["current_evaluation"]
            (run_dir / "result.json").write_text(
                json.dumps(res, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        except Exception as exc:  # noqa: BLE001
            run_dir.mkdir(parents=True, exist_ok=True)
            res = {"cell": str(rel), "candidate_kind": kind, "round_no": eval_round,
                   "accepted_round": acc_round if accepted else None,
                   "final_round": final_round, "same_round": same,
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
