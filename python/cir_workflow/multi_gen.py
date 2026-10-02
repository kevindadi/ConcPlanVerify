"""Config-driven four-system G0–G3 generation. Dry-run and live share this path.

The model transport is the only piece that changes. An empty or illegal config
does not fall back to a hard-coded model list.
"""

from __future__ import annotations

import fcntl
import hashlib
import json
import random
import threading
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Callable

from .channels import AuditedClient
from .experiments_v2 import ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER
from .generation import load_gen_tasks, run_g3_v2
from .live import BudgetExhausted, LiveBudget
from .providers import CandidateRequest, CandidateResponse
from .transport import build_registry, require_experiment_model

CELL_CAPS = {"G0_direct": 2, "G1_self_iter": 8, "G2_tools_iter": 8, "G3_concir": 14}
# v23 round caps include the first generation (initial + at most two repairs).
ROUND_CAPS = {"G0_direct": 1, "G1_self_iter": 3, "G2_tools_iter": 3, "G3_concir": 3}
G3_CODE_ROUND_CAP = 3
MAIN_ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir")
ARM_MODE = {
    "G0_direct": ("direct", ARM_DIRECT, 1),
    "G1_self_iter": ("self", ARM_SELF_ITER, 4),
    "G2_tools_iter": ("tools", ARM_TOOLS_ITER, 4),
}


class ConfigError(RuntimeError):
    pass


def _sha_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def _sha_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_config(path: Path) -> dict:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if "models" not in data or "tasks" not in data or "arms" not in data:
        raise ConfigError("config must list models, tasks, and arms")
    if not isinstance(data["models"], list) or not isinstance(data["tasks"], list):
        raise ConfigError("models and tasks must be lists")
    for item in data["models"]:
        if not item.get("model_id") or not item.get("display_name"):
            raise ConfigError("a model entry is missing model_id or display_name")
    unknown = [arm for arm in data["arms"] if arm not in MAIN_ARMS]
    if unknown:
        raise ConfigError(f"unknown arms {unknown}")
    data["_sha256"] = _sha_file(Path(path))
    return data


def pilot_tasks(manifest_path: Path) -> list[dict]:
    """First task of each family, in the frozen manifest's family order."""

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    first: dict[str, str] = {}
    for entry in manifest["tasks"]:
        first.setdefault(entry["family"], entry["task"])
    chosen = []
    for family in manifest["families"]:
        task = first.get(family)
        if task is None:
            raise ConfigError(f"family {family} has no task in the manifest")
        chosen.append({"family": family, "task": task,
                       "reason": "first task of this family in the frozen manifest task list"})
    return chosen


def matrix_from_config(config: dict) -> list[dict]:
    reps = list(config.get("reps") or [0])
    cells = []
    for model in config["models"]:
        for task in config["tasks"]:
            task_id = task["task"] if isinstance(task, dict) else task
            for arm in config["arms"]:
                for rep in reps:
                    cells.append({
                        "model_id": model["model_id"],
                        "display_name": model["display_name"],
                        "task": task_id,
                        "arm": arm,
                        "rep": int(rep),
                        "cell_cap": int((config.get("cell_caps") or CELL_CAPS)[arm]),
                        "round_cap": int((config.get("rounds") or ROUND_CAPS)[arm]),
                        "g3_code_cap": int(config.get("g3_code_rounds") or G3_CODE_ROUND_CAP),
                        "ablation": dict(config.get("ablation") or {}),
                    })
    return cells


def schedule_cells(cells: list[dict], first_task: str, seed: int = 20261002) -> list[dict]:
    wave = [cell for cell in cells if cell["task"] == first_task]
    rest = [cell for cell in cells if cell["task"] != first_task]
    rng = random.Random(seed)
    rng.shuffle(wave)
    rng.shuffle(rest)
    return wave + rest


