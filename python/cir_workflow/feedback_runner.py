"""Two-arm repair runner for the frozen feedback pilot.

Round 0 is the frozen candidate and does not call a model. Each arm may make
at most ``max_repairs`` new model calls. Correctness comes from the independent
oracle. Tool failures and capability gaps do not schedule another call.
"""

from __future__ import annotations

import hashlib
import inspect
import json
import time
from pathlib import Path
from typing import Any, Callable

from .generation import _extract_rust_body
from .pilot_cases import CASES, evaluate_role, load_case
from .toolchain_feedback import feedback_from_toolchain, has_defect_signal
from .live import BudgetExhausted
from .transport import (
    ModelIdentityError, ModelUnavailable, build_registry, require_experiment_model,
    verify_identity,
)

PROMPT_PATH = Path(__file__).resolve().parents[2] / "prompts" / "feedback_repair_v1.md"


def _sha_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def _sha_file(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def render_prompt(requirements: str, cir_text: str, previous: str, feedback: str | None) -> str:
    template = PROMPT_PATH.read_text(encoding="utf-8").strip()
    parts = [template, "", "Requirements:", requirements.strip(), "",
             "ConcIR design:", cir_text.strip(), "",
             "Previous Rust program:", previous.rstrip()]
    if feedback:
        parts += ["", "Feedback:", feedback.rstrip()]
    return "\n".join(parts) + "\n"


def _usage(outcome) -> dict[str, Any]:
    """Normalize provider usage. A reported 0 stays 0; a missing field stays null."""

    from .audit import parse_usage
    raw = getattr(outcome, "usage", None)
    if not isinstance(raw, dict):
        raw = None
    parsed = parse_usage(raw)
    return {
        "input_tokens": parsed.input_tokens,
        "output_tokens": parsed.output_tokens,
        "cache_read_tokens": parsed.cache_read_tokens,
        "cache_write_tokens": parsed.cache_write_tokens,
        "reasoning_tokens": parsed.reasoning_tokens,
        "total_tokens": parsed.total_tokens,
        "model_time_ms": getattr(outcome, "wall_ms", None),
    }


def _load_state(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {"requests": []}
    return json.loads(path.read_text(encoding="utf-8"))


def _save_state(path: Path, state: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def _default_evaluate(source: str, case: dict[str, Any], work: Path) -> dict[str, Any]:
    from .candidate_eval import evaluate_candidate
    binary = Path(__file__).resolve().parents[2].parent / "ConcIR/target/release/concir-backend"
    instrument = binary.with_name("concir-instrument")
    cir_path = Path(case.get("cir_path") or (case["directory"] / "design.cir.json"))
    contract_path = Path(case.get("contract_path") or (case["directory"] / "contract.json"))
    tools = case.get("tools") or {}
    if tools.get("backend", {}).get("path"):
        binary = Path(tools["backend"]["path"])
    if tools.get("instrument", {}).get("path"):
        instrument = Path(tools["instrument"]["path"])
    n_runs = int(tools.get("n_runs") or 2)
    timeout = float(tools.get("run_timeout_s") or 8.0)
    if case.get("id") == "no_join":
        timeout = float(tools.get("no_join_run_timeout_s") or timeout)
    functional = None
    if str(case.get("id") or "").startswith("compute"):
        functional = {"test_id": "stdout_eq", "kind": "stdout_eq", "expected": "DONE done=6"}
    return evaluate_candidate(
        source, cir_path, contract_path, work, binary=binary, instrument=instrument,
        n_runs=n_runs, run_timeout=timeout, functional_spec=functional, cell_id=case["id"])


def load_frozen(row: dict, tools: dict | None = None) -> dict[str, Any]:
    """Load the case from the validated manifest paths, not from a fixed root."""

    def read(item: dict, label: str) -> str:
        path = Path(item["path"])
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            raise FileNotFoundError(f"{label} missing: {path}") from exc
        digest = hashlib.sha256(text.encode()).hexdigest()
        if digest != item.get("sha256"):
            raise ValueError(f"{label} hash mismatch for {path}")
        return text

    case = load_case(row["id"])
    defect = read(row["defect"], "defect")
    control = read(row["control"], "control")
    cir_text = read(row["cir"], "cir")
    contract_text = read(row["contract"], "contract")
    requirements = read(row["requirements"], "requirements")
    case["defect"] = defect
    case["control"] = control
    case["requirements"] = requirements
    case["cir"] = json.loads(cir_text)
    case["contract"] = json.loads(contract_text)
    case["defect_path"] = row["defect"]["path"]
    case["cir_path"] = row["cir"]["path"]
    case["contract_path"] = row["contract"]["path"]
    case["tools"] = tools or {}
    case["fingerprint_parts"] = {
        "case": row["id"],
        "defect_sha256": row["defect"]["sha256"],
        "control_sha256": row["control"]["sha256"],
        "cir_sha256": row["cir"]["sha256"],
        "contract_sha256": row["contract"]["sha256"],
        "requirements_sha256": row["requirements"]["sha256"],
        "prompt_sha256": row.get("prompt_sha256"),
        "oracle_sha256": row.get("oracle_sha256"),
        "cases_sha256": row.get("cases_sha256"),
        "tools": {key: (tools or {}).get(key, {}).get("sha256") for key in ("backend", "instrument")},
        "n_runs": (tools or {}).get("n_runs"),
        "run_timeout_s": (tools or {}).get("run_timeout_s"),
        "no_join_run_timeout_s": (tools or {}).get("no_join_run_timeout_s"),
    }
    return case


def _fingerprint(spec, arm: str, case: dict, max_repairs: int) -> str:
    payload = {"model": spec.model_id, "arm": arm, "max_repairs": max_repairs,
               **(case.get("fingerprint_parts") or {"case": case["id"],
                                                    "defect": _sha_text(case["defect"])})}
    return _sha_text(json.dumps(payload, sort_keys=True))


def _audit_response_text(event: dict) -> str | None:
    if isinstance(event.get("response_text"), str):
        return event["response_text"]
    path = event.get("response_path")
    if not path:
        return None
    try:
        return Path(path).read_text(encoding="utf-8")
    except OSError:
        return None


def _invoke_complete(client, system: str, user: str, *, candidate_round: int, attempt: int):
    """Pass the persisted repair identity when the client accepts it."""

    fn = client.complete
    try:
        params = inspect.signature(fn).parameters
    except (TypeError, ValueError):
        params = {}
    accepts = "candidate_round" in params or any(
        param.kind == inspect.Parameter.VAR_KEYWORD for param in params.values())
    if accepts:
        return fn(system, user, candidate_round=candidate_round, attempt=attempt)
    return fn(system, user)


def _physical_of(outcome=None, exc: BaseException | None = None, client=None) -> int:
    if isinstance(exc, BudgetExhausted):
        return int(getattr(exc, "physical_attempts", 0) or 0)
    logged = getattr(client, "last_physical_attempts", None)
    if isinstance(logged, int):
        return logged
    reported = getattr(outcome, "transport_attempt", None)
    if isinstance(reported, int) and reported > 0:
        return reported
    return 1 if outcome is not None else 0


def budget_stop_name(exc: BaseException) -> str:
    text = str(exc)
    if "deadline" in text or "wall-clock" in text:
        return "global_time_budget_exhausted"
    return "global_request_budget_exhausted"


def _persistent_unknown(state: dict) -> bool:
    if state.get("outcome_unknown"):
        return True
    return any(item.get("status") == "outcome_unknown" for item in state.get("requests") or [])


def _event_matches_pending(event: dict, pending: dict) -> bool:
    """Match one request identity. Prompt text alone is not an identity."""

    if event.get("kind") != "model-call":
        return False
    if pending.get("run_id") and event.get("run_id") != pending.get("run_id"):
        return False
    if pending.get("cell_id") and event.get("cell_id") != pending.get("cell_id"):
        return False
    if pending.get("model") and event.get("requested_model") != pending.get("model"):
        return False
    if event.get("arm") != pending.get("arm"):
        return False
    if event.get("candidate_round") != pending.get("round"):
        return False
    attempt = pending.get("attempt")
    if attempt is not None and event.get("attempt_id") != f"a{attempt}":
        return False
    wanted = pending.get("audit_prompt_sha256")
    if not wanted or event.get("prompt_sha256") != wanted:
        return False
    return True


def _recover_pending(state: dict, audit_events: list[dict]) -> str | None:
    """Match a saved pending call to one audit event. Never send it again.

    ``outcome_unknown`` stays on the state across later process starts.
    """

    if _persistent_unknown(state) and not state.get("pending"):
        return "outcome_unknown"
    pending = state.get("pending")
    if not pending:
        return None
    matches = [event for event in audit_events if _event_matches_pending(event, pending)]
    if len(matches) != 1:
        state["outcome_unknown"] = {
            "round": pending.get("round"), "arm": pending.get("arm"),
            "run_id": pending.get("run_id"), "cell_id": pending.get("cell_id"),
            "model": pending.get("model"), "attempt": pending.get("attempt"),
            "prompt_sha256": pending.get("prompt_sha256"),
            "reason": "pending request has no unique matching audit record; not resent",
        }
        state["pending"] = None
        return "outcome_unknown"
    match = matches[0]
    response = _audit_response_text(match)
    response_hash = _sha_text(response) if isinstance(response, str) else None
    recorded_hash = match.get("response_sha256")
    if response is None or (recorded_hash and response_hash != recorded_hash):
        state["outcome_unknown"] = {
            "round": pending.get("round"), "arm": pending.get("arm"),
            "run_id": pending.get("run_id"), "cell_id": pending.get("cell_id"),
            "model": pending.get("model"), "attempt": pending.get("attempt"),
            "prompt_sha256": pending.get("prompt_sha256"),
            "reason": "audit response missing or hash mismatch; not resent",
        }
        state["pending"] = None
        return "outcome_unknown"
    status = match.get("status") or "outcome_unknown"
    confirmed = bool(match.get("identity_confirmed"))
    if status == "error":
        status = "outcome_unknown"
        confirmed = False
    elif status == "identity_mismatch" or (match.get("returned_model") not in
                                           (None, match.get("requested_model"))):
        status = "identity_mismatch"
        confirmed = False
    elif match.get("returned_model") is None:
        status = "identity_unconfirmed"
        confirmed = False
    identity = {
        "requested_model": match.get("requested_model"),
        "returned_model": match.get("returned_model"),
        "identity_confirmed": confirmed and status == "ok",
        "error": match.get("error"),
        "recovered_from_audit": True,
    }
    state["requests"].append({
        "round": pending["round"], "arm": pending["arm"], "attempt": pending.get("attempt"),
        "recovered_from_audit": True, "response": response, "identity": identity,
        "usage": match.get("usage"), "prompt_sha256": pending.get("prompt_sha256"),
        "response_sha256": response_hash, "feedback": pending.get("feedback"),
        "status": status,
        "physical_attempts": match.get("transport_attempt") if match.get("transport_attempt") is not None else 1,
        "logical_call": True,
        "send_status": "sent",
        "usage_raw": (match.get("usage") or {}).get("raw") if isinstance(match.get("usage"), dict) else None,
    })
    state["pending"] = None
    if status == "outcome_unknown":
        state["outcome_unknown"] = {
            "round": pending.get("round"), "reason": "audit recorded an unfinished request",
        }
    if status == "ok" and identity["identity_confirmed"]:
        return "recovered"
    return status


def _cumulative(state: dict) -> dict[str, Any]:
    physical = 0
    logical = 0
    unknown = 0
    for item in state.get("requests") or []:
        if item.get("send_status") == "unknown":
            unknown += 1
            continue
        sent = item.get("physical_attempts")
        if isinstance(sent, int):
            physical += sent
        if item.get("logical_call"):
            logical += 1
    return {"physical_attempts_cumulative": physical,
            "logical_calls_cumulative": logical,
            "physical_attempts_unknown": unknown}


def run_arm(case: dict[str, Any], spec, arm: str, client, out_dir: Path, *,
            source: str | None = None, max_repairs: int = 2,
            evaluate: Callable | None = None,
            score: Callable | None = None,
            halt_after_new_requests: int | None = None,
            audit_events: list[dict] | None = None) -> dict[str, Any]:
    """Run one arm from a frozen candidate. ``source`` defaults to the defect."""

    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    state_path = out_dir / "state.json"
    state = _load_state(state_path)
    fingerprint = _fingerprint(spec, arm, case, max_repairs)
    if state.get("fingerprint") and state["fingerprint"] != fingerprint:
        return {"case": case["id"], "arm": arm, "model": spec.model_id, "stop": "fingerprint_mismatch",
                "actual_model_requests_this_run": 0, "physical_attempts_this_run": 0,
                "logical_calls_this_run": 0, "requests_recorded": len(state.get("requests") or []),
                "in_defect_denominator": False, "first_requirement_round": None,
                "first_design_round": None, "first_both_round": None, "rounds": [],
                "usage": [], "identity": [], "feedback_provenance": [], **_cumulative(state)}
    state["fingerprint"] = fingerprint
    recovery = _recover_pending(state, audit_events or [])
    _save_state(state_path, state)
    if recovery in {"outcome_unknown", "identity_mismatch", "identity_unconfirmed",
                    "request_error", "global_request_budget_exhausted",
                    "global_time_budget_exhausted"}:
        return {"case": case["id"], "arm": arm, "model": spec.model_id, "stop": recovery,
                "actual_model_requests_this_run": 0, "physical_attempts_this_run": 0,
                "logical_calls_this_run": 0,
                "requests_recorded": len(state.get("requests") or []),
                "in_defect_denominator": False, "first_requirement_round": None,
                "first_design_round": None, "first_both_round": None, "rounds": [],
                "usage": [item.get("usage") for item in state.get("requests") or []],
                "identity": [], "feedback_provenance": [],
                "outcome_unknown": state.get("outcome_unknown"), **_cumulative(state)}
    evaluate = evaluate or _default_evaluate
    score = score or evaluate_role
    current = source if source is not None else case["defect"]
    cir_text = json.dumps(case["cir"], sort_keys=True)
    new_requests = 0
    physical_this = 0
    rounds = []
    provenance: list[dict] = []
    first_requirement = None
    first_design = None
    first_both = None
    stop = None

    def consider(round_no: int, text: str, evaluation: dict, delivery: dict) -> bool:
        nonlocal first_requirement, first_design, first_both, stop
        requirement_pass = evaluation["requirement"]["status"] == "pass"
        design_pass = evaluation["design"]["status"] == "pass"
        if requirement_pass and first_requirement is None:
            first_requirement = round_no
        if design_pass and first_design is None:
            first_design = round_no
        if requirement_pass and design_pass and first_both is None:
            first_both = round_no
        action = (delivery.get("followup") or {}).get("action")
        category = (delivery.get("followup") or {}).get("category")
        record = {"round": round_no, "requirement_status": evaluation["requirement"]["status"],
                  "design_status": evaluation["design"]["status"],
                  "delivery_status": (delivery.get("ledger") or {}).get("delivery_status"),
                  "stop_action": action, "stop_category": category,
                  "support": evaluation["support"]}
        rounds.append(record)
        if evaluation["tool_error"] or category == "tool_failure" or action == "stop" and category == "tool_failure":
            stop = "tool_failure"
            return True
        if evaluation["capability_gap"] or category == "capability_gap":
            stop = "capability_gap"
            return True
        if requirement_pass:
            stop = "requirement_pass"
            return True
        return False

    tool_started = time.perf_counter()
    initial = score(case, current, out_dir / "round-0")
    initial_eval = evaluate(current, case, out_dir / "eval-0")
    initial_eval["tool_time_s"] = time.perf_counter() - tool_started
    finished = consider(0, current, initial, initial_eval)
    if (not finished and not has_defect_signal(initial_eval)
            and (initial_eval.get("followup") or {}).get("category") not in
            {"tool_failure", "capability_gap"}):
        stop = "no_toolchain_signal"
        finished = True
    delivery_for_feedback = initial_eval
    in_denominator = (initial["requirement_error"] and not initial["tool_error"]
                      and not initial["capability_gap"] and initial["support"]["supported"]
                      and stop not in {"tool_failure", "capability_gap"})
    if not finished:
        for repair_round in range(1, max_repairs + 1):
            saved = next((item for item in state["requests"] if item.get("round") == repair_round
                          and item.get("arm") == arm), None)
            feedback, items = feedback_from_toolchain(arm, delivery_for_feedback)
            for item in items:
                item["arm"] = arm
                item["round"] = repair_round - 1
            provenance.extend(items)
            prompt = render_prompt(case["requirements"], cir_text, current, feedback)
            if saved:
                if saved.get("prompt_sha256") not in (None, _sha_text(prompt)):
                    stop = "fingerprint_mismatch"
                    break
                if saved.get("status") in {"error", "outcome_unknown", "identity_mismatch",
                                           "identity_unconfirmed",
                                           "global_request_budget_exhausted",
                                           "global_time_budget_exhausted"}:
                    stop = "request_error" if saved.get("status") == "error" else saved["status"]
                    if stop == "outcome_unknown":
                        state["outcome_unknown"] = state.get("outcome_unknown") or {
                            "round": saved.get("round"), "reason": "saved unknown request",
                        }
                    break
                response = saved["response"]
                identity = saved["identity"]
            else:
                system = PROMPT_PATH.read_text(encoding="utf-8").strip()
                pending = {"round": repair_round, "arm": arm, "attempt": repair_round,
                           "run_id": getattr(client, "run_id", None),
                           "cell_id": getattr(client, "cell_id", None),
                           "model": spec.model_id,
                           "prompt_sha256": _sha_text(prompt),
                           "audit_prompt_sha256": _sha_text(system + "\n\n" + prompt.strip()),
                           "feedback": feedback}
                state["pending"] = pending
                _save_state(state_path, state)
                started = time.perf_counter()
                try:
                    outcome = _invoke_complete(
                        client, PROMPT_PATH.read_text(encoding="utf-8"), prompt,
                        candidate_round=repair_round, attempt=repair_round)
                except ModelIdentityError as exc:
                    outcome = exc.outcome
                    returned = getattr(outcome, "response_model", None) if outcome else None
                    sent = _physical_of(outcome, client=client)
                    raw_usage = getattr(outcome, "usage", None) if outcome else None
                    record = {
                        "round": repair_round, "arm": arm, "attempt": repair_round,
                        "case": case["id"], "model": spec.model_id,
                        "prompt_sha256": pending["prompt_sha256"], "feedback": feedback,
                        "response": getattr(outcome, "text", "") if outcome else "",
                        "identity": {"requested_model": spec.model_id, "returned_model": returned,
                                     "channel": spec.channel, "display_name": spec.display_name,
                                     "identity_confirmed": False, "error": str(exc)},
                        "usage": _usage(outcome) if outcome else {
                            "input_tokens": None, "output_tokens": None,
                            "cache_read_tokens": None, "cache_write_tokens": None,
                            "model_time_ms": None},
                        "usage_raw": raw_usage if isinstance(raw_usage, dict) else None,
                        "physical_attempts": sent, "logical_call": bool(sent),
                        "send_status": "sent" if sent else "unknown",
                        "transport_log": list(getattr(client, "last_transport_log", None) or []),
                        "status": "identity_mismatch",
                    }
                    state["requests"].append(record)
                    state["pending"] = None
                    physical_this += sent
                    new_requests += 1
                    _save_state(state_path, state)
                    stop = "identity_mismatch"
                    break
                except BudgetExhausted as exc:
                    stop = budget_stop_name(exc)
                    sent = _physical_of(exc=exc, client=client)
                    physical_this += sent
                    if sent:
                        new_requests += 1
                    state["requests"].append({
                        "round": repair_round, "arm": arm, "attempt": repair_round,
                        "case": case["id"], "model": spec.model_id,
                        "prompt_sha256": pending["prompt_sha256"], "feedback": feedback,
                        "response": "",
                        "identity": {"requested_model": spec.model_id, "returned_model": None,
                                     "channel": spec.channel, "identity_confirmed": False,
                                     "error": str(exc)},
                        "usage": {"input_tokens": None, "output_tokens": None,
                                  "cache_read_tokens": None, "cache_write_tokens": None,
                                  "model_time_ms": None},
                        "usage_raw": None,
                        "physical_attempts": sent,
                        "logical_call": bool(sent),
                        "send_status": "sent" if sent else "not_sent",
                        "status": stop, "error_type": "BudgetExhausted",
                    })
                    state["pending"] = None
                    _save_state(state_path, state)
                    break
                except Exception as exc:  # noqa: BLE001
                    kind = type(exc).__name__
                    sent = _physical_of(exc=exc, client=client)
                    log = list(getattr(client, "last_transport_log", None) or [])
                    if sent and log:
                        send_status = "sent"
                    elif sent:
                        send_status = "sent"
                    elif isinstance(exc, BudgetExhausted):
                        send_status = "not_sent"
                    else:
                        send_status = "unknown"
                    record = {
                        "round": repair_round, "arm": arm, "attempt": repair_round,
                        "case": case["id"], "model": spec.model_id,
                        "prompt_sha256": pending["prompt_sha256"], "feedback": feedback,
                        "response": "",
                        "identity": {"requested_model": spec.model_id, "returned_model": None,
                                     "channel": spec.channel, "identity_confirmed": False,
                                     "error": str(exc)},
                        "usage": {"input_tokens": None, "output_tokens": None,
                                  "cache_read_tokens": None, "cache_write_tokens": None,
                                  "model_time_ms": None},
                        "usage_raw": None,
                        "physical_attempts": sent, "logical_call": bool(sent),
                        "send_status": send_status, "transport_log": log,
                        "status": "error", "error_type": kind,
                    }
                    state["requests"].append(record)
                    state["pending"] = None
                    physical_this += sent
                    if sent:
                        new_requests += 1
                    _save_state(state_path, state)
                    stop = "request_error"
                    break
                model_ms = getattr(outcome, "wall_ms", None)
                if model_ms is None:
                    model_ms = int((time.perf_counter() - started) * 1000)
                returned = getattr(outcome, "response_model", None)
                requested = spec.model_id
                try:
                    confirmed = verify_identity(requested, returned)
                    identity_error = None
                except ModelIdentityError as exc:
                    confirmed = False
                    identity_error = str(exc)
                identity = {"requested_model": requested, "returned_model": returned,
                            "channel": spec.channel, "display_name": spec.display_name,
                            "identity_confirmed": bool(confirmed), "error": identity_error}
                usage = _usage(outcome)
                usage["model_time_ms"] = model_ms
                sent = _physical_of(outcome, client=client)
                physical_this += sent
                raw_usage = getattr(outcome, "usage", None)
                state["requests"].append({
                    "round": repair_round, "arm": arm, "attempt": repair_round,
                    "case": case["id"],
                    "model": spec.model_id, "prompt_sha256": pending["prompt_sha256"],
                    "feedback": feedback, "response": getattr(outcome, "text", ""),
                    "identity": identity, "usage": usage,
                    "usage_raw": raw_usage if isinstance(raw_usage, dict) else None,
                    "physical_attempts": sent, "logical_call": True, "send_status": "sent",
                    "transport_log": list(getattr(client, "last_transport_log", None) or []),
                    "tool_time_s": None, "status": "ok" if confirmed else "identity_unconfirmed",
                })
                state["pending"] = None
                _save_state(state_path, state)
                new_requests += 1
                response = state["requests"][-1]["response"]
                if not identity["identity_confirmed"]:
                    stop = "identity_mismatch" if identity["error"] else "identity_unconfirmed"
                    break
                if halt_after_new_requests is not None and new_requests >= halt_after_new_requests:
                    stop = "halted"
                    break
            if stop in {"identity_mismatch", "identity_unconfirmed", "halted"}:
                break
            if not identity["identity_confirmed"]:
                stop = "identity_mismatch" if identity.get("error") else "identity_unconfirmed"
                break
            extracted = _extract_rust_body(response)
            if not extracted:
                current_eval = score(case, current, out_dir / f"round-{repair_round}")
                # Protocol miss: the response is not a program. Count the request,
                # keep the previous source, and allow another repair if budget remains.
                rounds.append({"round": repair_round, "protocol_noncompliance": True,
                               "requirement_status": current_eval["requirement"]["status"],
                               "design_status": current_eval["design"]["status"]})
                continue
            current = extracted
            tool_started = time.perf_counter()
            evaluation = score(case, current, out_dir / f"round-{repair_round}")
            delivery = evaluate(current, case, out_dir / f"eval-{repair_round}")
            elapsed = time.perf_counter() - tool_started
            delivery["tool_time_s"] = elapsed
            delivery_for_feedback = delivery
            if state["requests"]:
                state["requests"][-1]["tool_time_s"] = elapsed
                _save_state(state_path, state)
            _save_state(state_path, state)
            if consider(repair_round, current, evaluation, delivery):
                break
        else:
            stop = stop or "cell_repair_budget_exhausted"
    if stop is None:
        stop = "cell_repair_budget_exhausted"
    return {
        "case": case["id"], "arm": arm, "model": spec.model_id,
        "display_name": spec.display_name, "channel": spec.channel,
        "stop": stop,
        "first_requirement_round": first_requirement,
        "first_design_round": first_design,
        "in_defect_denominator": in_denominator and source is None,
        "suggested_repairs": max_repairs,
        "actual_model_requests_this_run": new_requests,
        "logical_calls_this_run": new_requests,
        "physical_attempts_this_run": physical_this,
        "first_both_round": first_both,
        "semantic_repair_rounds": sum(1 for item in rounds if item.get("round")),
        "requests_recorded": len(state["requests"]),
        "rounds": rounds,
        "usage": [item.get("usage") for item in state["requests"]],
        "identity": [item.get("identity") for item in state["requests"]],
        "feedback_provenance": provenance,
        "tool_time_s": initial_eval.get("tool_time_s"),
        **_cumulative(state),
    }


def validate_config(config: dict, inputs: list[dict]) -> dict[str, Any]:
    """Check the frozen matrix. Any failure sets ``ok`` false and blocks a run."""

    errors = []
    specs = []
    registry = build_registry()
    for name in config.get("models") or []:
        try:
            specs.append(require_experiment_model(registry, name))
        except (KeyError, ModelUnavailable) as exc:
            errors.append(f"model {name!r}: {exc}")
    arms = list(config.get("arms") or [])
    if arms != ["verdict_only", "counterexample"]:
        errors.append(f"arms must be verdict_only and counterexample, got {arms}")
    if int(config.get("max_repairs", 2)) != 2:
        errors.append("max_repairs must be 2")
    prompt = PROMPT_PATH
    prompt_sha = _sha_file(prompt)
    if prompt_sha != config.get("prompt_sha256"):
        errors.append(f"prompt hash mismatch: {prompt_sha} != {config.get('prompt_sha256')}")
    for tool_key in ("backend", "instrument"):
        tool = (config.get("tools") or {}).get(tool_key) or {}
        path = Path(tool.get("path") or "")
        digest = _sha_file(path)
        if digest is None:
            errors.append(f"tool missing: {tool_key} {path}")
        elif digest != tool.get("sha256"):
            errors.append(f"tool hash mismatch: {tool_key}")
    preflight_ref = config.get("preflight") or {}
    preflight_path = Path(preflight_ref.get("path") or "")
    preflight_sha = _sha_file(preflight_path)
    if preflight_sha is None or preflight_sha != preflight_ref.get("sha256"):
        errors.append(f"preflight missing or hash mismatch: {preflight_path}")
        preflight_doc = {}
        preflight_cases = {}
    else:
        preflight_doc = json.loads(preflight_path.read_text(encoding="utf-8"))
        preflight_cases = {item["id"]: item for item in preflight_doc.get("cases") or []}
        tool = preflight_doc.get("tool") or {}
        frozen_tools = config.get("tools") or {}
        if tool.get("backend_sha256") != (frozen_tools.get("backend") or {}).get("sha256"):
            errors.append("preflight backend hash does not match the frozen backend")
        if tool.get("instrument_sha256") != (frozen_tools.get("instrument") or {}).get("sha256"):
            errors.append("preflight instrument hash does not match the frozen instrument")
        for key in ("n_runs", "run_timeout_s", "no_join_run_timeout_s"):
            if tool.get(key) != frozen_tools.get(key):
                errors.append(f"preflight budget {key} does not match the frozen config")
        if preflight_doc.get("oracle_sha256") != config.get("oracle_sha256"):
            errors.append("preflight oracle implementation does not match the frozen oracle")
        if preflight_doc.get("cases_sha256") != config.get("cases_sha256"):
            errors.append("preflight case wiring does not match the frozen wiring")
    oracle_py = Path(__file__).resolve().parent / "pilot_oracle.py"
    cases_py = Path(__file__).resolve().parent / "pilot_cases.py"
    if _sha_file(oracle_py) != config.get("oracle_sha256"):
        errors.append("oracle implementation hash mismatch")
    if _sha_file(cases_py) != config.get("cases_sha256"):
        errors.append("oracle case-wiring hash mismatch")
    seen = []
    for row in inputs:
        if row.get("id") not in CASES:
            errors.append(f"unknown case {row.get('id')}")
            continue
        for role in ("defect", "control"):
            item = row.get(role) or {}
            path = Path(item.get("path") or "")
            digest = _sha_file(path)
            if digest is None:
                errors.append(f"{row.get('id')} {role} missing: {path}")
            elif digest != item.get("sha256"):
                errors.append(f"{row.get('id')} {role} hash mismatch")
        for dep in ("cir", "contract", "requirements"):
            item = row.get(dep) or {}
            path = Path(item.get("path") or "")
            digest = _sha_file(path)
            if digest is None:
                errors.append(f"{row.get('id')} {dep} missing: {path}")
            elif digest != item.get("sha256"):
                errors.append(f"{row.get('id')} {dep} hash mismatch")
        if row.get("origin_kind") == "model-generation" and "kimi-k3" in (row.get("origin") or ""):
            errors.append(f"{row.get('id')} attributes K3 output to the pilot")
        prior = preflight_cases.get(row.get("id"))
        if prior is None:
            errors.append(f"{row.get('id')} has no matching preflight")
        elif prior.get("cir_sha256") != (row.get("cir") or {}).get("sha256"):
            errors.append(f"{row.get('id')} preflight CIR hash does not match the manifest")
        elif any(prior.get(f"{role}_sha256") != (row.get(role) or {}).get("sha256")
                 for role in ("defect", "control", "contract", "requirements")):
            errors.append(f"{row.get('id')} preflight input fingerprint does not match the manifest")
        elif prior.get("functional_spec") != (
                {"test_id": "stdout_eq", "kind": "stdout_eq", "expected": "DONE done=6"}
                if str(row.get("id") or "").startswith("compute") else None):
            errors.append(f"{row.get('id')} preflight functional spec does not match")
        elif prior.get("input_protocol_error") or not prior.get("model_verified"):
            errors.append(f"{row.get('id')} preflight failed: CIR is not a verified backend input")
        else:
            control = (prior.get("roles") or {}).get("control") or {}
            defect = (prior.get("roles") or {}).get("defect") or {}
            if (control.get("current_evaluation") != "satisfied_bounded"
                    or control.get("requirement") != "pass"):
                errors.append(f"{row.get('id')} control preflight is not a passing bounded delivery")
            signal = (
                defect.get("current_evaluation") in {
                    "explicit_failure", "functional_failure", "timeout", "runtime_crash",
                    "requirement_failure"}
                or defect.get("trace_state") == "observed_violation"
                or defect.get("functional") == "fail"
                or defect.get("run_state") == "timeout")
            if defect.get("requirement") != "fail" or not signal:
                errors.append(f"{row.get('id')} defect preflight has no toolchain defect signal")
        seen.append(row["id"])
    if seen != list(CASES):
        errors.append(f"case set {seen} != {list(CASES)}")
    # Both arms share the initial source and the non-feedback prompt.
    contexts = []
    if specs and inputs and not errors:
        sample = load_frozen(inputs[0], config.get("tools"))
        cir_text = json.dumps(sample["cir"], sort_keys=True)
        for arm in arms:
            feedback = ("bounded_trace_deviation=violation\n" if arm == "verdict_only"
                        else "bounded_trace_deviation=violation\nconform.got=mutex_lock:main::b\n")
            prompt = render_prompt(sample["requirements"], cir_text, sample["defect"], feedback)
            head, _, tail = prompt.rpartition("\nFeedback:\n")
            contexts.append(head)
            if "requirement_oracle=" in tail or "design_oracle=" in tail:
                errors.append("feedback still comes from the scoring oracle")
        if len(set(contexts)) != 1:
            errors.append("arms differ outside the feedback section")
    matrix = [{"model": spec.display_name, "model_id": spec.model_id, "case": case_id, "arm": arm,
               "max_repairs": 2} for spec in specs for case_id in seen for arm in arms]
    upper = len(matrix) * 2
    return {"ok": not errors, "errors": errors, "models": [spec.display_name for spec in specs],
            "model_ids": [spec.model_id for spec in specs], "cases": seen, "arms": arms,
            "cells": len(matrix), "requests_upper_bound": upper if not errors else 0,
            "llm_calls": 0, "matrix": matrix, "executable": not errors}
