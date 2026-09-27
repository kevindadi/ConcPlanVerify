"""Single candidate-evaluation entry (strong-link-v5).

Both the online generation loop and the offline re-execution call this module.
It separates *how a candidate is produced* (generation) from *how it is
evaluated* (evidence acquisition + interpretation):

- ``interpret(result, contract, ...)`` turns a recorded evidence bundle into the
  layered ledger (``evidence_v2.evaluate_reexecution``) - the single rule.
- ``evaluate_candidate(...)`` runs the full offline chain and returns that
  ledger.

Delivery policy and evidence grade are separate: ``historical_acceptance`` is
the old pipeline's fact; ``current_evaluation`` comes from current evidence; a
new run's acceptance uses ``ACCEPTANCE_POLICY``.
"""

from __future__ import annotations

from pathlib import Path

from .evidence_v2 import ReexecutionLedger, evaluate_reexecution

ACCEPTANCE_POLICY = "ledger-v5"  # new-run policy; not applied to history


def interpret(result: dict, contract: dict, *, accepted: bool,
              cir_props: dict[str, str] | None = None,
              cir_complete: bool | None = None) -> ReexecutionLedger:
    """The single interpretation rule (online and offline both use this)."""

    return evaluate_reexecution(result, contract, accepted=accepted,
                                cir_props=cir_props or {}, cir_complete=cir_complete)


def evaluate_candidate(source: str, cir_path: Path, contract_path: Path, out_dir: Path,
                       *, binary, instrument, n_runs: int = 32, run_timeout: float = 10.0,
                       accepted: bool = False, cir_props: dict | None = None,
                       cir_complete: bool | None = None, functional: dict | None = None,
                       manifest_path: Path | None = None, cell_id: str = "",
                       candidate_kind: str = "final",
                       round_no: int | None = None) -> dict:
    """Run the full offline chain and return the result (with ``ledger``)."""

    import sys

    repo = Path(__file__).resolve().parents[2]
    if str(repo) not in sys.path:
        sys.path.insert(0, str(repo))
    from scripts.reexecute_and_bind import reexecute  # type: ignore

    return reexecute(source, cir_path, contract_path, out_dir, binary=binary,
                     instrument=instrument, n_runs=n_runs, run_timeout=run_timeout,
                     accepted=accepted, cir_props=cir_props, cir_complete=cir_complete,
                     functional=functional, manifest_path=manifest_path,
                     cell_id=cell_id, candidate_kind=candidate_kind, round_no=round_no)