class SharedCounterBudget:
    """One physical cap in front of the process-wide SQLite budget."""

    def __init__(self, shared: LiveBudget, cap: int, counter: list[int], lock: threading.Lock,
                 label: str):
        self.shared = shared
        self.cap = cap
        self.counter = counter
        self.lock = lock
        self.label = label

    def reserve(self, link: dict | None = None) -> int:
        with self.lock:
            if self.counter[0] >= self.cap:
                raise BudgetExhausted(f"{self.label} reached ({self.counter[0]}/{self.cap})")
            used = self.shared.reserve(link)
            self.counter[0] += 1
            return used

    def __getattr__(self, name: str) -> Any:
        return getattr(self.shared, name)


class CellBudgetExhausted(BudgetExhausted):
    """One cell used its own physical cap. That cell ends; the batch continues."""


class CellBudget(SharedCounterBudget):
    def __init__(self, inner: SharedCounterBudget, cap: int, cell_id: str | None = None):
        self.inner = inner
        self.cell_cap = cap
        self.cell_id = cell_id
        self.cell_lock = threading.Lock()
        # The per-cell cap persists across restarts: count the attempts already
        # recorded for this cell so a resume cannot grant a fresh cap.
        count = getattr(inner, "cell_attempt_count", None)
        self.cell_used = int(count(cell_id)) if callable(count) and cell_id else 0

    def reserve(self, link: dict | None = None) -> int:
        with self.cell_lock:
            if self.cell_used >= self.cell_cap:
                raise CellBudgetExhausted(
                    f"cell physical cap reached ({self.cell_used}/{self.cell_cap})")
            used = self.inner.reserve(link)
            self.cell_used += 1
            return used

    def __getattr__(self, name: str) -> Any:
        return getattr(self.inner, name)


