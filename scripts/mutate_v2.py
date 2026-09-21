"""conform-mutation-v2 (§2): operation-bound mutation operators on the 23 v2
programs. No LLM requests."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.conformance import collect_traces, conform_all  # noqa: E402

A3 = REPO / "experiments/a3-to-rust-v2"
OUT = REPO / "experiments/conform-mutation-v2"
BATCH = REPO / ("experiments/flash-repair-main-v1/"
                "run-20260920T083344-41096-8172c5")
BIN = REPO.parent / "ConcIR/target/release/concir-backend"

LOCK_RE = re.compile(r'^\s*_g_\w+ = Some\(crate::cir_trace::lock\(&shared\.\w+, tag, "[^"]*", "[^"]*"\)\);\s*$')
UNLOCK_RE = re.compile(r'^\s*crate::cir_trace::unlock\(_g_\w+\.take\(\), tag, "[^"]*", "[^"]*"\);\s*$')
NOTIFY_RE = re.compile(r'crate::cir_trace::notify_(one|all)\(')
CHAN_RE = re.compile(r'\.(send_sid|recv_sid)\(')
SPAWN_RE = re.compile(r'^\s*__hs\.push\((crate::cir_trace::spawn|std::thread::spawn)\(')
SCOPE_RE = re.compile(r'crate::cir_trace::scope\(')


def _function_spans(lines):
    spans = []
    for i, l in enumerate(lines):
        if not re.search(r"\bfn\s+\w+\s*\([^)]*\)\s*\{", l):
            continue
        depth = l.count("{") - l.count("}")
        j = i + 1
        while j < len(lines) and depth > 0:
            depth += lines[j].count("{") - lines[j].count("}")
            j += 1
        spans.append((i, j))
    return spans


def _mutants(src: str):
    lines = src.splitlines()
    out = []

    def emit(op, desc, new_lines):
        out.append((op, desc, "\n".join(new_lines) + "\n"))

    lock_idx = [i for i, l in enumerate(lines) if LOCK_RE.match(l)]
    for start, end in _function_spans(lines):
        in_fn = [i for i in lock_idx if start <= i < end]
        if len(in_fn) >= 2:
            i, j = in_fn[0], in_fn[1]
            nl = list(lines)
            nl[i], nl[j] = nl[j], nl[i]
            emit("M1", f"swap locks at {i+1},{j+1}", nl)
            break

    unlock_idx = [i for i, l in enumerate(lines) if UNLOCK_RE.match(l)]
    if unlock_idx:
        nl = list(lines)
        del nl[unlock_idx[0]]
        emit("M2", f"delete unlock at {unlock_idx[0]+1}", nl)

    if "notify_all(" in src or "notify_one(" in src:
        nl = src.replace("notify_all(", "notify_one(") if "notify_all(" in src \
            else src.replace("notify_one(", "notify_all(")
        emit("M4", "flip notify", nl.splitlines())

    for i, l in enumerate(lines):
        if CHAN_RE.search(l) and i > 0:
            nl = list(lines)
            nl[i - 1], nl[i] = nl[i], nl[i - 1]
            emit("M5", f"move channel op at {i+1} one statement earlier", nl)
            break

    for i, l in enumerate(lines):
        if NOTIFY_RE.search(l) or CHAN_RE.search(l) or SCOPE_RE.search(l):
            nl = list(lines)
            del nl[i]
            emit("M6", f"delete sync call at {i+1}", nl)
            break

    spawn_idx = [i for i, l in enumerate(lines) if SPAWN_RE.match(l)]
    if len(spawn_idx) >= 2:
        i, j = spawn_idx[0], spawn_idx[1]
        nl = list(lines)
        nl[i], nl[j] = nl[j], nl[i]
        emit("M7", f"swap spawn at {i+1},{j+1}", nl)

    resources = sorted(set(re.findall(r'crate::cir_trace::lock\(&shared\.\w+, tag, "([^"]+)"', src)))
    if lock_idx and len(resources) >= 2:
        i = lock_idx[0]
        old = re.search(r'lock\(&shared\.\w+, tag, "([^"]+)"', lines[i]).group(1)
        newr = next(r for r in resources if r != old)
        nl = list(lines)
        nl[i] = lines[i].replace(f'"{old}"', f'"{newr}"')
        emit("M8", f"lock at {i+1}: {old} -> {newr}", nl)

    return out


def _programs():
    summary = json.loads((A3 / "SUMMARY.json").read_text(encoding="utf-8"))
    run = sorted(A3.glob("run-*"))[-1]
    main = json.loads((BATCH / "SUMMARY.json").read_text(encoding="utf-8"))
    cir = {}
    for rep in main["reps"]:
        for t in rep["tasks"]:
            for arm in ("A3_local", "A3_whole", "A3_tiered"):
                rec = (t.get("arms") or {}).get(arm) or {}
                if rec.get("accepted") and rec.get("final_cir"):
                    cir[hashlib.sha256(Path(rec["final_cir"]).read_bytes()).hexdigest()] = rec["final_cir"]
    progs = []
    for sha, r in summary["results"].items():
        key = f"{r['task'].replace('/', '__')}__{r['arm']}__{sha[:12]}"
        skel = run / key / "skeleton"
        if (skel / "src/main.rs").is_file() and sha in cir:
            progs.append({"sha": sha, "task": r["task"], "arm": r["arm"], "skel": skel, "cir": cir[sha]})
    return progs


def _run(mdir, cir):
    build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=mdir,
                           capture_output=True, text=True, timeout=300)
    rec = {"build_ok": build.returncode == 0}
    if not rec["build_ok"]:
        rec["reason"] = "build"
        return rec
    try:
        proc = subprocess.run([str(mdir / "target/debug/cir_generated")],
                              capture_output=True, text=True, timeout=8)
        rec["behavior"] = "terminated_ok" if proc.returncode == 0 else "crash"
    except subprocess.TimeoutExpired:
        rec["behavior"] = "hang"
    tries = collect_traces(mdir, native_runs=8, miri_seeds=16, timeout_s=8.0,
                           calls_dir=mdir / "traces", run_miri=True)
    native = [t for t in tries if t.kind == "native"]
    agg = conform_all(Path(cir), native, binary=BIN, lenient_unlock=False, attempt_events=False)
    rec["conform"] = "PASS" if agg["violation"] == 0 and agg["conformant"] == len(native) else "FAIL"
    bad = next((d for d in agg["details"] if d.get("status") != "conformant"), None)
    rec["conform_reason"] = None if rec["conform"] == "PASS" else str((bad or {}).get("status"))
    miri = [t for t in tries if t.kind == "miri"]
    rec["miri_detected"] = sum(1 for t in miri if t.hang)
    return rec


def main() -> int:
    progs = _programs()
    if len(sys.argv) > 1:
        progs = [p for p in progs if sys.argv[1] in p["task"]]
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "mutants").mkdir(parents=True, exist_ok=True)
    records = []
    for prog in progs:
        src = (prog["skel"] / "src/main.rs").read_text(encoding="utf-8")
        for n, (op, desc, new_src) in enumerate(_mutants(src)):
            mdir = OUT / "mutants" / f"{prog['sha'][:12]}" / f"{op}-{n}"
            (mdir / "src").mkdir(parents=True, exist_ok=True)
            (mdir / "src/main.rs").write_text(new_src, encoding="utf-8")
            shutil.copy(prog["skel"] / "Cargo.toml", mdir / "Cargo.toml")
            shutil.copy(prog["skel"] / "src/cir_trace.rs", mdir / "src/cir_trace.rs")
            records.append({"op": op, "desc": desc, "sha": prog["sha"],
                            "task": prog["task"], "arm": prog["arm"], **_run(mdir, prog["cir"])})
    ops, reasons = {}, {}
    for op in ("M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8"):
        rs = [r for r in records if r["op"] == op]
        if not rs:
            continue
        fail = [r for r in rs if r.get("conform") == "FAIL"]
        ops[op] = {"mutants": len(rs), "build_ok": sum(1 for r in rs if r.get("build_ok")),
                   "conform_fail": len(fail),
                   "conform_recall": round(len(fail) / len(rs), 3),
                   "miri_detected": sum(1 for r in rs if r.get("miri_detected")),
                   "behavior_hang": sum(1 for r in rs if r.get("behavior") == "hang")}
        reasons[op] = dict(Counter(r.get("conform_reason") for r in fail))
    payload = {"schema_version": "conform-mutation-v2",
               "binary_sha256": hashlib.sha256(BIN.read_bytes()).hexdigest(),
               "programs": len(progs), "mutants": len(records), "records": records}
    (OUT / "MUTANTS.json").write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    lines = ["# conform-mutation-v2 — SUMMARY", "",
             f"- programs: {len(progs)}; mutants: {len(records)}",
             f"- binary sha256: `{payload['binary_sha256']}`",
             "- M3 (move ev) is n/a by construction in v2 (no standalone ev).", "",
             "| op | mutants | build_ok | conform FAIL | recall | reasons | miri | hang |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for op, s in ops.items():
        lines.append(f"| {op} | {s['mutants']} | {s['build_ok']} | {s['conform_fail']} | "
                     f"{s['conform_recall']} | {reasons.get(op,{})} | {s['miri_detected']} | "
                     f"{s['behavior_hang']} |")
    (OUT / "SUMMARY.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT / "SUMMARY.json").write_text(json.dumps({"ops": ops, "reasons": reasons}, indent=1) + "\n",
                                      encoding="utf-8")
    print(json.dumps(ops, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
