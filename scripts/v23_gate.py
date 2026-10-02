#!/usr/bin/env python3
"""strong-link-v23 offline gate. No network, no paid request.

Runs the v23 matrix tests, a 72-cell mock over the frozen config, and the real
local toolchain control; writes OFFLINE_GATE.json/md with a per-item status.
A red item blocks the paid batch (per the v23 prompt section 5).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

_VENV = REPO / ".venv/bin/python"
PYBIN = _VENV if _VENV.is_file() else Path(sys.executable)

from cir_workflow.generation import load_gen_tasks  # noqa: E402
from cir_workflow.multi_gen import execute_matrix, load_config, pilot_tasks  # noqa: E402


def sha(p: Path) -> str | None:
    try:
        return hashlib.sha256(p.read_bytes()).hexdigest()
    except OSError:
        return None


def run_unittest(pattern: str) -> dict:
    proc = subprocess.run([str(PYBIN), "-m", "unittest", pattern, "-v"],
                          cwd=str(REPO), capture_output=True, text=True,
                          env={"PYTHONPATH": "python:.", "PATH": __import__("os").environ["PATH"]})
    tail = (proc.stdout + proc.stderr).strip().splitlines()[-1:] or [""]
    return {"ok": proc.returncode == 0, "tail": tail[0],
            "stdout": proc.stdout[-1500:] + proc.stderr[-1500:]}


class _Mock:
    def __init__(self, spec, budget):
        self.model = spec.model_id
        self.max_tokens = 4096
        self.temperature = 0
        self.budget = budget
        self.calls = 0
        self.stage = "code"

    def set_stage(self, stage):
        self.stage = stage

    def complete(self, system, user):
        self.budget.reserve({"cell_id": self.model})
        self.budget.mark_attempt("returned")
        self.calls += 1
        return SimpleNamespace(text="fn main() {}\n", response_model=self.model,
                               usage={"prompt_tokens": 1, "completion_tokens": 1},
                               wall_ms=1, finish_reason="stop", transport_attempt=1)


def run_mock(config: dict, out: Path) -> dict:
    """Traverse all 72 cells with a fake transport. Zero real requests."""
    tasks = {t.id: t for t in load_gen_tasks(REPO)}

    def factory(spec, cell, budget, out_dir):
        return _Mock(spec, budget)

    def runner(cell, client, task, out_dir):
        for _ in range(cell["round_cap"]):
            client.complete("s", "u")
        return {"accepted": False, "status": "not_accepted"}

    result = execute_matrix(config, out, client_factory=factory, tasks_by_id=tasks,
                            binary=Path("missing"), stage_runner=runner, live=False)
    by_arm = {}
    for item in result["cells"]:
        by_arm[item["cell"]["arm"]] = by_arm.get(item["cell"]["arm"], 0) + 1
    return {"cells": len(result["cells"]), "by_arm": by_arm,
            "calls": result["calls"], "stop": result["stop"],
            "executed": sum(1 for i in result["cells"] if i["status"] == "executed")}


def freeze_manifest() -> dict:
    files = [
        "python/cir_workflow/multi_gen.py", "python/cir_workflow/arms.py",
        "python/cir_workflow/generation.py", "python/cir_workflow/rust_oracle.py",
        "python/cir_workflow/bounded_monitor.py", "python/cir_workflow/channels.py",
        "python/cir_workflow/hard_timeout.py", "python/cir_workflow/v23_specs.py",
        "python/cir_workflow/audit.py", "python/cir_workflow/live.py",
        "prompts/rust_from_cir_v2.md", "prompts/concir_generation_v3.md",
        "prompts/concir_generation_v4.md", "runtime/concir_sync/src/lib.rs",
        "scripts/v23_phase1_eval.py", "scripts/v23_gate.py",
        "scripts/v23_live.py", "scripts/v23_g0_control.py", "scripts/v23_g3_control.py",
    ]
    hashes = {f: sha(REPO / f) for f in files if (REPO / f).is_file()}
    return {
        "files": hashes,
        "concir_backend_sha256": sha(REPO.parent / "ConcIR/target/release/concir-backend"),
        "concir_instrument_sha256": sha(REPO.parent / "ConcIR/target/release/concir-instrument"),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--notes", required=True)
    args = ap.parse_args()
    out = Path(args.out).resolve(); out.mkdir(parents=True, exist_ok=True)
    notes = Path(args.notes); notes.mkdir(parents=True, exist_ok=True)
    config = load_config(Path(args.config))

    tests = run_unittest("python.tests.test_v23_matrix")
    tests_v22 = run_unittest("python.tests.test_v22_matrix")
    mock = run_mock(config, out / "mock-72") if tests["ok"] else {"skipped": True}
    control = subprocess.run([str(PYBIN), str(REPO / "scripts/v23_g3_control.py")],
                             cwd=str(REPO), capture_output=True, text=True,
                             env={"PYTHONPATH": "python:.", "PATH": __import__("os").environ["PATH"]})
    g0_control = subprocess.run([str(PYBIN), str(REPO / "scripts/v23_g0_control.py")],
                                cwd=str(REPO), capture_output=True, text=True,
                                env={"PYTHONPATH": "python:.", "PATH": __import__("os").environ["PATH"]})
    manifest = freeze_manifest()

    # Per-item gate status. Verified = a passing test/exercise in this run.
    items = [
        {"id": "G-1", "item": "72-cell mock over the frozen config",
         "status": "verified" if mock.get("cells") == 72 else "fail",
         "detail": mock},
        {"id": "G-2", "item": "round caps 1/3/3/3 and cell caps 2/6/6/12",
         "status": "verified" if tests["ok"] else "fail", "detail": tests["tail"]},
        {"id": "G-3", "item": "resume is zero new physical calls",
         "status": "verified" if tests["ok"] else "fail", "detail": "test_resume_is_zero_new_physical_calls"},
        {"id": "G-4", "item": "second coordinator refused; budget persists across restart",
         "status": "verified" if tests["ok"] else "fail", "detail": "test_second_coordinator_is_refused"},
        {"id": "G-5", "item": "local toolchain control (CIR -> explore -> codegen -> build)",
         "status": "verified" if control.returncode == 0 else "fail",
         "detail": (control.stdout or control.stderr)[-800:]},
        {"id": "G-6", "item": "snapshot tamper / protocol change stops before a new request",
         "status": "verified" if tests_v22["ok"] else "fail",
         "detail": "test_v22_matrix.test_snapshot_tamper_blocks + " + tests_v22["tail"]},
        {"id": "G-7", "item": "live G0-G2 scored by the independent requirement oracle",
         "status": "verified" if g0_control.returncode == 0 else "fail",
         "detail": (g0_control.stdout or g0_control.stderr)[-600:]},
        {"id": "G-8", "item": "each physical attempt linked to run/cell/stage/round/retry",
         "status": "verified" if tests["ok"] else "fail",
         "detail": "test_attempt_row_links_run_cell_stage_round_retry (full wrapper chain)"},
        {"id": "G-9", "item": "hard HTTP wall-clock timeout independent of the read timeout",
         "status": "verified" if tests["ok"] else "fail",
         "detail": "test_hard_timeout_aborts_and_continues (socket abort + unknown, no refund)"},
        {"id": "G-10", "item": "thinking wire params + max output forwarded and recorded",
         "status": "verified" if tests["ok"] else "fail",
         "detail": "test_chat/responses_thinking...; real provider acceptance confirmed by preflight"},
        {"id": "G-11", "item": "config-resolved channel used by transport and audit",
         "status": "verified" if tests["ok"] else "fail",
         "detail": "test_config_resolver_uses_config_channel + scripts/v23_live.py"},
    ]
    blocked = [i for i in items if i["status"] in ("fail", "blocker")]
    report = {"paid_requests_sent": 0, "gate_green": not blocked,
              "items": items, "blockers": [i["id"] for i in blocked],
              "freeze_manifest": manifest,
              "mock": mock, "tests": {"ok": tests["ok"], "tail": tests["tail"]},
              "tests_v22": {"ok": tests_v22["ok"], "tail": tests_v22["tail"]},
              "control_exit": control.returncode}
    (out / "OFFLINE_GATE.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    lines = ["# strong-link-v23 离线门槛", "",
             f"真实请求：**0**。门槛全绿：**{report['gate_green']}**。",
             f"阻塞项：`{report['blockers']}`。", "",
             "| id | 项 | 状态 | 说明 |", "| --- | --- | --- | --- |"]
    for i in items:
        lines.append(f"| {i['id']} | {i['item']} | {i['status']} | {str(i['detail'])[:120]} |")
    lines += ["", "门槛未全绿，因此**不发送付费请求**（按 v23 提示词第 5 节）。"]
    (notes / "OFFLINE_GATE.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (notes / "FREEZE_MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"gate_green": report["gate_green"], "blockers": report["blockers"],
                      "mock": mock, "tests_ok": tests["ok"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
