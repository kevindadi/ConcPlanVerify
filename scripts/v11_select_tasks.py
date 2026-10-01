#!/usr/bin/env python3
"""Deterministic task selection and offline preflight for the three-arm ablation.

The pool is the benchmark families that the 2026-09-26 G3 generation batch used.
Selection does not look at this round's model outcomes. Defects are the
benchmark reference programs, labeled as reference defects rather than
model-generated mistakes.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FAM = REPO / "benchmarks/families"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
HIST = REPO / "experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash/SUMMARY.json"

SPAWN = re.compile(r"thread::spawn")
DONE = re.compile(r"print exactly the line `([^`]+)`")
UNSUPPORTED = ("thread::scope", "Condvar::", "Semaphore", "async ", ".await", "RwLock")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_line(requirements: str) -> str | None:
    match = DONE.search(requirements)
    return match.group(1) if match else None


def run_prog(source: str, timeout: float = 2.0) -> dict:
    from cir_workflow.candidate_eval import capture_program_stdout
    from cir_workflow import rust_oracle
    work = Path(tempfile.mkdtemp(prefix="v11-"))
    (work / "src").mkdir()
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    built, log = rust_oracle.cargo_build(work)
    if not built:
        return {"built": False, "status": "fail", "reason": "build_failed", "log": log[-400:]}
    run = capture_program_stdout(work / "target/debug/probe", timeout=timeout)
    return {"built": True, "status": "ran", **run}


def requirement(source: str, line: str | None, repeats: int = 3) -> dict:
    """Termination is the auditable requirement. A required print line is checked
    only when this source is expected to produce it. Timeout is nontermination,
    not by itself a proved deadlock."""

    runs = [run_prog(source) for _ in range(repeats)]
    if any(not item.get("built") for item in runs):
        return {"status": "fail", "reason": "build_failed"}
    timed = [bool(item.get("timed_out")) for item in runs]
    codes = [item.get("returncode") for item in runs]
    stdout = (runs[-1].get("stdout") or "").strip()
    completed = all(item.get("kind") == "completed" and item.get("returncode") == 0
                    and not item.get("timed_out") for item in runs)
    printed = line is None or stdout == line
    if any(timed):
        return {"status": "fail", "reason": "timeout", "stdout": stdout[:200],
                "returncode": codes[-1], "timed_out": True}
    if not completed:
        return {"status": "fail", "reason": "exit", "stdout": stdout[:200],
                "returncode": codes[-1], "timed_out": False}
    if line is not None and not printed:
        return {"status": "fail", "reason": "stdout", "stdout": stdout[:200],
                "returncode": 0, "timed_out": False}
    return {"status": "pass", "reason": "pass", "stdout": stdout[:200],
            "returncode": 0, "timed_out": False}


def main() -> int:
    hist_tasks = set(json.loads(HIST.read_text())["tasks"])
    pool = []
    for req in sorted(FAM.glob("*/**/generation_input/REQUIREMENTS.md")):
        task = str(req.relative_to(FAM).parent.parent).replace("\\", "/")
        if task not in hist_tasks:
            continue
        directory = req.parent.parent
        buggy = directory / "rust/buggy.rs"
        fixed = directory / "rust/fixed.rs"
        cir = directory / "fixed.cir.json"
        contract = directory / "contract.json"
        text = req.read_text(encoding="utf-8")
        record = {"task": task, "origin": "benchmark reference defect used by the 2026-09-26 G3 batch",
                  "natural_model_generation": False,
                  "historical_requested_model": "deepseek-flash",
                  "historical_batch": HIST.name}
        if not buggy.is_file() or not fixed.is_file() or not cir.is_file():
            record["decision"] = "excluded"
            record["reason"] = "missing buggy.rs, fixed.rs, or fixed.cir.json"
            pool.append(record)
            continue
        bug_src = buggy.read_text(encoding="utf-8")
        fix_src = fixed.read_text(encoding="utf-8")
        spawns = len(SPAWN.findall(bug_src))
        record.update({"spawns_in_defect": spawns,
                       "defect_sha256": sha(buggy), "control_sha256": sha(fixed),
                       "cir_sha256": sha(cir)})
        if spawns < 2:
            record["decision"] = "excluded"
            record["reason"] = "fewer than two thread::spawn calls"
            pool.append(record)
            continue
        if any(token in bug_src or token in fix_src for token in UNSUPPORTED):
            record["decision"] = "excluded"
            record["reason"] = "uses a feature outside the stable build/run support set"
            pool.append(record)
            continue
        check = subprocess.run([str(BIN), "check", str(cir)], capture_output=True, text=True)
        explore = subprocess.run([str(BIN), "explore", str(cir), str(contract), "petri"],
                                 capture_output=True, text=True)
        verified = False
        if explore.returncode == 0 and explore.stdout.strip():
            payload = json.loads(explore.stdout)
            verified = payload.get("complete") is True and payload.get("outcome") == "PASS"
        record["cir_check"] = check.returncode
        record["cir_verified"] = verified
        if check.returncode != 0 or not verified:
            record["decision"] = "excluded"
            record["reason"] = "reference CIR is not a verified backend input"
            pool.append(record)
            continue
        line = expected_line(text)
        # Use the print clause only when the reference control already emits it.
        control_once = run_prog(fix_src)
        control_prints = line is not None and (control_once.get("stdout") or "").strip() == line
        scored_line = line if control_prints else None
        record["expected_stdout"] = scored_line
        record["print_clause_in_reference_control"] = control_prints
        bug = requirement(bug_src, scored_line)
        good = requirement(fix_src, scored_line)
        # A behavior-preserving edit must still pass. This is not a byte match.
        alternate = fix_src.replace("fn main()", "fn main()", 1) + "\n// alternate layout\n"
        if "fn main()" in fix_src:
            alternate = fix_src.replace("fn main() {", "fn main() {\n    let _layout = 1;", 1)
        alt = requirement(alternate, scored_line)
        record["defect_requirement"] = bug["status"]
        record["defect_reason"] = bug["reason"]
        record["control_requirement"] = good["status"]
        record["alternate_requirement"] = alt["status"]
        if bug["status"] != "fail" or good["status"] != "pass" or alt["status"] != "pass":
            record["decision"] = "excluded"
            record["reason"] = "behavioral oracle did not separate the defect, the control, and an alternate layout"
            pool.append(record)
            continue
        record["decision"] = "eligible"
        record["reason"] = "two workers, verified CIR, defect fails behavior, control and alternate layout pass"
        record["requirements_sha256"] = sha(req)
        record["contract_sha256"] = sha(contract)
        pool.append(record)
    eligible = [row for row in pool if row["decision"] == "eligible"]
    selected = eligible[:6]
    for row in selected:
        row["decision"] = "selected"
    out = {
        "rule": [
            "Pool is the task list of experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash.",
            "Sort by task id.",
            "Require buggy.rs, fixed.rs, fixed.cir.json, at least two thread::spawn calls.",
            "Exclude thread::scope, Condvar, Semaphore, async, RwLock.",
            "Require concir-backend check and explore PASS complete.",
            "Require the reference defect to fail the stdout/termination check, and both the reference control and a layout-only edit to pass.",
            "Take the first 6 eligible tasks. Do not use this round's model results.",
            "These defects are benchmark reference programs, not natural model mistakes.",
        ],
        "pool_size": len(pool),
        "eligible": len(eligible),
        "selected": [row["task"] for row in selected],
        "pool": pool,
    }
    dest = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes/strong-link-v11/CANDIDATE_POOL.json")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
    print("eligible", len(eligible), "selected", out["selected"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
