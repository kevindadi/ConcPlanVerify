#!/usr/bin/env python3
"""End-to-end closed-loop cases (strong-link-v5), with real assertions.

Each case runs the production chain on a persisted fixture and asserts the
expected final evidence, exiting non-zero on mismatch:

- Group 1 (sync order): order_ok model-verified and sync-conformant;
  order_swap still compiles/runs and its order deviation is located.
- Group 2 (computation): compute_ok sync-conformant + functional pass;
  compute_wrong sync-conformant + functional FAIL, final not just "unknown".

Truth comes from the source transformation / requirement, never from the
checker under test. The CVN explore result is obtained from the real backend,
not hand-filled.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO))

from cir_workflow import rust_oracle  # noqa: E402
from scripts.reexecute_and_bind import reexecute  # type: ignore  # noqa: E402

FIX = REPO / "python/tests/fixtures/stronglink"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INSTR = REPO.parent / "ConcIR/target/release/concir-instrument"


def _explore(cir: Path, contract: Path) -> tuple[dict, bool]:
    proc = subprocess.run([str(BIN), "explore", str(cir), str(contract), "petri"],
                          capture_output=True, text=True, timeout=120)
    d = json.loads(proc.stdout)
    props = {str(p["id"]).replace("preserved: ", ""): p.get("outcome")
             for p in d.get("properties", [])}
    return props, bool(d.get("complete"))


def _functional(source: str, work: Path, expected: str) -> dict:
    work.mkdir(parents=True, exist_ok=True)
    (work / "src").mkdir(exist_ok=True)
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    built, _ = rust_oracle.cargo_build(work)
    stdout = None
    if built:
        proc = subprocess.run([str(work / "target/debug/probe")], capture_output=True,
                              text=True, timeout=20)
        stdout = proc.stdout.strip()
    ev = work / "functional.txt"
    ev.write_text(f"expected={expected!r}\nstdout={stdout!r}\n")
    return {"status": "pass" if stdout == expected else ("fail" if stdout is not None else "not_run"),
            "evidence": f"stdout={stdout!r} expected={expected!r}",
            "evidence_path": str(ev)}


def _reexec(name: str, source: str, cir: str, contract: str, out: Path,
            cir_props: dict, cir_complete: bool, functional: dict | None = None) -> dict:
    return reexecute(source, FIX / cir, FIX / contract, out / name,
                     binary=BIN, instrument=INSTR, accepted=True, cell_id=name,
                     cir_props=cir_props, cir_complete=cir_complete,
                     functional=functional)


def _violation_location(trace: dict) -> str | None:
    for v in trace.get("violations", []):
        return f"event_index={v.get('event_index')} got={v.get('got')}"
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    checks: list[tuple[str, bool, str]] = []
    results = []

    # ---- Group 1 -----------------------------------------------------------
    o_props, o_complete = _explore(FIX / "order.cir.json", FIX / "order.contract.json")
    ok = (FIX / "order_ok.rs").read_text()
    swap = (FIX / "order_swap.rs").read_text()
    r_ok = _reexec("order_ok", ok, "order.cir.json", "order.contract.json", out, o_props, o_complete)
    r_swap = _reexec("order_swap", swap, "order.cir.json", "order.contract.json", out, o_props, o_complete)
    checks.append(("g1_order_ok_conformant",
                   r_ok["ledger"]["trace"]["state"] == "observed_conformant",
                   r_ok["ledger"]["trace"]["state"]))
    checks.append(("g1_order_ok_model_verified", r_ok["ledger"]["model"]["verified"],
                   str(r_ok["ledger"]["model"]["verified"])))
    checks.append(("g1_order_swap_runs", r_swap["ledger"]["run"]["state"] == "completed",
                   r_swap["ledger"]["run"]["state"]))
    checks.append(("g1_order_swap_violation",
                   r_swap["ledger"]["trace"]["state"] == "observed_violation",
                   r_swap["ledger"]["trace"]["state"]))
    results.append({"group": 1, "case": "sync_order",
                    "model_obligation": "w: lock a then lock b (CIR statement order)",
                    "design_deviation": "sync order exchanged (b then a) in w",
                    "order_ok": {"trace": r_ok["ledger"]["trace"]["state"],
                                 "model_verified": r_ok["ledger"]["model"]["verified"]},
                    "order_swap": {"run": r_swap["ledger"]["run"]["state"],
                                   "trace": r_swap["ledger"]["trace"]["state"],
                                   "violation_location": _violation_location(r_swap["ledger"]["trace"])}})

    # ---- Group 2 -----------------------------------------------------------
    c_props, c_complete = _explore(FIX / "compute.cir.json", FIX / "compute.contract.json")
    ok_c = (FIX / "compute_ok.rs").read_text()
    wrong_c = (FIX / "compute_wrong.rs").read_text()
    f_ok = _functional(ok_c, out / "func-compute_ok", "DONE done=6")
    f_wrong = _functional(wrong_c, out / "func-compute_wrong", "DONE done=6")
    r_cok = _reexec("compute_ok", ok_c, "compute.cir.json", "compute.contract.json", out,
                    c_props, c_complete, functional=f_ok)
    r_cwrong = _reexec("compute_wrong", wrong_c, "compute.cir.json", "compute.contract.json", out,
                       c_props, c_complete, functional=f_wrong)
    checks.append(("g2_compute_ok_sync", r_cok["ledger"]["trace"]["state"] == "observed_conformant",
                   r_cok["ledger"]["trace"]["state"]))
    checks.append(("g2_compute_ok_functional_pass", f_ok["status"] == "pass", f_ok["status"]))
    checks.append(("g2_compute_wrong_sync", r_cwrong["ledger"]["trace"]["state"] == "observed_conformant",
                   r_cwrong["ledger"]["trace"]["state"]))
    checks.append(("g2_compute_wrong_functional_fail", f_wrong["status"] == "fail", f_wrong["status"]))
    checks.append(("g2_compute_wrong_final_explicit",
                   r_cwrong["ledger"]["current_evaluation"] == "functional_failure",
                   r_cwrong["ledger"]["current_evaluation"]))
    results.append({"group": 2, "case": "compute_wrong",
                    "dimensions": {"sync": "not deviated (same events)",
                                   "data": "deviated (CIR c:=6, Rust writes 5)",
                                   "external_functional": "deviated (DONE done=5 != 6)"},
                    "truth_basis": "independent functional test (run + compare stdout)",
                    "compute_ok": {"sync": r_cok["ledger"]["trace"]["state"],
                                   "functional": f_ok["status"],
                                   "final": r_cok["ledger"]["current_evaluation"]},
                    "compute_wrong": {"sync": r_cwrong["ledger"]["trace"]["state"],
                                      "functional": f_wrong["status"],
                                      "final": r_cwrong["ledger"]["current_evaluation"]}})

    # ---- Regressions -------------------------------------------------------
    renamed = ok.replace("let ga = a.lock()", "let g1 = a.lock()").replace(
        "let gb = b.lock()", "let g2 = b.lock()").replace(
        "drop(gb);", "drop(g2);").replace("drop(ga);", "drop(g1);")
    r_ren = _reexec("order_renamed", renamed, "order.cir.json", "order.contract.json",
                    out, o_props, o_complete)
    checks.append(("reg_rename_same_verdict",
                   r_ren["ledger"]["current_evaluation"] == r_ok["ledger"]["current_evaluation"],
                   r_ren["ledger"]["current_evaluation"]))
    man = out / "wrong_manifest.json"
    man.write_text('[{"rust": "a_mutex0", "cir": "main::b"}]')
    r_bad = reexecute(ok, FIX / "order.cir.json", FIX / "order.contract.json",
                      out / "order_bad_manifest", binary=BIN, instrument=INSTR,
                      accepted=True, cell_id="order_bad_manifest", cir_props=o_props,
                      cir_complete=o_complete, manifest_path=man)
    checks.append(("reg_wrong_manifest_declaration_error",
                   r_bad["ledger"]["current_evaluation"] == "binding_declaration_error",
                   r_bad["ledger"]["current_evaluation"]))
    proc = subprocess.run([str(REPO.parent / "ConcIR/target/release/bind_check"),
                           "--resources", str(out / "missing.json"),
                           "--cir", str(FIX / "order.cir.json")], capture_output=True, text=True)
    checks.append(("reg_checker_input_error", proc.returncode == 2, str(proc.returncode)))

    (out / "CASES.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    (out / "ASSERTIONS.json").write_text(json.dumps(
        [{"name": n, "pass": p, "detail": d} for n, p, d in checks],
        ensure_ascii=False, indent=2) + "\n")
    failed = [n for n, p, _ in checks if not p]
    for n, p, d in checks:
        print(f"  [{'PASS' if p else 'FAIL'}] {n}: {d}")
    print(f"{len(checks) - len(failed)}/{len(checks)} assertions passed")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
