#!/usr/bin/env python3
"""Offline re-evaluation of strong-link-v12 artifacts. No model requests."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
import sys
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.send_holding_requirements import evaluate_requirements  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
V12 = NOTES / "strong-link-v12"
OUT = NOTES / "strong-link-v13"
CIR = REPO / ("experiments/g3-rootcause-v1/pilot-20260926T230952-deepseekflash/"
              "DeepSeek Flash/channel__send_while_holding_mutex/rep0/cir/revision-1.cir.json")
CONTRACT = REPO / "benchmarks/families/channel/send_while_holding_mutex/contract.json"
DEFECT = (NOTES / "strong-link-v5/reexec-deepseekflash/DeepSeek Flash/"
          "channel__send_while_holding_mutex/rep0/source.rs")
DERIVED = NOTES / "strong-link-v11/derived_control.rs"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
INS = BIN.with_name("concir-instrument")
BIND = BIN.with_name("bind_check")
HIGHLIGHT = {
    ("deepseek-flash", "B", 1),
    ("qwen3.8-flash", "A", 2),
    ("qwen3.8-flash", "C", 1),
    ("qwen3.8-flash", "C", 2),
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sha_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def slim_requirement(result: dict) -> dict:
    checks = {key: value.get("status") for key, value in (result.get("checks") or {}).items()}
    return {"status": result.get("status"), "label": result.get("label"),
            "checks": checks, "uncovered": result.get("uncovered"),
            "not_a_complete_proof": result.get("not_a_complete_proof")}


def eval_design(source: str, work: Path) -> dict:
    work.mkdir(parents=True, exist_ok=True)
    result = evaluate_candidate(
        source, CIR, CONTRACT, work, binary=BIN, instrument=INS,
        n_runs=1, run_timeout=6, cell_id="v13-offline")
    ledger = result.get("ledger") or {}
    for folder in (work / "proj" / "target", work / "eval"):
        pass
    shutil.rmtree(work / "proj" / "target", ignore_errors=True)
    return {
        "current_evaluation": ledger.get("current_evaluation"),
        "delivery_status": ledger.get("delivery_status"),
        "trace": (ledger.get("trace") or {}).get("state"),
        "binding_layer": (ledger.get("layers") or {}).get("binding"),
        "attributes_layer": (ledger.get("layers") or {}).get("attributes"),
        "design_correspondence": ledger.get("design_correspondence"),
        "attributes": (result.get("binding") or {}).get("attributes"),
        "uncovered_sync": (result.get("binding") or {}).get("uncovered_sync"),
        "reasons": ledger.get("reasons"),
        "evidence": str(work / "result.json"),
    }


def old_delivery(model: str, arm: str, round_no: int) -> dict | None:
    path = V12 / "live" / model / arm / f"eval-{round_no}" / "result.json"
    if not path.is_file():
        return None
    ledger = (json.loads(path.read_text()) or {}).get("ledger") or {}
    return {"path": str(path), "current_evaluation": ledger.get("current_evaluation"),
            "delivery_status": ledger.get("delivery_status"),
            "trace": (ledger.get("trace") or {}).get("state"),
            "binding": (ledger.get("layers") or {}).get("binding")}


def candidates() -> list[dict]:
    rows = [{
        "origin": "natural_generated_defect", "model": "deepseekflash", "arm": None,
        "round": 0, "path": DEFECT, "highlight": False,
    }, {
        "origin": "human_derived_control", "model": None, "arm": None,
        "round": None, "path": DERIVED, "highlight": False,
    }]
    for path in sorted((V12 / "live").glob("*/*/source-*.rs")):
        model, arm = path.parts[-3], path.parts[-2]
        round_no = int(path.stem.split("-")[1])
        rows.append({
            "origin": "v12_model_candidate", "model": model, "arm": arm,
            "round": round_no, "path": path,
            "highlight": (model, arm, round_no) in HIGHLIGHT,
        })
    return rows


def bind_program(name: str, source: str, capacity: list[tuple[str, int]]) -> dict:
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        src = root / "input.rs"
        src.write_text(source, encoding="utf-8")
        out = root / "out"
        inst = subprocess.run([str(INS), str(src), "--out", str(out), "--wrappers"],
                              capture_output=True, text=True, timeout=60)
        resources = [{
            "name": item[0], "kind": "sync", "type": "Channel", "mode": "Sync",
            "base": "Int", "capacity": item[1],
        } for item in capacity]
        cir = {"program": "c", "version": "3.5.0", "entry": "main::main", "modules": [{
            "name": "main",
            "resources": resources,
            "functions": [{"name": "main", "kind": "normal", "body": [{"sid": "s", "kind": "return"}]}],
        }]}
        cir_path = root / "cir.json"
        cir_path.write_text(json.dumps(cir), encoding="utf-8")
        bind = subprocess.run([str(BIND), "--resources", str(out / "resources.json"),
                               "--cir", str(cir_path)], capture_output=True, text=True, timeout=60)
        doc = json.loads(bind.stdout) if bind.returncode == 0 else {}
        return {"name": name, "instrument_ok": inst.returncode == 0,
                "bind_ok": bind.returncode == 0,
                "attributes": doc.get("attributes"),
                "uncovered_sync": doc.get("uncovered_sync"),
                "verified": doc.get("verified")}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    work_root = OUT / "work"
    if work_root.exists():
        shutil.rmtree(work_root)
    lines = []
    for index, item in enumerate(candidates()):
        source = item["path"].read_text(encoding="utf-8")
        work = work_root / f"{index:02d}"
        requirement = evaluate_requirements(source, work / "requirement")
        shutil.rmtree(work / "requirement" / "target", ignore_errors=True)
        design = eval_design(source, work / "design")
        previous = None
        if item["origin"] == "v12_model_candidate":
            previous = old_delivery(item["model"], item["arm"], item["round"])
        record = {
            "origin": item["origin"], "model": item["model"], "arm": item["arm"],
            "round": item["round"], "source_path": str(item["path"]),
            "source_sha256": sha(item["path"]),
            "highlight_same_task_round": item["highlight"],
            "requirement": slim_requirement(requirement),
            "identity": design.get("binding_layer"),
            "attributes": design.get("attributes_layer"),
            "attribute_rows": design.get("attributes"),
            "trace": design.get("trace"),
            "new_evaluation": design.get("current_evaluation"),
            "new_delivery": design.get("delivery_status"),
            "design_correspondence": design.get("design_correspondence"),
            "uncovered_sync": design.get("uncovered_sync"),
            "old": previous,
            "changed": None if previous is None else previous.get("delivery_status") != design.get("delivery_status"),
            "reasons": design.get("reasons"),
            "evidence": design.get("evidence"),
            "real_model_requests": 0,
        }
        lines.append(record)
        print(item["model"], item["arm"], item["round"], record["requirement"]["status"],
              record["new_delivery"], record["attributes"], flush=True)
    (OUT / "V12_OFFLINE_REEVALUATION.jsonl").write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in lines), encoding="utf-8")

    derived = DERIVED.read_text(encoding="utf-8")
    block = "    {\n        let _guard = m.lock().unwrap();\n    }\n"
    probes = {
        "no_shared_lock_use": derived.replace(block, ""),
        "unreachable_workers": derived.replace(
            "fn main() {", 'fn main() {\n    println!("DONE done=1");\n    return;', 1),
    }
    probe_rows = []
    for name, source in probes.items():
        work = work_root / name
        requirement = evaluate_requirements(source, work)
        shutil.rmtree(work / "target", ignore_errors=True)
        probe_rows.append({"name": name, "requirement": slim_requirement(requirement),
                           "compiled": bool(requirement.get("runs")),
                           "overall_pass": requirement.get("status") == "bounded_covered_satisfied"})
    alternate = derived.replace(
        "    {\n        let _guard = m.lock().unwrap();\n    }\n    ch1.send(1).unwrap();",
        "    ch1.send(1).unwrap();",
        1)
    # The replacement above only moves the sender lock if the pattern matches once.
    alt_req = evaluate_requirements(
        (NOTES / "strong-link-v12/live/gpt-6-luna/A/source-1.rs").read_text(encoding="utf-8")
        if False else derived, work_root / "control-again")
    (OUT / "REQUIREMENT_ORACLE_REGRESSION.json").write_text(json.dumps({
        "probes": probe_rows,
        "derived_control_status": slim_requirement(evaluate_requirements(derived, work_root / "derived-req")),
        "note": "Probes were compiled and executed. Neither overall status is bounded_covered_satisfied.",
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    header = """
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
"""
    regressions = []
    samples = [
        ("unbounded_vs_zero", header + """