def acquire_coordinator(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    handle = path.open("a", encoding="utf-8")
    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError as exc:
        handle.close()
        raise ConfigError("another coordinator holds this run") from exc
    return handle


def cell_directory(out_dir: Path, cell: dict) -> Path:
    return (out_dir / cell["task"].replace("/", "__") / cell["model_id"] / cell["arm"]
            / f"rep{cell['rep']}")


def run_pin_error(out_dir: Path, workflow: dict) -> str | None:
    """Read-only guard usable before preflight as well as before cell scheduling."""
    path = out_dir / "WORKFLOW.json"
    if path.is_file():
        try:
            pinned = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            pinned = None
        return None if pinned == workflow else "workflow_fingerprint_mismatch"
    if any(out_dir.glob("*/*/G*/rep*/state.json")) or any(out_dir.glob("*/*/G*/rep*/cache.json")):
        return "workflow_unversioned; use a new output directory"
    return None


def workflow_fingerprint(*tool_paths: Path | None) -> str:
    """Pin evaluator and prompt contents, plus the actual tool binaries.

    Do not resume a frozen run after changing these inputs. This digest is
    independent of API keys, output paths, and scheduler-only deviations.
    """
    import os
    root = Path(__file__).resolve().parents[2]
    assets = sorted((root / "python/cir_workflow").glob("*.py"))
    assets += sorted((root / "prompts").glob("*.md"))
    assets += sorted(p for p in (root / "runtime").rglob("*.rs")
                     if "target" not in p.relative_to(root / "runtime").parts)
    assets += sorted(p for p in (root / "runtime").rglob("Cargo.toml")
                     if "target" not in p.relative_to(root / "runtime").parts)
    rows = [(str(p.relative_to(root)), hashlib.sha256(p.read_bytes()).hexdigest())
            for p in assets]
    rows.append(("CIR_PROMPT_VERSION", os.environ.get("CIR_PROMPT_VERSION", "v3")))
    for index, path in enumerate(tool_paths):
        p = Path(path) if path is not None else None
        rows.append((f"tool:{index}", hashlib.sha256(p.read_bytes()).hexdigest()
                     if p is not None and p.is_file() else None))
    return _sha_text(json.dumps(rows, sort_keys=True))


def send_fingerprint(spec, cell: dict, config: dict, raw) -> str:
    from .channels import _public_send_record
    from .transport import CHANNELS
    protocol = _public_send_record(
        raw, channel=spec.channel, surface=spec.surface,
        endpoint=CHANNELS[spec.channel].base_url)
    return _sha_text(json.dumps({
        "model": spec.model_id, "channel": spec.channel, "surface": spec.surface,
        "task": cell["task"], "arm": cell["arm"], "rep": cell["rep"],
        "config": config.get("_sha256"),
        "protocol": protocol,
        "workflow": config.get("_workflow_sha256") or workflow_fingerprint(),
    }, sort_keys=True, default=str))


def _digest(record: dict) -> str:
    payload = {
        "system": record.get("system"),
        "user": record.get("user"),
        "response": record.get("response"),
        "response_model": record.get("response_model"),
    }
    return _sha_text(json.dumps(payload, sort_keys=True))


_CACHE_FIELDS = ("status", "fingerprint", "accepted", "record_status", "calls")


def _cache_digest(payload: dict) -> str:
    body = {key: payload.get(key) for key in _CACHE_FIELDS}
    return _sha_text(json.dumps(body, sort_keys=True, default=str))


def write_cell_cache(directory: Path, payload: dict) -> None:
    body = {key: payload.get(key) for key in _CACHE_FIELDS}
    body["cache_sha256"] = _cache_digest(body)
    path = directory / "cache.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(body, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def read_cell_cache(directory: Path) -> dict | None:
    path = directory / "cache.json"
    if not path.is_file():
        return None
    payload = json.loads(path.read_text(encoding="utf-8"))
    if payload.get("cache_sha256") != _cache_digest(payload):
        return {"status": "cache_corrupt"}
    return payload


def seed_executed_cache(out_dir: Path, summary: dict) -> int:
    """Remember cells that already finished so a resume does not send them again."""

    written = 0
    for item in summary.get("cells") or []:
        if item.get("status") != "executed":
            continue
        cell = item["cell"]
        directory = cell_directory(out_dir, cell)
        state_path = directory / "state.json"
        if not state_path.is_file():
            continue
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if not state.get("calls") or not state.get("fingerprint"):
            continue
        if (directory / "cache.json").is_file():
            continue
        write_cell_cache(directory, {
            "status": "executed",
            "fingerprint": state["fingerprint"],
            "accepted": item.get("accepted"),
            "record_status": item.get("record_status"),
            "calls": item.get("calls") if item.get("calls") is not None else len(state["calls"]),
        })
        written += 1
    return written


class SnapshotClient:
    """Persist each prompt and response. A bad digest blocks the next send."""

    def __init__(self, inner, state_path: Path, fingerprint: str):
        self.inner = inner
        self.state_path = state_path
        self.fingerprint = fingerprint
        self.calls = 0
        state = {}
        if state_path.is_file():
            state = json.loads(state_path.read_text(encoding="utf-8"))
        saved_calls = state.get("calls") or []
        if state.get("fingerprint") and state["fingerprint"] != fingerprint and saved_calls:
            self.blocked = "fingerprint_mismatch"
        else:
            self.blocked = None
        for item in saved_calls:
            if not item.get("snapshot_sha256"):
                self.blocked = self.blocked or "snapshot_legacy"
            elif item["snapshot_sha256"] != _digest(item):
                self.blocked = "snapshot_corrupt"
        state["fingerprint"] = fingerprint
        state.setdefault("calls", [])
        self.state = state
        self._replay_at = 0
        if self.blocked is None:
            self._save()

    def _save(self) -> None:
        self.state_path.parent.mkdir(parents=True, exist_ok=True)
        self.state_path.write_text(json.dumps(self.state, ensure_ascii=False, indent=2) + "\n",
                                   encoding="utf-8")

    def set_stage(self, stage: str) -> None:
        if hasattr(self.inner, "set_stage"):
            self.inner.set_stage(stage)

    def protocol_record(self) -> dict:
        explicit = getattr(self.inner, "protocol_record", None)
        if callable(explicit):
            return explicit()
        return {}

    def complete(self, system: str, user: str):
        if self.blocked:
            raise ConfigError(self.blocked)
        # The audit wrapper binds the reservation context on this snapshot
        # wrapper; forward it so the innermost client's reservation_link sees
        # the run/cell/stage/round instead of null.
        ctx = getattr(self, "call_context", None)
        if ctx:
            from .call_context import bind_context
            bind_context(self.inner, ctx)
        cached = self.state["calls"][self._replay_at] if self._replay_at < len(self.state["calls"]) else None
        if cached is not None:
            if cached.get("system") != system or cached.get("user") != user:
                raise ConfigError("snapshot_prompt_mismatch")
            self._replay_at += 1
            return SimpleNamespace(
                text=cached.get("response") or "",
                response_model=cached.get("response_model"), usage=None, wall_ms=0,
                finish_reason="cache", request_id=None, prompt_sha256=None, cost=None,
                transport_attempt=1)
        outcome = self.inner.complete(system, user)
        self.calls += 1
        record = {
            "system": system,
            "user": user,
            "response": getattr(outcome, "text", ""),
            "response_model": getattr(outcome, "response_model", None),
        }
        record["snapshot_sha256"] = _digest(record)
        self.state.setdefault("calls", []).append(record)
        self._replay_at = len(self.state["calls"])
        try:
            self._save()
        except OSError:
            # The response is already in memory. Do not send again to replace it.
            self.state["calls"][-1]["save_error"] = "oserror"
        return outcome


class GenerationProvider:
    def __init__(self, client, mode: str, model_id: str):
        self.client = client
        self.mode = mode
        self.model_id = model_id
        self.calls: list[dict] = []

    def propose(self, request: CandidateRequest) -> CandidateResponse:
        if self.mode == "direct":
            system = "Write one Rust program from the requirements. Do not invent extra requirements."
        elif self.mode == "tools":
            system = "Revise the Rust program using the tool feedback."
        else:
            system = "Review your Rust program and repair concurrency defects you can justify."
        if request.feedback is None and not request.previous_candidate:
            user = f"Specification:\n{request.requirements}\n\nOutput the complete Rust program."
        elif self.mode == "tools":
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    f"Tool diagnostics:\n{request.feedback or ''}\n\n"
                    "Output the complete corrected Rust program.")
        else:
            user = (f"Specification:\n{request.requirements}\n\n"
                    f"Current program:\n```rust\n{request.previous_candidate}\n```\n\n"
                    f"Self-review instructions:\n{request.feedback or ''}\n\n"
                    "Review the current program against the requirements. "
                    "If you find a justified defect, output the complete corrected Rust program. "
                    "If no further defect is found, reply exactly NO_ISSUES.")
        outcome = self.client.complete(system, user)
        self.calls.append({"attempt": request.attempt, "model": getattr(outcome, "response_model", None)})
        usage = getattr(outcome, "usage", None)
        response = CandidateResponse(
            text=getattr(outcome, "text", "") or "", source="llm", provider=self.model_id,
            model_id=getattr(outcome, "response_model", None) or self.model_id, usage=usage)
        response.wall_ms = int(getattr(outcome, "wall_ms", 0) or 0)
        if usage:
            response.input_tokens = usage.get("prompt_tokens")
            response.output_tokens = usage.get("completion_tokens")
        return response


