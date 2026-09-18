"""Flash smoke batch: validate the multi-arm chain on a few frozen tasks.

This is the only live entry point for the v2 harness. It reuses
:mod:`cir_workflow.live` for the pinned DeepSeek Flash client, the shared
persisted budget, the model-identity check and the sanitized evidence log, and
reuses the offline arm runners for the actual per-arm loops. It refuses to run
unless the frozen protocol's sha256 matches the confirmation credential.

It is a chain-validation smoke run, not an effect-size experiment: 3 tasks, 6
arms, K=4, <= 48 HTTP requests, <= 60 minutes.
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

from .arms import run_cir_arm, run_rust_arm
from .concir_client import ConcirClient
from .experiments_v2 import K, load_manifest
from .live import (
    ALLOWED_PROVIDER, DEFAULT_MAX_SECONDS, DeepSeekFlashClient, LiveBudget,
    RecordingPatchProvider, RecordingProvider, assert_allowed_model,
)
from .offline_workflow import _exclusive_run_dir
from .patch_repair import ExternalPatchRepairWorkflow
from .providers import CandidateRequest, CandidateResponse
from .experiments_v2 import (
    ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER, ARM_OURS_REVISION, ARM_OURS_PATCH,
)

PROMPTS = Path(__file__).resolve().parents[2] / "prompts"
SMOKE_HTTP_CAP = 48
SMOKE_WALL_CAP_S = 3600.0
SMOKE_TASKS = ("lock-order/abba_2lock",
               "condvar/lost_wakeup_notify_before_wait",
               "lock-order/partial_deadlock_bystander")
SMOKE_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter", "A3_ours_revision",
              "A3p_ours_patch", "A3_tool_repair")


def _read(name: str) -> str:
    return (PROMPTS / name).read_text(encoding="utf-8")


def _rust_system(mode: str) -> str:
    if mode == "self":
        return _read("rust_self_review_v1.md")
    if mode == "tools":
        return _read("rust_tool_feedback_v1.md")
    return _read("rust_generation_v1.md")


class RustLiveProvider:
    """Adapt the pinned Flash client to the Rust arm provider protocol."""

    name = "llm"

    def __init__(self, client: DeepSeekFlashClient, mode: str) -> None:
        self.client = client
        self.mode = mode
        self.calls: list[dict[str, Any]] = []

    def propose(self, request: CandidateRequest) -> CandidateResponse:
        system = _rust_system(self.mode)
        if request.feedback is None:
            user = (f"Specification:\n{request.requirements}\n\n"
                    "Output the complete Rust program.")
        elif self.mode == "tools":
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    f"Tool diagnostics:\n{request.feedback}\n\n"
                    "Output the complete corrected Rust program.")
        else:
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    "If there is no concurrency defect reply exactly NO_ISSUES; "
                    "otherwise output the complete corrected Rust program.")
        outcome = self.client.complete(system, user)
        self.calls.append({
            "attempt": request.attempt, "request_id": outcome.request_id,
            "response_model": outcome.response_model,
            "usage": outcome.usage, "wall_ms": outcome.wall_ms,
            "transport_attempt": outcome.transport_attempt,
            "prompt_sha256": outcome.prompt_sha256,
        })
        response = CandidateResponse(
            text=outcome.text, source="llm", provider=ALLOWED_PROVIDER,
            model_id=outcome.response_model or "deepseek-flash", usage=outcome.usage)
        response.wall_ms = outcome.wall_ms
        return response


def assert_protocol_confirmed(protocol_path: Path | str, expected_sha256: str) -> None:
    import hashlib

    actual = hashlib.sha256(Path(protocol_path).read_bytes()).hexdigest()
    if actual != expected_sha256:
        raise RuntimeError("live smoke refused: protocol hash mismatch "
                           f"({actual} != {expected_sha256})")


def run_flash_smoke(manifest_path: Path | str, out_dir: Path | str, *, binary: Path | str,
                    api_key: str, protocol_path: Path | str, protocol_sha256: str,
                    k: int = K, timeout: float = 90.0, max_tokens: int = 4096,
                    sdk_client: Any | None = None,
                    tasks_filter: tuple[str, ...] | None = None) -> dict[str, Any]:
    assert_protocol_confirmed(protocol_path, protocol_sha256)
    assert_allowed_model(ALLOWED_PROVIDER, "deepseek-flash")
    root = Path(manifest_path).resolve().parent
    out = Path(out_dir).expanduser().resolve()
    batch = _exclusive_run_dir(out)
    budget = LiveBudget(batch / "budget.json", max_requests=SMOKE_HTTP_CAP,
                        max_seconds=min(SMOKE_WALL_CAP_S, DEFAULT_MAX_SECONDS))
    wanted = tasks_filter or SMOKE_TASKS
    tasks = {t.id: t for t in load_manifest(manifest_path)}

    summary: dict[str, Any] = {
        "batch_dir": str(batch),
        "provider": ALLOWED_PROVIDER, "requested_model": "deepseek-flash",
        "protocol_sha256": protocol_sha256, "k": k,
        "max_requests": budget.max_requests, "max_seconds": budget.max_seconds,
        "tasks": [], "stop_reason": None,
    }
    stop_reason = None
    for task_id in wanted:
        task = tasks.get(task_id)
        if task is None:
            summary["tasks"].append({"task": task_id, "skipped": "not_in_manifest"})
            continue
        reason = budget.exhausted()
        if reason:
            summary["tasks"].append({"task": task_id, "skipped": reason})
            stop_reason = reason
            break
        task_dir = batch / task_id.replace("/", "__")
        contract_path = root / task.contract
        contract = json.loads(contract_path.read_text(encoding="utf-8"))
        spec = (root / task.spec).read_text(encoding="utf-8") if task.spec else task_id
        client = ConcirClient(binary, workdir=task_dir / "calls", timeout=30.0)
        record: dict[str, Any] = {"task": task_id, "arms": {}}
        try:
            # A0 / A1 / A2 (Rust), then A3 (CIR), A3p, tool-repair
            for arm in (ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER):
                mode = {"A0_direct": "direct", "A1_self_iter": "self",
                        "A2_tools_iter": "tools"}[arm]
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                provider = RustLiveProvider(client_llm, mode)
                run = run_rust_arm(provider, arm=arm, task=task_id, spec=spec,
                                   contract=contract, out_dir=task_dir / arm,
                                   k=k, tool_timeout_s=60.0)
                record["arms"][arm] = run.as_dict()
                record["arms"][arm]["llm_calls"] = provider.calls
                if budget.exhausted():
                    stop_reason = budget.exhausted()
                    break
            if not stop_reason:
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                cir_provider = RecordingProvider(client_llm)
                a3 = run_cir_arm(client, cir_provider, arm=ARM_OURS_REVISION, task=task_id,
                                 spec=spec, contract=contract, out_dir=task_dir / "A3",
                                 k=k)
                record["arms"][ARM_OURS_REVISION] = a3.as_dict()
                record["arms"][ARM_OURS_REVISION]["llm_calls"] = cir_provider.calls
            if not stop_reason and task.buggy_cir:
                client_llm = DeepSeekFlashClient(
                    api_key=api_key, budget=budget, evidence_dir=batch / "llm",
                    timeout=timeout, max_tokens=max_tokens, sdk_client=sdk_client)
                patch_provider = RecordingPatchProvider(client_llm)
                a3p = ExternalPatchRepairWorkflow(
                    client, patch_provider, out_dir=task_dir / "A3p", max_rounds=k).run(
                    root / task.buggy_cir, contract_path)
                payload = a3p.__dict__
                payload["rounds"] = [r.__dict__ for r in a3p.rounds]
                payload["llm_calls"] = patch_provider.calls
                record["arms"][ARM_OURS_PATCH] = payload
            if not stop_reason and task.buggy_cir:
                repair = client.repair(root / task.buggy_cir, contract_path, strategy="c")
                record["arms"]["A3_tool_repair"] = {
                    "status": repair.status, "outcome": repair.outcome,
                    "complete": repair.complete, "artifact_path": repair.artifact_path,
                    "wall_ms": repair.wall_ms,
                }
        except Exception as exc:  # noqa: BLE001 - stop the batch and record
            stop_reason = f"{type(exc).__name__}: {exc}"
            record["error"] = stop_reason
        summary["tasks"].append(record)
        if stop_reason:
            summary["stop_reason"] = stop_reason
            break

    summary["requests_used"] = budget.requests_used
    summary["requests_remaining"] = budget.remaining
    (batch / "SUMMARY.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (batch / "SUMMARY.md").write_text(render_smoke_markdown(summary), encoding="utf-8")
    return summary


def render_smoke_markdown(summary: dict[str, Any]) -> str:
    lines = ["# Flash smoke batch — SUMMARY", "",
             f"- batch dir: `{summary['batch_dir']}`",
             f"- protocol sha256: `{summary['protocol_sha256']}`",
             f"- requests used: {summary.get('requests_used')}",
             f"- stop reason: {summary.get('stop_reason')}", "",
             "| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for task in summary["tasks"]:
        if "arms" not in task:
            lines.append(f"| {task['task']} | — | — | — | — | — | — | — | {task.get('skipped', '')} |")
            continue
        for arm, run in task["arms"].items():
            consumption = run.get("consumption", {}) if isinstance(run, dict) else {}
            lines.append(
                f"| {task['task']} | {arm} | {run.get('accepted')} | "
                f"{run.get('accepted_round')} | {consumption.get('http_requests')} | "
                f"{consumption.get('total_tokens')} | {consumption.get('llm_wall_ms')} | "
                f"{consumption.get('tool_wall_ms')} |")
    return "\n".join(lines) + "\n"