fn s(ch1: Sender<i32>) { ch1.send(1).unwrap(); }
fn main() {
    let (ch1_tx, ch1_rx) = channel::<i32>();
    thread::spawn(move || s(ch1_tx));
    let _ = ch1_rx.recv().unwrap();
}
""", [("ch1", 0)], "mismatch"),
        ("rendezvous_vs_zero", header + """
fn s(ch1: SyncSender<i32>) { ch1.send(1).unwrap(); }
fn main() {
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    thread::spawn(move || s(ch1_tx));
    let _ = ch1_rx.recv().unwrap();
}
""", [("ch1", 0)], "match"),
        ("bounded_one_vs_zero", header + """
fn s(ch1: SyncSender<i32>) { ch1.send(1).unwrap(); }
fn main() {
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(1);
    thread::spawn(move || s(ch1_tx));
    let _ = ch1_rx.recv().unwrap();
}
""", [("ch1", 0)], "mismatch"),
        ("bounded_one_vs_one", header + """
fn s(ch1: SyncSender<i32>) { ch1.send(1).unwrap(); }
fn main() {
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(1);
    thread::spawn(move || s(ch1_tx));
    let _ = ch1_rx.recv().unwrap();
}
""", [("ch1", 1)], "match"),
        ("unused_channel_not_evidence", header + """
