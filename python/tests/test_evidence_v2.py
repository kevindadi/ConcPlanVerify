"""Evidence state machine v6: layered verdicts tied to raw checker output."""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from cir_workflow.evidence_v2 import evaluate_reexecution, hash_verification

CONTRACT = {"properties": [{"id": "p1", "kind": "safety", "req": ["R1"]}]}


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _files(tmp: Path) -> dict:
    src = tmp / "s.rs"; cir = tmp / "c.json"; con = tmp / "ct.json"
    src.write_text("fn main(){}")
    cir.write_text("{}")
    con.write_text("{}")
    roles = {"source": src, "cir": cir, "contract": con}
    arts = [{"role": role, "path": str(p), "sha256": _sha(p)} for role, p in roles.items()]
    return {"source_path": str(src), "cir_path": str(cir), "contract_path": str(con),
            "source_sha256": _sha(src), "cir_sha256": _sha(cir), "contract_sha256": _sha(con),
            "backend_sha256": "backend", "artifacts": arts}


def _art(role: str, path: Path, binds: dict) -> dict:
    return {"role": role, "path": str(path), "sha256": _sha(path), "binds": binds}


def _materialize(r: dict, props: dict, complete) -> None:
    """Write raw checker files that match the scenario in ``r``."""

    tmp = Path(r["source_path"]).parent
    r["acquisition_id"] = "acq-test"
    r.setdefault("backend_sha256", "backend")
    binds = {"acquisition_id": r["acquisition_id"], "source_sha256": r["source_sha256"],
             "cir_sha256": r["cir_sha256"], "contract_sha256": r["contract_sha256"],
             "backend_sha256": r["backend_sha256"]}
    explore = tmp / "explore.stdout"
    explore.write_text(json.dumps({
        "properties": [{"id": k, "outcome": v} for k, v in props.items()],
        "complete": complete}))
    trace = tmp / "trace.jsonl"
    if not trace.exists():
        trace.write_text('{"op":"mutex_lock","r":"m"}\n')
    binding = r.get("binding") or {"mapping": {}, "ambiguous": [], "violated": {}}
    unresolved = {}
    for item in binding.get("ambiguous") or []:
        unresolved[item.get("rust")] = {k: v for k, v in item.items() if k != "rust"}
    violated = binding.get("violated") or {}
    if isinstance(violated, list):
        violated = {"declared": violated} if violated else {}
    bind_path = tmp / "binding-check.stdout"
    bind_path.write_text(json.dumps({
        "verified": {k: {"cir": v} for k, v in (binding.get("mapping") or {}).items()},
        "unresolved": unresolved, "violated": violated}))
    # Keep the in-memory summary consistent with the raw binding file.
    r["binding"] = {"mapping": binding.get("mapping") or {},
                    "ambiguous": binding.get("ambiguous") or [],
                    "violated": violated, "source": "rust-cli"}
    statuses = (r.get("conform") or {}).get("statuses") or {"conformant": 1}
    violations = list((r.get("conform") or {}).get("violations") or [])
    checks = []
    trace_sha = _sha(trace)
    for status, count in statuses.items():
        for _ in range(count):
            if status == "conformant":
                raw = {"status": "conformant"}
            else:
                item = violations.pop(0) if violations else {"status": status, "got": "op:res"}
                raw = {"status": item.get("status", status), "got": item.get("got"),
                       "event_index": item.get("event_index"), "expected": item.get("expected"),
                       "detail": item.get("detail")}
            checks.append({"trace_sha256": trace_sha, "stdout": json.dumps(raw)})
    if not checks:
        checks.append({"trace_sha256": trace_sha, "stdout": json.dumps({"status": "conformant"})})
    conform_path = tmp / "conform-check.json"
    conform_path.write_text(json.dumps({"checks": checks}))
    parsed_status = {}
    for check in checks:
        st = json.loads(check["stdout"]).get("status")
        parsed_status[st] = parsed_status.get(st, 0) + 1
    r["conform"] = {**(r.get("conform") or {}), "statuses": parsed_status, "traces": len(checks)}
    mon = r.get("monitor") or {"status": "ok", "properties": []}
    pairs = [tuple(p) for p in mon.get("properties") or []]
    mon_path = tmp / "monitor.stdout"
    mon_path.write_text(json.dumps({
        "status": mon.get("status"),
        "properties": [{"id": pid, "status": st} for pid, st in pairs]}))
    r["monitor"] = {"status": mon.get("status"), "properties": pairs}
    fresh = [a for a in r["artifacts"] if a.get("role") in {"source", "cir", "contract"}]
    fresh += [_art("model_check", explore, binds), _art("binding_check", bind_path, binds),
              _art("monitor", mon_path, binds), _art("execution", trace, binds),
              _art("conform_check", conform_path, binds)]
    func = r.get("functional") or {}
    if func.get("evidence_path") and Path(func["evidence_path"]).is_file():
        fresh.append(_art("functional", Path(func["evidence_path"]), binds))
    r["artifacts"] = fresh


