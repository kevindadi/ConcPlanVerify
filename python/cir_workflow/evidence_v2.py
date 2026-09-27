"""Evidence state machine (strong-link-v3).

One decision entry used by re-execution, summarisation and the pipeline. It
never collapses layers into a single "sufficient": model evidence, identity
evidence, trace evidence, run evidence and per-property obligations are kept
separate and only combined conservatively.

Guarantee ranges (see STATE_MACHINE.md):

- model:  the CIR was decided by exhaustive exploration (not arbitrary Rust)
- identity: construction-site / entry bindings verified, others unresolved
- trace:  op-resource conformance over a non-empty projection (finite runs)
- run:    observed execution outcome (timeout != deadlock)
- property: PASS_bounded supports a bounded observation only
"""

from __future__ import annotations

import hashlib
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .evidence import contract_properties, obligation_for

TRACE_DECIDABLE = {"safety", "never_holds_all", "unreachable", "reachable",
                   "holds_all", "always_reachable", "deadlock_free", "reachable_all"}
TOOL_STATUSES = {"error", "unsupported", "unknown_sid", "incomplete", "tool_error"}


def _sha(path: str | None) -> str | None:
    if not path:
        return None
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return None


def hash_verification(result: dict) -> dict:
    """Re-read the recorded artifacts and compare to the recorded hashes.

    Presence of a field is not verification; the file must exist and match.
    """

    checks = {
        "source_sha256": result.get("source_path"),
        "cir_sha256": result.get("cir_path"),
        "contract_sha256": result.get("contract_path"),
    }
    ok = True
    detail = {}
    for key, path in checks.items():
        recorded = result.get(key)
        actual = _sha(path) if path else None
        match = bool(recorded and actual and recorded == actual)
        detail[key] = {"recorded": bool(recorded), "match": match}
        ok = ok and match
    return {"verified": ok, "detail": detail}


def _model_state(result: dict, cir_props: dict, cir_complete: bool | None) -> dict:
    failed = [pid for pid, outcome in cir_props.items() if outcome == "FAIL"]
    return {
        "complete": cir_complete,
        "failed_properties": sorted(failed),
        "properties_checked": len(cir_props),
        # complete=true is NOT all-PASS; a FAIL property refutes the model.
        "verified": bool(cir_props) and cir_complete is True and not failed,
    }


def _identity_state(result: dict) -> dict:
    b = result.get("binding") or {}
    verified = b.get("mapping", {}) or {}
    unresolved = b.get("ambiguous", []) or []
    violated = b.get("violated", []) or []
    return {"verified": len(verified), "unresolved": len(unresolved),
            "violated": len(violated),
            "relevant_unresolved": len(unresolved) > 0}


def _trace_state(result: dict) -> dict:
    conform = result.get("conform") or {}
    statuses = conform.get("statuses", {}) or {}
    traces = int(conform.get("traces", 0) or 0)
    projected = int(result.get("projected_events") or 0)
    unresolved_names = {a.get("rust") for a in
                        (result.get("binding") or {}).get("ambiguous", [])}
    if traces == 0:
        state = "incomplete"
    elif any(k in TOOL_STATUSES for k in statuses):
        state = "tool_error"
    elif statuses.get("violation", 0) > 0:
        # A "no model statement matches <name>" violation on a resource whose
        # identity was never established is a binding gap, not a proved code
        # deviation.
        detail = str((conform.get("first_violation") or {}).get("detail", ""))
        if any(name and name in detail for name in unresolved_names):
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
            "projected_events": projected,
            "first_violation": conform.get("first_violation")}


def _run_state(result: dict) -> dict:
    n = int(result.get("n_runs", 0) or 0)
    completed = int(result.get("runs_completed", 0) or 0)
    hang = bool(result.get("hang"))
    if n == 0:
        state = "not_run"
    elif completed == 0:
        state = "crash" if not hang else "timeout"
    elif completed < n:
        state = "partial"
    else:
        state = "ok"
    return {"n_runs": n, "completed": completed, "hang": hang, "state": state}


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
    obligation_state: str          # satisfied | violated | unresolved | not_applicable
    guarantee: str                 # model | bounded_observation | none


@dataclass
class ReexecutionLedger:
    cell: str
    historical_acceptance: bool
    current_evaluation: str        # explicit_failure | inconclusive | not_accepted
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
    model = _model_state(result, cir_props, cir_complete)
    identity = _identity_state(result)
    trace = _trace_state(result)
    run = _run_state(result)
    hashes = hash_verification(result)
    monitor = result.get("monitor") or {}
    monitor_props = dict(monitor.get("properties") or [])
    contract_props = contract_properties(contract)

    props: list[PropertyLedger] = []
    for cp in contract_props:
        pid = cp["id"]
        mv = cir_props.get(pid)
        status = monitor_props.get(pid, "not_observed")
        checker_available = status not in {"not_observed"} and cp["kind"] in TRACE_DECIDABLE
        relevant_ok = not identity["relevant_unresolved"]
        if status == "FAIL":
            state, guarantee = "violated", "none"
        elif cp["kind"] == "deadlock_free":
            # CIR proof + completed runs does not prove the implementation keeps
            # blocking/enabling relations; bounded observation only.
            state = "satisfied" if (model["verified"] and run["state"] == "ok"
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
        accepted and contract_props and model["verified"] and hashes["verified"]
        and trace["state"] == "observed_conformant" and not identity["relevant_unresolved"]
        and run["state"] == "ok"
        and all(p.obligation_state == "satisfied" for p in props))

    reasons: list[str] = []
    if all_satisfied:
        verdict = "satisfied_bounded"
    elif not accepted:
        verdict = "not_accepted"
        reasons.append("no accepted round")
    elif trace["state"] == "observed_violation":
        verdict = "explicit_failure"
        reasons.append("conformance violation on a non-empty projection")
    elif run["state"] in {"crash", "timeout", "partial"}:
        verdict = "explicit_failure"
        reasons.append(f"run state {run['state']}")
    elif any(p.obligation_state == "violated" for p in props):
        verdict = "explicit_failure"
        reasons.append("a declared property is violated")
    else:
        verdict = "inconclusive"
        if not contract_props:
            reasons.append("empty contract: nothing to verify")
        if not model["verified"]:
            reasons.append("model not fully verified (incomplete or a failed property)")
        if not hashes["verified"]:
            reasons.append("artifact hashes do not verify")
        if trace["state"] in {"empty_projection", "incomplete", "tool_error",
                              "binding_unresolved"}:
            reasons.append(f"trace state {trace['state']}")
        if identity["relevant_unresolved"]:
            reasons.append("relevant identity bindings unresolved")
        if run["state"] != "ok":
            reasons.append(f"run state {run['state']}")
        if any(p.obligation_state == "unresolved" for p in props):
            reasons.append("some obligations are unchecked")

    return ReexecutionLedger(
        cell=result.get("cell", ""), historical_acceptance=accepted,
        current_evaluation=verdict, model=model, identity=identity, trace=trace,
        run=run, hashes=hashes, properties=props,
        all_obligations_satisfied=all_satisfied, reasons=reasons)