def _rust_system_hides_contract(user: str, contract_marker: str) -> bool:
    return contract_marker not in user


def _oracle_score_cell(out_dir: Path, task, record: dict, binary: Path,
                       instrument: Path | None) -> dict:
    """Score every emitted G0-G2 candidate with the shared requirement oracle.

    The Rust arms must be judged by the same oracle as G3, not only by their own
    build/tool acceptance. Tool cost is bounded: 32 native runs, no Miri.
    """
    from . import rust_oracle
    rounds = {}
    for path in sorted(out_dir.glob("run-*/round-*/candidate.rs")):
        try:
            rounds[int(path.parent.name.split("-")[1])] = path
        except (IndexError, ValueError):
            continue
    per_round: dict[str, Any] = {}
    for rd in record.get("rounds") or []:
        path = rounds.get(rd.get("round"))
        if path is None:
            continue
        try:
            ev = rust_oracle.evaluate(
                path.read_text(encoding="utf-8"), task.contract_path,
                task.reference_cir_path, out_dir / f"oracle-round-{rd.get('round')}",
                n_native=32, miri_seeds=0, run_timeout=10.0,
                binary=binary, instrument_binary=instrument)
            rd["requirement_coverage"] = ev.get("coverage")
            rd["oracle_status"] = ev.get("status")
            per_round[str(rd.get("round"))] = ev.get("coverage")
        except Exception as exc:  # noqa: BLE001 - one round must not stop the cell
            rd["oracle_error"] = f"{type(exc).__name__}: {exc}"[:200]
    return per_round


