"""Bounded requirement oracle for Rust artifacts (§1 harness).

Wraps `concir-backend monitor` and turns its per-clause verdicts into
per-requirement statuses, so the Rust arms of the generation batch can be
scored with the same RC/RF metrics as the model arm.

The oracle is deliberately *bounded*: it observes a finite set of runs.  The
clause status vocabulary is:

  PASS_bounded  every observed state satisfied the clause (ForAll), or at least
                one observed state satisfied it (Exists)
  FAIL          a required-to-hold clause was violated on some run
  not_observed  an Exists clause was never witnessed
  unmapped      a resource in the clause could not be aligned to the trace
  unsupported   the trace stream cannot carry the predicate (e.g. value
                predicates, or a completion event the instrumenter did not emit)
  deferred      the clause is decided outside the monitor (deadlock_free via
                behavior/Miri); resolved by ``behavior_ok`` when supplied.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

_DECIDABLE = {"PASS_bounded", "FAIL", "not_observed"}
_RANK = ["FAIL", "unmapped", "unsupported", "not_observed", "deferred",
         "PASS_bounded"]


def resolve_binary(binary: str | Path | None = None) -> Path | None:
    import os
    if binary is not None:
        path = Path(binary)
        return path if path.is_file() else None
    env = os.environ.get("CONCIR_BACKEND")
    if env and Path(env).is_file():
        return Path(env)
    candidates = [
        Path("/Users/kevin/local-repos/ConcIR/target/release/concir-backend"),
        Path("/Users/kevin/local-repos/ConcIR/target/debug/concir-backend"),
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def run_monitor(
    contract: str | Path,
    traces: str | Path,
    *,
    resources: str | Path | None = None,
    mapping: str | Path | None = None,
    binary: str | Path | None = None,
    timeout: float = 180.0,
) -> dict[str, Any]:
    """Invoke `concir-backend monitor` and return its report (FAIL is not fatal)."""
    exe = resolve_binary(binary)
    if exe is None:
        raise FileNotFoundError("concir-backend not found")
    argv = [str(exe), "monitor", "--contract", str(contract), "--traces", str(traces)]
    if resources is not None:
        argv += ["--resources", str(resources)]
    if mapping is not None:
        argv += ["--mapping", str(mapping)]
    proc = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    if not proc.stdout.strip():
        raise RuntimeError(f"monitor produced no output (exit {proc.returncode}): "
                           f"{proc.stderr[-400:]}")
    report = json.loads(proc.stdout)
    report["_exit"] = proc.returncode
    report["_argv"] = argv
    report["_stderr"] = proc.stderr[-2000:]
    return report


def _clause_priority(statuses: list[str]) -> str:
    for candidate in _RANK:
        if candidate in statuses:
            return candidate
    return "not_observed"


def requirement_statuses(
    contract: dict[str, Any],
    report: dict[str, Any],
    requirements: list[str],
    unverifiable: list[int],
    *,
    behavior_ok: bool | None = None,
) -> dict[str, str]:
    """Map clause verdicts onto requirement ids.

    ``behavior_ok`` resolves a `deferred` deadlock_free clause: True -> the run
    terminated (PASS_bounded), False -> hang/timeout (FAIL), None -> stays
    deferred.  Requirements with no covering clause are `unverifiable`.
    """
    total = len(requirements)
    unv = {f"R{i}" for i in unverifiable}
    by_req: dict[str, list[str]] = {f"R{i}": [] for i in range(1, total + 1)}
    for prop in report.get("properties", []):
        status = prop.get("status", "not_observed")
        if status == "deferred" and behavior_ok is not None:
            status = "PASS_bounded" if behavior_ok else "FAIL"
        for rid in prop.get("req", []):
            if rid in by_req:
                by_req[rid].append(status)
    out: dict[str, str] = {}
    for rid in by_req:
        if not by_req[rid]:
            out[rid] = "unverifiable" if rid in unv else "not_observed"
        else:
            out[rid] = _clause_priority(by_req[rid])
    return out


@dataclass
class Coverage:
    total: int
    decidable: int = 0
    satisfied: int = 0
    failed: int = 0
    not_observed: int = 0
    unmapped: int = 0
    unsupported: int = 0
    deferred: int = 0
    unverifiable: int = 0
    statuses: dict[str, str] = field(default_factory=dict)

    @property
    def rc(self) -> float:
        return (self.decidable / self.total) if self.total else 0.0

    @property
    def rf(self) -> float:
        return (self.satisfied / self.total) if self.total else 0.0

    def as_dict(self) -> dict[str, Any]:
        return {
            "total": self.total, "decidable": self.decidable,
            "satisfied": self.satisfied, "failed": self.failed,
            "not_observed": self.not_observed, "unmapped": self.unmapped,
            "unsupported": self.unsupported, "deferred": self.deferred,
            "unverifiable": self.unverifiable, "rc": self.rc, "rf": self.rf,
            "statuses": self.statuses,
        }


def coverage(
    contract: dict[str, Any],
    report: dict[str, Any],
    requirements: list[str],
    unverifiable: list[int],
    *,
    behavior_ok: bool | None = None,
) -> Coverage:
    statuses = requirement_statuses(contract, report, requirements, unverifiable,
                                    behavior_ok=behavior_ok)
    cov = Coverage(total=len(requirements), statuses=statuses)
    for status in statuses.values():
        if status in _DECIDABLE:
            cov.decidable += 1
        if status == "PASS_bounded":
            cov.satisfied += 1
        elif status == "FAIL":
            cov.failed += 1
        elif status == "not_observed":
            cov.not_observed += 1
        elif status == "unmapped":
            cov.unmapped += 1
        elif status == "unsupported":
            cov.unsupported += 1
        elif status == "deferred":
            cov.deferred += 1
        elif status == "unverifiable":
            cov.unverifiable += 1
    return cov
