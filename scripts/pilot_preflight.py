#!/usr/bin/env python3
"""Real backend preflight for the six pilot CIRs. No model calls."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.pilot_cases import CASES, FIXTURE_ROOT, evaluate_role, load_case  # noqa: E402

BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = REPO.parent / "ConcIR/target/release/concir-instrument"


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _run(argv: list[str]) -> dict:
    proc = subprocess.run(argv, capture_output=True, text=True, timeout=120)
    return {"argv": argv, "returncode": proc.returncode, "stdout": proc.stdout,
            "stderr": proc.stderr}


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else FIXTURE_ROOT / "preflight"
    out.mkdir(parents=True, exist_ok=True)
    oracle_py = REPO / "python/cir_workflow/pilot_oracle.py"
    cases_py = REPO / "python/cir_workflow/pilot_cases.py"
    oracle_sha = _sha(oracle_py)
    cases_sha = _sha(cases_py)
    tool = {"backend_sha256": _sha(BIN), "instrument_sha256": _sha(INS),
            "backend_git": "a35dc86", "backend_path": str(BIN), "instrument_path": str(INS),
            "n_runs": 2, "run_timeout_s": 8, "no_join_run_timeout_s": 2}
    rows = []
    for name in CASES:
        directory = FIXTURE_ROOT / name
        case = load_case(name)
        dest = out / name
        dest.mkdir(exist_ok=True)
        check = _run([str(BIN), "check", str(directory / "design.cir.json")])
        explore = _run([str(BIN), "explore", str(directory / "design.cir.json"),
                        str(directory / "contract.json"), "petri"])
        (dest / "check.stdout").write_text(check["stdout"])
        (dest / "check.stderr").write_text(check["stderr"])
        (dest / "explore.stdout").write_text(explore["stdout"])
        (dest / "explore.stderr").write_text(explore["stderr"])
        parsed = {}
        if explore["returncode"] == 0 and explore["stdout"].strip():
            parsed = json.loads(explore["stdout"])
        props = {str(p.get("id")).replace("preserved: ", ""): p.get("outcome")
                 for p in parsed.get("properties") or []}
        verified = bool(parsed.get("complete") is True and props and all(
            outcome == "PASS" for outcome in props.values()))
        roles = {}
        for role in ("control", "defect"):
            source = (directory / f"{role}.rs").read_text(encoding="utf-8")
            work = Path(tempfile.mkdtemp(prefix=f"pre-{name}-{role}-"))
            spec = ({"test_id": "stdout_eq", "kind": "stdout_eq", "expected": "DONE done=6"}
                    if name.startswith("compute") else None)
            timeout = 2.0 if name == "no_join" and role == "defect" else 8.0
            runs = 1 if name == "no_join" and role == "defect" else 2
            result = evaluate_candidate(
                source, directory / "design.cir.json", directory / "contract.json", work,
                binary=BIN, instrument=INS, n_runs=runs, run_timeout=timeout,
                functional_spec=spec, cell_id=f"{name}/{role}")
            score = evaluate_role(case, source, work / "oracle")
            ledger = result["ledger"]
            roles[role] = {
                "current_evaluation": ledger["current_evaluation"],
                "run_state": ledger["run"]["state"],
                "trace_state": ledger["trace"]["state"],
                "functional": (result.get("functional") or {}).get("status"),
                "followup": result["followup"]["status"],
                "requirement": score["requirement"]["status"],
                "design": score["design"]["status"],
                "input_protocol_error": check["returncode"] != 0 or explore["returncode"] != 0,
            }
            print(name, role, roles[role]["current_evaluation"], roles[role]["requirement"])
        functional = ({"test_id": "stdout_eq", "kind": "stdout_eq", "expected": "DONE done=6"}
                      if name.startswith("compute") else None)
        row = {
            "id": name,
            "defect_sha256": _sha(directory / "defect.rs"),
            "control_sha256": _sha(directory / "control.rs"),
            "cir_sha256": _sha(directory / "design.cir.json"),
            "contract_sha256": _sha(directory / "contract.json"),
            "requirements_sha256": _sha(directory / "requirements.md"),
            "functional_spec": functional,
            "oracle_sha256": oracle_sha,
            "cases_sha256": cases_sha,
            "check_returncode": check["returncode"],
            "explore_returncode": explore["returncode"],
            "explore_outcome": parsed.get("outcome"),
            "explore_complete": parsed.get("complete"),
            "properties": props,
            "model_verified": verified,
            "input_protocol_error": check["returncode"] != 0 or explore["returncode"] != 0,
            "roles": roles,
            "tool": tool,
        }
        rows.append(row)
    summary = {"tool": tool, "oracle_sha256": oracle_sha, "cases_sha256": cases_sha,
               "cases": rows,
               "ok": all(not row["input_protocol_error"] and row["model_verified"] for row in rows)}
    target = out / "SUMMARY.json"
    target.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    slim_cases = []
    for row in rows:
        slim_cases.append({key: row[key] for key in (
            "id", "defect_sha256", "control_sha256", "cir_sha256", "contract_sha256",
            "requirements_sha256", "functional_spec", "oracle_sha256", "cases_sha256",
            "check_returncode", "explore_returncode", "explore_outcome", "explore_complete",
            "model_verified", "input_protocol_error", "roles")})
    slim = {"ok": summary["ok"], "tool": tool, "oracle_sha256": oracle_sha,
            "cases_sha256": cases_sha, "cases": slim_cases}
    fixture = FIXTURE_ROOT / "PREFLIGHT.json"
    fixture.write_text(json.dumps(slim, indent=2) + "\n")
    print("wrote", target, "ok", summary["ok"])
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