def run_one_cell(cell: dict, client, task, out_dir: Path, *, binary: Path,
                 instrument: Path | None, stage_runner: Callable | None = None) -> dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    if stage_runner is not None:
        return stage_runner(cell, client, task, out_dir)
    arm = cell["arm"]
    if arm == "G3_concir":
        if hasattr(client, "set_stage"):
            client.set_stage("cir")
        record = run_g3_v2(client, binary, task, out_dir,
                           k_cir=int(cell.get("round_cap", 3)),
                           k_code=int(cell.get("g3_code_cap", 3)),
                           instrument_binary=instrument,
                           ablation=cell.get("ablation"))
        record["model_pass_is_not_rf"] = True
        return record
    mode, internal, k = ARM_MODE[arm]
    k = int(cell.get("round_cap", k))
    from .arms import run_rust_arm
    provider = GenerationProvider(client, mode, cell["model_id"])
    run = run_rust_arm(provider, arm=internal, task=task.id, spec=task.requirements_md,
                       contract=task.contract, out_dir=out_dir, k=k, tool_timeout_s=15.0,
                       run_miri_many_seeds=False, miri_seed_count=16)
    record = run.as_dict()
    record["arm"] = arm
    record["llm_calls"] = len(provider.calls)
    record["requirement_coverage"] = _oracle_score_cell(
        out_dir, task, record, binary, instrument)
    return record


