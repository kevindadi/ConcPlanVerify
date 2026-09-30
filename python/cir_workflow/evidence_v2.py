"""Evidence state machine (strong-link-v6).

One decision entry used by re-execution, summarisation and the online pipeline.
Layers stay separate. A conclusion is valid only when that layer's own evidence
is present and tied to the current candidate and the same acquisition. An
unrelated missing layer does not erase a counterexample that has its own
evidence. A missing file, a hash mismatch, or a swapped candidate blocks every
conclusion that depends on that evidence.

Delivery status is a frozen policy on top of the evidence grade. It is not a
knob for raising the pass rate. External stdout support does not prove an
internal ``var_eq`` property.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .evidence import contract_properties, obligation_for

TRACE_DECIDABLE = {"safety", "never_holds_all", "unreachable", "reachable",
                   "holds_all", "always_reachable", "deadlock_free", "reachable_all"}
TOOL_STATUSES = {"error", "unsupported", "unknown_sid", "incomplete", "tool_error"}
RUN_FAILURE_STATES = {"source_build_failed", "instrument_failed",
                      "instrument_build_failed", "not_run", "timeout",
                      "runtime_crash", "partial"}
PROTOCOL = "strong-link-v6"

# Roles a positive bounded conclusion must reference. Projected traces are
# checker inputs (role ``projection``), not the conform conclusion.
REQUIRED_ROLES = {"source", "cir", "contract", "model_check", "binding_check",
                  "monitor", "execution", "conform_check"}
BINDABLE_KINDS = {"Mutex", "Condvar", "RwLock", "Semaphore", "Channel",
                  "ChannelWrapper", "Spawn"}


def _sha(path: str | None) -> str | None:
    if not path:
        return None
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError:
        return None


def _arts(result: dict, role: str) -> list[dict]:
    return [a for a in (result.get("artifacts") or []) if a.get("role") == role]


def _file_ok(artifact: dict) -> bool:
    return bool(artifact.get("sha256")) and _sha(artifact.get("path")) == artifact.get("sha256")


def _binds_ok(artifact: dict, result: dict, keys: tuple[str, ...]) -> bool:
    binds = artifact.get("binds") or {}
    if result.get("acquisition_id") and binds.get("acquisition_id") != result.get("acquisition_id"):
        return False
    for key in keys:
        if result.get(key) and binds.get(key) != result.get(key):
            return False
    return True


def _read_json(path: str | None) -> dict | None:
    if not path:
        return None
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return data if isinstance(data, dict) else None


def _candidate_identity(result: dict) -> dict:
    mismatch, missing = [], []
    for role, pkey, skey in (("source", "source_path", "source_sha256"),
                             ("cir", "cir_path", "cir_sha256"),
                             ("contract", "contract_path", "contract_sha256")):
        arts = _arts(result, role)
        if not arts:
            missing.append(role)
            continue
        if not any(a.get("path") == result.get(pkey) and a.get("sha256") == result.get(skey)
                   and _file_ok(a) for a in arts):
            mismatch.append(role)
    return {"ok": not missing and not mismatch, "missing": missing,
            "mismatch": mismatch}


def _load_model(result: dict, cir_props: dict | None, cir_complete: bool | None) -> dict:
    arts = _arts(result, "model_check")
    if len(arts) != 1 or not _file_ok(arts[0]):
        return {"ok": False, "props": {}, "complete": None, "reason": "missing_or_hash"}
    art = arts[0]
    if art.get("summary_only"):
        return {"ok": False, "props": {}, "complete": None, "reason": "summary_only"}
    if not _binds_ok(art, result, ("cir_sha256", "contract_sha256", "backend_sha256")):
        return {"ok": False, "props": {}, "complete": None, "reason": "bind_mismatch"}
    data = _read_json(art.get("path"))
    props_list = data.get("properties") if data else None
    if not isinstance(props_list, list):
        return {"ok": False, "props": {}, "complete": None, "reason": "not_raw_explore"}
    props: dict[str, str] = {}
    for prop in props_list:
        if not isinstance(prop, dict) or "id" not in prop:
            return {"ok": False, "props": {}, "complete": None, "reason": "not_raw_explore"}
        props[str(prop["id"]).replace("preserved: ", "")] = prop.get("outcome")
    complete = data.get("complete")
    if cir_props is not None and dict(cir_props) != props:
        return {"ok": False, "props": props, "complete": complete,
                "reason": "disagrees_with_argument"}
    if cir_complete is not None and cir_complete != complete:
        return {"ok": False, "props": props, "complete": complete,
                "reason": "disagrees_with_argument"}
    return {"ok": True, "props": props, "complete": complete, "reason": None}


def _load_binding(result: dict) -> dict:
    arts = _arts(result, "binding_check")
    if len(arts) != 1 or not _file_ok(arts[0]):
        return {"ok": False, "binding": {}, "reason": "missing_or_hash"}
    if not _binds_ok(arts[0], result, ("source_sha256", "cir_sha256")):
        return {"ok": False, "binding": {}, "reason": "bind_mismatch"}
    data = _read_json(arts[0].get("path"))
    if not data or not isinstance(data.get("verified", {}), dict):
        return {"ok": False, "binding": {}, "reason": "not_raw_bind"}
    verified = data.get("verified") or {}
    unresolved = data.get("unresolved") or {}
    violated = data.get("violated") or {}
    mapping = {}
    for name, value in verified.items():
        if isinstance(value, dict) and "cir" in value:
            mapping[name] = value["cir"]
        elif isinstance(value, str):
            mapping[name] = value
    if isinstance(unresolved, dict):
        ambiguous = [{"rust": key, **(value if isinstance(value, dict) else {"detail": value})}
                     for key, value in unresolved.items()]
    elif isinstance(unresolved, list):
        ambiguous = unresolved
    else:
        ambiguous = []
    binding = {"mapping": mapping, "ambiguous": ambiguous, "violated": violated,
               "source": "rust-cli"}
    claimed = result.get("binding") or {}
    if claimed.get("mapping") is not None and claimed.get("mapping") != mapping:
        return {"ok": False, "binding": binding, "reason": "summary_disagrees"}
    return {"ok": True, "binding": binding, "reason": None}


def _load_conform(result: dict) -> dict:
    arts = _arts(result, "conform_check")
    if len(arts) != 1 or not _file_ok(arts[0]):
        return {"ok": False, "conform": {}, "reason": "missing_or_hash"}
    if not _binds_ok(arts[0], result, ("source_sha256", "cir_sha256", "backend_sha256")):
        return {"ok": False, "conform": {}, "reason": "bind_mismatch"}
    data = _read_json(arts[0].get("path"))
    checks = data.get("checks") if data else None
    if not isinstance(checks, list):
        return {"ok": False, "conform": {}, "reason": "not_raw_conform"}
    exec_shas = {a.get("sha256") for a in _arts(result, "execution") if _file_ok(a)}
    # The checker reads projected traces. Those projections must belong to this
    # acquisition; they are not a substitute for the checker stdout.
    projected_shas = {a.get("sha256") for a in _arts(result, "projection") if _file_ok(a)}
    if not exec_shas:
        return {"ok": False, "conform": {}, "reason": "execution_missing"}
    known = exec_shas | projected_shas
    statuses: dict[str, int] = {}
    violations = []
    first = None
    for check in checks:
        stdout = check.get("stdout")
        if not isinstance(stdout, str):
            return {"ok": False, "conform": {}, "reason": "missing_stdout"}
        if check.get("trace_sha256") not in known:
            return {"ok": False, "conform": {}, "reason": "trace_not_in_execution"}
        try:
            raw = json.loads(stdout)
        except json.JSONDecodeError:
            raw = {"status": "error", "detail": stdout[:200]}
        if "status" in check and check.get("status") != raw.get("status"):
            return {"ok": False, "conform": {}, "reason": "status_disagrees_with_stdout"}
        status = raw.get("status")
        statuses[status] = statuses.get(status, 0) + 1
        if status != "conformant":
            got = str(raw.get("got", ""))
            item = {"status": status, "got": got,
                    "resource": got.split(":", 1)[1] if ":" in got else None,
                    "event_index": raw.get("event_index"),
                    "expected": raw.get("expected"), "detail": raw.get("detail")}
            violations.append(item)
            if first is None:
                first = raw
    parsed = {"traces": len(checks), "statuses": statuses,
              "conformant": statuses.get("conformant", 0),
              "violations": violations, "first_violation": first}
    claimed = (result.get("conform") or {}).get("statuses")
    if claimed is not None and claimed != statuses:
        return {"ok": False, "conform": parsed, "reason": "summary_disagrees"}
    return {"ok": True, "conform": parsed, "reason": None}


def _pair(prop) -> tuple | None:
    if isinstance(prop, dict) and "id" in prop:
        return (prop.get("id"), prop.get("status"))
    if isinstance(prop, (list, tuple)) and len(prop) == 2:
        return (prop[0], prop[1])
    return None


def _load_monitor(result: dict) -> dict:
    arts = _arts(result, "monitor")
    if len(arts) != 1 or not _file_ok(arts[0]):
        return {"ok": False, "monitor": {}, "reason": "missing_or_hash"}
    if not _binds_ok(arts[0], result, ("source_sha256", "contract_sha256", "backend_sha256")):
        return {"ok": False, "monitor": {}, "reason": "bind_mismatch"}
    data = _read_json(arts[0].get("path"))
    props = data.get("properties") if data else None
    if not isinstance(props, list):
        return {"ok": False, "monitor": {}, "reason": "not_raw_monitor"}
    pairs = []
    for prop in props:
        pair = _pair(prop)
        if pair is None:
            return {"ok": False, "monitor": {}, "reason": "not_raw_monitor"}
        pairs.append(pair)
    claimed = (result.get("monitor") or {}).get("properties")
    if claimed is not None and [tuple(item) for item in claimed] != pairs:
        return {"ok": False, "monitor": {"properties": pairs}, "reason": "summary_disagrees"}
    return {"ok": True, "monitor": {"status": data.get("status"), "properties": pairs},
            "reason": None}


def _execution_ok(result: dict) -> bool:
    arts = _arts(result, "execution")
    if not arts or not all(_file_ok(a) for a in arts):
        return False
    return all(_binds_ok(a, result, ("source_sha256",)) for a in arts)


def hash_verification(result: dict) -> dict:
    """Re-read role artifacts. A positive grade needs every required role.

    Candidate path/hash must match the role artifact. A functional pass/fail
    additionally needs its own evidence file. This does not, by itself, decide
    a counterexample: that uses the layer's own evidence.
    """

    artifacts = result.get("artifacts") or []
    identity = _candidate_identity(result)
    if not artifacts:
        return {"verified": False, "legacy": True,
                "missing_roles": sorted(REQUIRED_ROLES), "roles": [],
                "candidate_mismatch": identity["mismatch"], "detail": {}}
    roles = {a.get("role") for a in artifacts}
    missing = sorted(REQUIRED_ROLES - roles)
    func = result.get("functional") or {}
    if func.get("status") in {"pass", "fail"} and "functional" not in roles:
        missing.append("functional")
    detail = {}
    ok = True
    for art in artifacts:
        match = _file_ok(art)
        detail[art.get("path")] = {"role": art.get("role"),
                                   "recorded": bool(art.get("sha256")), "match": match}
        ok = ok and match
    return {"verified": bool(ok and not missing and identity["ok"] and _execution_ok(result)),
            "missing_roles": missing, "candidate_mismatch": identity["mismatch"],
            "identity_ok": identity["ok"],
            "roles": sorted(r for r in roles if r), "detail": detail}


def _model_state(cir_props: dict, cir_complete: bool | None, required: list[str],
                 evidence_ok: bool) -> dict:
    missing = [pid for pid in required if pid not in cir_props]
    non_pass = sorted(pid for pid, outcome in cir_props.items() if outcome != "PASS")
    return {
        "complete": cir_complete,
        "required": sorted(required),
        "missing_properties": sorted(missing),
        "non_pass_properties": non_pass,
        "evidence_ok": evidence_ok,
        "verified": bool(evidence_ok and required and cir_complete is True
                         and not missing and not non_pass),
    }


def _identity_state(binding: dict, evidence_ok: bool) -> dict:
    verified = binding.get("mapping", {}) or {}
    unresolved = binding.get("ambiguous", []) or []
    violated = binding.get("violated", []) or []
    return {"verified": len(verified), "unresolved": len(unresolved),
            "violated": len(violated) if not isinstance(violated, dict) else len(violated),
            "relevant_unresolved": len(unresolved) > 0,
            "declaration_error": (len(violated) > 0) if not isinstance(violated, dict)
            else len(violated) > 0,
            "evidence_ok": evidence_ok}


def _trace_state(result: dict, conform: dict, binding: dict, evidence_ok: bool) -> dict:
    if not evidence_ok:
        return {"state": "incomplete", "raw_statuses": {}, "traces": 0,
                "projected_events": int(result.get("projected_events") or 0),
                "binding_gap": False, "independent_violation": False,
                "violations": [], "evidence_ok": False}
    statuses = conform.get("statuses", {}) or {}
    traces = int(conform.get("traces", 0) or 0)
    projected = int(result.get("projected_events") or 0)
    unresolved_names = {a.get("rust") for a in binding.get("ambiguous", [])}
    violated = binding.get("violated") or {}
    violated_entries = violated.values() if isinstance(violated, dict) else violated
    for item in violated_entries:
        if isinstance(item, dict) and item.get("runtime"):
            unresolved_names.add(item["runtime"])
    violations = conform.get("violations") or []
    violation_resources = [v.get("resource") for v in violations if v.get("resource") is not None]
    binding_gap = any(r in unresolved_names for r in violation_resources)
    independent = any(r not in unresolved_names for r in violation_resources)
    if traces == 0:
        state = "incomplete"
    elif any(k in TOOL_STATUSES for k in statuses):
        state = "tool_error"
    elif violations:
        state = "observed_violation" if independent or not binding_gap else "binding_unresolved"
    elif projected == 0:
        state = "empty_projection"
    elif statuses.get("conformant", 0) == traces:
        state = "observed_conformant"
    else:
        state = "incomplete"
    return {"state": state, "raw_statuses": statuses, "traces": traces,
            "projected_events": projected, "binding_gap": binding_gap,
            "independent_violation": independent, "violations": violations,
            "evidence_ok": True}


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


def _functional_state(result: dict) -> dict:
    """A functional conclusion needs the source, the test definition, and the raw stdout.

    A status that disagrees with the raw stdout is not a result. A missing file
    is not a failure. ``not_run`` is the absence of a check, not a pass.
    """

    recorded = result.get("functional") or {}
    status = recorded.get("status", "not_run")
    if not recorded or status == "not_run":
        return {"status": "not_run", "valid": False,
                "reason": recorded.get("reason", "not_run"),
                "evidence": None, "evidence_path": None, "supports": []}
    path = recorded.get("evidence_path")
    arts = _arts(result, "functional")
    if not path or not Path(path).is_file():
        return {"status": "invalid", "valid": False, "reason": "missing_file",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    if not any(a.get("path") == path and _file_ok(a) for a in arts):
        return {"status": "invalid", "valid": False, "reason": "unlinked_file",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    try:
        doc = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"status": "invalid", "valid": False, "reason": "not_json",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    if doc.get("source_sha256") != result.get("source_sha256") or \
            _sha(result.get("source_path")) != result.get("source_sha256"):
        return {"status": "invalid", "valid": False, "reason": "source_mismatch",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    if not doc.get("test_id") or "expected" not in doc or doc.get("raw_stdout") is None:
        return {"status": "invalid", "valid": False, "reason": "missing_test_or_raw",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    derived = "pass" if doc.get("raw_stdout") == doc.get("expected") else "fail"
    if doc.get("status") != derived or recorded.get("status") != derived:
        return {"status": "invalid", "valid": False, "reason": "status_disagrees_with_raw",
                "evidence": recorded.get("evidence"), "evidence_path": path, "supports": []}
    supports = ["external_stdout"] if derived == "pass" and doc.get("kind", "stdout_eq") == "stdout_eq" else []
    return {"status": derived, "valid": True, "reason": None,
            "evidence": recorded.get("evidence"), "evidence_path": path,
            "supports": supports, "test_id": doc.get("test_id"),
            "expected": doc.get("expected"), "raw_stdout": doc.get("raw_stdout")}


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
    layers: dict = field(default_factory=dict)
    properties: list[PropertyLedger] = field(default_factory=list)
    all_obligations_satisfied: bool = False
    reasons: list[str] = field(default_factory=list)
    protocol: str = PROTOCOL
    evidence_grade: str = ""
    delivery_status: str = ""
    needs_human_review: bool = False
    needs_extra_check: bool = False
    functional: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        data = asdict(self)
        data["properties"] = [asdict(p) for p in self.properties]
        return data


def _delivery(*, verdict: str, functional: dict, properties: list[PropertyLedger],
              hashes: dict) -> dict:
    """Frozen delivery policy. External stdout never promotes an internal property."""

    unsupported = [p.property_id for p in properties
                   if p.independent_requirement_result in {"unsupported", "unmapped"}]
    unresolved = [p.property_id for p in properties if p.obligation_state == "unresolved"]
    if verdict in {"explicit_failure", "functional_failure", "requirement_failure",
                   "source_build_failed", "timeout", "runtime_crash", "partial"}:
        grade = "refuted" if verdict in {"explicit_failure", "functional_failure",
                                         "requirement_failure"} else "candidate_defect"
        return {"evidence_grade": grade, "delivery_status": "reject",
                "needs_human_review": False, "needs_extra_check": False}
    if verdict in {"tool_error", "instrument_failed", "instrument_build_failed"}:
        return {"evidence_grade": "tool_failure", "delivery_status": "withhold_tool",
                "needs_human_review": False, "needs_extra_check": False}
    if verdict == "binding_declaration_error":
        return {"evidence_grade": "protocol_noncompliance",
                "delivery_status": "withhold_protocol",
                "needs_human_review": False, "needs_extra_check": False}
    if verdict == "satisfied_bounded":
        return {"evidence_grade": "bounded_satisfaction", "delivery_status": "deliver_bounded",
                "needs_human_review": False, "needs_extra_check": False}
    if verdict in {"cir_not_accepted", "cir_unknown_or_raw_error", "cir_accepted_no_rust",
                   "not_run"}:
        return {"evidence_grade": "not_executed", "delivery_status": "not_a_candidate",
                "needs_human_review": False, "needs_extra_check": False}
    # Inconclusive. Invalid association is a tool/evidence stop, not a repair.
    if not hashes.get("verified") and not hashes.get("identity_ok", True):
        return {"evidence_grade": "invalid", "delivery_status": "withhold_tool",
                "needs_human_review": False, "needs_extra_check": False}
    if unsupported or (functional.get("status") == "pass" and unresolved):
        grade = "partial_external" if functional.get("status") == "pass" else "unresolved"
        return {"evidence_grade": grade, "delivery_status": "withhold_capability",
                "needs_human_review": True, "needs_extra_check": True}
    return {"evidence_grade": "unresolved", "delivery_status": "withhold_incomplete",
            "needs_human_review": bool(unresolved), "needs_extra_check": bool(unresolved)}


def binding_assessment(result: dict) -> dict:
    """Distinguish not-run, not-required, and sufficient.

    Sufficient means the binding stage ran, there is no unresolved or violated
    claim, and every bindable runtime object is covered. A compile failure is
    not-run and is never sufficient.
    """

    stages = result.get("stages") or {}
    stage = stages.get("binding")
    binding = result.get("binding") or {}
    ran = stage == "ok" and binding.get("source") == "rust-cli"
    resources = result.get("resources") or []
    bindable = [r for r in resources if r.get("kind") in BINDABLE_KINDS]
    names = {r.get("name") for r in bindable if r.get("name")}
    if not ran:
        return {"status": "not_run", "sufficient": False, "ran": False,
                "missing": sorted(names)}
    ledger_identity = ((result.get("ledger") or {}).get("identity") or {})
    unresolved = ledger_identity.get("relevant_unresolved")
    declaration = ledger_identity.get("declaration_error")
    if unresolved is None:
        unresolved = bool(binding.get("ambiguous"))
    if declaration is None:
        violated = binding.get("violated") or {}
        declaration = len(violated) > 0
    covered = names <= set((binding.get("mapping") or {}))
    if not names and not unresolved and not declaration:
        return {"status": "not_required", "sufficient": False, "ran": True, "missing": []}
    if unresolved or declaration or not covered:
        return {"status": "insufficient", "sufficient": False, "ran": True,
                "missing": sorted(names - set(binding.get("mapping") or {}))}
    return {"status": "sufficient", "sufficient": True, "ran": True, "missing": []}


def evaluate_reexecution(result: dict, contract: dict, *, accepted: bool,
                         cir_props: dict[str, str] | None = None,
                         cir_complete: bool | None = None) -> ReexecutionLedger:
    contract_props = contract_properties(contract)
    required = [p["id"] for p in contract_props]
    model_ev = _load_model(result, cir_props, cir_complete)
    binding_ev = _load_binding(result)
    conform_ev = _load_conform(result)
    monitor_ev = _load_monitor(result)
    model = _model_state(model_ev["props"], model_ev["complete"], required, model_ev["ok"])
    binding = binding_ev["binding"] if binding_ev["ok"] else (result.get("binding") or {})
    # A declaration recorded only in an unlinked summary is not a conclusion.
    if not binding_ev["ok"]:
        binding = {"mapping": {}, "ambiguous": [], "violated": {}}
    identity = _identity_state(binding, binding_ev["ok"])
    conform = conform_ev["conform"] if conform_ev["ok"] else {}
    trace = _trace_state(result, conform, binding, conform_ev["ok"])
    run = _run_state(result)
    hashes = hash_verification(result)
    monitor = monitor_ev["monitor"] if monitor_ev["ok"] else {"properties": []}
    monitor_props = dict(monitor.get("properties") or [])
    functional = _functional_state(result)

    props: list[PropertyLedger] = []
    for cp in contract_props:
        pid = cp["id"]
        mv = model_ev["props"].get(pid) if model_ev["ok"] else None
        status = monitor_props.get(pid, "not_observed") if monitor_ev["ok"] else "not_observed"
        checker_available = (monitor_ev["ok"] and status not in {"not_observed", "unsupported", "unmapped"}
                             and cp["kind"] in TRACE_DECIDABLE)
        relevant_ok = binding_ev["ok"] and not identity["relevant_unresolved"] \
            and not identity["declaration_error"]
        # External functional support does not satisfy an internal property.
        if monitor_ev["ok"] and status == "FAIL":
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

    identity_link = _candidate_identity(result)
    requirement_failed = identity_link["ok"] and monitor_ev["ok"] and any(
        p.obligation_state == "violated" for p in props)
    functional_failed = functional["valid"] and functional["status"] == "fail"
    claimed_functional = (result.get("functional") or {}).get("status")
    functional_claim_invalid = claimed_functional in {"pass", "fail"} and not functional["valid"]
    layers = {
        "conformance": trace["state"] if conform_ev["ok"] else "invalid",
        "requirement": ("violated" if requirement_failed else
                        ("invalid" if not monitor_ev["ok"] else "inconclusive")),
        "functional": functional["status"],
        "run": run["state"],
        "binding": ("invalid" if not binding_ev["ok"] else
                    ("violated" if identity["declaration_error"] else
                     ("unresolved" if identity["relevant_unresolved"] else "verified"))),
        "tool": ("error" if str((result.get("stages") or {}).get("binding", "")).startswith("tool_error")
                 or run["state"] in {"instrument_failed", "instrument_build_failed"}
                 else "ok"),
        "model": "verified" if model["verified"] else "invalid",
    }
    all_satisfied = bool(
        contract_props and model["verified"] and hashes["verified"]
        and conform_ev["ok"] and binding_ev["ok"] and monitor_ev["ok"]
        and trace["state"] == "observed_conformant"
        and not identity["relevant_unresolved"] and not identity["declaration_error"]
        and run["state"] == "completed" and not requirement_failed
        and functional["status"] != "fail" and not functional_claim_invalid
        and all(p.obligation_state == "satisfied" for p in props))

    reasons: list[str] = []
    binding_stage = str((result.get("stages") or {}).get("binding", ""))
    if run["state"] in {"source_build_failed", "instrument_failed", "instrument_build_failed"}:
        verdict = run["state"]
        reasons.append(f"stage failure: {run['state']}")
    elif binding_stage.startswith("tool_error"):
        verdict = "tool_error"
        reasons.append("binding check unavailable or failed (no fallback used)")
    elif functional_failed:
        verdict = "functional_failure"
        reasons.append("independent functional test failed: "
                       f"stdout {functional.get('raw_stdout')!r} != expected {functional.get('expected')!r}")
    elif identity_link["ok"] and conform_ev["ok"] and trace.get("independent_violation"):
        verdict = "explicit_failure"
        reasons.append("independent conformance violation on a resolved resource")
    elif requirement_failed:
        verdict = "requirement_failure"
        failed_ids = [p.property_id for p in props if p.obligation_state == "violated"]
        reasons.append("monitor FAIL on " + ", ".join(failed_ids))
    elif binding_ev["ok"] and identity["declaration_error"]:
        verdict = "binding_declaration_error"
        reasons.append("a binding manifest claim disagrees with structure")
    elif run["state"] in {"not_run", "timeout", "runtime_crash", "partial"} and \
            (result.get("stages") or {}).get("source_build") == "ok":
        verdict = run["state"]
        reasons.append(f"run state {run['state']}")
    elif all_satisfied:
        verdict = "satisfied_bounded"
    else:
        verdict = "inconclusive"
        if not hashes["verified"]:
            reasons.append("evidence_invalid: artifact association does not verify")
        if not model_ev["ok"]:
            reasons.append(f"model evidence {model_ev['reason']}")
        elif not model["verified"]:
            reasons.append("model not verified (incomplete, missing, or non-PASS property)")
        if not conform_ev["ok"]:
            reasons.append(f"conform evidence {conform_ev['reason']}")
        elif trace["state"] in {"empty_projection", "incomplete", "tool_error", "binding_unresolved"}:
            reasons.append(f"trace state {trace['state']}")
        if not binding_ev["ok"]:
            reasons.append(f"binding evidence {binding_ev['reason']}")
        elif identity["relevant_unresolved"]:
            reasons.append("relevant identity bindings unresolved")
        if not monitor_ev["ok"]:
            reasons.append(f"monitor evidence {monitor_ev['reason']}")
        if functional_claim_invalid:
            reasons.append(f"functional evidence invalid: {functional.get('reason')}")
        elif functional["status"] == "not_run":
            reasons.append("no independent functional test available")
        if any(p.obligation_state == "unresolved" for p in props):
            unsupported = [p.property_id for p in props
                           if p.independent_requirement_result in {"unsupported", "unmapped"}]
            if unsupported:
                reasons.append("checker unsupported for " + ", ".join(unsupported))
            else:
                reasons.append("some obligations are unchecked")
        if not contract_props:
            reasons.append("empty contract: nothing to verify")

    delivered = _delivery(verdict=verdict, functional=functional, properties=props, hashes=hashes)
    return ReexecutionLedger(
        cell=result.get("cell", ""), historical_acceptance=accepted,
        current_evaluation=verdict, model=model, identity=identity, trace=trace,
        run=run, hashes=hashes, layers=layers, properties=props,
        all_obligations_satisfied=all_satisfied, reasons=reasons,
        evidence_grade=delivered["evidence_grade"],
        delivery_status=delivered["delivery_status"],
        needs_human_review=delivered["needs_human_review"],
        needs_extra_check=delivered["needs_extra_check"],
        functional=functional)
