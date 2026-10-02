#!/usr/bin/env python3
"""v29 offline replay of frozen v28 G3 candidates (no model calls).

Re-evaluates each frozen G3 candidate (GPT 6 Luna / GLM 5.3 Flash) with the
v28-verified toolchain rebuilt for v29, into a NEW directory. It never overwrites
the v28 run and never calls a model. The output is a per-cell old/new diff that
separates "became evaluable / binding recovered" from "code is correct".
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import traceback
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow import candidate_eval  # noqa: E402

MODELS = ("gpt-6-luna", "glm-5.3-flash")


def _last_round_result(cell: Path) -> dict | None:
    rounds = sorted(cell.glob("code/round-*/result.json"))
    if not rounds:
        return None
    try:
        return json.loads(rounds[-1].read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def _cell_summary(old: dict | None, state: dict | None) -> dict:
    ledger = (old or {}).get("ledger") or {}
    monitor = (old or {}).get("monitor") or {}
    binding = (old or {}).get("binding") or {}
    return {
        "current_evaluation": ledger.get("current_evaluation"),
        "delivery_status": ledger.get("delivery_status"),
        "evidence_grade": ledger.get("evidence_grade"),
        "monitor": monitor.get("properties"),
        "monitor_status": monitor.get("status"),
        "binding_verified": sorted((binding.get("mapping") or {}).keys()),
        "binding_ambiguous": _ambig(binding),
        "stages": (old or {}).get("stages"),
        "functional_status": ((old or {}).get("functional") or {}).get("status"),
        "followup_status": ((old or {}).get("followup") or {}).get("status"),
        "followup_action": ((old or {}).get("followup") or {}).get("action"),
        "reasons": ledger.get("reasons") or [],
    }


def _ambig(binding: dict) -> list:
    return [row.get("rust") if isinstance(row, dict) else str(row)
            for row in (binding.get("ambiguous") or [])]


def _verdict(result: dict) -> dict:
    ledger = result.get("ledger") or {}
    monitor = result.get("monitor") or {}
    binding = result.get("binding") or {}
    return {
        "current_evaluation": ledger.get("current_evaluation"),
        "delivery_status": ledger.get("delivery_status"),
        "evidence_grade": ledger.get("evidence_grade"),
        "monitor": monitor.get("properties"),
        "monitor_status": monitor.get("status"),
        "binding_verified": sorted((binding.get("mapping") or {}).keys()),
        "binding_ambiguous": _ambig(binding),
        "stages": result.get("stages"),
        "functional_status": (result.get("functional") or {}).get("status"),
        "followup_status": (result.get("followup") or {}).get("status"),
        "followup_action": (result.get("followup") or {}).get("action"),
        "reasons": ledger.get("reasons") or [],
        "toolchain": {
            "backend_sha256": result.get("backend_sha256"),
            "instrument_sha256": result.get("instrument_sha256"),
            "binding_sha256": result.get("binding_sha256"),
        },
    }


def _monitor_diff(old_props, new_props) -> list[dict]:
    old = {p[0]: p[1] for p in (old_props or []) if isinstance(p, (list, tuple)) and p}
    new = {p[0]: p[1] for p in (new_props or []) if isinstance(p, (list, tuple)) and p}
    out = []
    for key in sorted(set(old) | set(new)):
        if old.get(key) != new.get(key):
            out.append({"property": key, "old": old.get(key), "new": new.get(key)})
    return out


def replay_cell(base: Path, out_root: Path, model: str, cell: Path, *, binary: Path,
                instrument: Path, bind_check: Path, n_runs: int = 32,
                timeout: float = 10.0, force: bool = False) -> dict:
    rel = cell.relative_to(base)
    task_id = rel.parts[0]
    old = _last_round_result(cell)
    entry: dict = {
        "model": model, "task": task_id, "cell": str(rel),
        "old": _cell_summary(old, None),
    }
    if old is None or not old.get("source_path"):
        entry["replay"] = "no_candidate"
        entry["note"] = "no code-stage candidate was frozen for this cell"
        return entry

    source_path = Path(old["source_path"])
    cir_path = Path(old["cir_path"])
    contract_path = Path(old["contract_path"])
    round_no = old.get("round_no")
    out = out_root / str(rel) / f"round-{round_no}"
    result_path = out / "result.json"
    entry["cir_path"] = str(cir_path)
    entry["source_sha256"] = old.get("source_sha256")

    if result_path.is_file() and not force:
        new = json.loads(result_path.read_text(encoding="utf-8"))
        entry["replay"] = "cached"
    else:
        if not source_path.is_file():
            entry["replay"] = "missing_source"
            return entry
        source = source_path.read_text(encoding="utf-8")
        try:
            new = candidate_eval.evaluate_candidate(
                source, cir_path, contract_path, out,
                binary=binary, instrument=instrument, n_runs=n_runs,
                run_timeout=timeout, functional_spec=None, cell_id=task_id,
                candidate_kind="offline_replay", round_no=round_no,
                binding_binary=bind_check)
            entry["replay"] = "ok"
        except Exception as exc:  # noqa: BLE001
            entry["replay"] = "error"
            entry["error"] = f"{type(exc).__name__}: {exc}"
            entry["traceback"] = traceback.format_exc()[-1500:]
            return entry

    entry["new"] = _verdict(new)
    entry["monitor_diff"] = _monitor_diff(
        entry["old"].get("monitor"), entry["new"].get("monitor"))
    entry["changed"] = (
        entry["old"].get("current_evaluation") != entry["new"].get("current_evaluation")
        or entry["old"].get("delivery_status") != entry["new"].get("delivery_status")
        or bool(entry["monitor_diff"])
        or entry["old"].get("binding_ambiguous") != entry["new"].get("binding_ambiguous")
        or entry["old"].get("followup_status") != entry["new"].get("followup_status")
    )
    entry["causes"] = _causes(entry)
    return entry


def _causes(entry: dict) -> list:
    """Label a change by the repaired capability, from the observed diff only."""
    old, new = entry["old"], entry["new"]
    causes = []
    old_mon = {p[0]: p[1] for p in (old.get("monitor") or []) if isinstance(p, (list, tuple)) and p}
    new_mon = {p[0]: p[1] for p in (new.get("monitor") or []) if isinstance(p, (list, tuple)) and p}
    if any(v == "unsupported" and new_mon.get(k) in ("PASS_bounded", "FAIL", "not_observed")
           for k, v in old_mon.items()):
        causes.append("value_comparison_recovered")
    if old.get("stages", {}).get("instrumented_build") == "failed" \
            and new.get("stages", {}).get("instrumented_build") == "ok":
        causes.append("cross_module_instrumentation")
    if old.get("binding_ambiguous") and not new.get("binding_ambiguous"):
        causes.append("identity_or_binding_recovered")
    if new.get("followup_status") == "channel_endpoint_protocol":
        causes.append("channel_naming_protocol_feedback")
    if any("resource attributes are unknown" in r for r in (old.get("reasons") or [])) \
            and not any("resource attributes are unknown" in r for r in (new.get("reasons") or [])):
        causes.append("channel_identity_or_attributes")
    if (old.get("current_evaluation"), old.get("delivery_status")) \
            != (new.get("current_evaluation"), new.get("delivery_status")) and not causes:
        causes.append("verdict_changed")
    return causes


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="experiments/strong-link-v28-main")
    ap.add_argument("--out", default="experiments/strong-link-v29-replay")
    ap.add_argument("--toolchain", required=True)
    ap.add_argument("--models", default=",".join(MODELS))
    ap.add_argument("--n-runs", type=int, default=32)
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--only", default="", help="substring filter on the cell path")
    args = ap.parse_args()

    base = (REPO / args.base).resolve()
    out_root = (REPO / args.out).resolve()
    out_root.mkdir(parents=True, exist_ok=True)
    tool = Path(args.toolchain)
    binary = tool / "concir-backend"
    instrument = tool / "concir-instrument"
    bind_check = tool / "bind_check"
    for path in (binary, instrument, bind_check):
        if not path.is_file():
            print(f"missing tool: {path}", file=sys.stderr)
            return 2

    entries: list[dict] = []
    for model in args.models.split(","):
        for cell in sorted(base.glob(f"*/{model}/G3_concir/rep0")):
            if args.only and args.only not in str(cell):
                continue
            entry = replay_cell(base, out_root, model, cell, binary=binary,
                                instrument=instrument, bind_check=bind_check,
                                n_runs=args.n_runs, force=args.force)
            entries.append(entry)
            print(f"{entry['replay']:14} {model:14} {entry['task']}", flush=True)
            # Persist incrementally so an interrupted run can resume.
            (out_root / "OFFLINE_REPLAY_DIFF.json").write_text(
                json.dumps(_aggregate(entries, tool), indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8")

    doc = _aggregate(entries, tool)
    (out_root / "OFFLINE_REPLAY_DIFF.json").write_text(
        json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(doc["summary"], indent=2, ensure_ascii=False))
    return 0


def _aggregate(entries: list[dict], tool: Path) -> dict:
    replayed = [e for e in entries if e["replay"] in {"ok", "cached"}]
    changed = [e for e in replayed if e.get("changed")]
    recovered = [
        e for e in replayed
        if (e["old"].get("current_evaluation"), e["old"].get("delivery_status"))
        != (e["new"].get("current_evaluation"), e["new"].get("delivery_status"))
    ]
    return {
        "toolchain": {
            "dir": str(tool),
            "backend_sha256": _sha(tool / "concir-backend"),
            "instrument_sha256": _sha(tool / "concir-instrument"),
            "binding_sha256": _sha(tool / "bind_check"),
        },
        "summary": {
            "cells": len(entries),
            "replayed": len(replayed),
            "no_candidate": sum(1 for e in entries if e["replay"] == "no_candidate"),
            "errors": sum(1 for e in entries if e["replay"] == "error"),
            "changed": len(changed),
            "verdict_changed": len(recovered),
        },
        "entries": entries,
    }


def _sha(path: Path) -> str | None:
    import hashlib
    if not path.is_file():
        return None
    return hashlib.sha256(path.read_bytes()).hexdigest()


if __name__ == "__main__":
    raise SystemExit(main())
