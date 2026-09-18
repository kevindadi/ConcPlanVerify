"""Shared types for the v2 comparison framework (protocol: EXPERIMENTS_V2_PROTOCOL.md).

This module owns only bookkeeping that is model- and tool-agnostic:

- arm identifiers and frozen round cap;
- measured consumption accounting (HTTP, tokens, LLM/tool wall-clock, rounds);
- per-round and per-arm records with stable JSON shapes;
- an independent terminal-verdict record and the derived false-accept /
  behavior-loss / conservative-reject flags.

It never invokes the model, the backend or a cargo tool; it never invents a
measurement. Missing provider usage is recorded as ``None`` (unknown), not zero.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# Frozen arm ids (protocol B3).
ARM_DIRECT = "A0_direct"
ARM_SELF_ITER = "A1_self_iter"
ARM_TOOLS_ITER = "A2_tools_iter"
ARM_OURS_REVISION = "A3_ours_revision"
ARM_OURS_PATCH = "A3p_ours_patch"
ARM_TOOL_REPAIR = "A3_tool_repair"
ARM_ABLATIONS = ("A3_nodiag", "A3_nopreserved", "A3_nofidelity")
ARMS = (ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER, ARM_OURS_REVISION,
        ARM_OURS_PATCH, ARM_TOOL_REPAIR) + ARM_ABLATIONS

# Frozen constants (protocol "Frozen constants").
K = 4
DIAGNOSTIC_TRUNCATION_BYTES = 8192
LIVE_HTTP_CAP = 260
LIVE_WALL_CAP_S = 14400.0
MIRI_COMBOS = ((0, 0.01), (1, 0.05), (2, 0.10), (3, 0.20), (4, 0.50))


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def sha256_file(path: Path | str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


@dataclass
class Consumption:
    """Actual measured cost. Unknown usage stays ``None``."""

    http_requests: int = 0
    prompt_tokens: int | None = 0
    completion_tokens: int | None = 0
    llm_wall_ms: int = 0
    tool_wall_ms: int = 0
    rounds: int = 0
    first_correct_round: int | None = None

    def add_usage(self, usage: dict[str, Any] | None) -> None:
        if usage is None:
            self.prompt_tokens = None
            self.completion_tokens = None
            return
        prompt = usage.get("prompt_tokens", usage.get("input_tokens"))
        completion = usage.get("completion_tokens", usage.get("output_tokens"))
        if prompt is None or completion is None:
            self.prompt_tokens = None
            self.completion_tokens = None
            return
        if self.prompt_tokens is not None:
            self.prompt_tokens += int(prompt)
        if self.completion_tokens is not None:
            self.completion_tokens += int(completion)

    @property
    def total_tokens(self) -> int | None:
        if self.prompt_tokens is None or self.completion_tokens is None:
            return None
        return self.prompt_tokens + self.completion_tokens

    def as_dict(self) -> dict[str, Any]:
        return {
            "http_requests": self.http_requests,
            "prompt_tokens": self.prompt_tokens,
            "completion_tokens": self.completion_tokens,
            "total_tokens": self.total_tokens,
            "llm_wall_ms": self.llm_wall_ms,
            "tool_wall_ms": self.tool_wall_ms,
            "rounds": self.rounds,
            "first_correct_round": self.first_correct_round,
        }


@dataclass
class RoundRecord:
    round: int
    request_sha256: str | None = None
    prompt_tokens: int | None = None
    completion_tokens: int | None = None
    llm_wall_ms: int = 0
    tool_wall_ms: int = 0
    tool_output_sha256: str | None = None
    decision: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return dict(self.__dict__)


@dataclass
class ArmResult:
    arm: str
    task: str
    rounds: list[RoundRecord] = field(default_factory=list)
    arm_accepted: bool = False
    accepted_round: int | None = None
    consumption: Consumption = field(default_factory=Consumption)
    final_artifact_path: str | None = None
    notes: dict[str, Any] = field(default_factory=dict)

    def as_dict(self) -> dict[str, Any]:
        return {
            "arm": self.arm,
            "task": self.task,
            "rounds": [r.as_dict() for r in self.rounds],
            "arm_accepted": self.arm_accepted,
            "accepted_round": self.accepted_round,
            "consumption": self.consumption.as_dict(),
            "final_artifact_path": self.final_artifact_path,
            "notes": self.notes,
        }


@dataclass
class RustOracle:
    """Terminal verdict for a Rust artifact (four separate checks)."""

    build_ok: bool | None = None
    behavior_test_ok: bool | None = None
    lockbud_detected: bool | None = None
    miri_detected: bool | None = None
    bug_present: bool | None = None
    evidence: dict[str, Any] = field(default_factory=dict)

    @property
    def clean(self) -> bool | None:
        if self.bug_present is None:
            return None
        return not self.bug_present

    def as_dict(self) -> dict[str, Any]:
        return {
            "build_ok": self.build_ok,
            "behavior_test_ok": self.behavior_test_ok,
            "lockbud_detected": self.lockbud_detected,
            "miri_detected": self.miri_detected,
            "bug_present": self.bug_present,
            "clean": self.clean,
            "evidence": self.evidence,
        }


@dataclass
class CirOracle:
    verify_pass: bool | None = None
    preserved_ok: bool | None = None
    fidelity_ok: bool | None = None
    bug_present: bool | None = None
    evidence: dict[str, Any] = field(default_factory=dict)

    @property
    def clean(self) -> bool | None:
        if self.bug_present is None:
            return None
        return not self.bug_present

    def as_dict(self) -> dict[str, Any]:
        return {
            "verify_pass": self.verify_pass,
            "preserved_ok": self.preserved_ok,
            "fidelity_ok": self.fidelity_ok,
            "bug_present": self.bug_present,
            "clean": self.clean,
            "evidence": self.evidence,
        }


def derived_flags(*, arm_accepted: bool,
                  oracle_bug_present: bool | None,
                  behavior_test_ok: bool | None = None) -> dict[str, bool | None]:
    """Derived, separately reported outcomes (protocol B4)."""

    if oracle_bug_present is None:
        false_accept: bool | None = None
        conservative_reject: bool | None = None
    else:
        false_accept = bool(arm_accepted and oracle_bug_present)
        conservative_reject = bool((not arm_accepted) and (not oracle_bug_present))
    if behavior_test_ok is None:
        behavior_loss: bool | None = None
    else:
        behavior_loss = bool(arm_accepted and not behavior_test_ok)
    return {
        "false_accept": false_accept,
        "behavior_loss": behavior_loss,
        "conservative_reject": conservative_reject,
    }


@dataclass
class TaskSpec:
    id: str
    status: str
    directory: str
    spec: str | None
    contract: str | None
    buggy_cir: str | None
    fixed_cir: str | None
    buggy_rs: str | None
    fixed_rs: str | None
    ground_truth: dict[str, Any]

    def resolve(self, root: Path, rel: str | None) -> Path | None:
        if not rel:
            return None
        return (root / rel).resolve()


def load_manifest(path: Path | str, *, verify_hashes: bool = True) -> list[TaskSpec]:
    """Load and validate ``benchmarks/MANIFEST.json``.

    Raises ``ValueError`` on a hash mismatch so a frozen batch cannot silently
    run against changed inputs.
    """

    path = Path(path)
    data = json.loads(path.read_text(encoding="utf-8"))
    root = path.parent
    tasks: list[TaskSpec] = []
    for entry in data.get("tasks", []):
        files = entry.get("files", {})
        if verify_hashes:
            for rel, digest in files.items():
                full = (root / rel).resolve()
                if not full.is_file():
                    raise ValueError(f"{entry['id']}: missing frozen file {rel}")
                actual = sha256_file(full)
                if actual != digest:
                    raise ValueError(
                        f"{entry['id']}: hash mismatch for {rel}: {actual} != {digest}")
        tasks.append(TaskSpec(
            id=entry["id"],
            status=entry.get("status", "ready"),
            directory=entry.get("directory", ""),
            spec=entry.get("spec"),
            contract=entry.get("contract"),
            buggy_cir=entry.get("buggy_cir"),
            fixed_cir=entry.get("fixed_cir"),
            buggy_rs=entry.get("buggy_rs"),
            fixed_rs=entry.get("fixed_rs"),
            ground_truth=entry.get("ground_truth", {}),
        ))
    return tasks
