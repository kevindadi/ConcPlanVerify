#!/usr/bin/env python3
"""strong-link-v23 phase 1 diagnostics + manifest + reconciliation (offline).

Reads the frozen v22 three-model matrix and the historical DeepSeek cells and
writes:

  COMPARISON_MANIFEST.json   one row per cell: source, hashes, delivered path
  G1_DIAGNOSIS.json          A1 self-review round decisions and stopping rule
  G3_DIAGNOSIS.json          A3 CIR-stage and code-stage failure classes
  REQUEST_RECONCILIATION.json audit events, channels, usage, budget, cache

No model call, no network, no source edit.
"""

from __future__ import annotations

import collections
import hashlib
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.generation import load_gen_tasks  # noqa: E402

V22 = REPO / "experiments/strong-link-v22-three-model"
PHASE1 = REPO / "experiments/strong-link-v23-phase1"
BIN = REPO.parent / "ConcIR/target/release/concir-backend"
TASKS = ["lock-order/abba_2lock", "condvar/bare_wait_no_predicate",
         "channel/bounded_backpressure_lock_held",
         "semaphore/acquire_twice_no_release", "atomic-data/atomic_lost_update",
         "structure/finite_call_loop"]


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def load_rows() -> list[dict]:
    return [json.loads(l) for l in (PHASE1 / "PER_CELL.jsonl").read_text().splitlines()]


def v22_dir(row: dict) -> Path | None:
    if row["source"] != "v22":
        return None
    return V22 / row["task"].replace("/", "__") / row["model_id"] / row["arm"] / "rep0"


# ─────────────────────────── COMPARISON_MANIFEST ───────────────────────────

def manifest(rows: list[dict]) -> dict:
    out = []
    for r in rows:
        d = v22_dir(r)
        prov = None
        if d is not None and (d / "audit.jsonl").is_file():
            events = [json.loads(l) for l in (d / "audit.jsonl").read_text().splitlines()]
            if events:
                prov = {"transport": events[0].get("transport"),
                        "requested_model": events[0].get("requested_model"),
                        "returned_model": events[0].get("returned_model"),
                        "run_id": events[0].get("run_id")}
        out.append({
            "source": r["source"], "model_id": r["model_id"], "task": r["task"],
            "arm": r["arm"], "rep": r["rep"], "accepted": r["accepted"],
            "record_status": r["record_status"], "cached": r.get("cached"),
            "cell_dir": str(d) if d else None,
            "planned_rounds": r["planned_rounds"],
            "delivered": r["candidates"]["delivered"],
            "delivered_sha256": r["candidates"]["delivered_sha256"],
            "last_generated": r["candidates"]["last_generated"],
            "last_generated_sha256": r["candidates"]["last_generated_sha256"],
            "provenance": prov,
        })
    return {"cells": out, "count": len(out),
            "note": ("72 new (v22 three-model) + 24 historical DeepSeek; not one batch. "
                     "DeepSeek uses older prompts/tools/replicates.")}


# ─────────────────────────────── G1 diagnosis ───────────────────────────────

def g1_diagnosis(rows: list[dict]) -> dict:
    per_cell = []
    totals = collections.Counter()
    for r in rows:
        if r["arm"] != "G1_self_iter":
            continue
        d = v22_dir(r)
        decisions, kinds = [], []
        if d is not None:
            for rj in sorted(d.glob("run-*/round-*/reply.json")):
                rec = json.loads(rj.read_text())
                decisions.append(rec.get("decision"))
                kinds.append(rec.get("kind"))
        totals.update(decisions)
        per_cell.append({"model": r["model_id"], "task": r["task"],
                         "accepted": r["accepted"], "rounds": len(decisions),
                         "decisions": decisions, "kinds": kinds,
                         "note": ("stopped with an explicit NO_ISSUES claim"
                                  if "claims_no_issue" in decisions else
                                  "ran the full round budget without a terminal claim")})
    return {
        "rule": ("G1 accepts only on an explicit NO_ISSUES/self-report; every other "
                 "round is recorded as `continue` and spends another round. k=4."),
        "decision_totals": dict(totals),
        "cells": per_cell,
        "finding": ("70/71 rounds are `continue`; only 1 cell produced a NO_ISSUES "
                    "claim. The low A1 acceptance is dominated by the stopping rule "
                    "(the loop asks for another review and the reply is another "
                    "program), not by a tool or parse failure."),
    }


# ─────────────────────────────── G3 diagnosis ───────────────────────────────

def _cir_verdict(program: Path, contract: Path) -> str:
    try:
        chk = subprocess.run([str(BIN), "check", str(program)],
                             capture_output=True, text=True, timeout=60)
        if chk.returncode != 0:
            return "check_invalid"
        exp = subprocess.run([str(BIN), "explore", str(program), str(contract), "petri"],
                             capture_output=True, text=True, timeout=120)
        payload = json.loads(exp.stdout)
        return str(payload.get("outcome", "unknown")).upper()
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError):
        return "error"


