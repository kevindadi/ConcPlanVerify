"""Evidence state machine (strong-link-v4).

One decision entry used by re-execution, summarisation and the pipeline. Layers
are never collapsed into a single "sufficient": model, identity, trace, run,
hash and per-property evidence are kept separate and combined conservatively.

v4 changes over v3:

- ``model.verified`` requires ``complete=true`` AND every required property
  explicitly ``PASS`` (UNKNOWN/UNSUPPORTED/missing/unknown all block it).
- run state distinguishes ``source_build_failed`` / ``instrument_failed`` /
  ``instrument_build_failed`` / ``not_run`` / ``timeout`` / ``runtime_crash`` /
  ``partial`` / ``completed``; the planned ``n_runs`` is not the started count.
- ``current_evaluation`` depends only on current evidence, never on
  ``historical_acceptance`` (which is recorded separately).
- trace violations are read from structured ``got`` fields, so a binding gap and
  an independent violation are both preserved.
- hash verification re-reads every referenced artifact (source, CIR, contract,
  binding, monitor, traces) and compares to the recorded hashes.
"""

from __future__ import annotations

import hashlib
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .evidence import contract_properties, obligation_for

TRACE_DECIDABLE = {"safety", "never_holds_all", "unreachable", "reachable",
                   "holds_all", "always_reachable", "deadlock_free", "reachable_all"}
TOOL_STATUSES = {"error", "unsupported", "unknown_sid", "incomplete", "tool_error"}
RUN_FAILURE_STATES = {"source_build_failed", "instrument_failed",
                      "instrument_build_failed", "not_run", "timeout",
                      "runtime_crash", "partial"}


def _sha(path: str | None) -> str | None:
    if not path:
        return None
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return None


def hash_verification(result: dict) -> dict:
    """Re-read every recorded artifact and compare to its recorded hash."""

    artifacts = result.get("artifacts") or []
    detail = {}
    ok = True
    for a in artifacts:
        path, recorded = a.get("path"), a.get("sha256")
        actual = _sha(path)
        match = bool(recorded and actual and recorded == actual)
        detail[path] = {"recorded": bool(recorded), "match": match}
        ok = ok and match
    # Fall back to the three inputs when no artifact manifest is present.
    if not artifacts:
        for key, path in (("source_sha256", result.get("source_path")),
                          ("cir_sha256", result.get("cir_path")),
                          ("contract_sha256", result.get("contract_path"))):
            recorded = result.get(key)
            actual = _sha(path) if path else None
            match = bool(recorded and actual and recorded == actual)
            detail[path or key] = {"recorded": bool(recorded), "match": match}
            ok = ok and match
    return {"verified": ok, "detail": detail}


def _model_state(cir_props: dict, cir_complete: bool | None,
                 required: list[str]) -> dict:
    missing = [pid for pid in required if pid not in cir_props]
    non_pass = sorted(pid for pid, outcome in cir_props.items() if outcome != "PASS")
    return {
        "complete": cir_complete,
        "required": sorted(required),
        "missing_properties": sorted(missing),
        "non_pass_properties": non_pass,
        # complete=true is NOT all-PASS; every required property must be PASS.
        "verified": bool(required) and cir_complete is True
                    and not missing and not non_pass,
    }


def _identity_state(result: dict) -> dict:
    b = result.get("binding") or {}
    verified = b.get("mapping", {}) or {}
    unresolved = b.get("ambiguous", []) or []
    violated = b.get("violated", []) or []
    return {"verified": len(verified), "unresolved": len(unresolved),
            "violated": len(violated),
            "relevant_unresolved": len(unresolved) > 0,
            "declaration_error": len(violated) > 0}


def _trace_state(result: dict) -> dict:
    conform = result.get("conform") or {}
    statuses = conform.get("statuses", {}) or {}
    traces = int(conform.get("traces", 0) or 0)
    projected = int(result.get("projected_events") or 0)
    binding = result.get("binding") or {}
    unresolved_names = {a.get("rust") for a in binding.get("ambiguous", [])}
    # A violated declaration also leaves the object unbound; its conformance
    # mismatch is a declaration error, not an independent program violation.
    violated = binding.get("violated") or {}
    violated_entries = violated.values() if isinstance(violated, dict) else violated
    for v in violated_entries:
        if isinstance(v, dict) and v.get("runtime"):
            unresolved_names.add(v["runtime"])
    violations = conform.get("violations") or []
    violation_resources = [v.get("resource") for v in violations
                           if v.get("resource") is not None]
    # Both kinds can coexist across traces; neither may hide the other.
    binding_gap = any(r in unresolved_names for r in violation_resources)
    independent = any(r not in unresolved_names for r in violation_resources)

    if traces == 0:
        state = "incomplete"
    elif any(k in TOOL_STATUSES for k in statuses):
        state = "tool_error"
    elif violations:
        if independent:
            state = "observed_violation"
        elif binding_gap:
            state = "binding_unresolved"
        else:
            state = "observed_violation"
    elif projected == 0:
        state = "empty_projection"
    elif statuses.get("conformant", 0) == traces:
        state = "observed_conformant"
    else:
        state = "incomplete"
    return {"state": state, "raw_statuses": statuses, "traces": traces,
            "projected_events": projected, "binding_gap": binding_gap,
            "independent_violation": independent, "violations": violations}


