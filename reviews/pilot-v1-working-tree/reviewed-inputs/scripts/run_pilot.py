#!/usr/bin/env python3
"""Lightweight, manifest-driven pilot runner for the ConcIR repair loop.

It drives the existing `concir-backend` CLI (no core changes), runs strategy
A/B/C on identical inputs/config, saves raw stdout/stderr/exit and the complete
artifact, replays every complete artifact with a separate CLI call, and records
a JSONL result plus a CSV summary.

Design notes
------------
* One release binary is used for the whole run; its sha256 is part of the run
  identity, so a rebuilt binary cannot silently reuse old records.
* Every search is a separate process with a wall-clock timeout. On timeout the
  whole process group is killed and the partial output is kept.
* External timeout, resource errors, JSON errors and replay failures are
  classified separately; none of them is reported as the searcher's UNKNOWN or
  no-acceptable-candidate.
* Resume reuses a completed record only when the run fingerprint (inputs +
  contract + config + strategy + binary sha) matches exactly. A mismatched
  existing record is reported and re-run.

Usage:
  python3 scripts/run_pilot.py run --suite smoke --suite pilot
  python3 scripts/run_pilot.py validate-witness
  python3 scripts/run_pilot.py summarize
  python3 scripts/run_pilot.py selftest
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import shutil
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PILOT = REPO / "experiments" / "pilot-v1"
MANIFEST = PILOT / "manifest.json"
DEFAULT_BIN = REPO / "target" / "release" / "concir-backend"
DEFAULT_OUT = PILOT
STRATEGIES = ["a", "b", "c"]

SEARCH_OUTCOMES = {
    "repaired",
    "already_satisfied",
    "no_acceptable_candidate",
    "budget_exhausted",
    "analysis_unknown",
    "invalid",
    "unsupported",
    "invalid_config",
}


# ───────────────────────────── helpers ─────────────────────────────


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def canonical(obj) -> str:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


def git_info() -> dict:
    def git(*args) -> str:
        try:
            return subprocess.run(
                ["git", *args], cwd=REPO, capture_output=True, text=True, check=True
            ).stdout.strip()
        except Exception:
            return ""

    commit = git("rev-parse", "HEAD")
    status = git("status", "--porcelain")
    diff = git("diff", "HEAD")
    staged = git("diff", "--cached")
    dirty = bool(status)
    return {
        "commit": commit,
        "dirty": dirty,
        "status_porcelain": status,
        "diff_sha256": sha256_bytes(diff.encode()) if diff else "",
        "staged_diff_sha256": sha256_bytes(staged.encode()) if staged else "",
        "source_id": commit + ("+dirty:" + sha256_bytes(diff.encode())[:12] if dirty else ""),
    }


def environment(binary: Path) -> dict:
    def v(cmd):
        try:
            return subprocess.run(cmd, capture_output=True, text=True).stdout.strip()
        except Exception:
            return ""

    return {
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
        "git": git_info(),
        "binary_path": str(binary),
        "binary_sha256": sha256_file(binary) if binary.exists() else "",
        "rustc": v(["rustc", "--version"]),
        "cargo": v(["cargo", "--version"]),
        "python": sys.version.split()[0],
        "uname": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "cpu_count": os.cpu_count(),
    }


def run_process(cmd, timeout_s, cwd=REPO):
    """Run cmd in its own process group with a hard timeout. Returns a dict."""
    t0 = time.time()
    start_new = os.name == "posix"
    proc = subprocess.Popen(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=start_new,
    )
    timed_out = False
    try:
        out, err = proc.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        timed_out = True
        if start_new:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except ProcessLookupError:
                pass
        else:
            proc.kill()
        out, err = proc.communicate()
    wall_ms = int((time.time() - t0) * 1000)
    return {
        "cmd": [str(c) for c in cmd],
        "exit_code": proc.returncode,
        "stdout": out.decode("utf-8", "replace"),
        "stderr": err.decode("utf-8", "replace"),
        "wall_ms": wall_ms,
        "timeout_s": timeout_s,
        "timed_out": timed_out,
    }


def write(path: Path, text: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def run_key(case, config_name, strategy, repeat):
    return f"{case}|{config_name}|{strategy}|r{repeat}"


def run_fingerprint(case_entry, config, config_name, strategy, repeat, binary_sha, source_id):
    model = (PILOT / case_entry["model"]).resolve()
    contract = (PILOT / case_entry["contract"]).resolve()
    payload = {
        "case": case_entry["case"],
        "model_sha256": sha256_file(model),
        "contract_sha256": sha256_file(contract),
        "config": config,
        "config_name": config_name,
        "strategy": strategy,
        "repeat": repeat,
        "binary_sha256": binary_sha,
        "source_id": source_id,
    }
    return sha256_bytes(canonical(payload).encode())


def plan_runs(manifest, suite):
    """Yield (case_entry, config_name, config, strategy, repeat)."""
    if suite == "smoke":
        for c in manifest["smoke_cases"]:
            c = dict(c)
            cfg = dict(manifest["search_configs"]["main"])
            if "config_override" in c:
                cfg.update(c["config_override"])
            for st in STRATEGIES:
                yield c, "smoke", cfg, st, 1
    elif suite == "pilot":
        for c in manifest["cases"]:
            for st in STRATEGIES:
                for rep in range(1, 4):
                    yield c, "main", dict(manifest["search_configs"]["main"]), st, rep
                yield c, "tight", dict(manifest["search_configs"]["tight"]), st, 1
    else:
        raise SystemExit(f"unknown suite '{suite}'")


def classify_search(result, artifact_text):
    """Return (final_classification, raw_outcome, parsed_artifact, notes)."""
    if result["timed_out"]:
        return "external_timeout", None, None, "process group killed after timeout"
    if result["exit_code"] is not None and result["exit_code"] < 0:
        return "resource_error", None, None, f"killed by signal {-result['exit_code']}"
    text = artifact_text if artifact_text is not None else result["stdout"]
    try:
        artifact = json.loads(text)
    except Exception as e:
        return "json_error", None, None, f"stdout is not valid JSON: {e}"
    raw = artifact.get("outcome")
    notes = ""
    if raw not in SEARCH_OUTCOMES:
        return "json_error", raw, artifact, f"unknown outcome '{raw}'"
    return raw, raw, artifact, notes


# ───────────────────────────── run ─────────────────────────────


def load_existing(results_path: Path):
    """key -> record (last wins); also keep list of mismatches seen."""
    by_key = {}
    if results_path.exists():
        for line in results_path.read_text().splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except Exception:
                continue
            by_key[run_key(rec["case"], rec["config_name"], rec["strategy"], rec["repeat"])] = rec
    return by_key


def append_result(results_path: Path, record: dict):
    results_path.parent.mkdir(parents=True, exist_ok=True)
    with results_path.open("a") as f:
        f.write(json.dumps(record, sort_keys=True) + "\n")


def run_one(case_entry, config_name, config, strategy, repeat, binary, binary_sha, source_id,
            outdir, timeout_s, resume_index, results_path):
    key = run_key(case_entry["case"], config_name, strategy, repeat)
    fp = run_fingerprint(case_entry, config, config_name, strategy, repeat, binary_sha, source_id)
    if resume_index is not None:
        prev = resume_index.get(key)
        if prev and prev.get("run_fingerprint") == fp and prev.get("final_classification") not in (
            "external_timeout",
            "resource_error",
            "json_error",
            "replay_failed",
            "artifact_missing",
        ):
            prev = dict(prev)
            prev["reused_from_previous"] = True
            return prev, "reused"
        if prev:
            return None, "mismatch"

    model = (PILOT / case_entry["model"]).resolve()
    contract = (PILOT / case_entry["contract"]).resolve()
    rundir = outdir / "raw" / case_entry.get("suite", "pilot") / (
        f"{case_entry['case']}__{config_name}__{strategy}__r{repeat}"
    )
    rundir.mkdir(parents=True, exist_ok=True)
    artifact_path = rundir / "artifact.json"

    cmd = [
        str(binary),
        "repair",
        str(model),
        str(contract),
        "--strategy",
        strategy,
        "--candidate-budget",
        str(config["candidate_budget"]),
        "--verification-budget",
        str(config["verification_budget"]),
        "--max-depth",
        str(config["max_depth"]),
        "--max-total-edits",
        str(config["max_total_edits"]),
        "--artifact",
        str(artifact_path),
    ]
    res = run_process(cmd, timeout_s)
    write(rundir / "search.stdout", res["stdout"])
    write(rundir / "search.stderr", res["stderr"])
    write(rundir / "search.exit", str(res["exit_code"]))
    write(rundir / "search.command", " ".join(res["cmd"]) + "\n")

    artifact_text = None
    if artifact_path.exists():
        artifact_text = artifact_path.read_text()
    final, raw, artifact, notes = classify_search(res, artifact_text)

    replay_exit = None
    replay_wall_ms = None
    replay_ok = None
    if final in SEARCH_OUTCOMES and artifact is not None and artifact_path.exists():
        rep = run_process([str(binary), "replay", str(artifact_path)], timeout_s)
        write(rundir / "replay.stdout", rep["stdout"])
        write(rundir / "replay.stderr", rep["stderr"])
        write(rundir / "replay.exit", str(rep["exit_code"]))
        replay_exit = rep["exit_code"]
        replay_wall_ms = rep["wall_ms"]
        replay_ok = (not rep["timed_out"]) and rep["exit_code"] == 0
        if rep["timed_out"]:
            final = "replay_timeout"
        elif not replay_ok:
            final = "replay_failed"
            notes = (notes + "; " if notes else "") + rep["stderr"].strip()[:300]
    elif artifact is None:
        final = final if final not in SEARCH_OUTCOMES else "artifact_missing"

    counts = (artifact or {}).get("counts", {})
    chain = (artifact or {}).get("patch_chain", [])
    root_outcome = None
    nodes = (artifact or {}).get("nodes", [])
    if nodes:
        root_outcome = nodes[0].get("report", {}).get("outcome")

    record = {
        "suite": case_entry.get("suite", "pilot"),
        "case": case_entry["case"],
        "family": case_entry.get("family", ""),
        "params": case_entry.get("params", {}),
        "strategy": strategy,
        "repeat": repeat,
        "config_name": config_name,
        "config": config,
        "model": str(model),
        "contract": str(contract),
        "model_sha256": sha256_file(model),
        "contract_sha256": sha256_file(contract),
        "input_fingerprint": sha256_bytes(
            canonical(
                {
                    "model_sha256": sha256_file(model),
                    "contract_sha256": sha256_file(contract),
                    "config": config,
                }
            ).encode()
        ),
        "run_fingerprint": fp,
        "binary_sha256": binary_sha,
        "source_id": source_id,
        "command": res["cmd"],
        "exit_code": res["exit_code"],
        "wall_ms": res["wall_ms"],
        "timeout_s": timeout_s,
        "timed_out": res["timed_out"],
        "raw_outcome": raw,
        "final_classification": final,
        "stop_reason": (artifact or {}).get("stop_reason"),
        "truncation": (artifact or {}).get("truncation"),
        "saw_unknown": (artifact or {}).get("saw_unknown"),
        "root_outcome": root_outcome,
        "proposals": counts.get("proposals"),
        "unique_programs": counts.get("unique_candidate_programs"),
        "verification_calls": counts.get("verification_calls"),
        "cache_hits": counts.get("cache_hits"),
        "states_explored": counts.get("states_explored"),
        "nodes": counts.get("nodes"),
        "patch_len": sum(len(e.get("changes", [])) for e in chain),
        "accepted_chain_len": len(chain),
        "replay_exit": replay_exit,
        "replay_wall_ms": replay_wall_ms,
        "replay_ok": replay_ok,
        "classification_notes": notes,
        "artifact_path": str(artifact_path),
        "raw_dir": str(rundir),
        "reused_from_previous": False,
    }
    append_result(results_path, record)
    return record, "ran"


def do_run(args):
    binary = Path(args.bin).resolve()
    if not binary.exists():
        raise SystemExit(f"binary not found: {binary} (build with `cargo build --release`)")
    outdir = Path(args.out).resolve()
    outdir.mkdir(parents=True, exist_ok=True)
    binary_sha = sha256_file(binary)
    source_id = git_info()["source_id"]
    env = environment(binary)
    write(outdir / "environment.json", json.dumps(env, indent=2) + "\n")

    manifest = json.loads(MANIFEST.read_text())
    results_path = outdir / "results.jsonl"
    resume_index = load_existing(results_path) if args.resume else None

    # expand suites
    runs = []
    for suite in args.suite:
        for case_entry, config_name, config, strategy, repeat in plan_runs(manifest, suite):
            ce = dict(case_entry)
            ce["suite"] = suite
            runs.append((ce, config_name, config, strategy, repeat))

    deadline = time.time() + args.total_budget
    executed = reused = mismatched = 0
    not_executed = []
    for ce, config_name, config, strategy, repeat in runs:
        if time.time() >= deadline:
            not_executed.append(run_key(ce["case"], config_name, strategy, repeat))
            continue
        rec, status = run_one(
            ce, config_name, config, strategy, repeat, binary, binary_sha, source_id,
            outdir, args.timeout, resume_index, results_path,
        )
        if status == "reused":
            executed += 1
            reused += 1
        elif status == "mismatch":
            mismatched += 1
            rec, status = run_one(
                ce, config_name, config, strategy, repeat, binary, binary_sha, source_id,
                outdir, args.timeout, None, results_path,
            )
            executed += 1
        else:
            executed += 1
        print(
            f"[{executed}/{len(runs)}] {ce['case']:24} {config_name:5} {strategy} r{repeat} "
            f"{rec['final_classification']:<22} {rec['wall_ms']}ms"
            + (" (reused)" if status == "reused" else "")
        )

    summary = {
        "binary": str(binary),
        "binary_sha256": binary_sha,
        "source_id": source_id,
        "suites": args.suite,
        "planned": len(runs),
        "executed": executed,
        "reused": reused,
        "resume_mismatches": mismatched,
        "not_executed": not_executed,
        "timeout_s": args.timeout,
        "total_budget_s": args.total_budget,
        "results_path": str(results_path),
    }
    write(outdir / "run_meta.json", json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


# ───────────────────────────── witness validation ─────────────────────────────


def do_validate_witness(args):
    binary = Path(args.bin).resolve()
    manifest = json.loads(MANIFEST.read_text())
    out = []
    for c in manifest["cases"]:
        w = PILOT / c["witness"]
        wc = PILOT / c["witness_contract"]
        expected = c.get("witness_expected", "PASS")
        row = {"case": c["case"], "expected": expected}
        for engine in ("petri", "interp"):
            r = run_process([str(binary), "explore", str(w), str(wc), engine], args.timeout)
            try:
                d = json.loads(r["stdout"])
                row[engine] = {"outcome": d.get("outcome"), "states": d.get("states_explored"),
                               "wall_ms": r["wall_ms"], "exit": r["exit_code"]}
            except Exception:
                row[engine] = {"outcome": None, "error": r["stderr"][:200], "exit": r["exit_code"]}
        out.append(row)
        print(f"{c['case']:26} want={expected:5} petri={row['petri'].get('outcome')} interp={row['interp'].get('outcome')}")
    write(PILOT / "witness_results.json", json.dumps(out, indent=2) + "\n")
    failures = [
        r for r in out
        if r["petri"].get("outcome") != r["expected"] or r["interp"].get("outcome") != r["expected"]
    ]
    print(f"witness validation: {len(out) - len(failures)}/{len(out)} match expected outcome on both engines")
    return 0 if not failures else 1


# ───────────────────────────── summarize ─────────────────────────────


def load_results(outdir: Path):
    path = outdir / "results.jsonl"
    recs = []
    if path.exists():
        for line in path.read_text().splitlines():
            if line.strip():
                recs.append(json.loads(line))
    return recs


def non_time_fields(r):
    return {
        k: r.get(k)
        for k in (
            "raw_outcome", "final_classification", "stop_reason", "truncation", "saw_unknown",
            "proposals", "unique_programs", "verification_calls", "cache_hits", "states_explored",
            "nodes", "patch_len", "accepted_chain_len", "replay_exit", "replay_ok", "exit_code",
        )
    }


def do_summarize(args):
    outdir = Path(args.out).resolve()
    recs = load_results(outdir)
    fields = [
        "suite", "case", "family", "strategy", "repeat", "config_name",
        "config", "raw_outcome", "final_classification", "stop_reason", "truncation",
        "saw_unknown", "root_outcome", "proposals", "unique_programs", "verification_calls",
        "cache_hits", "states_explored", "nodes", "patch_len", "accepted_chain_len",
        "exit_code", "wall_ms", "replay_exit", "replay_wall_ms", "replay_ok",
        "model_sha256", "contract_sha256", "binary_sha256", "source_id",
        "artifact_path", "reused_from_previous",
    ]
    with (outdir / "summary.csv").open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, extrasaction="ignore")
        w.writeheader()
        for r in recs:
            row = dict(r)
            row["config"] = canonical(r.get("config", {}))
            w.writerow(row)

    # determinism: same (case, config, strategy) repeats must have equal non-time fields
    groups = {}
    for r in recs:
        groups.setdefault((r["case"], r["config_name"], r["strategy"]), []).append(r)
    nondeterministic = []
    for key, rs in groups.items():
        if len(rs) < 2:
            continue
        base = non_time_fields(rs[0])
        for r in rs[1:]:
            if non_time_fields(r) != base:
                nondeterministic.append({"group": list(key), "repeat": r["repeat"]})
                break

    # paired B/C on successful repairs
    by = {}
    for r in recs:
        by.setdefault((r["case"], r["config_name"], r["repeat"]), {})[r["strategy"]] = r
    paired = []
    for (case, cfg, rep), d in sorted(by.items()):
        b, c = d.get("b"), d.get("c")
        if not b or not c:
            continue
        b_ok = b["final_classification"] == "repaired"
        c_ok = c["final_classification"] == "repaired"
        paired.append({
            "case": case, "config": cfg, "repeat": rep,
            "b": b["final_classification"], "c": c["final_classification"],
            "b_ver": b["verification_calls"], "c_ver": c["verification_calls"],
            "b_states": b["states_explored"], "c_states": c["states_explored"],
            "b_wall_ms": b["wall_ms"], "c_wall_ms": c["wall_ms"],
            "both_repaired": b_ok and c_ok, "success_differs": b_ok != c_ok,
        })

    summary = {
        "records": len(recs),
        "by_suite": {},
        "nondeterministic": nondeterministic,
        "paired_bc": paired,
    }
    for r in recs:
        s = summary["by_suite"].setdefault(r["suite"], {})
        s[r["final_classification"]] = s.get(r["final_classification"], 0) + 1
    write(outdir / "summary.json", json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"records": len(recs), "by_suite": summary["by_suite"],
                      "nondeterministic": nondeterministic}, indent=2))
    return 0


# ───────────────────────────── selftest ─────────────────────────────


def do_selftest(args):
    binary = Path(args.bin).resolve()
    work = Path(args.out).resolve() / "selftest"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(MANIFEST.read_text())
    cfg = manifest["search_configs"]["main"]
    model = PILOT / "cases" / "c_scope_restricted.json"
    contract = PILOT / "cases" / "c_scope_restricted_contract.json"
    checks = []

    # 1. nonzero exit but a legal, replayable artifact.
    art = work / "legal.json"
    r = run_process([
        str(binary), "repair", str(model), str(contract),
        "--strategy", "b", "--candidate-budget", str(cfg["candidate_budget"]),
        "--verification-budget", str(cfg["verification_budget"]),
        "--max-depth", str(cfg["max_depth"]), "--max-total-edits", str(cfg["max_total_edits"]),
        "--artifact", str(art),
    ], args.timeout)
    ok_art = art.exists() and "no_acceptable_candidate" in art.read_text()
    rp = run_process([str(binary), "replay", str(art)], args.timeout)
    checks.append(("nonzero_exit_legal_artifact", r["exit_code"] == 1 and ok_art and rp["exit_code"] == 0,
                   f"exit={r['exit_code']} replay={rp['exit_code']}"))

    # 2. normal repaired output.
    m2 = PILOT / "cases" / "p1_same.json"
    c2 = PILOT / "cases" / "p1_same_contract.json"
    art2 = work / "repaired.json"
    r2 = run_process([
        str(binary), "repair", str(m2), str(c2), "--strategy", "b",
        "--candidate-budget", str(cfg["candidate_budget"]),
        "--verification-budget", str(cfg["verification_budget"]),
        "--max-depth", str(cfg["max_depth"]), "--max-total-edits", str(cfg["max_total_edits"]),
        "--artifact", str(art2),
    ], args.timeout)
    rp2 = run_process([str(binary), "replay", str(art2)], args.timeout)
    checks.append(("normal_repaired_and_replay", r2["exit_code"] == 0 and rp2["exit_code"] == 0,
                   f"exit={r2['exit_code']} replay={rp2['exit_code']}"))

    # 3. external timeout reaping on a deliberately tiny timeout.
    rt = run_process([
        str(binary), "repair", str(m2), str(c2), "--strategy", "b",
    ], 0.001)
    checks.append(("external_timeout_reaped", rt["timed_out"] and rt["exit_code"] is not None,
                   f"timed_out={rt['timed_out']} exit={rt['exit_code']}"))

    # 4. corrupted artifact -> replay failure; corrupt JSON -> parse error.
    bad = work / "corrupt.json"
    tampered = json.loads(art.read_text())
    tampered["stop_reason"] = "solved"
    bad.write_text(json.dumps(tampered))
    rpb = run_process([str(binary), "replay", str(bad)], args.timeout)
    notyet = work / "notyet.json"
    notyet.write_text("{ this is not json ")
    rpn = run_process([str(binary), "replay", str(notyet)], args.timeout)
    checks.append(("corrupt_replay_fails", rpb["exit_code"] != 0 and rpn["exit_code"] != 0,
                   f"tamper_exit={rpb['exit_code']} parse_exit={rpn['exit_code']}"))

    # 5. resume mismatch detection.
    res = work / "results.jsonl"
    entry = {"case": "p1_same", "family": "one_defect", "model": "cases/p1_same.json",
             "contract": "cases/p1_same_contract.json"}
    fake = {
        "case": "p1_same", "config_name": "main", "strategy": "b", "repeat": 1,
        "run_fingerprint": "deadbeef", "final_classification": "repaired",
    }
    res.write_text(json.dumps(fake) + "\n")
    idx = load_existing(res)
    fp = run_fingerprint(entry, cfg, "main", "b", 1, "binarysha", "src")
    prev = idx.get(run_key("p1_same", "main", "b", 1))
    mismatch = prev is not None and prev.get("run_fingerprint") != fp
    checks.append(("resume_mismatch_detected", mismatch, f"prev={prev is not None} match={prev and prev.get('run_fingerprint')==fp}"))

    ok = all(c[1] for c in checks)
    for name, good, detail in checks:
        print(f"{'PASS' if good else 'FAIL'}  {name}: {detail}")
    print(f"selftest {'passed' if ok else 'FAILED'}")
    return 0 if ok else 1


# ───────────────────────────── main ─────────────────────────────


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--bin", default=str(DEFAULT_BIN))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    sub = ap.add_subparsers(dest="cmd", required=True)

    p_run = sub.add_parser("run")
    p_run.add_argument("--suite", action="append", required=True, choices=["smoke", "pilot"])
    p_run.add_argument("--timeout", type=float, default=30.0)
    p_run.add_argument("--total-budget", type=float, default=600.0)
    p_run.add_argument("--resume", action="store_true")
    p_run.set_defaults(func=do_run)

    p_w = sub.add_parser("validate-witness")
    p_w.add_argument("--timeout", type=float, default=180.0)
    p_w.set_defaults(func=do_validate_witness)

    p_s = sub.add_parser("summarize")
    p_s.set_defaults(func=do_summarize)

    p_t = sub.add_parser("selftest")
    p_t.add_argument("--timeout", type=float, default=30.0)
    p_t.set_defaults(func=do_selftest)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
