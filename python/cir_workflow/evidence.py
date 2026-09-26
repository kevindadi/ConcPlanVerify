"""Per-property evidence ledger.

Separates the claims the historical ``accepted`` flag conflated:

- ``cir``  — the property was proved by exhaustive CIR exploration (with the
  exploration completeness);
- ``impl`` — the accepted Rust faithfully corresponds to the CIR part that
  carries the property (operation-bound conformance evidence);
- ``run``  — the bounded run did not violate it.

``unsupported`` / ``unmapped`` / ``not_observed`` are inconclusive, never PASS.
``deadlock_free`` reported ``deferred`` is linked to the CIR proof and the
bounded run. A finite run that observed no relevant events is not evidence that
the implementation matches the model.
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


def _sha(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def _norm(pid: str) -> str:
    return pid.replace("preserved: ", "").strip()


@dataclass
class PropertyVerdict:
    property_id: str
    kind: str
    requirements: list[str]
    monitor_status: str
    cir_outcome: str | None       # PASS | FAIL | None
    cir_complete: bool | None
    impl_correspondence: str      # supported | insufficient | not_applicable
    verdict: str                  # supported | violated | inconclusive | not_applicable
    basis: str


@dataclass
class CellEvidence:
    cell: str
    cir_verified: bool
    cir_complete: bool | None
    observable_ops: int
    spawns: int
    observed_events: int
    traces_total: int
    traces_empty: int
    correspondence: str
    properties: list[PropertyVerdict] = field(default_factory=list)
    verdict: str = "inconclusive"
    reasons: list[str] = field(default_factory=list)
    source_sha256: str | None = None
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


def _accepted_round(cell_dir: Path, hint: int | None) -> Path | None:
    if hint:
        cand = cell_dir / "code" / f"round-{hint}"
        if (cand / "monitor.json").is_file():
            return cand
    for r in reversed(sorted(cell_dir.glob("code/round-*"))):
        if (r / "monitor.json").is_file():
            return r
    return None


def load_cir_properties(cell_dir: Path, accepted_cir: Path | None
                        ) -> tuple[dict[str, str], bool | None]:
    """Find the explore call whose program matches the accepted CIR."""

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
        props = {_norm(p["id"]): p.get("outcome")
                 for p in data.get("properties", [])}
        return props, data.get("complete")
    return {}, None


def classify_property(prop: dict, *, cir_outcome: str | None, cir_complete: bool | None,
                      impl: str, behavior_ok: bool | None) -> PropertyVerdict:
    status = prop.get("status", "not_observed")
    kind = prop.get("kind", "")
    pid = prop.get("id", "")
    reqs = prop.get("req", [])
    if status == "FAIL":
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "violated", "monitor reported FAIL")
    if cir_outcome == "FAIL":
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "violated", "CIR exploration refuted the property")
    cir_proved = cir_outcome == "PASS" and cir_complete is True
    if cir_proved and impl in {"supported", "not_applicable"}:
        basis = "CIR proved it and the implementation correspondence is evidenced"
        if impl == "not_applicable":
            basis = "CIR proved it; no observable operation to contradict it"
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "supported", basis)
    if cir_proved and impl == "insufficient":
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "inconclusive",
                               "CIR proved it, but implementation correspondence "
                               "has no observed evidence")
    if status == "PASS_bounded" and impl != "insufficient":
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "supported", "observed on every bounded run")
    if status == "deferred" and kind == "deadlock_free":
        return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                               impl, "inconclusive",
                               "deferred and no complete CIR proof attached")
    return PropertyVerdict(pid, kind, reqs, status, cir_outcome, cir_complete,
                           impl, "inconclusive",
                           f"monitor status {status}; CIR {cir_outcome or 'absent'}")


def evaluate_cell(cell_dir: Path, cell_row: dict) -> CellEvidence:
    accepted_cir = _accepted_cir(cell_dir)
    ops, spawns = cir_observability(accepted_cir) if accepted_cir else (0, 0)
    cir_props, cir_complete = load_cir_properties(cell_dir, accepted_cir)
    accepted_round = _accepted_round(cell_dir, cell_row.get("first_code_accept_round"))
    traces_total = traces_empty = observed = 0
    if accepted_round and (accepted_round / "conform-traces").is_dir():
        for f in sorted((accepted_round / "conform-traces").glob("*.jsonl")):
            traces_total += 1
            lines = [ln for ln in f.read_text(encoding="utf-8").splitlines() if ln.strip()]
            traces_empty += (not lines)
            observed += len(lines)
    if ops + spawns == 0:
        correspondence = "not_applicable"
    elif observed == 0:
        correspondence = "insufficient"
    else:
        correspondence = "supported"

    monitor = {}
    if accepted_round and (accepted_round / "monitor.json").is_file():
        monitor = json.loads((accepted_round / "monitor.json").read_text(encoding="utf-8"))
    behavior_ok = cell_row.get("behavior_ok")
    props = [classify_property(p, cir_outcome=cir_props.get(_norm(p.get("id", ""))),
                               cir_complete=cir_complete, impl=correspondence,
                               behavior_ok=behavior_ok)
             for p in monitor.get("properties", [])]

    reasons: list[str] = []
    if any(p.verdict == "violated" for p in props):
        verdict = "explicit_failure"
    elif correspondence == "insufficient":
        verdict = "insufficient"
        reasons.append("no relevant events observed though the CIR models observable ops")
    elif any(p.verdict == "inconclusive" for p in props):
        verdict = "insufficient"
        reasons.append("some declared properties lack decisive evidence")
    else:
        verdict = "sufficient"

    return CellEvidence(
        cell=cell_row.get("cell") or f"{cell_row.get('model')}/{cell_row.get('task')}",
        cir_verified=bool(cell_row.get("cir_accepted")), cir_complete=cir_complete,
        observable_ops=ops, spawns=spawns, observed_events=observed,
        traces_total=traces_total, traces_empty=traces_empty,
        correspondence=correspondence, properties=props, verdict=verdict,
        reasons=reasons,
        source_sha256=_sha(cell_dir / f"code/round-{cell_row.get('first_code_accept_round')}.rs")
        if cell_row.get("first_code_accept_round") else None)