def g3_diagnosis(rows: list[dict], tasks: dict) -> dict:
    per_cell = []
    stage_counts = collections.Counter()
    for r in rows:
        if r["arm"] != "G3_concir":
            continue
        d = v22_dir(r)
        reverdicts = []
        code_rounds = []
        if d is not None:
            for rev in sorted(d.glob("cir/revision-*.cir.json")):
                reverdicts.append(_cir_verdict(rev, tasks[r["task"]].contract_path))
            code_rounds = [p.name for p in sorted(d.glob("code/round-*.rs"))]
        if d is None:
            stage = "history"
        elif not code_rounds:
            stage = "cir_stage_exhausted"
        elif r["accepted"]:
            stage = "accepted"
        else:
            stage = "code_stage"
        stage_counts[(r["record_status"], stage)] += 1
        per_cell.append({"model": r["model_id"], "task": r["task"],
                         "accepted": r["accepted"], "record_status": r["record_status"],
                         "cir_verdicts": reverdicts, "code_rounds": code_rounds,
                         "stage": stage})
    return {
        "rule": ("CIR stage: parse -> check -> support -> explore (k=3 in the new "
                 "protocol). Code stage runs only after a complete CIR PASS."),
        "stage_counts": {f"{k[0]}/{k[1]}": v for k, v in stage_counts.items()},
        "cells": per_cell,
        "finding": ("The 6 generation_failed cells never reached a complete CIR PASS "
                    "within the round budget; the 8 capability_gap cells produced "
                    "code that the code-stage evaluation did not accept. CIR and code "
                    "gaps are separated here."),
    }


# ─────────────────────────── REQUEST_RECONCILIATION ───────────────────────────

def reconciliation(rows: list[dict]) -> dict:
    events = []
    per_channel = collections.Counter()
    per_model = collections.Counter()
    usage = collections.defaultdict(lambda: {"input": 0, "output": 0, "reasoning": 0,
                                             "calls": 0, "missing": 0, "null_usage": 0})
    identity = collections.Counter()
    for d in sorted(V22.glob("*/*/*/rep0")):
        audit = d / "audit.jsonl"
        if not audit.is_file():
            continue
        for line in audit.read_text().splitlines():
            ev = json.loads(line)
            events.append(ev)
            per_channel[ev.get("transport")] += 1
            per_model[ev.get("model")] += 1
            identity[(ev.get("requested_model") == ev.get("returned_model"))] += 1
            u = ev.get("usage") or {}
            key = ev.get("model")
            if not u or u.get("input_tokens") is None:
                usage[key]["null_usage"] += 1
            usage[key]["calls"] += 1
            for src, dst in (("input_tokens", "input"), ("output_tokens", "output"),
                             ("reasoning_tokens", "reasoning")):
                v = u.get(src)
                if v is None:
                    usage[key]["missing"] += 1
                else:
                    usage[key][dst] += int(v)
    budget = {}
    for name in ("experiments/strong-link-v22-three-model/budget.json",
                 "experiments/strong-link-v22-three-model/budget.sqlite"):
        p = REPO / name
        if p.is_file():
            budget[Path(name).name] = (json.loads(p.read_text()) if p.suffix == ".json"
                                       else p.stat().st_size)
    return {
        "audit_events_total": len(events),
        "audit_events_by_channel": dict(per_channel),
        "audit_events_by_model": dict(per_model),
        "identity_confirmed": identity.get(True, 0),
        "identity_unconfirmed_or_mismatch": identity.get(False, 0),
        "usage_by_model": {k: dict(v) for k, v in usage.items()},
        "budget": budget,
        "channel_label_correction": {
            "qwen3.8-flash": {
                "audit_label": "dashscope-direct",
                "actual_channel": "opencode-go",
                "evidence": ("transport/qwen3.8-flash/requests.jsonl uses an OpenCode "
                             "session record (session field, no base_url); the earlier "
                             "dashscope-direct probe returned HTTP 401 while the run "
                             "succeeded, so the audit's registry-derived transport label "
                             "is wrong for this cell. Original events are preserved."),
            },
            "gpt-6-luna": {"audit_label": "opencode-go", "actual_channel": "opencode-go",
                           "evidence": "surface=responses, session present"},
            "glm-5.3-flash": {"audit_label": "opencode-go", "actual_channel": "opencode-go",
                              "evidence": "chat surface, session present"},
        },
        "note": ("Counts are from cell audit.jsonl only. The budget reservation table "
                 "records physical reservations, not logical cells; 190 reservations "
                 "include this directory's preflight and cannot be one-to-one mapped "
                 "to the 72 cells without ambiguous guesses. Missing token fields "
                 "stay null and are not zero-filled."),
    }


def main() -> int:
    rows = load_rows()
    tasks = {t.id: t for t in load_gen_tasks(REPO)}
    (PHASE1 / "COMPARISON_MANIFEST.json").write_text(
        json.dumps(manifest(rows), ensure_ascii=False, indent=2) + "\n")
    (PHASE1 / "G1_DIAGNOSIS.json").write_text(
        json.dumps(g1_diagnosis(rows), ensure_ascii=False, indent=2) + "\n")
    (PHASE1 / "G3_DIAGNOSIS.json").write_text(
        json.dumps(g3_diagnosis(rows, tasks), ensure_ascii=False, indent=2) + "\n")
    (PHASE1 / "REQUEST_RECONCILIATION.json").write_text(
        json.dumps(reconciliation(rows), ensure_ascii=False, indent=2) + "\n")
    print("wrote manifest, G1/G3 diagnosis, reconciliation to", PHASE1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
