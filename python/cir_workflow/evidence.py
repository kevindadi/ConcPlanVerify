"""Per-property evidence ledger (strong-link v1).

Each declared contract property gets an independent record. The ledger never
upgrades a tool limitation into a pass, never falls back to a rejected round as
if it were accepted, and never lets a model proof stand in for implementation
evidence that the property's own obligation requires.

Per property:

- ``model_verdict``               CIR exploration outcome (PASS/FAIL/None)
- ``model_complete``              exploration completeness
- ``implementation_obligation``   what the Rust must preserve for this property
- ``implementation_evidence``     op-resource conformance + observed coverage
- ``observed_trace_result``       raw monitor status (unsupported/unmapped/...)
- ``independent_requirement_result``  a non-trace check, or ``none``
- ``final_claim``                 supported | violated | inconclusive | not_applicable
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import asdict, dataclass, field
from pathlib import Path

OBSERVABLE_KINDS = {
    "mutex_lock", "mutex_unlock", "read_lock", "write_lock",
    "condvar_wait", "condvar_notify", "condvar_notify_all",
    "semaphore_acquire", "semaphore_release",
    "channel_send", "channel_recv", "send", "recv",
}
SPAWN_KINDS = {"spawn", "spawn_async", "scope"}

# Property kinds a finite observed trace can carry.
TRACE_DECIDABLE = {"safety", "never_holds_all", "unreachable", "reachable",
                   "holds_all", "always_reachable", "deadlock_free", "reachable_all"}


def _sha(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def _norm(pid: str) -> str:
    return (pid or "").replace("preserved: ", "").strip()


def contract_properties(contract: dict) -> list[dict]:
    """Enumerate every declared property from the frozen contract."""

    out: list[dict] = []
    for p in contract.get("properties", []) or []:
        out.append({"id": p.get("id"), "kind": p.get("kind"),
                    "req": p.get("req", []), "source": "properties",
                    "goal": p.get("goal")})
    for p in contract.get("preserved", []) or []:
        out.append({"id": p.get("description") or p.get("id"), "kind": p.get("kind"),
                    "req": p.get("req", []), "source": "preserved",
                    "goal": p.get("goal")})
    return out


def obligation_for(kind: str) -> str:
    if kind == "deadlock_free":
        return "no reachable deadlock; blocked/enabled relations preserved"
    if kind in {"safety", "never_holds_all", "unreachable"}:
        return "the forbidden state is never observed"
    if kind in {"reachable", "holds_all", "always_reachable", "reachable_all"}:
        return "the goal state is reached and observed"
    return "the property holds under the implementation"


@dataclass
class PropertyEvidence:
    property_id: str
    kind: str
    requirements: list[str]
    model_verdict: str | None
    model_complete: bool | None
    implementation_obligation: str
    implementation_evidence: str
    observed_trace_result: str
    independent_requirement_result: str
    final_claim: str
    reason_and_artifact_refs: str


@dataclass
class CellEvidence:
    cell: str
    accepted: bool
    model_verified: bool
    model_complete: bool | None
    bounded_trace_pass: bool
    evidence_sufficient: bool
    correspondence: str
    observed_events: int
    observable_ops: int
    spawns: int
    source_sha256: str | None
    properties: list[PropertyEvidence] = field(default_factory=list)
    verdict: str = "not_accepted"
    reasons: list[str] = field(default_factory=list)
    human_review: str = "pending"

    def to_dict(self) -> dict:
        data = asdict(self)
        data["properties"] = [asdict(p) for p in self.properties]
        return data


def cir_observability(cir_path: Path) -> tuple[int, int]:
    try:
        cir = json.loads(cir_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return 0, 0
    ops = spawns = 0
    for module in cir.get("modules", []):
        for fn in module.get("functions", []):
            for stmt in fn.get("body", []):
                kind = stmt.get("kind", "")
                ops += kind in OBSERVABLE_KINDS
                spawns += kind in SPAWN_KINDS
    return ops, spawns


def _accepted_cir(cell_dir: Path) -> Path | None:
    revs = sorted(cell_dir.glob("cir/revision-*.cir.json"),
                  key=lambda p: int(p.stem.split("-")[1].split(".")[0]))
    return revs[-1] if revs else None


def _accepted_round(cell_dir: Path, round_no: int | None) -> Path | None:
    """The accepted round only; never a fallback to a rejected round."""

    if not round_no:
        return None
    cand = cell_dir / "code" / f"round-{round_no}"
    return cand if (cand / "monitor.json").is_file() else None


def load_cir_properties(cell_dir: Path, accepted_cir: Path | None
                        ) -> tuple[dict[str, str], bool | None]:
    if accepted_cir is None:
        return {}, None
    target = _sha(accepted_cir)
    for d in sorted((cell_dir / "cir/calls").glob("*-explore")):
        prog, out = d / "program.json", d / "stdout.json"
        if not prog.is_file() or not out.is_file() or _sha(prog) != target:
            continue
        try:
            data = json.loads(out.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        return ({_norm(p["id"]): p.get("outcome") for p in data.get("properties", [])},
                data.get("complete"))
    return {}, None


def _claim_for(kind: str, observed: str, model_verdict: str | None,
               model_complete: bool | None, correspondence: str,
               behavior_ok: bool | None) -> tuple[str, str]:
    """Return (final_claim, reason). Conservative by design."""

    if observed == "FAIL":
        return "violated", "monitor reported FAIL"
    if model_verdict == "FAIL":
        return "violated", "CIR exploration refuted the property"
    cir_proved = model_verdict == "PASS" and model_complete is True
    if kind == "deadlock_free":
        if behavior_ok is False:
            return "violated", "the bounded run did not complete on every run"
        if cir_proved and behavior_ok is True:
            return "supported", "CIR proved deadlock-freedom and every run completed"
        if cir_proved:
            return "inconclusive", "CIR proved deadlock-freedom but no run outcome is recorded"
        return "inconclusive", "deferred and no complete CIR proof attached"
    if observed == "PASS_bounded" and correspondence == "supported":
        return "supported", "observed on every bounded run and correspondence evidenced"
    if observed == "PASS_bounded" and correspondence == "not_applicable":
        return "supported", "observed on every bounded run"
    if observed in {"unsupported", "unmapped", "not_observed", "deferred"}:
        if kind not in TRACE_DECIDABLE:
            return "inconclusive", (f"monitor status {observed}: a trace cannot carry "
                                    "this property, and a model proof does not replace "
                                    "implementation evidence")
        if correspondence == "insufficient":
            return "inconclusive", (f"monitor status {observed} and implementation "
                                    "correspondence has no observed evidence")
        return "inconclusive", f"monitor status {observed}: no decisive evidence"
    if correspondence == "insufficient":
        return "inconclusive", "no relevant events observed"
    return "inconclusive", f"unrecognised monitor status {observed}"


def evaluate_cell(cell_dir: Path, cell_row: dict, contract: dict) -> CellEvidence:
    accepted = cell_row.get("accepted") is True
    accepted_cir = _accepted_cir(cell_dir)
    ops, spawns = cir_observability(accepted_cir) if accepted_cir else (0, 0)
    cir_props, cir_complete = load_cir_properties(cell_dir, accepted_cir)
    round_no = cell_row.get("first_code_accept_round") if accepted else None
    rnd = _accepted_round(cell_dir, round_no)

    observed = 0
    if rnd and (rnd / "conform-traces").is_dir():
        for f in sorted((rnd / "conform-traces").glob("*.jsonl")):
            observed += len([ln for ln in f.read_text(encoding="utf-8").splitlines()
                             if ln.strip()])
    if ops + spawns == 0:
        correspondence = "not_applicable"
    elif observed == 0:
        correspondence = "insufficient"
    else:
        correspondence = "supported"

    monitor = {}
    if rnd and (rnd / "monitor.json").is_file():
        monitor = json.loads((rnd / "monitor.json").read_text(encoding="utf-8"))
    status_by_id = {_norm(p.get("id")): p.get("status", "not_observed")
                    for p in monitor.get("properties", [])}
    behavior_ok = cell_row.get("behavior_ok")

    props: list[PropertyEvidence] = []
    for cp in contract_properties(contract):
        pid = cp["id"]
        observed_status = status_by_id.get(_norm(pid), "not_observed")
        mv = cir_props.get(_norm(pid))
        claim, reason = _claim_for(cp["kind"], observed_status, mv, cir_complete,
                                   correspondence, behavior_ok)
        props.append(PropertyEvidence(
            property_id=pid, kind=cp["kind"], requirements=cp.get("req", []),
            model_verdict=mv, model_complete=cir_complete,
            implementation_obligation=obligation_for(cp["kind"]),
            implementation_evidence=(f"conform-traces events={observed}; "
                                     f"correspondence={correspondence}"),
            observed_trace_result=observed_status,
            independent_requirement_result="none",
            final_claim=claim, reason_and_artifact_refs=reason))

    model_verified = bool(cell_row.get("cir_accepted")) and cir_complete is True
    bounded_trace_pass = bool(monitor) and all(
        s in {"PASS_bounded", "deferred", "unverifiable", "unsupported", "unmapped",
              "not_observed"} and s != "FAIL"
        for s in status_by_id.values()) and not monitor.get("properties", []) == []

    reasons: list[str] = []
    if not accepted:
        verdict = "not_accepted"
        reasons.append("no accepted round; the ledger does not fall back to a rejected round")
    elif behavior_ok is False:
        verdict = "explicit_failure"
        reasons.append("the bounded run did not complete on every run")
    elif any(p.final_claim == "violated" for p in props):
        verdict = "explicit_failure"
        reasons.append("at least one property is violated")
    elif not props:
        verdict = "insufficient"
        reasons.append("no declared properties to check")
    elif correspondence == "insufficient":
        verdict = "insufficient"
        reasons.append("no relevant events observed though the CIR models observable ops")
    elif any(p.final_claim == "inconclusive" for p in props):
        verdict = "insufficient"
        reasons.append("some declared properties lack decisive evidence")
    elif _sha(cell_dir / f"code/round-{round_no}.rs") is None:
        verdict = "insufficient"
        reasons.append("missing accepted source hash")
    else:
        verdict = "sufficient"

    return CellEvidence(
        cell=cell_row.get("cell") or f"{cell_row.get('model')}/{cell_row.get('task')}",
        accepted=accepted, model_verified=model_verified, model_complete=cir_complete,
        bounded_trace_pass=bounded_trace_pass, evidence_sufficient=(verdict == "sufficient"),
        correspondence=correspondence, observed_events=observed, observable_ops=ops,
        spawns=spawns,
        source_sha256=_sha(cell_dir / f"code/round-{round_no}.rs") if round_no else None,
        properties=props, verdict=verdict, reasons=reasons)