def _run_state(result: dict) -> dict:
    stages = result.get("stages", {}) or {}
    started = int(result.get("runs_started", 0) or 0)
    completed = int(result.get("runs_completed", 0) or 0)
    hang = bool(result.get("hang"))
    if stages.get("source_build") == "failed":
        state = "source_build_failed"
    elif str(stages.get("instrument", "")).startswith("error") or \
            stages.get("instrument") == "failed":
        state = "instrument_failed"
    elif stages.get("instrumented_build") == "failed":
        state = "instrument_build_failed"
    elif started == 0:
        state = "not_run"
    elif completed == 0 and hang:
        state = "timeout"
    elif completed == 0:
        state = "runtime_crash"
    elif completed < started:
        state = "partial"
    else:
        state = "completed"
    return {"state": state, "started": started, "completed": completed, "hang": hang}


@dataclass
class PropertyLedger:
    property_id: str
    kind: str
    requirements: list[str]
    model_verdict: str | None
    implementation_obligation: str
    checker_available: bool
    identity_relevant_verified: bool
    trace_state: str
    independent_requirement_result: str
    obligation_state: str
    guarantee: str


@dataclass
class ReexecutionLedger:
    cell: str
    historical_acceptance: bool
    current_evaluation: str
    model: dict
    identity: dict
    trace: dict
    run: dict
    hashes: dict
    properties: list[PropertyLedger] = field(default_factory=list)
    all_obligations_satisfied: bool = False
    reasons: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        data = asdict(self)
        data["properties"] = [asdict(p) for p in self.properties]
        return data


def evaluate_reexecution(result: dict, contract: dict, *, accepted: bool,
                         cir_props: dict[str, str] | None = None,
                         cir_complete: bool | None = None) -> ReexecutionLedger:
    cir_props = cir_props or {}
    contract_props = contract_properties(contract)
    required = [p["id"] for p in contract_props]
    model = _model_state(cir_props, cir_complete, required)
    identity = _identity_state(result)
    trace = _trace_state(result)
    run = _run_state(result)
    hashes = hash_verification(result)
    monitor = result.get("monitor") or {}
    monitor_props = dict(monitor.get("properties") or [])

    props: list[PropertyLedger] = []
    for cp in contract_props:
        pid = cp["id"]
        mv = cir_props.get(pid)
        status = monitor_props.get(pid, "not_observed")
        checker_available = status not in {"not_observed"} and cp["kind"] in TRACE_DECIDABLE
        relevant_ok = not identity["relevant_unresolved"] and not identity["declaration_error"]
        if status == "FAIL":
            state, guarantee = "violated", "none"
        elif cp["kind"] == "deadlock_free":
            state = "satisfied" if (model["verified"] and run["state"] == "completed"
                                    and trace["state"] == "observed_conformant"
                                    and relevant_ok) else "unresolved"
            guarantee = "bounded_observation" if state == "satisfied" else "none"
        elif status == "PASS_bounded" and trace["state"] == "observed_conformant" and relevant_ok:
            state, guarantee = "satisfied", "bounded_observation"
        else:
            state, guarantee = "unresolved", "none"
        props.append(PropertyLedger(
            property_id=pid, kind=cp["kind"], requirements=cp.get("req", []),
            model_verdict=mv, implementation_obligation=obligation_for(cp["kind"]),
            checker_available=checker_available,
            identity_relevant_verified=relevant_ok, trace_state=trace["state"],
            independent_requirement_result=status, obligation_state=state,
            guarantee=guarantee))

    all_satisfied = bool(
        contract_props and model["verified"] and hashes["verified"]
        and trace["state"] == "observed_conformant"
        and not identity["relevant_unresolved"] and not identity["declaration_error"]
        and run["state"] == "completed"
        and all(p.obligation_state == "satisfied" for p in props))

    # current_evaluation depends only on current evidence, never on accepted.
    reasons: list[str] = []
    if str((result.get("stages") or {}).get("binding", "")).startswith("tool_error"):
        verdict = "tool_error"
        reasons.append("binding check unavailable or failed (no fallback used)")
    elif run["state"] in {"source_build_failed", "instrument_failed", "instrument_build_failed"}:
        verdict = run["state"]
        reasons.append(f"tool/stage failure: {run['state']}")
    elif run["state"] in {"not_run", "timeout", "runtime_crash", "partial"}:
        verdict = run["state"]
        reasons.append(f"run state {run['state']}")
    elif trace.get("independent_violation"):
        verdict = "explicit_failure"
        reasons.append("independent conformance violation on a resolved resource")
    elif identity["declaration_error"]:
        verdict = "binding_declaration_error"
        reasons.append("a binding manifest claim disagrees with structure "
                       "(distinct from a program requirement error)")
    elif all_satisfied:
        verdict = "satisfied_bounded"
    else:
        verdict = "inconclusive"
        if not contract_props:
            reasons.append("empty contract: nothing to verify")
        if not model["verified"]:
            reasons.append("model not verified (incomplete, missing, or non-PASS property)")
        if not hashes["verified"]:
            reasons.append("artifact hashes do not verify")
        if trace["state"] in {"empty_projection", "incomplete", "tool_error",
                              "binding_unresolved"}:
            reasons.append(f"trace state {trace['state']}")
        if identity["relevant_unresolved"]:
            reasons.append("relevant identity bindings unresolved")
        if run["state"] != "completed":
            reasons.append(f"run state {run['state']}")
        if any(p.obligation_state == "unresolved" for p in props):
            reasons.append("some obligations are unchecked")

    return ReexecutionLedger(
        cell=result.get("cell", ""), historical_acceptance=accepted,
        current_evaluation=verdict, model=model, identity=identity, trace=trace,
        run=run, hashes=hashes, properties=props,
        all_obligations_satisfied=all_satisfied, reasons=reasons)
