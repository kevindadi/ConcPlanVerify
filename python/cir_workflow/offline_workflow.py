"""Minimal offline CIR generation -> check -> feedback -> verify -> tool-repair workflow.

Responsibilities split:

- **provider** (scripted this round, LLM later) proposes CIR candidates;
- the **tool** (Rust ``concir-backend`` via :class:`ConcirClient`) validates,
  checks supportability, verifies and performs the deterministic repair.

The workflow never validates or repairs CIR itself. After the initial CIR is
frozen it never lets the provider replace the whole program; the only repair
path is the backend's strategy search under the frozen contract and its
``allowed_scope``. A tool repair is labelled ``tool``, never ``LLM``.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient, ConcirResult, sha256_file
from .json_utils import extract_json
from .prompts import (
    build_check_feedback,
    build_explore_feedback,
    render_feedback,
)
from .providers import CandidateProvider, CandidateRequest, CandidateResponse

STRATEGY_ENUM = {"a": "single", "b": "composite", "c": "diagnostic"}


@dataclass
class OfflineStep:
    step: str
    source: str
    status: str
    detail: dict[str, Any] = field(default_factory=dict)
    wall_ms: int = 0
    artifact_path: str | None = None


@dataclass
class OfflineResult:
    status: str
    requirements: str
    contract_sha256: str
    frozen_cir_sha256: str | None
    out_dir: str
    steps: list[OfflineStep] = field(default_factory=list)
    generation_rounds: list[dict[str, Any]] = field(default_factory=list)
    check: dict[str, Any] | None = None
    support: dict[str, Any] | None = None
    explore: dict[str, Any] | None = None
    repair: dict[str, Any] | None = None
    replay: dict[str, Any] | None = None
    error: str | None = None

    @property
    def repaired_by_tool(self) -> bool:
        return self.status == "repaired"


def _result_record(result: ConcirResult) -> dict[str, Any]:
    return {
        "command": result.command,
        "kind": result.kind,
        "status": result.status,
        "exit_code": result.exit_code,
        "outcome": result.outcome,
        "complete": result.complete,
        "artifact_path": result.artifact_path,
        "error": result.error,
        "wall_ms": result.wall_ms,
        "identity": {
            "binary_sha256": result.identity.binary_sha256,
            "argv": result.identity.argv,
            "input_sha256": result.identity.input_sha256,
            "contract_sha256": result.identity.contract_sha256,
            "config": result.identity.config,
            "engine": result.identity.engine,
        },
    }


class OfflineWorkflow:
    def __init__(
        self,
        client: ConcirClient,
        provider: CandidateProvider,
        *,
        out_dir: Path | str,
        max_generation_rounds: int = 3,
        engine: str = "petri",
        repair_strategy: str = "c",
        repair_config: dict[str, int] | None = None,
    ) -> None:
        self.client = client
        self.provider = provider
        self.out_dir = Path(out_dir).expanduser().resolve()
        self.max_generation_rounds = max(1, max_generation_rounds)
        self.engine = engine
        self.repair_strategy = repair_strategy
        self.repair_config = repair_config or {
            "candidate_budget": 64, "verification_budget": 64,
            "max_depth": 4, "max_total_edits": 4,
        }

    def run(self, requirements: str, contract: dict[str, Any]) -> OfflineResult:
        self.out_dir.mkdir(parents=True, exist_ok=True)
        contract_path = self.out_dir / "contract.json"
        contract_path.write_text(json.dumps(contract, ensure_ascii=False, indent=2) + "\n")
        contract_sha = sha256_file(contract_path)
        result = OfflineResult(
            status="generation_failed", requirements=requirements,
            contract_sha256=contract_sha, frozen_cir_sha256=None,
            out_dir=str(self.out_dir),
        )

        # 1. generate + check + bounded feedback retry
        candidate_path: Path | None = None
        feedback: dict[str, Any] | None = None
        previous_text: str | None = None
        for attempt in range(1, self.max_generation_rounds + 1):
            request = CandidateRequest(
                requirements=requirements, contract=contract,
                feedback=render_feedback(feedback) if feedback else None,
                attempt=attempt, previous_candidate=previous_text,
            )
            response = self.provider.propose(request)
            attempt_dir = self.out_dir / "generation" / f"attempt-{attempt}"
            attempt_dir.mkdir(parents=True, exist_ok=True)
            round_record = {
                "attempt": attempt,
                "source": response.source,
                "provider": response.provider,
                "model_id": response.model_id,
                "error": response.error,
                "input_tokens": response.input_tokens,
                "output_tokens": response.output_tokens,
            }
            result.generation_rounds.append(round_record)
            if response.error:
                round_record["result"] = "provider_error"
                feedback = {"stage": "provider", "error": response.error}
                result.steps.append(OfflineStep(
                    step=f"generate[{attempt}]", source=response.source,
                    status="provider_error", detail={"error": response.error},
                ))
                continue
            (attempt_dir / "raw.txt").write_text(response.text, encoding="utf-8")
            previous_text = response.text
            extracted = extract_json(response.text)
            try:
                parsed = json.loads(extracted)
            except json.JSONDecodeError as exc:
                round_record["result"] = "parse_error"
                feedback = {"stage": "parse", "error": f"invalid JSON: {exc}"}
                result.steps.append(OfflineStep(
                    step=f"generate[{attempt}]", source=response.source,
                    status="parse_error", detail={"error": str(exc)},
                ))
                continue
            candidate_file = attempt_dir / "candidate.cir.json"
            candidate_file.write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n")
            check = self.client.check(candidate_file)
            result.check = _result_record(check)
            result.steps.append(OfflineStep(
                step=f"check[{attempt}]", source="tool", status=check.status,
                detail=_result_record(check), wall_ms=check.wall_ms,
            ))
            if check.kind != "semantic":
                result.status = "tool_error"
                result.error = check.error or "check failed at the process/protocol level"
                return result
            if check.status == "invalid":
                round_record["result"] = "check_invalid"
                feedback = build_check_feedback(check)
                continue
            round_record["result"] = "check_valid"
            candidate_path = candidate_file
            break

        if candidate_path is None:
            result.status = "generation_failed"
            result.error = "no candidate passed static checking within the retry budget"
            return result

        # 2. freeze the initial CIR
        frozen = self.out_dir / "frozen_initial.cir.json"
        frozen.write_bytes(candidate_path.read_bytes())
        result.frozen_cir_sha256 = sha256_file(frozen)

        # 3. support
        support = self.client.support(frozen)
        result.support = _result_record(support)
        result.steps.append(OfflineStep(step="support", source="tool",
                                        status=support.status, detail=_result_record(support),
                                        wall_ms=support.wall_ms))
        if support.kind != "semantic":
            result.status = "tool_error"
            result.error = support.error or "support failed"
            return result
        if support.status == "unsupported":
            result.status = "unsupported"
            return result

        # 4. explore
        explore = self.client.explore(frozen, contract_path, self.engine)
        result.explore = _result_record(explore)
        result.steps.append(OfflineStep(step="explore", source="tool",
                                        status=explore.status, detail=_result_record(explore),
                                        wall_ms=explore.wall_ms))
        if explore.kind != "semantic":
            result.status = "tool_error"
            result.error = explore.error or "explore failed"
            return result
        if explore.status == "pass":
            result.status = "already_satisfied" if explore.complete else "unknown"
            return result
        if explore.status == "unknown":
            result.status = "unknown"
            return result
        if explore.status in ("invalid", "unsupported"):
            result.status = explore.status
            return result

        # 5. FAIL: deterministic tool repair, then artifact replay
        repair = self.client.repair(
            frozen, contract_path, strategy=self.repair_strategy, **self.repair_config,
        )
        result.repair = _result_record(repair)
        result.steps.append(OfflineStep(step="repair", source="tool", status=repair.status,
                                        detail=_result_record(repair), wall_ms=repair.wall_ms,
                                        artifact_path=repair.artifact_path))
        if repair.kind != "semantic":
            result.status = "tool_error"
            result.error = repair.error or "repair failed"
            return result
        if repair.artifact_path is None:
            # A legitimate non-repair outcome (no acceptable candidate, budget,
            # analysis unknown, invalid config, unsupported).
            result.status = repair.status
            return result

        binding_ok, binding_reason = self._bind_artifact(repair, explore)
        if not binding_ok:
            result.status = "tool_error"
            result.error = f"repair artifact is not bound to this invocation: {binding_reason}"
            return result

        replay = self.client.replay(repair.artifact_path)
        result.replay = _result_record(replay)
        result.steps.append(OfflineStep(step="replay", source="tool", status=replay.status,
                                        detail=_result_record(replay), wall_ms=replay.wall_ms,
                                        artifact_path=repair.artifact_path))
        if replay.kind != "semantic" or replay.status != "replayed":
            result.status = "tool_error"
            result.error = replay.error or "artifact replay failed"
            return result

        result.status = "repaired" if repair.status == "repaired" else repair.status

        # contract must be byte-identical to the frozen contract
        if sha256_file(contract_path) != contract_sha:
            result.status = "tool_error"
            result.error = "contract changed during the workflow"
        return result

    # ---- artifact binding (protocol-level, using tool fingerprints) --------
    def _bind_artifact(self, repair: ConcirResult, explore: ConcirResult) -> tuple[bool, str]:
        try:
            artifact = json.loads(Path(repair.artifact_path).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            return False, f"artifact unreadable: {exc}"
        if not isinstance(artifact, dict):
            return False, "artifact is not a JSON object"
        if artifact.get("schema_version") != "concir-repair-artifact-v1":
            return False, f"unexpected schema {artifact.get('schema_version')!r}"

        config = artifact.get("effective_config") or {}
        want = dict(self.repair_config)
        if config.get("strategy") != STRATEGY_ENUM[self.repair_strategy]:
            return False, f"strategy {config.get('strategy')!r}"
        for key in ("candidate_budget", "verification_budget", "max_depth", "max_total_edits"):
            if config.get(key) != want[key]:
                return False, f"{key} {config.get(key)!r} != {want[key]!r}"

        nodes = artifact.get("nodes") or []
        if not nodes:
            return False, "artifact has no nodes"
        root_report = (nodes[0] or {}).get("report") or {}
        explore_payload = explore.payload or {}
        for field in ("model_fingerprint", "contract_fingerprint"):
            if root_report.get(field) != explore_payload.get(field):
                return False, f"{field} mismatch"
        return True, "ok"
