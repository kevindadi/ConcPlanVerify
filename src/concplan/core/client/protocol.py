"""Wire protocol maps, result datatypes, and semantic outcome classification."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# Documented exit-code maps per command.
CHECK_EXIT = {"valid": 0, "invalid": 4}
SUPPORT_EXIT = {"supported": 0, "unsupported": 5, "invalid": 4}
EXPLORE_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
LOWER_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
SUMMARIZE_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
CHECK_REQ_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
VERIFY_DESIGN_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
VERIFY_REFINEMENT_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
EXPLAIN_CONTRACT_EXIT = {"pass": 0, "invalid": 4}
VALIDATE_CONTRACT_EXIT = {"pass": 0, "fail": 1}
EXPORT_CVN_EXIT = {"pass": 0, "invalid": 4, "unsupported": 5}
REPAIR_EXIT = {
    "repaired": 0,
    "already_satisfied": 0,
    "no_acceptable_candidate": 1,
    "budget_exhausted": 1,
    "analysis_unknown": 3,
    "invalid": 4,
    "invalid_config": 4,
    "unsupported": 5,
}
REPLAY_EXIT = {"replayed": 0}

_OUTCOME_COMMANDS: dict[str, dict[str, int]] = {
    "explore": EXPLORE_EXIT,
    "repair": REPAIR_EXIT,
    "check-req": CHECK_REQ_EXIT,
    "verify-design": VERIFY_DESIGN_EXIT,
}

_REPLAY_INTS = ("nodes", "chain_len")
SEARCH_ARTIFACT_SCHEMA = "concir-repair-artifact-v1"
EXTERNAL_ARTIFACT_SCHEMA = "concir-external-patch-artifact-v1"
_EXTERNAL_OUTCOME_MAP = {
    "PASS": "repaired",
    "FAIL": "no_acceptable_candidate",
    "UNKNOWN": "analysis_unknown",
    "INVALID": "invalid",
    "UNSUPPORTED": "unsupported",
}


@dataclass
class ConcirIdentity:
    """Everything that makes one CLI invocation reproducible."""

    binary: str
    binary_sha256: str
    argv: list[str]
    cwd: str
    input_sha256: str | None = None
    contract_sha256: str | None = None
    config: dict[str, Any] = field(default_factory=dict)
    engine: str | None = None


@dataclass
class ConcirResult:
    """One typed CLI invocation.

    ``kind`` is ``semantic``, ``process_error``, ``protocol_error``, or ``usage_error``.
    ``exit_code == 0`` does not by itself mean the verified property holds; use
    ``outcome`` / ``status``.
    """

    command: str
    argv: list[str]
    exit_code: int | None
    status: str
    kind: str
    payload: dict[str, Any] | None
    identity: ConcirIdentity
    stderr: str = ""
    error: str | None = None
    wall_ms: int = 0
    timed_out: bool = False
    call_dir: str | None = None
    stdout_path: str | None = None
    stderr_path: str | None = None
    exit_path: str | None = None
    input_path: str | None = None
    contract_path: str | None = None
    artifact_path: str | None = None

    @property
    def is_semantic(self) -> bool:
        return self.kind == "semantic"

    @property
    def outcome(self) -> str | None:
        if not self.payload:
            return None
        value = self.payload.get("outcome")
        return str(value) if value is not None else None

    @property
    def complete(self) -> bool | None:
        if not self.payload:
            return None
        value = self.payload.get("complete")
        return bool(value) if isinstance(value, bool) else None

    @property
    def unsupported(self) -> list[Any]:
        if not self.payload:
            return []
        return list(self.payload.get("unsupported", []) or [])

    @property
    def diagnostics(self) -> list[Any]:
        if not self.payload:
            return []
        return list(self.payload.get("diagnostics", []) or [])

    @property
    def valid(self) -> bool | None:
        if not self.payload:
            return None
        value = self.payload.get("valid")
        return bool(value) if isinstance(value, bool) else None


def _protocol(result: ConcirResult, message: str) -> ConcirResult:
    result.kind, result.status, result.error = "protocol_error", "protocol_error", message
    return result


def _eval_reject_exit(reason: Any) -> int:
    code = (reason or {}).get("code") if isinstance(reason, dict) else None
    if code == "verification_unknown":
        return 3
    if code in ("unsupported", "verification_unsupported"):
        return 5
    if code == "verification_fail":
        return 1
    return 4


def _is_invalid_payload(payload: dict[str, Any]) -> bool:
    return payload.get("valid") is False or "diagnostics" in payload or "error" in payload


def _classify(result: ConcirResult) -> ConcirResult:
    """Map a parsed payload + exit code to the documented semantic outcome."""
    payload = result.payload or {}
    command = result.command
    exit_code = result.exit_code

    if command == "check":
        valid = payload.get("valid")
        if not isinstance(valid, bool):
            return _protocol(result, "check payload has no boolean 'valid'")
        status = "valid" if valid else "invalid"
        expected = CHECK_EXIT[status]
    elif command == "support":
        supported = payload.get("supported")
        if not isinstance(supported, bool):
            return _protocol(result, "support payload has no boolean 'supported'")
        status = "supported" if supported else "unsupported"
        expected = SUPPORT_EXIT[status]
    elif command in _OUTCOME_COMMANDS:
        exit_map = _OUTCOME_COMMANDS[command]
        outcome = payload.get("outcome")
        if outcome is None:
            return _protocol(result, f"{command} payload missing outcome (exit {exit_code})")
        status = str(outcome) if command == "repair" else str(outcome).lower()
        if status not in exit_map:
            return _protocol(result, f"{command} payload has unknown outcome {outcome!r}")
        expected = exit_map[status]
    elif command == "repair-context":
        if payload.get("schema_version") != "concir-repair-context-v1":
            return _protocol(result, f"unexpected context schema {payload.get('schema_version')!r}")
        if exit_code != 0:
            return _protocol(result, f"repair-context exited {exit_code}")
        result.kind, result.status = "semantic", "context"
        return result
    elif command == "lower":
        outcome = payload.get("outcome")
        if outcome is not None:
            status = str(outcome).lower()
            if status not in LOWER_EXIT:
                return _protocol(result, f"lower payload has unknown outcome {outcome!r}")
            expected = LOWER_EXIT[status]
        elif exit_code == 0 and any(k in payload for k in ("modules", "program", "lowered", "schema")):
            status, expected = "pass", 0
        elif exit_code == 4 and _is_invalid_payload(payload):
            status, expected = "invalid", 4
        elif exit_code in (5, 3, 1):
            status = {5: "unsupported", 3: "unknown", 1: "fail"}[exit_code]
            expected = exit_code
        else:
            return _protocol(result, f"lower payload unrecognized for exit {exit_code}")
    elif command == "summarize":
        outcome = payload.get("outcome")
        if outcome is not None:
            status = str(outcome).lower()
            if status not in SUMMARIZE_EXIT:
                return _protocol(result, f"summarize payload has unknown outcome {outcome!r}")
            expected = SUMMARIZE_EXIT[status]
        elif (
            payload.get("schema") == "concir-module-summary/1"
            or "cross_module_lock_cycle" in payload
            or "modules" in payload
        ):
            has_cycle = bool(payload.get("cross_module_lock_cycle")) or any(
                bool(m.get("cross_module_lock_cycle"))
                for m in (payload.get("modules") or [])
                if isinstance(m, dict)
            )
            status = "fail" if has_cycle else "pass"
            expected = SUMMARIZE_EXIT[status]
        elif exit_code == 4 and _is_invalid_payload(payload):
            status, expected = "invalid", 4
        elif exit_code in (5, 3):
            status = {5: "unsupported", 3: "unknown"}[exit_code]
            expected = exit_code
        else:
            return _protocol(result, f"summarize payload unrecognized for exit {exit_code}")
    elif command == "verify-refinement":
        outcome = payload.get("outcome")
        if outcome is not None:
            status = str(outcome).lower()
            if status not in VERIFY_REFINEMENT_EXIT:
                return _protocol(result, f"verify-refinement payload has unknown outcome {outcome!r}")
            expected = VERIFY_REFINEMENT_EXIT[status]
        elif exit_code == 4 and _is_invalid_payload(payload):
            status, expected = "invalid", 4
        else:
            return _protocol(result, f"verify-refinement payload missing outcome (exit {exit_code})")
    elif command == "explain-contract":
        if exit_code == 0 and (
            payload.get("schema") == "concir-contract-explanation/1" or "properties" in payload
        ):
            status, expected = "pass", 0
        elif exit_code == 4 and _is_invalid_payload(payload):
            status, expected = "invalid", 4
        else:
            return _protocol(result, f"explain-contract payload unrecognized for exit {exit_code}")
    elif command == "validate-contract":
        if payload.get("schema") != "concir-contract-validation/1" or not isinstance(payload.get("ok"), bool):
            return _protocol(result, f"validate-contract payload unrecognized for exit {exit_code}")
        status = "pass" if payload["ok"] else "fail"
        expected = VALIDATE_CONTRACT_EXIT[status]
    elif command == "export-cvn":
        if exit_code == 0 and any(
            k in payload for k in ("schema", "net", "cvn", "places", "transitions", "initial_state", "degradation")
        ):
            status, expected = "pass", 0
        elif exit_code == 4 and _is_invalid_payload(payload):
            status, expected = "invalid", 4
        elif exit_code == 5:
            status, expected = "unsupported", 5
        else:
            return _protocol(result, f"export-cvn payload unrecognized for exit {exit_code}")
    elif command == "evaluate-patch":
        if payload.get("schema_version") != EXTERNAL_ARTIFACT_SCHEMA:
            return _protocol(result, f"unexpected artifact schema {payload.get('schema_version')!r}")
        status = payload.get("status")
        if status not in ("accepted", "rejected"):
            return _protocol(result, f"evaluate-patch has unknown status {status!r}")
        expected = 0 if status == "accepted" else _eval_reject_exit(payload.get("reject_reason"))
        if exit_code != expected:
            return _protocol(result, f"evaluate-patch status {status!r} implies exit {expected}, got {exit_code}")
        result.kind, result.status = "semantic", status
        return result
    else:  # pragma: no cover - internal invariant
        return _protocol(result, f"unknown command {command!r}")

    if exit_code != expected:
        return _protocol(
            result,
            f"{command} outcome {status!r} implies exit {expected}, got {exit_code}",
        )
    result.kind, result.status = "semantic", status
    return result


def validate_replay_payload(payload: dict, artifact_path: Path | None) -> str | None:
    """Check a replay payload is a real ReplayResult consistent with the artifact."""
    for name in ("nodes", "chain_len", "accepted_ok", "input_outcome", "outcome"):
        if name not in payload:
            return f"replay payload missing field {name!r}"
    for name in _REPLAY_INTS:
        if type(payload[name]) is not int:
            return f"replay field {name!r} must be an integer, got {type(payload[name]).__name__}"
    if type(payload["accepted_ok"]) is not bool and payload["accepted_ok"] is not None:
        return "replay field 'accepted_ok' must be a boolean or null"
    if "accepted_node" not in payload:
        return "replay payload missing field 'accepted_node'"
    if type(payload["accepted_node"]) is not int and payload["accepted_node"] is not None:
        return "replay field 'accepted_node' must be an integer or null"
    if not isinstance(payload["input_outcome"], str):
        return "replay field 'input_outcome' must be a string"
    if payload["outcome"] not in REPAIR_EXIT:
        return f"replay payload has unknown outcome {payload['outcome']!r}"
    if payload["nodes"] < 0 or payload["chain_len"] < 0:
        return "replay payload has negative counts"

    if artifact_path is None or not Path(artifact_path).exists():
        return "cannot read the replayed artifact for correspondence checking"
    try:
        artifact = json.loads(Path(artifact_path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return f"replayed artifact is not readable JSON: {exc}"
    if not isinstance(artifact, dict):
        return "replayed artifact is not a JSON object"

    schema = artifact.get("schema_version")
    if schema == EXTERNAL_ARTIFACT_SCHEMA:
        return _validate_external_replay(payload, artifact)
    if schema == SEARCH_ARTIFACT_SCHEMA:
        return _validate_search_replay(payload, artifact)
    return f"replayed artifact has unknown schema {schema!r}"


def _validate_search_replay(payload: dict, artifact: dict) -> str | None:
    nodes = artifact.get("nodes")
    if not isinstance(nodes, list):
        return "search artifact has no nodes list"
    if payload["nodes"] != len(nodes):
        return f"replay nodes {payload['nodes']} != artifact nodes {len(nodes)}"
    if payload["outcome"] != artifact.get("outcome"):
        return f"replay outcome {payload['outcome']!r} != artifact outcome {artifact.get('outcome')!r}"
    chain = artifact.get("patch_chain")
    if isinstance(chain, list) and payload["chain_len"] != len(chain):
        return f"replay chain_len {payload['chain_len']} != artifact patch_chain length {len(chain)}"
    if nodes:
        root_outcome = ((nodes[0] or {}).get("report") or {}).get("outcome")
        if root_outcome is not None and payload["input_outcome"] != root_outcome:
            return f"replay input_outcome {payload['input_outcome']!r} != artifact root outcome {root_outcome!r}"
    if artifact.get("outcome") == "repaired":
        if payload["accepted_ok"] is not True or payload["accepted_node"] is None:
            return "repaired search artifact replay must report an accepted node"
        if payload["accepted_node"] != artifact.get("accepted_node"):
            return "replay accepted_node does not match the artifact"
    else:
        if payload["accepted_ok"] is not None or payload["accepted_node"] is not None:
            return "non-repaired search artifact replay must report no accepted node"
    return None


def _validate_external_replay(payload: dict, artifact: dict) -> str | None:
    accepted = artifact.get("accepted")
    if not isinstance(accepted, bool):
        return "external artifact has no boolean 'accepted'"
    verification = artifact.get("verification")
    if not isinstance(verification, dict):
        return "external artifact has no verification report"
    expected_ok = accepted and verification.get("outcome") == "PASS"
    if payload["nodes"] != 1 or payload["chain_len"] != 1:
        return "external artifact replay must report exactly one node and one edit"
    if payload["accepted_ok"] != expected_ok:
        return f"replay accepted_ok {payload['accepted_ok']!r} != artifact accepted {expected_ok!r}"
    if expected_ok:
        if payload["accepted_node"] != 0:
            return "accepted external artifact replay must report accepted_node 0"
    elif payload["accepted_node"] is not None:
        return "non-accepted external artifact replay must report no accepted node"
    root = artifact.get("root_report") or {}
    if root.get("outcome") is not None and payload["input_outcome"] != root.get("outcome"):
        return f"replay input_outcome {payload['input_outcome']!r} != artifact root outcome {root.get('outcome')!r}"
    expected_outcome = _EXTERNAL_OUTCOME_MAP.get(str(verification.get("outcome")))
    if payload["outcome"] != expected_outcome:
        return (
            f"replay outcome {payload['outcome']!r} != verification outcome "
            f"{verification.get('outcome')!r} mapped to {expected_outcome!r}"
        )
    return None
