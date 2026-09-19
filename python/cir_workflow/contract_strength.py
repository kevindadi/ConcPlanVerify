"""Contract-strength recomputation (G-1).

Enumerates A3 accepted CIRs from historical batch directories and replays each
against **the task's frozen contract** (`benchmarks/families/<task>/contract.json`)
as well as the old contract stored in that run directory, recording both
outcomes and contract hashes. No LLM; the script is the reproducible source of
`experiments/contract-strength-v1/CONTRACT_STRENGTH.json`.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

# task id from a run path fragment
TASK_MARKERS = (
    ("partial_deadlock_bystander", "lock-order/partial_deadlock_bystander"),
    ("abba_2lock", "lock-order/abba_2lock"),
    ("cross_module_cycle", "lock-order/cross_module_cycle"),
    ("cycle_3lock", "lock-order/cycle_3lock"),
    ("bare_wait_no_predicate", "condvar/bare_wait_no_predicate"),
    ("lost_wakeup_notify_before_wait", "condvar/lost_wakeup_notify_before_wait"),
    ("nested_scope_lock_order", "structure/nested_scope_lock_order"),
    ("scope_worker_abba", "structure/scope_worker_abba"),
    ("notify_one_multi_waiter_wrong_pick", "condvar/notify_one_multi_waiter_wrong_pick"),
    ("bounded_backpressure_lock_held", "channel/bounded_backpressure_lock_held"),
    ("send_while_holding_mutex", "channel/send_while_holding_mutex"),
    ("acquire_twice_no_release", "semaphore/acquire_twice_no_release"),
    ("permit_leak", "semaphore/permit_leak"),
    ("/P1/", "lock-order/abba_2lock"),
    ("/P4/", "lock-order/cycle_3lock"),
)


@dataclass
class Record:
    run: str
    task: str
    accepted_version: int | None
    cir_sha256: str
    old_contract_path: str | None
    old_contract_sha256: str | None
    old_outcome: str | None
    frozen_contract_path: str
    frozen_contract_sha256: str
    new_outcome: str
    rejected: list[str]
    binary_sha256: str
    git_rev: str


def _sha(path: Path) -> str:
    import hashlib

    return hashlib.sha256(path.read_bytes()).hexdigest()


def _binary(binary: Path | str | None) -> Path:
    if binary is not None:
        return Path(binary).resolve()
    import os

    env = os.environ.get("CONCIR_BACKEND")
    if env:
        return Path(env).resolve()
    repo = Path(__file__).resolve().parents[2]
    return (repo.parent / "ConcIR/target/release/concir-backend").resolve()


def _binary_sha(binary: Path) -> str:
    return _sha(binary)


def _git_rev(binary: Path) -> str:
    try:
        repo = binary.resolve().parents[2]
        out = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                             capture_output=True, text=True, timeout=10)
        if out.returncode == 0:
            return out.stdout.strip()
    except Exception:  # noqa: BLE001
        pass
    return "unknown"


def _task_for(path: str) -> str | None:
    for needle, task in TASK_MARKERS:
        if needle in path:
            return task
    return None


def _explore(binary: Path, cir: Path, contract: Path) -> tuple[str, list[str]]:
    proc = subprocess.run([str(binary), "explore", str(cir), str(contract), "petri"],
                          capture_output=True, text=True, timeout=120)
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return "PROTOCOL_ERROR", []
    rejected = [p["id"] for p in payload.get("properties", []) if p.get("outcome") != "PASS"]
    return str(payload.get("outcome")), rejected


def recompute(batch_dirs: list[Path | str], workspace: Path | str, *,
              binary: Path | str | None = None) -> list[Record]:
    workspace = Path(workspace).resolve()
    bin_path = _binary(binary)
    bsha, rev = _binary_sha(bin_path), _git_rev(bin_path)
    records: list[Record] = []
    seen: set[str] = set()
    for batch in batch_dirs:
        for result_path in sorted(Path(batch).rglob("result.json")):
            try:
                data = json.loads(result_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                continue
            if data.get("repair_mode") != "llm_revision":
                continue
            accepted = next((v for v in data.get("versions", []) if v.get("accepted")), None)
            if not accepted:
                continue
            cir = accepted.get("artifact_path")
            if not cir or not Path(cir).is_file() or cir in seen:
                continue
            task = _task_for(str(result_path))
            if task is None:
                continue
            seen.add(cir)
            frozen = workspace / "benchmarks/families" / task / "contract.json"
            if not frozen.is_file():
                continue
            old = result_path.parent / "contract.json"
            old_outcome = None
            old_sha = None
            if old.is_file():
                old_sha = _sha(old)
                old_outcome, _ = _explore(bin_path, Path(cir), old)
            new_outcome, rejected = _explore(bin_path, Path(cir), frozen)
            records.append(Record(
                run=str(result_path.parent), task=task,
                accepted_version=accepted.get("version"),
                cir_sha256=_sha(Path(cir)),
                old_contract_path=str(old) if old.is_file() else None,
                old_contract_sha256=old_sha, old_outcome=old_outcome,
                frozen_contract_path=str(frozen),
                frozen_contract_sha256=_sha(frozen),
                new_outcome=new_outcome, rejected=rejected,
                binary_sha256=bsha, git_rev=rev))
    return records


def write_report(records: list[Record], out_dir: Path | str) -> dict[str, Any]:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": "contract-strength-v1",
        "precondition": "build_families.py validates 21 buggy FAIL / all fixed PASS "
                        "under the frozen contracts",
        "records": [r.__dict__ for r in records],
    }
    (out / "CONTRACT_STRENGTH.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    lines = ["# Contract strength (frozen contracts, G-1)", "",
             payload["precondition"], "",
             "| task | version | old outcome | frozen outcome | rejected by frozen contract |",
             "| --- | --- | --- | --- | --- |"]
    for r in records:
        lines.append(f"| {r.task} | {r.accepted_version} | {r.old_outcome} | "
                     f"{r.new_outcome} | {'; '.join(r.rejected) or '—'} |")
    (out / "CONTRACT_STRENGTH.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return payload