def execute_matrix(config: dict, out_dir: Path, *, client_factory: Callable, tasks_by_id: dict,
                   binary: Path, instrument: Path | None = None, live: bool = False,
                   stage_runner: Callable | None = None, seed: int = 20261002,
                   spec_resolver: Callable | None = None,
                   model_concurrency: dict | None = None) -> dict:
    if not config["models"]:
        return {"cells": [], "calls": 0, "stop": "empty_matrix", "real_requests": 0}
    config = dict(config)
    from .binding import default_binary
    config["_workflow_sha256"] = workflow_fingerprint(binary, instrument, default_binary())
    cells = matrix_from_config(config)
    if not cells:
        return {"cells": [], "calls": 0, "stop": "empty_matrix", "real_requests": 0}
    first_task = config["tasks"][0]["task"] if isinstance(config["tasks"][0], dict) else config["tasks"][0]
    ordered = schedule_cells(cells, first_task, seed)
    out_dir.mkdir(parents=True, exist_ok=True)
    lock = acquire_coordinator(out_dir / "coordinator.lock")
    try:
        # Reject changed or unversioned partial runs before scheduling *any*
        # cell. A per-cell check alone can still send pending cells from a new
        # workflow while other threads discover old cached fingerprints.
        workflow_path = out_dir / "WORKFLOW.json"
        workflow = {"workflow_sha256": config["_workflow_sha256"],
                    "config_sha256": config.get("_sha256")}
        pin_error = run_pin_error(out_dir, workflow)
        if pin_error:
            budget_file = out_dir / "budget.json"
            try:
                used = json.loads(budget_file.read_text()).get("requests_used", 0) if budget_file.is_file() else 0
            except (OSError, ValueError):
                used = None
            blocked = [{"cell": cell, "status": "protocol_stop" if index == 0 else "not_started",
                        "error": pin_error, "record": {}, "calls": 0}
                       for index, cell in enumerate(ordered)]
            return {"cells": blocked, "calls": 0, "stop": "protocol_stop",
                    "real_requests": used if live else 0, "order_seed": seed,
                    "first_wave_task": first_task, "first_wave_cells": 0}
        if not workflow_path.is_file():
            workflow_path.write_text(json.dumps(workflow, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        budget_path = out_dir / "budget.json"
        shared = LiveBudget(budget_path, max_requests=int(config.get("global_physical_cap", 800)),
                            max_seconds=float(config.get("wall_seconds", 12 * 3600)))
        matrix_counter = [0]
        counter_lock = threading.Lock()
        matrix_cap = int(config.get("matrix_cap", 768))
        results = []
        calls = 0
        stop = None
        from concurrent.futures import ThreadPoolExecutor

        model_concurrency = (dict(model_concurrency) if model_concurrency is not None
                             else dict(config.get("model_concurrency") or {}))
        _sems: dict[str, threading.Semaphore] = {}
        _sems_lock = threading.Lock()

        def _model_sem(model_id: str) -> threading.Semaphore:
            with _sems_lock:
                if model_id not in _sems:
                    limit = int(model_concurrency.get(
                        model_id, int(config.get("concurrency") or 3)))
                    _sems[model_id] = threading.Semaphore(max(1, limit))
                return _sems[model_id]

        def _run(cell: dict) -> dict:
            sem = _model_sem(cell["model_id"])
            sem.acquire()
            try:
                return _run_cell(cell)
            finally:
                sem.release()

        def _run_cell(cell: dict) -> dict:
            spec = (spec_resolver(cell) if spec_resolver is not None
                    else require_experiment_model(build_registry(), cell["display_name"]))
            matrix_budget = SharedCounterBudget(shared, matrix_cap, matrix_counter, counter_lock, "matrix")
            cell_id = f"{cell['task']}/{cell['model_id']}/{cell['arm']}/rep{cell['rep']}"
            cell_budget = CellBudget(matrix_budget, cell["cell_cap"], cell_id)
            raw = client_factory(spec, cell, cell_budget, out_dir)
            fingerprint = send_fingerprint(spec, cell, config, raw)
            cell_dir = cell_directory(out_dir, cell)
            state_path = cell_dir / "state.json"
            if state_path.is_file():
                saved = json.loads(state_path.read_text(encoding="utf-8"))
                for item in saved.get("calls") or []:
                    if not item.get("snapshot_sha256"):
                        return {"cell": cell, "status": "protocol_stop", "error": "snapshot_legacy",
                                "record": {}, "calls": 0}
                    if item["snapshot_sha256"] != _digest(item):
                        return {"cell": cell, "status": "protocol_stop", "error": "snapshot_corrupt",
                                "record": {}, "calls": 0}
            cached = read_cell_cache(cell_dir)
            if cached and cached.get("status") == "cache_corrupt":
                return {"cell": cell, "status": "protocol_stop", "error": "cache_corrupt",
                        "record": {}, "calls": 0}
            if (cached and cached.get("status") == "executed"
                    and cached.get("fingerprint") == fingerprint):
                return {"cell": cell, "status": "executed", "error": None, "cached": True,
                        "record": {"accepted": cached.get("accepted"),
                                   "status": cached.get("record_status")},
                        "calls": 0}
            if cached or (state_path.is_file() and saved.get("calls") and saved.get("fingerprint") != fingerprint):
                return {"cell": cell, "status": "protocol_stop", "error": "workflow_fingerprint_mismatch",
                        "record": {}, "calls": 0}
            wrapped = SnapshotClient(raw, cell_dir / "state.json", fingerprint)
            from .audit import AuditLog
            audited = AuditedClient(
                wrapped, audit=AuditLog(cell_dir / "audit.jsonl"),
                run_id=config.get("run_id", "v22"),
                cell_id=f"{cell['task']}/{spec.model_id}/{cell['arm']}/rep{cell['rep']}",
                spec=spec, arm=cell["arm"], task_id=cell["task"], replicate=cell["rep"], stage="code")
            task = tasks_by_id[cell["task"]]
            try:
                record = run_one_cell(cell, audited, task, cell_dir, binary=binary,
                                      instrument=instrument, stage_runner=stage_runner)
                write_cell_cache(cell_dir, {
                    "status": "executed", "fingerprint": fingerprint,
                    "accepted": record.get("accepted"),
                    "record_status": record.get("status"),
                    "calls": wrapped.calls,
                })
                return {"cell": cell, "status": "executed", "error": None, "record": record,
                        "calls": wrapped.calls}
            except CellBudgetExhausted as exc:
                # one cell spent its own cap; the other cells must continue
                return {"cell": cell, "status": "budget_exhausted", "scope": "cell",
                        "error": str(exc), "record": {}, "calls": wrapped.calls}
            except BudgetExhausted as exc:
                return {"cell": cell, "status": "budget_exhausted", "scope": "global",
                        "error": str(exc), "record": {}, "calls": wrapped.calls}
            except ConfigError as exc:
                return {"cell": cell, "status": "protocol_stop", "error": str(exc),
                        "record": {}, "calls": wrapped.calls}
            except Exception as exc:  # noqa: BLE001 - one cell must not kill the batch
                name = type(exc).__name__
                # A transport reset or timeout is a property of one attempt, not
                # of the harness: record it and keep the batch going so a resume
                # can retry it. Only a protocol/snapshot/audit fault is fatal.
                recoverable = name in {
                    "APIConnectionError", "APITimeoutError", "RateLimitError",
                    "InternalServerError", "TransientLlmError", "HardTimeoutError",
                    "ModelIdentityError", "EmptyResponseError",
                }
                return {"cell": cell,
                        "status": "transport_error" if recoverable else "infrastructure_error",
                        "error": f"{name}: {exc}"[:300], "record": {}, "calls": wrapped.calls}

        def _consume(batch: list[dict]) -> str | None:
            nonlocal calls
            reason = None
            with ThreadPoolExecutor(max_workers=int(config.get("concurrency") or 3)) as pool:
                for item in pool.map(_run, batch):
                    results.append(item)
                    calls += item["calls"]
                    if item["status"] in ("infrastructure_error", "protocol_stop"):
                        reason = reason or item["status"]
                    elif (item["status"] == "budget_exhausted"
                          and item.get("scope") != "cell"):
                        reason = "budget_exhausted"
            return reason

        first_wave_tasks = set(config.get("first_wave_tasks") or [first_task])
        only_rep0 = bool(config.get("first_wave_rep0"))
        first_wave = [cell for cell in ordered
                      if cell["task"] in first_wave_tasks
                      and (not only_rep0 or cell["rep"] == 0)]
        rest = [cell for cell in ordered if cell not in first_wave]
        if live:
            reason = _consume(first_wave)
            if reason == "infrastructure_error" or any(item["status"] == "infrastructure_error" for item in results):
                stop = "infrastructure_stop_after_first_wave"
                for pending in rest:
                    results.append({"cell": pending, "status": "not_started", "error": stop,
                                    "record": {}, "calls": 0})
            elif reason in {"budget_exhausted", "protocol_stop"}:
                stop = reason
                for pending in rest:
                    results.append({"cell": pending, "status": "not_started", "error": stop,
                                    "record": {}, "calls": 0})
            else:
                index = 0
                while index < len(rest):
                    width = int(config.get("concurrency") or 3)
                    reason = _consume(rest[index:index + width])
                    index += width
                    if reason in {"budget_exhausted", "protocol_stop", "infrastructure_error"}:
                        stop = reason
                        for pending in rest[index:]:
                            results.append({"cell": pending, "status": "not_started", "error": stop,
                                            "record": {}, "calls": 0})
                        break
        else:
            index = 0
            while index < len(ordered):
                width = int(config.get("concurrency") or 3)
                reason = _consume(ordered[index:index + width])
                index += width
                if reason in {"budget_exhausted", "protocol_stop", "infrastructure_error"}:
                    stop = reason
                    for pending in ordered[index:]:
                        results.append({"cell": pending, "status": "not_started", "error": stop,
                                        "record": {}, "calls": 0})
                    break
        return {"cells": results, "calls": calls, "stop": stop,
                "real_requests": 0 if not live else shared.requests_used,
                "order_seed": seed, "first_wave_task": first_task,
                "first_wave_cells": len(first_wave)}
    finally:
        lock.close()
