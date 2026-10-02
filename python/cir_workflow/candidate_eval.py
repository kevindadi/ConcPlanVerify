"""Shared candidate evaluation (strong-link-v6).

Online generation and offline re-evaluation both call ``evaluate_candidate``.
It receives the candidate source, CIR, contract, tool configuration, run budget,
and an optional functional check. It records build, instrumentation, execution,
binding, conformance, monitor, and the functional check, including on early
exits. Interpretation is ``evidence_v2.evaluate_reexecution``.

Online code decides whether to ask the model again. Offline code selects frozen
candidates. Follow-up actions come from the ledger:

- ``repair`` / ``candidate_error``: a confirmed candidate defect, with feedback.
- ``repair_protocol`` / ``protocol_noncompliance``: a fixable protocol miss.
- ``stop`` / ``tool_failure``: tool or evidence failure; do not call the model.
- ``stop`` / ``capability_gap``: the checker cannot decide; do not ask the model
  to change a correct program.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from .evidence_v2 import (
    PROTOCOL, ReexecutionLedger, binding_assessment, evaluate_reexecution,
)

ACCEPTANCE_POLICY = "ledger-v6-followup-v2"


def interpret(result: dict, contract: dict, *, accepted: bool,
              cir_props: dict[str, str] | None = None,
              cir_complete: bool | None = None) -> ReexecutionLedger:
    """The single interpretation rule."""

    return evaluate_reexecution(result, contract, accepted=accepted,
                                cir_props=cir_props, cir_complete=cir_complete)


def _sha_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha_file(path: Path | None) -> str | None:
    if path is None:
        return None
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def capture_program_stdout(binary: Path, timeout: float = 20.0) -> dict[str, Any]:
    """Run one built program and keep the process result.

    A missing binary is tool unavailability. A timeout or nonzero exit is a
    candidate execution result. TimeoutExpired does not escape.
    """

    import time
    path = Path(binary)
    if not path.is_file():
        return {"stdout": None, "stderr": "", "returncode": None, "timed_out": False,
                "elapsed_s": 0.0, "error": "binary_missing", "kind": "tool_unavailable"}
    started = time.perf_counter()
    try:
        proc = subprocess.run([str(path)], capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return {"stdout": stdout, "stderr": stderr, "returncode": None, "timed_out": True,
                "elapsed_s": time.perf_counter() - started, "error": "timeout",
                "kind": "candidate_timeout"}
    except OSError as exc:
        return {"stdout": None, "stderr": "", "returncode": None, "timed_out": False,
                "elapsed_s": time.perf_counter() - started, "error": str(exc),
                "kind": "tool_unavailable"}
    return {"stdout": proc.stdout, "stderr": proc.stderr, "returncode": proc.returncode,
            "timed_out": False, "elapsed_s": time.perf_counter() - started,
            "error": None, "kind": "completed"}


def run_functional_check(source: str, work: Path, spec: dict | None) -> dict:
    """Shared functional check. ``spec is None`` records ``not_run``.

    A stdout check supports only the external output it names. It does not
    prove an internal value property.
    """

    from . import rust_oracle

    source_sha = hashlib.sha256(source.encode()).hexdigest()
    if not spec:
        return {"status": "not_run", "reason": "no_functional_spec",
                "source_sha256": source_sha, "test_id": None,
                "expected": None, "raw_stdout": None, "supports": []}
    work.mkdir(parents=True, exist_ok=True)
    (work / "src").mkdir(exist_ok=True)
    (work / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (work / "src" / "main.rs").write_text(source, encoding="utf-8")
    try:
        built, _log = rust_oracle.cargo_build(work)
    except (OSError, subprocess.SubprocessError) as exc:
        doc = {"status": "not_run", "reason": f"tool_error: {exc}", "source_sha256": source_sha,
               "test_id": spec.get("test_id") or "stdout_eq", "kind": spec.get("kind", "stdout_eq"),
               "expected": spec.get("expected"), "raw_stdout": None, "returncode": None,
               "timed_out": False, "execution_kind": "tool_unavailable"}
        path = work / "functional.json"
        path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
        return {**doc, "evidence": doc["reason"], "evidence_path": str(path), "supports": []}
    run = (capture_program_stdout(work / "target/debug/probe") if built else
           {"stdout": None, "stderr": "", "returncode": None, "timed_out": False,
            "elapsed_s": 0.0, "error": "build_failed", "kind": "candidate_build_failed"})
    expected = spec.get("expected")
    stdout = run.get("stdout")
    stripped = stdout.strip() if isinstance(stdout, str) else None
    if run.get("kind") == "tool_unavailable":
        status, reason = "not_run", run.get("error") or "tool_unavailable"
    elif run.get("timed_out"):
        status, reason = "fail", "timeout"
    elif run.get("returncode") not in (0, None):
        status, reason = "fail", "nonzero_exit"
    elif stripped is None:
        status, reason = "not_run", run.get("error") or "program_did_not_run"
    elif stripped == expected and run.get("returncode") == 0:
        status, reason = "pass", None
    else:
        status, reason = "fail", "stdout_mismatch"
    doc = {"status": status, "reason": reason, "source_sha256": source_sha,
           "test_id": spec.get("test_id") or "stdout_eq",
           "kind": spec.get("kind", "stdout_eq"),
           "expected": expected, "raw_stdout": stripped,
           "stderr": run.get("stderr"), "returncode": run.get("returncode"),
           "timed_out": bool(run.get("timed_out")), "elapsed_s": run.get("elapsed_s"),
           "execution_kind": run.get("kind")}
    path = work / "functional.json"
    path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    evidence = (f"stdout={stripped!r} returncode={run.get('returncode')} "
                f"timed_out={bool(run.get('timed_out'))}")
    return {**doc, "evidence": evidence, "evidence_path": str(path),
            "supports": ["external_stdout"] if status == "pass" else []}


def run_model_check(binary: Path, cir_path: Path, contract_path: Path, out_dir: Path) -> dict:
    """Run explore and keep the raw stdout. A rebuilt PASS summary is not evidence."""

    out_dir.mkdir(parents=True, exist_ok=True)
    raw_path = out_dir / "explore.stdout"
    try:
        proc = subprocess.run([str(binary), "explore", str(cir_path), str(contract_path), "petri"],
                              capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.SubprocessError) as exc:
        raw_path.write_text("", encoding="utf-8")
        return {"ok": False, "path": str(raw_path), "error": str(exc)}
    raw_path.write_text(proc.stdout, encoding="utf-8")
    if proc.returncode != 0 or not proc.stdout.strip():
        return {"ok": False, "path": str(raw_path), "error": (proc.stderr or "")[-300:]}
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        return {"ok": False, "path": str(raw_path), "error": str(exc)}
    return {"ok": True, "path": str(raw_path), "payload": payload}


def _artifact(role: str, path: Path, *, binds: dict) -> dict | None:
    if not path.is_file():
        return None
    return {"role": role, "path": str(path), "sha256": _sha_file(path), "binds": binds}


def _acquisition_id(parts: dict) -> str:
    blob = json.dumps(parts, sort_keys=True, default=str).encode()
    return hashlib.sha256(blob).hexdigest()


def _adaptation_feedback(ledger, result: dict) -> str | None:
    """A safe, CIR-preserving encoding-adaptation request.

    Only when the identity is resolved (a binding exists) but the runtime cannot
    observe a required value/attribute. Returns non-empty feedback so the code
    stage gets a real repair round; it never accepts and never loosens a gate.
    """
    unsupported = [p.property_id for p in getattr(ledger, "properties", [])
                   if p.independent_requirement_result in ("unsupported", "unmapped")]
    if not unsupported:
        return None
    if not (result.get("binding") or {}).get("mapping"):
        return None
    ids = ", ".join(unsupported)
    return (
        "Representation adaptation required for: " + ids + ". The CIR identity is "
        "resolved but the runtime cannot observe the required value/attribute. Expose "
        "the protected variable as a primitive stored through its declared lock (or as "
        "a directly observable field), and keep the exact CIR synchronization "
        "operations, protection relation and thread completion obligations unchanged. "
        "Do not change the requirements or the contract. This is an encoding change; "
        "the program will be re-compiled, re-bound, re-run and re-checked."
    )


def decide_followup(ledger: ReexecutionLedger, result: dict) -> dict[str, Any]:
    """Map a ledger to the next action. Repair feedback is never empty."""

    verdict = ledger.current_evaluation
    if verdict == "source_build_failed":
        log = ""
        log_path = result.get("source_build_log")
        if log_path and Path(log_path).is_file():
            from .generation import _compiler_errors
            log = _compiler_errors(Path(log_path).read_text(encoding="utf-8"))
        feedback = "The Rust source did not compile:\n" + (log or "cargo build failed")
        return _repair("candidate_error", "source_build_failed", feedback)
    if verdict in {"instrument_failed", "instrument_build_failed", "tool_error"} or \
            ledger.delivery_status == "withhold_tool":
        status = "binding_tool_error" if verdict == "tool_error" else (
            "instrument_error" if verdict == "instrument_failed" else
            "instrumented_build_failed" if verdict == "instrument_build_failed" else
            "evidence_invalid")
        return {"action": "stop", "category": "tool_failure", "status": status,
                "feedback": "", "semantic_retry": False, "protocol_retry": False}
    if result.get("instrument_unsupported"):
        return {"action": "stop", "category": "capability_gap",
                "status": "instrument_unsupported", "feedback": "",
                "semantic_retry": False, "protocol_retry": False}
    if verdict == "functional_failure":
        feedback = " ".join(ledger.reasons) or "The observable output does not meet the requirement."
        return _repair("candidate_error", "functional_failure", feedback)
    if verdict == "attribute_conflict":
        feedback = " ".join(ledger.reasons) or "A resource attribute conflicts with the CIR."
        mismatches = [row for row in (result.get("binding") or {}).get("attributes") or []
                      if isinstance(row, dict) and row.get("status") == "mismatch"]
        if mismatches:
            details = [{key: row.get(key) for key in (
                "resource_id", "expected", "expected_capacity", "observed",
                "observed_capacity", "construction_site")} for row in mismatches]
            feedback += "\nConstructor mismatches:\n" + json.dumps(details, ensure_ascii=False)
            feedback += ("\nMatch each constructor to the verified CIR attributes. "
                         "Preserve the synchronization operations and do not change the contract.")
        return _repair("candidate_error", "attribute_conflict", feedback)
    if verdict == "explicit_failure":
        violation = (ledger.trace.get("violations") or [{}])[0]
        from .generation import _conform_kind, _explain_violation
        if violation.get("status") in {"error", "unknown_sid"}:
            return {"action": "stop", "category": "capability_gap", "status": "capability_gap",
                    "feedback": "", "semantic_retry": False, "protocol_retry": False}
        feedback = _explain_violation(violation, _conform_kind(violation))
        if not feedback.strip():
            return _capability()
        return _repair("candidate_error", "explicit_failure", feedback)
    if verdict == "requirement_failure":
        feedback = " ".join(ledger.reasons) or "A requirement check failed."
        return _repair("candidate_error", "requirement_failure", feedback)
    if verdict == "binding_declaration_error":
        feedback = ("The binding declaration disagrees with the program structure. "
                    + " ".join(ledger.reasons)
                    + " Resend the program with declarations that match the CIR resources.")
        return {"action": "repair_protocol", "category": "protocol_noncompliance",
                "status": "binding_declaration_error", "feedback": feedback,
                "semantic_retry": False, "protocol_retry": True}
    if verdict in {"timeout", "runtime_crash", "partial"}:
        return _repair("candidate_error", verdict,
                       "The program did not finish on every run. Join every spawned thread.")
    if verdict == "satisfied_bounded" and ledger.all_obligations_satisfied \
            and ledger.delivery_status == "deliver_bounded":
        return {"action": "accept", "category": "accepted", "status": "accepted",
                "feedback": "", "semantic_retry": False, "protocol_retry": False}
    # Anonymous worker closures violate the published one-function-per-CIR-
    # function generation protocol. This is fixable without guessing a binding
    # or treating unresolved identities as semantic counterexamples.
    ambiguous = (result.get("binding") or {}).get("ambiguous") or []
    anonymous = [row for row in ambiguous if isinstance(row, dict)
                 and row.get("reason") == "thread entry is not unambiguous"
                 and row.get("entry") is None]
    if (verdict == "inconclusive" and ledger.hashes.get("verified")
            and anonymous):
        names = ", ".join(str(row.get("rust")) for row in anonymous)
        return {"action": "repair_protocol", "category": "protocol_noncompliance",
                "status": "anonymous_worker_entry",
                "feedback": (f"Spawned workers {names} have no unambiguous named entry. "
                             "Implement each CIR worker as a named Rust function with the "
                             "same name and call it from its spawn closure. Preserve the "
                             "verified CIR synchronization, resource names, and shared-state "
                             "updates. Do not guess thread identity from spawn order."),
                "semantic_retry": False, "protocol_retry": True}
    if ledger.delivery_status == "withhold_capability" or any(
            "unsupported" in reason for reason in ledger.reasons):
        fb = _adaptation_feedback(ledger, result)
        return (_repair("candidate_error", "representation_adaptation", fb)
                if fb else _capability())
    if ledger.delivery_status == "withhold_tool" or \
            any(reason.startswith("evidence_invalid") for reason in ledger.reasons):
        return {"action": "stop", "category": "tool_failure", "status": "evidence_invalid",
                "feedback": "", "semantic_retry": False, "protocol_retry": False}
    fb = _adaptation_feedback(ledger, result)
    return (_repair("candidate_error", "representation_adaptation", fb)
            if fb else _capability())


def _repair(category: str, status: str, feedback: str) -> dict[str, Any]:
    if not feedback.strip():
        return _capability()
    return {"action": "repair", "category": category, "status": status,
            "feedback": feedback, "semantic_retry": True, "protocol_retry": False}


def _capability() -> dict[str, Any]:
    return {"action": "stop", "category": "capability_gap", "status": "capability_gap",
            "feedback": "", "semantic_retry": False, "protocol_retry": False}


def protocol_rejection(reason: str, feedback: str) -> dict[str, Any]:
    """A fixable generation-protocol miss, recorded separately from semantic repair."""

    ledger = {"protocol": PROTOCOL, "historical_acceptance": False,
              "current_evaluation": "protocol_noncompliance",
              "evidence_grade": "protocol_noncompliance",
              "delivery_status": "withhold_protocol",
              "needs_human_review": False, "needs_extra_check": False,
              "reasons": [reason], "functional": {"status": "not_run"}}
    return {"action": "repair_protocol", "category": "protocol_noncompliance",
            "status": "protocol_noncompliance", "feedback": feedback,
            "semantic_retry": False, "protocol_retry": True, "ledger": ledger,
            "stages": {"format": "format_error"}}


def _finalize(out: Path, result: dict, contract_path: Path, accepted: bool) -> dict:
    try:
        contract = json.loads(Path(contract_path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        contract = {"properties": [], "preserved": []}
    # Properties are read from the raw explore stdout. Pass them only as a
    # consistency check when that file was parsed.
    props = result.get("_model_props")
    complete = result.get("_model_complete")
    ledger = interpret(result, contract, accepted=accepted,
                       cir_props=props, cir_complete=complete)
    result.pop("_model_props", None)
    result.pop("_model_complete", None)
    result["ledger"] = ledger.to_dict()
    result["protocol"] = PROTOCOL
    result["acceptance_policy"] = ACCEPTANCE_POLICY
    result["followup"] = decide_followup(ledger, result)
    result["evidence_path"] = str(out / "result.json")
    result["binding_assessment"] = binding_assessment(result)
    (out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                                     encoding="utf-8")
    return result


def evaluate_candidate(source: str, cir_path: Path, contract_path: Path, out_dir: Path, *,
                       binary, instrument=None, n_runs: int = 32, run_timeout: float = 10.0,
                       accepted: bool = False, cir_props: dict | None = None,
                       cir_complete: bool | None = None, functional: dict | None = None,
                       functional_spec: dict | None = None, manifest_path: Path | None = None,
                       cell_id: str = "", candidate_kind: str = "final",
                       round_no: int | None = None, binding_binary: Path | None = None) -> dict:
    """Acquire evidence for one candidate and interpret it. Every exit writes a ledger."""

    from . import bounded_monitor, rust_oracle
    from . import generation
    from .binding import BindingUnavailable
    from .binding import bind as binding_bind

    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    binary, cir_path, contract_path = Path(binary), Path(cir_path), Path(contract_path)
    source_path = out / "source.rs"
    source_path.write_text(source, encoding="utf-8")
    source_sha = hashlib.sha256(source.encode()).hexdigest()
    cir_sha, contract_sha = _sha_file(cir_path), _sha_file(contract_path)
    from .binding import default_binary
    binding_binary = Path(binding_binary) if binding_binary is not None else default_binary()
    binding_sha = _sha_file(binding_binary)
    backend_sha, instrument_sha = _sha_file(binary), _sha_file(Path(instrument)) if instrument else None
    acquisition = _acquisition_id({
        "source": source_sha, "cir": cir_sha, "contract": contract_sha,
        "backend": backend_sha, "instrument": instrument_sha, "binding": binding_sha,
        "n_runs": n_runs, "timeout": run_timeout,
    })
    try:
        cir_doc = json.loads(cir_path.read_text(encoding="utf-8"))
        cir_has_sync = any(
            r.get("type") in {"Mutex", "Condvar", "Semaphore", "Channel"}
            for m in cir_doc.get("modules", []) for r in m.get("resources", []))
    except (OSError, json.JSONDecodeError):
        cir_has_sync = True  # unknown: keep the strict (defect) interpretation
    binds_base = {"acquisition_id": acquisition, "source_sha256": source_sha,
                  "cir_sha256": cir_sha, "contract_sha256": contract_sha,
                  "backend_sha256": backend_sha}
    result: dict[str, Any] = {
        "cell": cell_id, "protocol": PROTOCOL,
        "source_path": str(source_path), "cir_path": str(cir_path),
        "contract_path": str(contract_path),
        "source_sha256": source_sha, "cir_sha256": cir_sha, "contract_sha256": contract_sha,
        "backend_sha256": backend_sha, "instrument_sha256": instrument_sha,
        "binding_sha256": binding_sha, "cir_has_sync": cir_has_sync,
        "acquisition_id": acquisition, "n_runs": n_runs, "stages": {},
        "limitations": [], "candidate_kind": candidate_kind, "round_no": round_no,
        "functional": None, "artifacts": [],
    }
    artifacts: list[dict] = []
    for role, path in (("source", source_path), ("cir", cir_path), ("contract", contract_path)):
        art = _artifact(role, path, binds=binds_base)
        if art:
            artifacts.append(art)

    model = run_model_check(binary, cir_path, contract_path, out)
    if model.get("ok"):
        payload = model["payload"]
        props = {str(p["id"]).replace("preserved: ", ""): p.get("outcome")
                 for p in payload.get("properties", []) if isinstance(p, dict) and "id" in p}
        result["_model_props"] = props
        result["_model_complete"] = payload.get("complete")
    elif cir_props is not None:
        # An explicit frozen explore payload may be supplied only together with
        # a raw file written by the caller. The checker output is not rebuilt here.
        result["_model_props"] = cir_props
        result["_model_complete"] = cir_complete
    model_art = _artifact("model_check", Path(model["path"]), binds=binds_base) if model.get("path") else None
    if model_art:
        artifacts.append(model_art)

    src_proj = out / "source-proj"
    (src_proj / "src").mkdir(parents=True, exist_ok=True)
    (src_proj / "Cargo.toml").write_text(rust_oracle._container(), encoding="utf-8")
    (src_proj / "src" / "main.rs").write_text(source, encoding="utf-8")
    src_ok, src_log = rust_oracle.cargo_build(src_proj)
    log_path = out / "source-build.log"
    log_path.write_text(src_log, encoding="utf-8")
    result["source_build_log"] = str(log_path)
    result["stages"]["source_build"] = "ok" if src_ok else "failed"
    if functional is None:
        try:
            functional = run_functional_check(source, out / "functional", functional_spec)
        except subprocess.TimeoutExpired as exc:
            functional = {"status": "fail", "reason": "timeout", "raw_stdout": None,
                          "timed_out": True, "returncode": None, "evidence": str(exc),
                          "execution_kind": "candidate_timeout", "supports": []}
        except OSError as exc:
            functional = {"status": "not_run", "reason": f"tool_error: {exc}",
                          "raw_stdout": None, "timed_out": False, "returncode": None,
                          "evidence": str(exc), "execution_kind": "tool_unavailable",
                          "supports": []}
    result["functional"] = functional
    if functional.get("evidence_path"):
        art = _artifact("functional", Path(functional["evidence_path"]), binds=binds_base)
        if art:
            artifacts.append(art)
    result["artifacts"] = artifacts
    if not src_ok:
        return _finalize(out, result, contract_path, accepted)

    try:
        wrapped = rust_oracle.instrument_wrappers(source, out / "instrument", binary=instrument)
    except Exception as exc:  # noqa: BLE001
        result["stages"]["instrument"] = f"error: {exc}"
        result["artifacts"] = artifacts
        return _finalize(out, result, contract_path, accepted)
    result["stages"]["instrument"] = "ok"
    result["limitations"] = wrapped["limitations"]
    result["resources"] = wrapped["resources"]
    proj = out / "proj"
    rust_oracle.prepare_project(proj, wrapped["annotated"], wrapped["runtime"])
    built, build_log = rust_oracle.cargo_build(proj)
    (out / "instrumented-build.log").write_text(build_log, encoding="utf-8")
    result["stages"]["instrumented_build"] = "ok" if built else "failed"
    if not built:
        result["artifacts"] = artifacts
        return _finalize(out, result, contract_path, accepted)
    if any("thread::scope" in item for item in wrapped["limitations"]):
        result["instrument_unsupported"] = True
        result["stages"]["execution"] = "not_run"
        result["artifacts"] = artifacts
        return _finalize(out, result, contract_path, accepted)

    runs = rust_oracle.run_native(proj, out / "traces", n=n_runs, timeout=run_timeout)
    result["runs_started"] = len(runs)
    result["runs_completed"] = sum(1 for r in runs if r["completed"])
    result["hang"] = any(r["timed_out"] for r in runs)
    for trace in sorted((out / "traces").glob("*.jsonl")):
        art = _artifact("execution", trace, binds=binds_base)
        if art:
            artifacts.append(art)
    result["raw_events"] = _count_lines(out / "traces")

    try:
        rb = binding_bind(out / "instrument/resources.json", cir_path, manifest_path=manifest_path,
                          binary=binding_binary)
    except BindingUnavailable as exc:
        result["stages"]["binding"] = f"tool_error: {exc}"
        result["binding"] = {"mapping": {}, "ambiguous": [], "violated": {},
                             "source": "rust-cli-unavailable", "error": str(exc)}
        result["artifacts"] = artifacts
        return _finalize(out, result, contract_path, accepted)
    raw_bind = rb.pop("_raw_stdout", None) if isinstance(rb, dict) else None
    bind_path = out / "binding-check.stdout"
    bind_path.write_text(raw_bind if isinstance(raw_bind, str) else json.dumps(rb),
                         encoding="utf-8")
    mapping = {name: v["cir"] for name, v in (rb.get("verified") or {}).items()
               if isinstance(v, dict) and "cir" in v}
    ambiguous = [{"rust": k, **v} for k, v in (rb.get("unresolved") or {}).items()] \
        if isinstance(rb.get("unresolved"), dict) else list(rb.get("unresolved") or [])
    result["stages"]["binding"] = "ok"
    result["binding"] = {"mapping": mapping, "ambiguous": ambiguous,
                         "violated": rb.get("violated") or {}, "source": "rust-cli",
                         "attributes": rb.get("attributes"),
                         "uncovered_sync": rb.get("uncovered_sync") or []}
    art = _artifact("binding_check", bind_path, binds=binds_base)
    if art:
        artifacts.append(art)
    mapping_path = out / "binding.json"
    mapping_path.write_text(json.dumps({"mapping": mapping, "provenance": result["binding"]},
                                       indent=2) + "\n", encoding="utf-8")

    wrapper_names = {r["name"] for r in wrapped["resources"] if r.get("kind") == "ChannelWrapper"}
    generation._rewrite_traces(out / "traces", out / "conform-traces", mapping,
                               generation._DROP_OPS | {"value"}, wrapper_names)
    for trace in sorted((out / "conform-traces").glob("*.jsonl")):
        art = _artifact("projection", trace, binds=binds_base)
        if art:
            artifacts.append(art)
    result["projected_events"] = _count_lines(out / "conform-traces")
    conform = generation._conform_all_op_resource(binary, cir_path, out / "conform-traces")
    conform_doc = {"checks": [
        {"trace_sha256": item.get("trace_sha256"), "stdout": item.get("stdout", "")}
        for item in conform.get("raw") or []
    ]}
    conform_path = out / "conform-check.json"
    conform_path.write_text(json.dumps(conform_doc) + "\n", encoding="utf-8")
    result["conform"] = {k: conform[k] for k in ("traces", "statuses", "conformant",
                                                  "violations", "first_violation") if k in conform}
    art = _artifact("conform_check", conform_path, binds=binds_base)
    if art:
        artifacts.append(art)

    generation._rewrite_traces(out / "traces", out / "monitor-traces", {}, set(), wrapper_names)
    try:
        report = bounded_monitor.run_monitor(
            contract_path, out / "monitor-traces",
            resources=out / "instrument/resources.json",
            mapping=mapping_path, program=cir_path, binary=binary)
    except (OSError, RuntimeError, json.JSONDecodeError) as exc:
        result["stages"]["monitor"] = f"tool_error: {exc}"
        result["artifacts"] = artifacts
        return _finalize(out, result, contract_path, accepted)
    raw_mon = report.pop("_raw_stdout", None)
    mon_path = out / "monitor.stdout"
    mon_path.write_text(raw_mon if isinstance(raw_mon, str) else json.dumps(report),
                        encoding="utf-8")
    pairs = []
    for prop in report.get("properties") or []:
        if isinstance(prop, dict):
            pairs.append((prop.get("id"), prop.get("status")))
    result["monitor"] = {"status": report.get("status"), "properties": pairs}
    art = _artifact("monitor", mon_path, binds=binds_base)
    if art:
        artifacts.append(art)
    result["artifacts"] = artifacts
    return _finalize(out, result, contract_path, accepted)


def _count_lines(directory: Path) -> int:
    total = 0
    if not directory.is_dir():
        return 0
    for path in directory.glob("*.jsonl"):
        total += len([ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()])
    return total
