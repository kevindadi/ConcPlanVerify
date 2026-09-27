#!/usr/bin/env python3
"""Common external evaluation of G2 and G3 under one frozen evaluator.

Metrics are kept separate and never mixed:

- A ``delivered``  — utility of each arm's *accepted* artifact; an unaccepted
  cell scores 0 over the full task set.
- B ``accepted_quality`` — RF within the accepted set only (missing artifacts
  are reported, not silently dropped).
- C ``final_candidate`` — quality of each arm's *last* candidate (including
  internally rejected ones). Independent diagnostic; never called RF_acc.

Both arms' Rust is scored by the same ``rust_oracle.evaluate`` (same contract,
same 32-run budget, same checker/instrumenter binaries). G3's own CIR/conform
evidence is recorded separately and never folded into the code score.
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

from cir_workflow import rust_oracle  # noqa: E402
from cir_workflow.generation import load_gen_tasks  # noqa: E402


def _sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def _git_fingerprint(repo: Path) -> dict:
    def run(*a):
        try:
            return subprocess.run(["git", "-C", str(repo), *a], capture_output=True,
                                  text=True).stdout.strip()
        except Exception:
            return ""
    rev = run("rev-parse", "HEAD")
    diff = run("diff", "HEAD")
    return {"git_rev": rev, "worktree_diff_sha256": hashlib.sha256(diff.encode()).hexdigest(),
            "worktree_dirty": bool(diff)}


def _g2_candidates(cell_dir: Path) -> dict[str, Path | None]:
    cj = cell_dir / "CELL.json"
    if not cj.is_file():
        return {"accepted": None, "final": None}
    rec = json.loads(cj.read_text())
    accepted = None
    if rec.get("accepted") and rec.get("final_artifact_path"):
        p = Path(rec["final_artifact_path"])
        accepted = p if p.is_file() else None
    rounds = sorted(cell_dir.glob("run-*/round-*/candidate.rs"),
                    key=lambda p: int(p.parent.name.split("-")[1]))
    final = rounds[-1] if rounds else None
    return {"accepted": accepted, "final": final}


def _g3_candidates(cell_dir: Path, row: dict) -> dict[str, Path | None]:
    accepted = None
    acc = row.get("first_code_accept_round")
    if row.get("accepted") and acc:
        p = cell_dir / f"code/round-{acc}.rs"
        accepted = p if p.is_file() else None
    rusts = sorted(cell_dir.glob("code/round-*.rs"),
                   key=lambda p: int(p.stem.split("-")[1]))
    final = rusts[-1] if rusts else None
    return {"accepted": accepted, "final": final}


def _evaluate(rust: Path, task, work: Path, binary: Path, instrument: Path) -> dict:
    result = rust_oracle.evaluate(
        rust.read_text(encoding="utf-8"), task.contract_path, task.reference_cir_path,
        work, n_native=32, miri_seeds=0, run_timeout=10.0,
        binary=binary, instrument_binary=instrument)
    cov = result.get("coverage")
    status = result.get("status")
    failure = None
    if status == "build_failed":
        failure = "source_build_failed"
    elif not result.get("built", True):
        failure = "source_build_failed"
    elif cov is None:
        failure = "not_evaluable"
    return {"status": status, "failure": failure,
            "coverage": cov, "behavior_ok": result.get("behavior_ok"),
            "hang": result.get("hang"), "monitor_status": (result.get("monitor") or {}).get("status"),
            "limitations": result.get("limitations")}


def _record_candidate(out: Path, model: str, arm: str, task_id: str, rep: int,
                      kind: str, rust: Path | None, task, binary: Path, instrument: Path,
                      provenance: dict) -> dict:
    cand_dir = out / model / arm / task_id.replace("/", "__") / f"rep{rep}" / kind
    cand_dir.mkdir(parents=True, exist_ok=True)
    row = {"model": model, "arm": arm, "task": task_id, "rep": rep, "kind": kind,
           "rust": None, "rust_sha256": None, "coverage": None, "failure": "no_artifact",
           "provenance": provenance}
    if rust is None or not rust.is_file():
        (cand_dir / "eval.json").write_text(json.dumps(row, indent=2) + "\n")
        return row
    row["rust"] = str(rust); row["rust_sha256"] = _sha(rust)
    work = cand_dir / "work"; work.mkdir(parents=True, exist_ok=True)
    ev = _evaluate(rust, task, work, binary, instrument)
    row.update({"coverage": ev["coverage"], "failure": ev["failure"],
                "behavior_ok": ev["behavior_ok"], "hang": ev["hang"],
                "monitor_status": ev["monitor_status"], "limitations": ev["limitations"]})
    row["provenance"].update({"contract_sha256": _sha(task.contract_path),
                              "reference_cir_sha256": _sha(task.reference_cir_path)})
    (cand_dir / "eval.json").write_text(json.dumps(row, ensure_ascii=False, indent=2) + "\n")
    return row


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pairs", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    parser.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    args = parser.parse_args()
    pairs = json.loads(Path(args.pairs).read_text())
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    binary, instrument = Path(args.binary), Path(args.instrument)
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    concir_fp = _git_fingerprint(REPO.parent / "ConcIR")
    concpy_fp = _git_fingerprint(REPO)
    manifest = {"binary_sha256": _sha(binary), "instrument_sha256": _sha(instrument),
                "concir": concir_fp, "concpy": concpy_fp,
                "evaluator": "rust_oracle.evaluate n_native=32 miri_seeds=0 timeout=10",
                "pairs": pairs}
    (out / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")

    all_rows = []
    for model, b in pairs.items():
        g2dir, g3dir = REPO / b["g2"], REPO / b["g3"]
        g2 = {c["task"]: c for c in json.loads((g2dir / "SUMMARY.json").read_text())["cells"]}
        g3 = {c["task"]: c for c in json.loads((g3dir / "SUMMARY.json").read_text())["cells"]}
        for t in sorted(set(g2) & set(g3)):
            task = tasks[t]
            for arm, src, row, candfn, prov in (
                ("G2", g2dir, g2[t], _g2_candidates,
                 {"batch": b["g2"], "accepted": g2[t].get("accepted"),
                  "accepted_round": g2[t].get("first_pass_round")}),
                ("G3", g3dir, g3[t], _g3_candidates,
                 {"batch": b["g3"], "accepted": g3[t].get("accepted"),
                  "accepted_round": g3[t].get("first_code_accept_round"),
                  "cir_accepted": g3[t].get("cir_accepted"),
                  "failure_stage": g3[t].get("failure_stage")})):
                cdir = src / model / t.replace("/", "__") / f"rep{row.get('replicate',0)}"
                cands = candfn(cdir, row) if arm == "G3" else candfn(cdir)
                for kind in ("accepted", "final"):
                    all_rows.append(_record_candidate(
                        out, model, arm, t, row.get("replicate", 0), kind,
                        cands.get(kind), task, binary, instrument, dict(prov)))

    # metrics
    def rf(cov):
        return (cov or {}).get("rf")
    report = {}
    for model in pairs:
        mrows = [r for r in all_rows if r["model"] == model]
        n_tasks = len({r["task"] for r in mrows})
        per = {}
        for arm in ("G2", "G3"):
            acc = [r for r in mrows if r["arm"] == arm and r["kind"] == "accepted"]
            fin = [r for r in mrows if r["arm"] == arm and r["kind"] == "final"]
            delivered_rf = sum((rf(r["coverage"]) or 0.0) if r["coverage"] else 0.0
                               for r in acc) / n_tasks
            evaluable = [r for r in acc if r["coverage"]]
            accepted_quality = (sum(rf(r["coverage"]) for r in evaluable) / len(evaluable)
                                if evaluable else None)
            final_rf = sum((rf(r["coverage"]) or 0.0) if r["coverage"] else 0.0
                           for r in fin) / n_tasks
            missing = [r["task"] for r in acc if not r["coverage"]]
            per[arm] = {"tasks": n_tasks,
                        "delivered_rf_all": round(delivered_rf, 4),
                        "accepted_quality_rf": round(accepted_quality, 4) if accepted_quality is not None else None,
                        "accepted_evaluable": len(evaluable),
                        "accepted_missing": len(missing), "missing_tasks": missing,
                        "final_candidate_rf_all": round(final_rf, 4)}
        report[model] = per
        p = per
        print(f"{model:14s} | G2 del={p['G2']['delivered_rf_all']} q={p['G2']['accepted_quality_rf']} "
              f"fin={p['G2']['final_candidate_rf_all']} | G3 del={p['G3']['delivered_rf_all']} "
              f"q={p['G3']['accepted_quality_rf']} fin={p['G3']['final_candidate_rf_all']}")
    (out / "COMMON_EVAL_V2.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
