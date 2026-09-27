#!/usr/bin/env python3
"""End-to-end closed-loop cases (strong-link-v4).

Each case runs the real production chain on a persisted fixture:
source build -> instrument -> build -> execute -> binding (Rust CLI) ->
conform/monitor -> ledger. Group 1 checks an explicit synchronization-order
deviation; Group 2 checks that identical synchronization with a wrong
computation is caught only by an independent functional test. Truth is
established from the source transformation / requirement, never from the
checker under test.
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


def _run_source(source: str, work: Path, timeout: float = 20.0) -> dict:
    """Build and run an original program; capture stdout/exit."""

    work.mkdir(parents=True, exist_ok=True)
    (work / "src").mkdir(exist_ok=True)
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    built, log = rust_oracle.cargo_build(work)
    if not built:
        return {"built": False, "stdout": None, "returncode": None}
    proc = subprocess.run([str(work / "target/debug/probe")], capture_output=True,
                          text=True, timeout=timeout)
    return {"built": True, "stdout": proc.stdout.strip(), "returncode": proc.returncode}


def _reexec(name: str, source: str, cir: str, contract: str, out: Path,
            manifest: Path | None = None) -> dict:
    res = reexecute(source, FIX / cir, FIX / contract, out / name,
                    binary=BIN, instrument=INSTR, accepted=True, cell_id=name,
                    manifest_path=manifest)
    return {"case": name, "ledger": res.get("ledger"), "stages": res.get("stages"),
            "trace": res.get("ledger", {}).get("trace"),
            "binding": res.get("binding"), "run": res.get("ledger", {}).get("run")}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    out = Path(args.out); out.mkdir(parents=True, exist_ok=True)
    results = []

    # ---- Group 1: explicit synchronization-order deviation -----------------
    order_cir = (FIX / "order.cir.json").read_text()
    ok = (FIX / "order_ok.rs").read_text()
    swap = (FIX / "order_swap.rs").read_text()
    r_ok = _reexec("order_ok", ok, "order.cir.json", "order.contract.json", out)
    r_swap = _reexec("order_swap", swap, "order.cir.json", "order.contract.json", out)
    results.append({
        "group": 1, "case": "sync_order",
        "design_deviation": "yes (lock order a->b exchanged to b->a in w; same compile, "
                            "same events set)",
        "truth_basis": "source transformation + CIR statement order (a then b)",
        "order_ok": {"conform": r_ok["trace"], "run": r_ok["run"]},
        "order_swap": {"conform": r_swap["trace"], "run": r_swap["run"]},
        "expected": "order_ok observed_conformant; order_swap observed_violation (order)",
    })

    # ---- Group 2: identical synchronization, wrong computation -------------
    comp_cir = "compute.cir.json"
    ok_c = (FIX / "compute_ok.rs").read_text()
    wrong_c = (FIX / "compute_wrong.rs").read_text()
    f_ok = _run_source(ok_c, out / "func-compute_ok")
    f_wrong = _run_source(wrong_c, out / "func-compute_wrong")
    r_cok = _reexec("compute_ok", ok_c, comp_cir, "compute.contract.json", out)
    r_cwrong = _reexec("compute_wrong", wrong_c, comp_cir, "compute.contract.json", out)
    expected_out = "DONE done=6"
    results.append({
        "group": 2, "case": "compute_wrong",
        "design_deviation": "no (synchronization identical)",
        "requirement_violation": "yes (writes 5, not 6)",
        "truth_basis": "independent functional test: run the built program and compare "
                       "stdout to the requirement's expected line",
        "compute_ok": {"stdout": f_ok["stdout"], "functional_pass": f_ok["stdout"] == expected_out,
                       "conform": r_cok["trace"]},
        "compute_wrong": {"stdout": f_wrong["stdout"],
                          "functional_pass": f_wrong["stdout"] == expected_out,
                          "conform": r_cwrong["trace"]},
        "expected": "both conform observed_conformant; only the functional test "
                    "distinguishes them (sync-conformant != requirement satisfied)",
    })

    # ---- Regressions -------------------------------------------------------
    # pure local rename: rename a local variable (not a resource binding)
    renamed = ok.replace("let ga = a.lock()", "let g1 = a.lock()").replace(
        "let gb = b.lock()", "let g2 = b.lock()").replace(
        "drop(gb);", "drop(g2);").replace("drop(ga);", "drop(g1);")
    r_ren = _reexec("order_renamed", renamed, "order.cir.json", "order.contract.json", out)
    results.append({"group": "reg", "case": "pure_local_rename",
                    "same_identity": r_ren["binding"].get("mapping") == r_ok["binding"].get("mapping"),
                    "same_verdict": r_ren["ledger"].get("current_evaluation")
                    == r_ok["ledger"].get("current_evaluation")})

    # legal clone: order_ok already uses Arc::clone; record its binding
    results.append({"group": "reg", "case": "legal_clone",
                    "binding": r_ok["binding"].get("mapping")})

    # wrong manifest
    man = out / "wrong_manifest.json"
    man.write_text('[{"rust": "a_mutex0", "cir": "main::b"}]')
    r_bad = _reexec("order_bad_manifest", ok, "order.cir.json",
                    "order.contract.json", out, manifest=man)
    results.append({"group": "reg", "case": "wrong_manifest",
                    "violated": r_bad["binding"].get("violated"),
                    "verdict": r_bad["ledger"].get("current_evaluation")})

    # two same-field instances
    two = (FIX / "two_instances.rs").read_text() if (FIX / "two_instances.rs").is_file() else None
    if two:
        r_two = _reexec("two_instances", two, "two.cir.json", "order.contract.json", out)
        results.append({"group": "reg", "case": "two_instances",
                        "unresolved": r_two["binding"].get("ambiguous"),
                        "verdict": r_two["ledger"].get("current_evaluation")})

    # empty projection
    empty = (FIX / "no_sync.rs").read_text() if (FIX / "no_sync.rs").is_file() else None
    if empty:
        r_empty = _reexec("no_sync", empty, "order.cir.json", "order.contract.json", out)
        results.append({"group": "reg", "case": "empty_projection",
                        "trace": r_empty["trace"], "verdict": r_empty["ledger"].get("current_evaluation")})

    # checker exception: missing resources file
    proc = subprocess.run([str(REPO.parent / "ConcIR/target/release/bind_check"),
                           "--resources", str(out / "missing.json"), "--cir", str(FIX / "order.cir.json")],
                          capture_output=True, text=True)
    results.append({"group": "reg", "case": "checker_missing_input",
                    "exit": proc.returncode, "stderr": proc.stderr.strip()[:120]})

    (out / "CASES.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps([{k: v for k, v in r.items() if k in ("case", "expected", "verdict")}
                      for r in results], ensure_ascii=False, indent=2))
    print(f"wrote {out}/CASES.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