def _functional_doc(path: Path, source_sha: str, expected, raw, status=None) -> None:
    derived = "pass" if raw == expected else "fail"
    path.write_text(json.dumps({
        "status": derived if status is None else status,
        "source_sha256": source_sha, "test_id": "stdout_eq", "kind": "stdout_eq",
        "expected": expected, "raw_stdout": raw}))


def _result(tmp: Path, **over) -> dict:
    base = {
        **_files(tmp), "cell": "m/t",
        "stages": {"source_build": "ok", "instrument": "ok", "instrumented_build": "ok"},
        "n_runs": 32, "runs_started": 32, "runs_completed": 32, "hang": False,
        "raw_events": 10, "projected_events": 10,
        "conform": {"traces": 1, "statuses": {"conformant": 1}, "violations": []},
        "monitor": {"status": "ok", "properties": [["p1", "PASS_bounded"]]},
        "binding": {"mapping": {"m": "main::m"}, "ambiguous": [], "violated": []},
        "thread_lifecycle": {"spawn": 1},
    }
    base.update(over)
    return base


def _ev(r, contract=CONTRACT, accepted=True, props=None, complete=True, materialize=True):
    used = {"p1": "PASS"} if props is None else props
    if materialize:
        _materialize(r, used, complete)
    return evaluate_reexecution(r, contract, accepted=accepted,
                                cir_props=used, cir_complete=complete)


