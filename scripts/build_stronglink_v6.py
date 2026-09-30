#!/usr/bin/env python3
"""Rebuild the strong-link-v6 matrix from read-only v5 evidence.

Execution traces are reused when their hashes still match. Explore, conform,
monitor, and bind_check are spot-run only where the raw checker output is
missing or the concir-backend hash changed. v5 files are not modified.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO))

from cir_workflow.binding import BindingUnavailable, bind as binding_bind  # noqa: E402
from cir_workflow.candidate_eval import run_functional_check, run_model_check  # noqa: E402
from cir_workflow.evidence_v2 import PROTOCOL, binding_assessment, evaluate_reexecution  # noqa: E402
from cir_workflow.generation import _conform_all_op_resource  # noqa: E402
from cir_workflow import bounded_monitor  # noqa: E402

NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
V5 = NOTES / "strong-link-v5"
V6 = NOTES / "strong-link-v6"
SLUG = {
    "DeepSeek Flash": "deepseekflash",
    "Qwen": "qwen",
    "GPT 6 Luna": "gpt6luna",
    "Kimi": "kimi",
}


def _sha(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def _art(role: str, path: Path, binds: dict) -> dict | None:
    digest = _sha(path)
    if digest is None:
        return None
    return {"role": role, "path": str(path), "sha256": digest, "binds": binds}


def _missing_reason(row: dict) -> str:
    if str(row.get("status", "")).startswith("error"):
        return "cir_unknown_or_raw_error"
    if not row.get("cir_accepted"):
        return "cir_not_accepted"
    return "cir_accepted_no_rust"


def _write_missing(out: Path, rel: str, row: dict, reason: str) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    accepted = bool(row.get("accepted"))
    res = {
        "cell": rel, "protocol": PROTOCOL, "candidate_kind": "final",
        "round_no": None, "accepted_round": None, "final_round": None,
        "same_round": True, "historical_accepted": accepted,
        "model": row["model"], "task": row["task"], "rep": row.get("replicate", 0),
        "origin_batch": row.get("_batch"),
        "stages": {"pre": reason}, "runs_started": 0, "runs_completed": 0,
        "functional": {"status": "not_run", "reason": "no_source"},
        "spot_rerun": [], "reused": [],
        "ledger": {
            "protocol": PROTOCOL, "cell": rel, "historical_acceptance": accepted,
            "current_evaluation": reason, "evidence_grade": "not_executed",
            "delivery_status": "not_a_candidate", "needs_human_review": False,
            "needs_extra_check": False, "run": {"state": "not_run"},
            "trace": {"state": "incomplete"}, "reasons": [reason],
            "functional": {"status": "not_run"},
        },
        "binding_assessment": {"status": "not_run", "sufficient": False, "ran": False},
        "evidence_path": str(out / "result.json"),
    }
    (out / "result.json").write_text(json.dumps(res, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
    return res


def _link_execution(v5: dict, binds: dict) -> tuple[list[dict], list[str]]:
    arts, problems = [], []
    for art in v5.get("artifacts") or []:
        if art.get("role") != "execution":
            continue
        path = Path(art["path"])
        digest = _sha(path)
        if digest is None or digest != art.get("sha256"):
            problems.append(str(path))
            continue
        arts.append({"role": "execution", "path": str(path), "sha256": digest, "binds": binds})
    return arts, problems


def _projections(v5_dir: Path, binds: dict) -> list[dict]:
    directory = v5_dir / "conform-traces"
    arts = []
    if not directory.is_dir():
        return arts
    for path in sorted(directory.glob("*.jsonl")):
        art = _art("projection", path, binds)
        if art:
            arts.append(art)
    return arts


def assemble_cell(row: dict, *, binary: Path, instrument: Path) -> dict:
    slug = SLUG[row["model"]]
    rel = Path(row["model"]) / row["task"].replace("/", "__") / f"rep{row.get('replicate', 0)}"
    v5_dir = V5 / f"reexec-{slug}" / rel
    out = V6 / f"reexec-{slug}" / rel
    out.mkdir(parents=True, exist_ok=True)
    v5_result_path = v5_dir / "result.json"
    batch_cell = REPO / row["_batch"] / rel
    rusts = sorted(batch_cell.glob("code/round-*.rs"),
                   key=lambda p: int(p.stem.split("-")[1])) if batch_cell.is_dir() else []
    cirs = sorted(batch_cell.glob("cir/revision-*.cir.json"),
                  key=lambda p: int(p.stem.split("-")[1].split(".")[0])) if batch_cell.is_dir() else []
    accepted = bool(row.get("accepted"))
    acc_round = row.get("first_code_accept_round")
    final_round = int(rusts[-1].stem.split("-")[1]) if rusts else None
    accepted_path = batch_cell / "code" / f"round-{acc_round}.rs" if accepted and acc_round else None
    if accepted_path and not accepted_path.is_file():
        accepted_path = None
    chosen = accepted_path or (rusts[-1] if rusts else None)
    same = bool(chosen and rusts and chosen.resolve() == rusts[-1].resolve())
    if chosen is None or not cirs or not v5_result_path.is_file():
        reason = _missing_reason(row) if chosen is None or not cirs else "cir_accepted_no_rust"
        if not v5_result_path.is_file() and chosen is None:
            return _write_missing(out, str(rel), row, _missing_reason(row))
        if chosen is None or not cirs:
            return _write_missing(out, str(rel), row, reason)

    v5 = json.loads(v5_result_path.read_text(encoding="utf-8"))
    source_path = Path(v5["source_path"])
    cir_path = Path(v5["cir_path"])
    contract_path = Path(v5["contract_path"])
    source_sha, cir_sha, contract_sha = _sha(source_path), _sha(cir_path), _sha(contract_path)
    if source_sha != v5.get("source_sha256") or cir_sha != v5.get("cir_sha256"):
        raise RuntimeError(f"v5 identity drift: {rel}")
    backend_sha, instrument_sha = _sha(binary), _sha(instrument)
    n_runs = int(v5.get("n_runs") or 32)
    acquisition = hashlib.sha256(json.dumps({
        "source": source_sha, "cir": cir_sha, "contract": contract_sha,
        "backend": backend_sha, "instrument": instrument_sha, "n_runs": n_runs,
    }, sort_keys=True).encode()).hexdigest()
    binds = {"acquisition_id": acquisition, "source_sha256": source_sha,
             "cir_sha256": cir_sha, "contract_sha256": contract_sha,
             "backend_sha256": backend_sha}
    spot, reused = [], ["v5 source/cir/contract hashes"]
    if v5.get("backend_sha256") != backend_sha:
        spot.append({"stage": "model_check",
                     "reason": "concir-backend hash changed and v5 model-check.json is a rebuilt summary"})
        spot.append({"stage": "conform",
                     "reason": "concir-backend hash changed and v5 conform artifacts are projected traces, not checker stdout"})
        spot.append({"stage": "monitor",
                     "reason": "concir-backend hash changed since the v5 monitor run"})
    spot.append({"stage": "binding",
                 "reason": "bind_check raw stdout was not retained in v5"})

    artifacts = []
    for role, path, digest in (("source", source_path, source_sha),
                               ("cir", cir_path, cir_sha),
                               ("contract", contract_path, contract_sha)):
        artifacts.append({"role": role, "path": str(path), "sha256": digest, "binds": binds})
    kind = "accepted" if accepted and accepted_path is not None else "final"
    eval_round = int(chosen.stem.split("-")[1])
    result = {
        "cell": str(rel), "protocol": PROTOCOL, "candidate_kind": kind,
        "round_no": eval_round, "accepted_round": acc_round if accepted else None,
        "final_round": final_round, "same_round": same,
        "historical_accepted": accepted, "model": row["model"], "task": row["task"],
        "rep": row.get("replicate", 0), "origin_batch": row.get("_batch"),
        "source_path": str(source_path), "cir_path": str(cir_path),
        "contract_path": str(contract_path),
        "source_sha256": source_sha, "cir_sha256": cir_sha, "contract_sha256": contract_sha,
        "backend_sha256": backend_sha, "instrument_sha256": instrument_sha,
        "historical_backend_sha256": v5.get("backend_sha256"),
        "acquisition_id": acquisition, "n_runs": n_runs,
        "stages": dict(v5.get("stages") or {}),
        "runs_started": v5.get("runs_started", 0), "runs_completed": v5.get("runs_completed", 0),
        "hang": v5.get("hang", False), "resources": v5.get("resources") or [],
        "spot_rerun": spot, "reused": reused,
    }
    if (v5.get("stages") or {}).get("source_build") == "failed":
        result["functional"] = run_functional_check(source_path.read_text(encoding="utf-8"),
                                                    out / "functional", None)
        result["artifacts"] = artifacts
        result["stages"]["binding"] = "not_run"
        return _interpret(out, result, None, None)

    exec_arts, problems = _link_execution(v5, binds)
    if problems or not exec_arts:
        raise RuntimeError(f"execution evidence not reusable for {rel}: {problems[:3]}")
    reused.append("v5 execution traces (hash-checked)")
    artifacts.extend(exec_arts)
    artifacts.extend(_projections(v5_dir, binds))
    reused.append("v5 projected traces as checker inputs")

    model = run_model_check(binary, cir_path, contract_path, out)
    if not model.get("ok"):
        result["stages"]["model_check"] = f"tool_error: {model.get('error')}"
    art = _art("model_check", Path(model["path"]), binds)
    if art:
        artifacts.append(art)
    payload = model.get("payload") or {}
    props = {str(p["id"]).replace("preserved: ", ""): p.get("outcome")
             for p in payload.get("properties", []) if isinstance(p, dict) and "id" in p}
    complete = payload.get("complete")
    # Historical explore stdout stays in the original cell; record the path only.
    historical = _historical_explore(batch_cell, cir_sha)
    if historical:
        result["historical_explore"] = historical
        reused.append("historical explore stdout path (not the current verdict)")

    resources = v5_dir / "instrument/resources.json"
    try:
        rb = binding_bind(resources, cir_path)
    except BindingUnavailable as exc:
        result["stages"]["binding"] = f"tool_error: {exc}"
        result["binding"] = {"mapping": {}, "ambiguous": [], "violated": {},
                             "source": "rust-cli-unavailable", "error": str(exc)}
        result["artifacts"] = artifacts
        result["functional"] = {"status": "not_run", "reason": "no_functional_spec"}
        return _interpret(out, result, props if model.get("ok") else None,
                          complete if model.get("ok") else None)
    raw_bind = rb.pop("_raw_stdout", None)
    bind_path = out / "binding-check.stdout"
    bind_path.write_text(raw_bind if isinstance(raw_bind, str) else json.dumps(rb),
                         encoding="utf-8")
    verified = rb.get("verified") or {}
    unresolved = rb.get("unresolved") or {}
    mapping = {name: value["cir"] for name, value in verified.items()
               if isinstance(value, dict) and "cir" in value}
    ambiguous = [{"rust": key, **value} for key, value in unresolved.items()] \
        if isinstance(unresolved, dict) else []
    result["stages"]["binding"] = "ok"
    result["binding"] = {"mapping": mapping, "ambiguous": ambiguous,
                         "violated": rb.get("violated") or {}, "source": "rust-cli"}
    art = _art("binding_check", bind_path, binds)
    if art:
        artifacts.append(art)
    if resources.is_file():
        try:
            result["resources"] = json.loads(resources.read_text(encoding="utf-8")).get("resources") or []
        except json.JSONDecodeError:
            pass

    conform = _conform_all_op_resource(binary, cir_path, v5_dir / "conform-traces")
    conform_doc = {"checks": [{"trace_sha256": item.get("trace_sha256"),
                               "stdout": item.get("stdout", "")}
                              for item in conform.get("raw") or []]}
    conform_path = out / "conform-check.json"
    conform_path.write_text(json.dumps(conform_doc) + "\n", encoding="utf-8")
    result["conform"] = {key: conform.get(key) for key in
                         ("traces", "statuses", "violations", "first_violation")}
    result["projected_events"] = _count(v5_dir / "conform-traces")
    art = _art("conform_check", conform_path, binds)
    if art:
        artifacts.append(art)

    mapping_path = out / "binding.json"
    mapping_path.write_text(json.dumps({"mapping": mapping}) + "\n", encoding="utf-8")
    try:
        report = bounded_monitor.run_monitor(
            contract_path, v5_dir / "monitor-traces", resources=resources,
            mapping=mapping_path, binary=binary)
        raw_mon = report.pop("_raw_stdout", None)
        mon_path = out / "monitor.stdout"
        mon_path.write_text(raw_mon if isinstance(raw_mon, str) else json.dumps(report),
                            encoding="utf-8")
        pairs = []
        data = json.loads(mon_path.read_text(encoding="utf-8"))
        for prop in data.get("properties") or []:
            if isinstance(prop, dict):
                pairs.append((prop.get("id"), prop.get("status")))
        result["monitor"] = {"status": data.get("status"), "properties": pairs}
        art = _art("monitor", mon_path, binds)
        if art:
            artifacts.append(art)
    except (OSError, RuntimeError, json.JSONDecodeError) as exc:
        result["stages"]["monitor"] = f"tool_error: {exc}"

    result["functional"] = run_functional_check(source_path.read_text(encoding="utf-8"),
                                                out / "functional", None)
    result["artifacts"] = artifacts
    return _interpret(out, result, props if model.get("ok") else None,
                      complete if model.get("ok") else None)


def _count(directory: Path) -> int:
    total = 0
    if not directory.is_dir():
        return 0
    for path in directory.glob("*.jsonl"):
        total += len([ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()])
    return total


def _historical_explore(cell: Path, cir_sha: str) -> str | None:
    calls = cell / "cir/calls"
    if not calls.is_dir():
        return None
    for directory in sorted(calls.glob("*-explore")):
        program, stdout = directory / "program.json", directory / "stdout.json"
        if program.is_file() and stdout.is_file() and _sha(program) == cir_sha:
            return str(stdout)
    return None


def _interpret(out: Path, result: dict, props, complete) -> dict:
    try:
        contract = json.loads(Path(result["contract_path"]).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError, KeyError):
        contract = {"properties": [], "preserved": []}
    ledger = evaluate_reexecution(result, contract, accepted=bool(result.get("historical_accepted")),
                                  cir_props=props, cir_complete=complete)
    result["ledger"] = ledger.to_dict()
    result["binding_assessment"] = binding_assessment(result)
    result["evidence_path"] = str(out / "result.json")
    if result.get("same_round"):
        result["final_evidence_path"] = result["evidence_path"]
        result["final_evaluation"] = ledger.current_evaluation
    (out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--task", action="append", default=[])
    parser.add_argument("--binary", default=str(REPO.parent / "ConcIR/target/release/concir-backend"))
    parser.add_argument("--instrument", default=str(REPO.parent / "ConcIR/target/release/concir-instrument"))
    args = parser.parse_args()
    rows = []
    summaries = {}
    manifest_rows = []
    for line in (V5 / "INPUT_MANIFEST.jsonl").read_text(encoding="utf-8").splitlines():
        if line.strip():
            manifest_rows.append(json.loads(line))
    for batch in sorted({r["batch"] for r in manifest_rows}):
        for cell in json.loads((REPO / batch / "SUMMARY.json").read_text(encoding="utf-8"))["cells"]:
            summaries[(cell["model"], cell["task"], cell.get("replicate", 0))] = cell
    for row in manifest_rows:
        row["_batch"] = row["batch"]
        src = summaries[(row["model"], row["task"], row["rep"])]
        row["status"] = src.get("status")
        row["cir_accepted"] = src.get("cir_accepted")
        row["accepted"] = src.get("accepted")
        row["first_code_accept_round"] = src.get("first_code_accept_round")
        rows.append(row)
    if args.task:
        rows = [r for r in rows if r["task"] in args.task]
    binary, instrument = Path(args.binary), Path(args.instrument)
    failed = 0
    for index, row in enumerate(rows, 1):
        try:
            res = assemble_cell(row, binary=binary, instrument=instrument)
            verdict = (res.get("ledger") or {}).get("current_evaluation")
            print(f"[{index}/{len(rows)}] {res.get('cell')} {verdict}", flush=True)
        except Exception as exc:  # noqa: BLE001
            failed += 1
            print(f"[{index}/{len(rows)}] FAIL {row['model']} {row['task']}: {type(exc).__name__}: {exc}",
                  flush=True)
    print(f"done failures={failed}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