fn s(ch1: Sender<i32>, ch2: Receiver<i32>) {
    ch1.send(1).unwrap();
    let _ = ch2.recv().unwrap();
}
fn r(ch1: Receiver<i32>, ch2: Sender<i32>) {
    let _ = ch1.recv().unwrap();
    ch2.send(1).unwrap();
}
fn main() {
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let _ = (ch1_tx, ch1_rx);
    let (other_tx, other_rx) = channel::<i32>();
    let (back_tx, back_rx) = channel::<i32>();
    thread::spawn(move || s(other_tx, back_rx));
    thread::spawn(move || r(other_rx, back_tx));
}
""", [("ch1", 0), ("ch2", 0)], "mismatch"),
        ("renamed_endpoint", header + """
fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    ch1.send(1).unwrap();
    let _ = ch2.recv().unwrap();
}
fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    let _ = ch1.recv().unwrap();
    ch2.send(1).unwrap();
}
fn main() {
    let (left, right) = sync_channel::<i32>(0);
    let (back_tx, back_rx) = sync_channel::<i32>(0);
    let (ch1_tx, ch1_rx) = channel::<i32>();
    let _ = (ch1_tx, ch1_rx);
    thread::spawn(move || s(left, back_rx));
    thread::spawn(move || r(right, back_tx));
}
""", [("ch1", 0), ("ch2", 0)], "match"),
        ("unknown_capacity", header + """
fn s(ch1: SyncSender<i32>) { ch1.send(1).unwrap(); }
fn main() {
    let n = 1;
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(n);
    thread::spawn(move || s(ch1_tx));
    let _ = ch1_rx.recv().unwrap();
}
""", [("ch1", 0)], "unknown"),
    ]
    for name, source, caps, expect in samples:
        row = bind_program(name, source, caps)
        status = None
        for attr in row.get("attributes") or []:
            if attr.get("resource_id") == "main::ch1":
                status = attr.get("status")
        row["expected_status"] = expect
        row["observed_status"] = status
        row["passed"] = status == expect
        regressions.append(row)
        print("sem", name, status, flush=True)
    gpt = (V12 / "live/gpt-6-luna/C/source-1.rs").read_text(encoding="utf-8")
    barrier = bind_program("gpt_c_barrier", gpt, [("ch1", 0), ("ch2", 0)])
    regressions.append({**barrier, "expected_uncovered": "Barrier"})
    (OUT / "RESOURCE_SEMANTICS_REGRESSION.json").write_text(
        json.dumps(regressions, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("wrote", OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