class StateMachineTests(unittest.TestCase):
    def test_happy_path(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)))
        self.assertEqual(led.current_evaluation, "satisfied_bounded")
        self.assertTrue(led.all_obligations_satisfied)

    def test_binding_violated_blocks_obligations(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"m": "main::m"},
                                           "ambiguous": [], "violated": ["m"]})
            led = _ev(r)
        self.assertEqual(led.current_evaluation, "binding_declaration_error")
        self.assertFalse(led.all_obligations_satisfied)

    def test_model_unknown_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), props={"p1": "UNKNOWN"}, complete=True)
        self.assertFalse(led.model["verified"])
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")

    def test_model_missing_property_is_not_verified(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), props={}, complete=True)
        self.assertFalse(led.model["verified"])
        self.assertEqual(led.model["missing_properties"], ["p1"])

    def test_verdict_independent_of_historical_acceptance(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), conform={"traces": 1, "statuses": {"violation": 1},
                                           "violations": [{"status": "violation",
                                                           "got": "mutex_lock:m",
                                                           "resource": "m"}]})
            a = _ev(r, accepted=True)
            b = _ev(r, accepted=False)
        self.assertEqual(a.current_evaluation, "explicit_failure")
        self.assertEqual(b.current_evaluation, "explicit_failure")
        self.assertNotEqual(a.historical_acceptance, b.historical_acceptance)

    def test_source_build_failure_is_not_runtime_crash(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), stages={"source_build": "failed"},
                        runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "source_build_failed")
        self.assertNotEqual(led.run["state"], "runtime_crash")

    def test_instrument_build_failure_distinct(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), stages={"source_build": "ok", "instrument": "ok",
                                          "instrumented_build": "failed"},
                        runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "instrument_build_failed")

    def test_planned_runs_not_started_is_not_run(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), n_runs=32, runs_started=0, runs_completed=0)
            led = _ev(r)
        self.assertEqual(led.run["state"], "not_run")

    def test_empty_projection_is_inconclusive(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), projected_events=0, raw_events=64)
            led = _ev(r)
        self.assertEqual(led.trace["state"], "empty_projection")
        self.assertFalse(led.all_obligations_satisfied)

    def test_binding_gap_violation_is_unresolved_not_independent(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {}, "ambiguous": [{"rust": "tx"}],
                                           "violated": []},
                        conform={"traces": 1, "statuses": {"violation": 1},
                                 "violations": [{"status": "violation",
                                                 "got": "channel_send:tx",
                                                 "resource": "tx"}]})
            led = _ev(r)
        self.assertEqual(led.trace["state"], "binding_unresolved")
        self.assertFalse(led.trace["independent_violation"])

    def test_binding_gap_and_independent_both_preserved(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), binding={"mapping": {"m": "main::m"},
                                           "ambiguous": [{"rust": "tx"}],
                                           "violated": []},
                        conform={"traces": 2, "statuses": {"violation": 2},
                                 "violations": [
                                     {"status": "violation", "got": "channel_send:tx",
                                      "resource": "tx"},
                                     {"status": "violation", "got": "mutex_lock:m",
                                      "resource": "m"}]})
            led = _ev(r)
        self.assertTrue(led.trace["binding_gap"])
        self.assertTrue(led.trace["independent_violation"])
        self.assertEqual(led.current_evaluation, "explicit_failure")

    def test_hash_mismatch_blocks(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            r["artifacts"][0]["sha256"] = "WRONG"
            led = _ev(r)
        self.assertFalse(led.hashes["verified"])
        self.assertFalse(led.all_obligations_satisfied)

    def test_monitor_fail_is_requirement_failure(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), monitor={"status": "fail",
                                           "properties": [["p1", "FAIL"]]})
            led = _ev(r)
        self.assertEqual(led.current_evaluation, "requirement_failure")
        self.assertEqual(led.properties[0].obligation_state, "violated")
        self.assertTrue(led.reasons)

    def test_functional_fail_is_explicit(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            r = _result(tmp)
            ev = tmp / "functional.json"
            _functional_doc(ev, r["source_sha256"], "DONE done=6", "DONE done=5")
            r["functional"] = {"status": "fail", "evidence": "stdout mismatch",
                               "evidence_path": str(ev)}
            led = _ev(r)
        self.assertEqual(led.current_evaluation, "functional_failure")
        self.assertNotIn(led.current_evaluation, {"inconclusive", "satisfied_bounded"})

    def test_sync_pass_cannot_override_functional_fail(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            r = _result(tmp)
            ev = tmp / "functional.json"
            _functional_doc(ev, r["source_sha256"], "DONE done=6", "DONE done=5")
            r["functional"] = {"status": "fail", "evidence": "x", "evidence_path": str(ev)}
            led = _ev(r)
        self.assertEqual(led.trace["state"], "observed_conformant")
        self.assertEqual(led.current_evaluation, "functional_failure")

    def test_invalid_evidence_is_not_a_requirement_failure(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), monitor={"status": "fail",
                                           "properties": [["p1", "FAIL"]]})
            r["artifacts"][0]["sha256"] = "WRONG"
            led = _ev(r)
        self.assertNotEqual(led.current_evaluation, "requirement_failure")
        self.assertEqual(led.current_evaluation, "inconclusive")
        self.assertIn("evidence_invalid", " ".join(led.reasons))

    def test_swapped_candidate_fields_block(self):
        # Change the top-level source while keeping the old artifacts.
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            r["source_sha256"] = "SWAPPED"
            led = _ev(r)
        self.assertFalse(led.hashes["verified"])
        self.assertIn("source", led.hashes.get("candidate_mismatch", []))
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")

    def test_functional_with_missing_evidence_file_is_invalid(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td), functional={"status": "fail",
                                              "evidence": "stale",
                                              "evidence_path": "/nonexistent/f.json"})
            led = _ev(r)
        self.assertFalse(led.layers["functional"] == "fail"
                         and led.current_evaluation == "functional_failure")
        self.assertNotEqual(led.current_evaluation, "functional_failure")

    def test_layers_recorded(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)))
        for key in ("conformance", "requirement", "functional", "run", "binding", "tool"):
            self.assertIn(key, led.layers)

    def test_empty_contract_never_passes(self):
        with tempfile.TemporaryDirectory() as td:
            led = _ev(_result(Path(td)), contract={"properties": [], "preserved": []},
                      props={})
        self.assertFalse(led.all_obligations_satisfied)
        self.assertEqual(led.current_evaluation, "inconclusive")

    def test_changed_file_bytes_block_satisfaction(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            led_ok = _ev(r)
            self.assertEqual(led_ok.current_evaluation, "satisfied_bounded")
            Path(r["source_path"]).write_text("fn main(){ /* changed */ }")
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertFalse(led.hashes["verified"])
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")

    def test_missing_checker_file_blocks_satisfaction(self):
        with tempfile.TemporaryDirectory() as td:
            r = _result(Path(td))
            _ev(r)
            model = next(a for a in r["artifacts"] if a["role"] == "model_check")
            Path(model["path"]).unlink()
            led = evaluate_reexecution(r, CONTRACT, accepted=True,
                                       cir_props={"p1": "PASS"}, cir_complete=True)
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")
        self.assertFalse(led.model["verified"])

    def test_wrong_functional_status_is_not_a_failure(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            r = _result(tmp)
            ev = tmp / "functional.json"
            _functional_doc(ev, r["source_sha256"], "DONE done=6", "DONE done=6", status="fail")
            r["functional"] = {"status": "fail", "evidence": "stale", "evidence_path": str(ev)}
            led = _ev(r)
        self.assertNotEqual(led.current_evaluation, "functional_failure")
        self.assertEqual(led.functional["reason"], "status_disagrees_with_raw")

    def test_functional_fail_survives_missing_model_evidence(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            r = _result(tmp)
            ev = tmp / "functional.json"
            _functional_doc(ev, r["source_sha256"], "DONE done=6", "DONE done=5")
            r["functional"] = {"status": "fail", "evidence": "x", "evidence_path": str(ev)}
            _materialize(r, {"p1": "PASS"}, True)
            r["artifacts"] = [a for a in r["artifacts"] if a["role"] != "model_check"]
            led = evaluate_reexecution(r, CONTRACT, accepted=True)
        self.assertEqual(led.current_evaluation, "functional_failure")
        self.assertFalse(led.model["verified"])

    def test_stdout_pass_does_not_prove_internal_var_eq(self):
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            r = _result(tmp, monitor={"status": "ok", "properties": [
                ["no-deadlock", "PASS_bounded"], ["c==6", "unsupported"]]})
            ev = tmp / "functional.json"
            # source hash is assigned in _result before this write
            _functional_doc(ev, r["source_sha256"], "DONE done=6", "DONE done=6")
            r["functional"] = {"status": "pass", "evidence": "stdout ok", "evidence_path": str(ev)}
            contract = {"properties": [{"id": "no-deadlock", "kind": "deadlock_free", "req": ["R1"]}],
                        "preserved": [{"kind": "reachable", "description": "c==6",
                                       "goal": {"kind": "var_eq", "resource": "main::c", "value": 6},
                                       "req": ["R2"]}]}
            led = _ev(r, contract=contract,
                      props={"no-deadlock": "PASS", "c==6": "PASS"}, complete=True)
        self.assertEqual(led.functional["status"], "pass")
        self.assertEqual(led.functional["supports"], ["external_stdout"])
        internal = next(p for p in led.properties if p.property_id == "c==6")
        self.assertEqual(internal.obligation_state, "unresolved")
        self.assertNotEqual(led.current_evaluation, "satisfied_bounded")
        self.assertEqual(led.evidence_grade, "partial_external")
        self.assertEqual(led.delivery_status, "withhold_capability")
        self.assertTrue(led.needs_human_review)
        self.assertTrue(led.needs_extra_check)


if __name__ == "__main__":
    unittest.main()
