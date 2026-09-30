#!/usr/bin/env python3
"""Failure-path batch drill. Fake transport, real AuditedClient, runner, and tools."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))
sys.path.insert(0, str(REPO))

from cir_workflow.audit import AuditLog, read_events  # noqa: E402
from cir_workflow.candidate_eval import evaluate_candidate  # noqa: E402
from cir_workflow.channels import AuditedClient  # noqa: E402
from cir_workflow.feedback_runner import (  # noqa: E402
    PROMPT_PATH, _default_evaluate, _fingerprint, _sha_text, load_frozen, render_prompt,
    run_arm, validate_config,
)
from cir_workflow.pilot_cases import evaluate_role, load_case  # noqa: E402
from cir_workflow.pilot_oracle import requirement_hold  # noqa: E402
from cir_workflow.toolchain_feedback import feedback_from_toolchain  # noqa: E402
from cir_workflow.transport import build_registry, require_experiment_model  # noqa: E402
from scripts.feedback_pilot import freeze_inputs  # noqa: E402
from scripts.run_feedback_pilot import execute_batch  # noqa: E402


def _fence(source: str) -> str:
    return "```rust\n" + source + "\n```"


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


class _Cache:
    def __init__(self) -> None:
        self.score: dict = {}
        self.eval: dict = {}

    def score_of(self, case, source, work):
        key = (case["id"], _sha(source))
        if key not in self.score:
            self.score[key] = evaluate_role(case, source, work)
        return self.score[key]

    def eval_of(self, source, case, work):
        key = (case["id"], _sha(source))
        if key not in self.eval:
            self.eval[key] = _default_evaluate(source, case, work)
        return self.eval[key]


class _Inner:
    def __init__(self, spec, case_id, arm, defects, controls, record) -> None:
        self.model_id = spec.model_id
        self.case_id = case_id
        self.arm = arm
        self.defects = defects
        self.controls = controls
        self.record = record
        self.calls = 0

    def complete(self, system, user):
        self.calls += 1
        self.record.append({"cell": f"{self.model_id}/{self.case_id}/{self.arm}",
                            "call": self.calls})
        returned = self.model_id
        if (self.model_id == "kimi-k2.7-code" and self.case_id == "lock_order"
                and self.arm == "verdict_only"):
            returned = "kimi-k3"
        source = self._source()
        return SimpleNamespace(text=_fence(source), usage={"prompt_tokens": 11, "completion_tokens": 7},
                               wall_ms=1, response_model=returned, request_id="fake")

    def _source(self) -> str:
        defect = self.defects[self.case_id]
        control = self.controls[self.case_id]
        if self.case_id == "early_release":
            return defect
        if self.case_id == "lock_order" and self.arm == "counterexample":
            return defect if self.calls == 1 else control
        return control


def _row(results, model, case, arm):
    return next(item for item in results
                if item.get("model_id") == model and item.get("case") == case and item.get("arm") == arm)


def main() -> int:
    notes = Path(sys.argv[1])
    notes.mkdir(parents=True, exist_ok=True)
    config, rows, _ignored = freeze_inputs()
    plan = validate_config(config, rows)
    if not plan["ok"]:
        print(json.dumps(plan["errors"], ensure_ascii=False, indent=2))
        return 2
    defects = {row["id"]: Path(row["defect"]["path"]).read_text(encoding="utf-8") for row in rows}
    controls = {row["id"]: Path(row["control"]["path"]).read_text(encoding="utf-8") for row in rows}
    cache = _Cache()
    print("caching real local results", flush=True)
    for row in rows:
        case = load_frozen(row, config["tools"])
        for source in (defects[row["id"]], controls[row["id"]]):
            cache.score_of(case, source, Path(tempfile.mkdtemp()))
            cache.eval_of(source, case, Path(tempfile.mkdtemp()))
            print(" cached", row["id"], _sha(source)[:8], flush=True)

    registry = build_registry()
    deepseek = require_experiment_model(registry, "DeepSeek Flash")

    def normal_factory(spec, cell_id, arm, case_id):
        inner = _Inner(spec, case_id, arm, defects, controls, [])
        inner._source = lambda: controls[case_id]  # type: ignore[method-assign]
        audit = AuditLog(notes / "normal-cells" / cell_id.replace("/", "__") / "events.jsonl")
        return AuditedClient(inner, audit=audit, run_id="v9-normal", cell_id=cell_id,
                             spec=spec, arm=arm, task_id=case_id, replicate=0, stage="repair")

    narrow = dict(config)
    narrow["models"] = ["DeepSeek Flash"]
    print("normal 12-cell drill", flush=True)
    normal = execute_batch(
        narrow, rows, notes / "normal-cells", client_factory=normal_factory,
        evaluate=cache.eval_of, score=cache.score_of)
    normal_ok = (normal["planned_cells"] == 12 and normal["http_attempts"] == 12
                 and all(item["stop"] == "requirement_pass" for item in normal["results"]))
    print("normal", normal_ok, normal["http_attempts"], flush=True)

    calls: list[dict] = []

    def batch_factory(spec, cell_id, arm, case_id):
        inner = _Inner(spec, case_id, arm, defects, controls, calls)
        audit = AuditLog(notes / "batch-cells" / spec.model_id / case_id / arm / "events.jsonl")
        return AuditedClient(inner, audit=audit, run_id="v9-drill", cell_id=cell_id,
                             spec=spec, arm=arm, task_id=case_id, replicate=0, stage="repair")

    def batch_eval(source, case, work):
        if case["id"] == "omitted_lock" and _sha(source) == _sha(defects["omitted_lock"]):
            return evaluate_candidate(
                source, Path(case["cir_path"]), Path(case["contract_path"]), work,
                binary=Path(case["tools"]["backend"]["path"]),
                instrument=Path("/no/such/concir-instrument"), n_runs=1, run_timeout=2,
                cell_id=case["id"])
        return cache.eval_of(source, case, work)

    # Persistent unknown, and an audit response whose state was not finished.
    unknown_case = load_frozen(rows[3], config["tools"])  # compute_value
    unknown_dir = notes / "batch-cells" / "deepseek-flash" / "compute_value" / "verdict_only"
    unknown_dir.mkdir(parents=True, exist_ok=True)
    (unknown_dir / "state.json").write_text(json.dumps({
        "fingerprint": _fingerprint(deepseek, "verdict_only", unknown_case, 2),
        "requests": [],
        "outcome_unknown": {"round": 1, "reason": "drill: no audit for the pending call"},
    }), encoding="utf-8")

    recover_case = load_frozen(rows[3], config["tools"])
    recover_eval = cache.eval_of(defects["compute_value"], recover_case, Path(tempfile.mkdtemp()))
    feedback, _items = feedback_from_toolchain("counterexample", recover_eval)
    prompt = render_prompt(recover_case["requirements"],
                           json.dumps(recover_case["cir"], sort_keys=True),
                           defects["compute_value"], feedback)
    system = PROMPT_PATH.read_text(encoding="utf-8").strip()
    recover_dir = notes / "batch-cells" / "deepseek-flash" / "compute_value" / "counterexample"
    audit = AuditLog(recover_dir / "seed-events.jsonl")
    audit.model_call(
        run_id="v9-drill", cell_id="deepseek-flash/compute_value/counterexample",
        model=deepseek.display_name, provider=deepseek.provider, transport=deepseek.channel,
        arm="counterexample", task_id="compute_value", replicate=0, stage="repair",
        requested_model=deepseek.model_id, returned_model=deepseek.model_id,
        usage_raw={"prompt_tokens": 11, "completion_tokens": 7},
        started_at=1.0, ended_at=1.2, prompt=system + "\n\n" + prompt.strip(),
        response=_fence(controls["compute_value"]), candidate_round=1, attempt_id="a1",
        status="ok")
    seeded = read_events(audit.path)
    recover_dir.mkdir(parents=True, exist_ok=True)
    (recover_dir / "state.json").write_text(json.dumps({
        "fingerprint": _fingerprint(deepseek, "counterexample", recover_case, 2),
        "requests": [],
        "pending": {
            "round": 1, "arm": "counterexample", "attempt": 1,
            "run_id": "v9-drill", "cell_id": "deepseek-flash/compute_value/counterexample",
            "model": deepseek.model_id, "prompt_sha256": _sha_text(prompt),
            "audit_prompt_sha256": seeded[0]["prompt_sha256"], "feedback": feedback,
        },
    }), encoding="utf-8")

    print("48-cell failure drill", flush=True)
    full = execute_batch(
        config, rows, notes / "batch-cells", client_factory=batch_factory,
        evaluate=batch_eval, score=cache.score_of, audit_events=seeded)
    by_key = {(item["model_id"], item["case"], item["arm"]): item for item in full["results"]}
    interesting = {
        "round1": _row(full["results"], "deepseek-flash", "lock_order", "verdict_only"),
        "round2": _row(full["results"], "deepseek-flash", "lock_order", "counterexample"),
        "both_failed_verdict": _row(full["results"], "deepseek-flash", "early_release", "verdict_only"),
        "both_failed_counter": _row(full["results"], "deepseek-flash", "early_release", "counterexample"),
        "tool_failure": _row(full["results"], "deepseek-flash", "omitted_lock", "verdict_only"),
        "outcome_unknown": _row(full["results"], "deepseek-flash", "compute_value", "verdict_only"),
        "recovered": _row(full["results"], "deepseek-flash", "compute_value", "counterexample"),
        "identity": _row(full["results"], "kimi-k2.7-code", "lock_order", "verdict_only"),
        "identity_blocked": _row(full["results"], "kimi-k2.7-code", "lock_order", "counterexample"),
    }
    qwen_early = _row(full["results"], "qwen3.8-flash", "early_release", "counterexample")
    drill_ok = (
        full["planned_cells"] == 48 and len(by_key) == 48
        and interesting["round1"]["stop"] == "requirement_pass"
        and interesting["round1"]["first_requirement_round"] == 1
        and interesting["round2"]["stop"] == "requirement_pass"
        and interesting["round2"]["first_requirement_round"] == 2
        and interesting["both_failed_verdict"]["stop"] == "cell_repair_budget_exhausted"
        and interesting["both_failed_counter"]["stop"] == "cell_repair_budget_exhausted"
        and qwen_early["status"] == "executed"
        and interesting["tool_failure"]["stop"] == "tool_failure"
        and interesting["outcome_unknown"]["stop"] == "outcome_unknown"
        and interesting["outcome_unknown"]["actual_model_requests_this_run"] == 0
        and interesting["recovered"]["stop"] == "requirement_pass"
        and interesting["recovered"]["actual_model_requests_this_run"] == 0
        and interesting["identity"]["stop"] == "identity_mismatch"
        and interesting["identity"]["status"] == "executed"
        and interesting["identity_blocked"]["status"] == "blocked"
        and not any(item["model_id"] == "deepseek-flash" and item["status"] != "executed"
                    for item in full["results"])
    )
    print("drill", drill_ok, "http", full["http_attempts"], flush=True)

    # Same-prompt rounds and a tampered body, using the cached real defect result.
    same = notes / "recovery-check"
    feedback_v, _items = feedback_from_toolchain("verdict_only", cache.eval_of(
        defects["lock_order"], load_frozen(rows[0], config["tools"]), Path(tempfile.mkdtemp())))
    lock_case = load_frozen(rows[0], config["tools"])
    same_prompt = render_prompt(lock_case["requirements"], json.dumps(lock_case["cir"], sort_keys=True),
                                defects["lock_order"], feedback_v)
    packed = system + "\n\n" + same_prompt.strip()
    same_audit = AuditLog(same / "events.jsonl")
    common = dict(run_id="run-1", cell_id="cell-1", model=deepseek.display_name,
                  provider=deepseek.provider, transport=deepseek.channel, arm="verdict_only",
                  task_id="lock_order", replicate=0, stage="repair",
                  requested_model=deepseek.model_id, returned_model=deepseek.model_id,
                  usage_raw={"prompt_tokens": 11, "completion_tokens": 7},
                  started_at=1.0, ended_at=1.1, prompt=packed, status="ok")
    same_audit.model_call(response=_fence(defects["lock_order"]), candidate_round=1,
                          attempt_id="a1", **common)
    same_audit.model_call(response=_fence(controls["lock_order"]), candidate_round=2,
                          attempt_id="a2", **common)
    events = read_events(same_audit.path)
    dest = same / "run"
    dest.mkdir(parents=True)
    (dest / "state.json").write_text(json.dumps({
        "fingerprint": _fingerprint(deepseek, "verdict_only", lock_case, 2),
        "requests": [{
            "round": 1, "arm": "verdict_only", "status": "ok",
            "prompt_sha256": _sha_text(same_prompt), "response": _fence(defects["lock_order"]),
            "identity": {"identity_confirmed": True, "requested_model": deepseek.model_id,
                         "returned_model": deepseek.model_id},
            "usage": {"input_tokens": 11, "output_tokens": 7},
        }],
        "pending": {
            "round": 2, "arm": "verdict_only", "attempt": 2, "run_id": "run-1",
            "cell_id": "cell-1", "model": deepseek.model_id,
            "prompt_sha256": _sha_text(same_prompt),
            "audit_prompt_sha256": events[0]["prompt_sha256"], "feedback": feedback_v,
        },
    }), encoding="utf-8")
    holder = SimpleNamespace(calls=0, run_id="run-1", cell_id="cell-1",
                             complete=lambda system, user: (_ for _ in ()).throw(AssertionError("resent")))
    # A client object with complete(); SimpleNamespace lambda above is awkward. Use a class.
    class _NoSend:
        calls = 0
        run_id = "run-1"
        cell_id = "cell-1"

        def complete(self, system, user):
            self.calls += 1
            raise AssertionError("resent")

    nosend = _NoSend()
    recovered = run_arm(lock_case, deepseek, "verdict_only", nosend, dest,
                        evaluate=cache.eval_of, score=cache.score_of, audit_events=events)
    unknown_state = same / "unknown"
    unknown_state.mkdir()
    (unknown_state / "state.json").write_text(json.dumps({
        "fingerprint": _fingerprint(deepseek, "verdict_only", lock_case, 2),
        "requests": [{"round": 1, "status": "outcome_unknown", "arm": "verdict_only",
                      "usage": {"input_tokens": None, "output_tokens": None}}],
        "outcome_unknown": {"round": 1, "reason": "unconfirmed"},
    }), encoding="utf-8")
    repeats = []
    for _ in range(3):
        client = _NoSend()
        repeats.append(run_arm(lock_case, deepseek, "verdict_only", client, unknown_state,
                               evaluate=cache.eval_of, score=cache.score_of))
    tampered = json.loads(json.dumps(events[1]))
    Path(tampered["response_path"]).write_text("changed", encoding="utf-8")
    tamper_dir = same / "tamper"
    tamper_dir.mkdir()
    (tamper_dir / "state.json").write_text(json.dumps({
        "fingerprint": _fingerprint(deepseek, "verdict_only", lock_case, 2),
        "requests": [],
        "pending": {
            "round": 2, "arm": "verdict_only", "attempt": 2, "run_id": "run-1",
            "cell_id": "cell-1", "model": deepseek.model_id,
            "prompt_sha256": _sha_text(same_prompt),
            "audit_prompt_sha256": tampered["prompt_sha256"], "feedback": feedback_v,
        },
    }), encoding="utf-8")
    tamper_client = _NoSend()
    tamper = run_arm(lock_case, deepseek, "verdict_only", tamper_client, tamper_dir,
                     evaluate=cache.eval_of, score=cache.score_of, audit_events=[tampered])
    wrong_model = dict(tampered)
    wrong_model["requested_model"] = "kimi-k3"
    wrong_model["response_sha256"] = _sha_text(_fence(controls["lock_order"]))
    # Restore a matching body so only the model identity rejects it.
    Path(tampered["response_path"]).write_text(_fence(controls["lock_order"]), encoding="utf-8")
    wrong_dir = same / "wrong-model"
    wrong_dir.mkdir()
    (wrong_dir / "state.json").write_text((tamper_dir / "state.json").read_text(encoding="utf-8")
                                          .replace('"model": "deepseek-flash"',
                                                   '"model": "deepseek-flash"'), encoding="utf-8")
    # pending model stays deepseek-flash; event requested_model is kimi-k3.
    wrong_client = _NoSend()
    wrong = run_arm(lock_case, deepseek, "verdict_only", wrong_client, wrong_dir,
                    evaluate=cache.eval_of, score=cache.score_of,
                    audit_events=[{**events[1], "requested_model": "kimi-k3"}])

    recovered_text = json.loads((dest / "state.json").read_text())["requests"][1]["response"]
    recovery_doc = {
        "same_prompt_round2_selected": recovered["first_requirement_round"] == 2 and nosend.calls == 0,
        "recovered_response_is_control": recovered_text.find("let ga") < recovered_text.find("let gb"),
        "outcome_unknown_restarts": [item["actual_model_requests_this_run"] for item in repeats],
        "outcome_unknown_stops": [item["stop"] for item in repeats],
        "tampered_response": {"stop": tamper["stop"], "calls": tamper_client.calls},
        "wrong_model": {"stop": wrong["stop"], "calls": wrong_client.calls},
        "batch_unknown_cell_calls": interesting["outcome_unknown"]["actual_model_requests_this_run"],
        "batch_recovered_cell_calls": interesting["recovered"]["actual_model_requests_this_run"],
    }
    (notes / "RECOVERY_AUDIT.json").write_text(
        json.dumps(recovery_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    def public(item):
        return {
            "model_id": item.get("model_id"), "case": item.get("case"), "arm": item.get("arm"),
            "status": item.get("status"), "stop": item.get("stop"),
            "first_requirement_round": item.get("first_requirement_round"),
            "first_design_round": item.get("first_design_round"),
            "http_attempts": item.get("actual_model_requests_this_run"),
            "semantic_repair_rounds": item.get("semantic_repair_rounds"),
            "requests_recorded": item.get("requests_recorded"),
            "usage": item.get("usage"),
            "identity_confirmed": [
                (entry or {}).get("identity_confirmed") for entry in (item.get("identity") or [])],
            "rounds": item.get("rounds"),
            "feedback_layers": sorted({row.get("layer") for row in (item.get("feedback_provenance") or [])}),
        }

    summary = {
        "real_model_requests": 0,
        "framework_acceptance": "passed" if drill_ok and normal_ok else "failed",
        "model_experiment": "not run; framework acceptance is not a model result",
        "normal_12": {"ok": normal_ok, "planned_cells": normal["planned_cells"],
                      "http_attempts": normal["http_attempts"],
                      "results": [public(item) for item in normal["results"]]},
        "full_48": {
            "ok": drill_ok, "planned_cells": full["planned_cells"],
            "unique_cells": len(by_key),
            "http_attempts": full["http_attempts"],
            "semantic_repair_rounds": full["semantic_repair_rounds"],
            "global_budget_stop": full["global_budget_stop"],
            "global_budget_limit": 96,
            "highlights": {name: public(item) for name, item in interesting.items()},
            "results": [public(item) for item in full["results"]],
        },
    }
    (notes / "FULL_BATCH_DRILL.json").write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    # Oracle cases for the report. Build-backed checks reuse evaluate_role.
    early = load_case("early_release")["defect"]
    renamed = (early.replace("let ga ", "let guard_a ").replace("drop(ga)", "drop(guard_a)")
               .replace("let gb ", "let guard_b ").replace("drop(gb)", "drop(guard_b)"))
    lock = load_case("lock_order")
    returned = lock["control"].replace(
        "fn w(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {\n",
        "fn w(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {\n    return;\n", 1)
    branched = lock["control"].replace(
        "fn w(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {\n",
        "fn w(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {\n    if true { return; }\n", 1)
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        early_return = evaluate_role(lock, returned, root / "ret")
        branch = evaluate_role(lock, branched, root / "if")
        renamed_handle_src = (lock["control"].replace("let h =", "let worker_handle =")
                              .replace("h.join()", "worker_handle.join()"))
        renamed_handle = evaluate_role(lock, renamed_handle_src, root / "handle")
        control = evaluate_role(lock, lock["control"], root / "control")
    oracle_doc = {
        "early_return": {"requirement": early_return["requirement"]["status"],
                         "design": early_return["design"]["status"],
                         "build_ok": early_return["build_ok"]},
        "conditional_return": {"requirement": branch["requirement"]["status"],
                               "design": branch["design"]["status"]},
        "control": {"requirement": control["requirement"]["status"],
                    "design": control["design"]["status"]},
        "renamed_handle": {"requirement": renamed_handle["requirement"]["status"],
                           "design": renamed_handle["design"]["status"]},
        "renamed_guard": requirement_hold(renamed, "w", "a", "b", build_ok=True)["status"],
        "batch_highlights_ok": drill_ok,
        "normal_12_ok": normal_ok,
        "cell_repair_does_not_stop_model": qwen_early["status"] == "executed",
        "planned_cells": 48,
        "unique_cells": len(by_key),
    }
    (notes / "FAILURE_PATH_REGRESSIONS.json").write_text(
        json.dumps(oracle_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (notes / "ORACLE_REGRESSION_RESULTS.json").write_text(
        json.dumps({key: oracle_doc[key] for key in (
            "early_return", "conditional_return", "control", "renamed_handle", "renamed_guard")},
            ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("wrote drill ok", drill_ok and normal_ok)
    return 0 if drill_ok and normal_ok and recovery_doc["same_prompt_round2_selected"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
