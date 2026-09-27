"""Evidence ledger v2: built from a real re-execution result.

Separate, non-substitutable flags (a strong flag never implies a weaker one):

- ``model_verified``                    CIR exploration proved it (and completed)
- ``observed_trace_conformant``         op-resource conformance passed on a
                                        *non-empty* projection
- ``bounded_no_violation``              the bounded monitor saw no FAIL
- ``property_observation_coverage``     per-property: the property was observed
- ``implementation_obligations_satisfied``  the property's own obligation is
                                        met by real evidence (not by absence)
- ``independent_requirement_result``    the monitor status, verbatim

Rules: an empty projection never supports implementation correspondence; a
model proof never migrates to arbitrary-Rust guarantees; timeout/crash is not a
proved deadlock; missing hashes block reuse.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, field

from .evidence import contract_properties, obligation_for

TRACE_DECIDABLE = {"safety", "never_holds_all", "unreachable", "reachable",
                   "holds_all", "always_reachable", "deadlock_free", "reachable_all"}


@dataclass
class PropertyLedger:
    property_id: str
    kind: str
    requirements: list[str]
    model_verified: bool
    model_complete: bool | None
    implementation_obligation: str
    observed_trace_conformant: bool
    bounded_no_violation: bool
    property_observation_coverage: str      # observed | not_observed | not_carryable
    implementation_obligations_satisfied: bool
    independent_requirement_result: str
    final_claim: str                        # supported | violated | inconclusive | not_applicable
    evidence_ref: str


@dataclass
class ReexecutionLedger:
    cell: str
    accepted: bool
    source_build: str
    instrumented_build: str
    model_verified: bool
    observed_trace_conformant: bool
    bounded_no_violation: bool
    raw_events: int
    projected_events: int
    thread_lifecycle: dict
    binding_verified: int
    binding_unresolved: int
    hashes_present: bool
    properties: list[PropertyLedger] = field(default_factory=list)
    verdict: str = "not_accepted"
    reasons: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        data = asdict(self)
        data["properties"] = [asdict(p) for p in self.properties]
        return data


def evaluate_reexecution(result: dict, contract: dict, *, accepted: bool,
                         cir_props: dict[str, str] | None = None,
                         cir_complete: bool | None = None) -> ReexecutionLedger:
    stages = result.get("stages", {})
    conform = result.get("conform") or {}
    statuses = conform.get("statuses", {}) or {}
    projected = int(result.get("projected_events") or 0)
    raw = int(result.get("raw_events") or 0)
    conformant_all = (conform.get("traces", 0) > 0
                      and statuses.get("conformant", 0) == conform.get("traces", 0))
    observed_trace_conformant = bool(conformant_all and projected > 0)
    monitor = result.get("monitor") or {}
    monitor_props = dict(monitor.get("properties") or [])
    bounded_no_violation = bool(monitor) and all(
        s != "FAIL" for s in monitor_props.values())
    binding = result.get("binding") or {}
    mapping = binding.get("mapping", {}) or {}
    ambiguous = binding.get("ambiguous", []) or []
    hashes_present = all(result.get(k) for k in
                         ("source_sha256", "cir_sha256", "contract_sha256"))
    cir_props = cir_props or {}

    def observed_status(pid: str) -> str:
        return monitor_props.get(pid, "not_observed")

    props: list[PropertyLedger] = []
    for cp in contract_properties(contract):
        pid = cp["id"]
        mv = cir_props.get(pid)
        model_verified = mv == "PASS" and cir_complete is True
        status = observed_status(pid)
        if status in {"unsupported", "unmapped", "not_observed", "deferred"}:
            coverage = "not_carryable" if cp["kind"] not in TRACE_DECIDABLE else "not_observed"
        else:
            coverage = "observed"
        # Implementation obligations are met only by a non-vacuous conformant
        # projection plus an observed property (trace-decidable kinds).
        if status == "FAIL":
            satisfied, claim = False, "violated"
        elif cp["kind"] == "deadlock_free":
            if result.get("hang"):
                satisfied, claim = False, "inconclusive"  # not a proved deadlock
            elif observed_trace_conformant and bounded_no_violation and model_verified:
                satisfied, claim = True, "supported"
            else:
                satisfied, claim = False, "inconclusive"
        elif status == "PASS_bounded" and observed_trace_conformant:
            satisfied, claim = True, "supported"
        else:
            satisfied, claim = False, "inconclusive"
        props.append(PropertyLedger(
            property_id=pid, kind=cp["kind"], requirements=cp.get("req", []),
            model_verified=model_verified, model_complete=cir_complete,
            implementation_obligation=obligation_for(cp["kind"]),
            observed_trace_conformant=observed_trace_conformant,
            bounded_no_violation=bounded_no_violation,
            property_observation_coverage=coverage,
            implementation_obligations_satisfied=satisfied,
            independent_requirement_result=status,
            final_claim=claim,
            evidence_ref=f"{result.get('cell','')}/result.json#{pid}"))

    conform_present = bool(conform and conform.get("traces", 0) > 0)
    reasons: list[str] = []
    if not accepted:
        verdict = "not_accepted"
        reasons.append("no accepted round")
    elif stages.get("source_build") == "failed":
        verdict = "explicit_failure"
        reasons.append("the source does not compile independently")
    elif not hashes_present:
        verdict = "insufficient"
        reasons.append("missing source/CIR/contract hash binding")
    elif any(p.final_claim == "violated" for p in props):
        verdict = "explicit_failure"
        reasons.append("a declared property is violated")
    elif not conform_present:
        verdict = "insufficient"
        reasons.append("no conformance result: absence of a violation is not evidence")
    elif projected == 0:
        verdict = "insufficient"
        reasons.append("empty projection: the language check found no violation "
                       "but there is no implementation correspondence evidence")
    elif not observed_trace_conformant:
        verdict = "explicit_failure"
        reasons.append("conformance failed on a non-empty projection")
    elif any(p.final_claim == "inconclusive" for p in props):
        verdict = "insufficient"
        reasons.append("some declared properties lack decisive evidence")
    else:
        verdict = "sufficient"

    return ReexecutionLedger(
        cell=result.get("cell", ""), accepted=accepted,
        source_build=stages.get("source_build", "not_run"),
        instrumented_build=stages.get("instrumented_build", "not_run"),
        model_verified=bool(cir_props) and cir_complete is True,
        observed_trace_conformant=observed_trace_conformant,
        bounded_no_violation=bounded_no_violation, raw_events=raw,
        projected_events=projected, thread_lifecycle=result.get("thread_lifecycle", {}),
        binding_verified=len(mapping), binding_unresolved=len(ambiguous),
        hashes_present=hashes_present, properties=props, verdict=verdict, reasons=reasons)
