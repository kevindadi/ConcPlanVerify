"""Typed subprocess client for the current ConcIR ``concir-backend`` CLI.

The client owns only the process boundary and the wire protocol: it writes the
inputs, invokes the CLI with an argument array, captures stdout/stderr/exit, and
maps the documented protocol onto typed results. It never re-implements CIR
semantics, Petri translation, verification, candidate enumeration, patch
legality or repair acceptance.

Command protocols (see ConcIR ``doc/backend-usage.md`` and
``src/bin/concir-backend.rs``):

- ``check   <program.json>``                          -> ValidationReport
- ``support <program.json>``                          -> supportability report
- ``explore <program.json> <contract.json> [engine]`` -> VerificationReport
- ``repair  <program.json> <contract.json> --strategy a|b|c ... --artifact f``
                                                       -> SearchArtifact (replayable)
- ``replay  <artifact.json>``                          -> ReplayResult

The positional ``repair <model> <contract> <patches.json>`` form emits a legacy
patch report, **not** a replayable search artifact; it is deliberately not
wrapped here.
"""

from __future__ import annotations

import hashlib
import json
import os
import signal
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# Documented exit-code maps per command.
CHECK_EXIT = {"valid": 0, "invalid": 4}
SUPPORT_EXIT = {"supported": 0, "unsupported": 5, "invalid": 4}
EXPLORE_EXIT = {"pass": 0, "fail": 1, "unknown": 3, "invalid": 4, "unsupported": 5}
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


def sha256_file(path: Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


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

    ``kind`` is the failure class from the *client's* point of view:

    - ``semantic``      — the backend ran and produced its documented outcome,
      including legitimate non-success outcomes (exit 1/3/4/5);
    - ``process_error`` — spawn failure, timeout, or missing input file;
    - ``protocol_error``— non-JSON/empty output, unexpected payload shape, or a
      payload whose outcome contradicts the exit code;
    - ``usage_error``   — exit 2 (argument/input error).

    ``exit_code == 0`` does not by itself mean the verified property holds; use
    ``outcome``/``status``.
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

    # ---- convenience accessors (no semantics beyond the wire protocol) ----
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


class ConcirClient:
    """Invoke ``concir-backend`` from Python with a fresh directory per call."""

    def __init__(
        self,
        binary: Path | str,
        *,
        workdir: Path | str,
        timeout: float = 30.0,
        env: dict[str, str] | None = None,
        binary_sha256: str | None = None,
    ) -> None:
        self.binary = Path(binary).expanduser().resolve()
        self.workdir = Path(workdir).expanduser().resolve()
        self.timeout = float(timeout)
        self.env = dict(os.environ if env is None else env)
        self._counter = 0
        self._binary_sha256 = binary_sha256

    # ---- binary identity -------------------------------------------------
    @property
    def binary_sha256(self) -> str:
        if self._binary_sha256 is None:
            if not self.binary.is_file():
                raise FileNotFoundError(
                    f"concir-backend binary not found: {self.binary}. "
                    "Build it in the ConcIR repo (cargo build --release --bin concir-backend) "
                    "or pass --binary / CONCIR_BACKEND."
                )
            self._binary_sha256 = sha256_file(self.binary)
        return self._binary_sha256

    def _require_binary(self) -> None:
        if not self.binary.is_file():
            raise FileNotFoundError(
                f"concir-backend binary not found: {self.binary}. "
                "Build it in the ConcIR repo or pass --binary / CONCIR_BACKEND."
            )
        if not os.access(self.binary, os.X_OK):
            raise PermissionError(f"concir-backend is not executable: {self.binary}")

    # ---- inputs ----------------------------------------------------------
    def _call_dir(self, command: str) -> Path:
        self._counter += 1
        for _ in range(1000):
            token = f"{time.time_ns():x}-{os.getpid()}-{self._counter:04d}-{os.urandom(3).hex()}"
            path = self.workdir / f"{token}-{command}"
            try:
                path.mkdir(parents=True, exist_ok=False)
                return path
            except FileExistsError:
                continue
        raise RuntimeError("could not allocate a unique call directory")

    @staticmethod
    def _materialize(value: Path | str, dest: Path) -> Path:
        if isinstance(value, Path) or (isinstance(value, str) and os.path.exists(value) and "\n" not in value):
            source = Path(value)
            dest.write_bytes(source.read_bytes())
            return dest
        # Treat as raw text.
        dest.write_text(str(value), encoding="utf-8")
        return dest

    def _invoke(
        self,
        command: str,
        argv: list[str],
        *,
        call_dir: Path,
        input_path: Path | None,
        contract_path: Path | None,
        config: dict[str, Any] | None = None,
        engine: str | None = None,
    ) -> ConcirResult:
        self._require_binary()
        identity = ConcirIdentity(
            binary=str(self.binary),
            binary_sha256=self.binary_sha256,
            argv=[str(self.binary), *argv],
            cwd=str(call_dir),
            input_sha256=sha256_file(input_path) if input_path else None,
            contract_sha256=sha256_file(contract_path) if contract_path else None,
            config=dict(config or {}),
            engine=engine,
        )
        started = time.monotonic()
        timed_out = False
        spawn_error: str | None = None
        try:
            proc = subprocess.Popen(
                [str(self.binary), *argv],
                cwd=str(call_dir),
                env=self.env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except OSError as exc:
            return ConcirResult(
                command=command, argv=identity.argv, exit_code=None, status="process_error",
                kind="process_error", payload=None, identity=identity, error=str(exc),
                wall_ms=int((time.monotonic() - started) * 1000), call_dir=str(call_dir),
                input_path=str(input_path) if input_path else None,
                contract_path=str(contract_path) if contract_path else None,
            )
        try:
            out, err = proc.communicate(timeout=self.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except ProcessLookupError:
                pass
            out, err = proc.communicate()
        wall_ms = int((time.monotonic() - started) * 1000)

        stdout = out.decode("utf-8", "replace")
        stderr = err.decode("utf-8", "replace")
        stdout_path = call_dir / "stdout.json"
        stderr_path = call_dir / "stderr.txt"
        exit_path = call_dir / "exit.txt"
        stdout_path.write_text(stdout, encoding="utf-8")
        stderr_path.write_text(stderr, encoding="utf-8")
        exit_path.write_text(str(proc.returncode), encoding="utf-8")

        result = ConcirResult(
            command=command,
            argv=identity.argv,
            exit_code=proc.returncode,
            status="unknown",
            kind="semantic",
            payload=None,
            identity=identity,
            stderr=stderr.strip(),
            wall_ms=wall_ms,
            timed_out=timed_out,
            call_dir=str(call_dir),
            stdout_path=str(stdout_path),
            stderr_path=str(stderr_path),
            exit_path=str(exit_path),
            input_path=str(input_path) if input_path else None,
            contract_path=str(contract_path) if contract_path else None,
        )
        if input_path:
            result.payload = None
        if timed_out:
            result.kind, result.status = "process_error", "timeout"
            result.error = f"concir-backend timed out after {self.timeout:g}s"
            return result
        if proc.returncode == 2:
            result.kind, result.status = "usage_error", "usage_error"
            result.error = stderr.strip() or "usage error"
            return result
        if command == "replay":
            # `replay` prints a structured ReplayResult only on success; a
            # non-zero exit is the documented replay failure (stderr
            # "artifact replay failed: ..."). Exit 0 alone is not enough: the
            # payload must be a valid ReplayResult that corresponds to the
            # artifact we handed the tool.
            if proc.returncode != 0:
                result.kind, result.status = "semantic", "replay_failed"
                result.error = stderr.strip() or "artifact replay failed"
                return result
            text = stdout.strip()
            if not text:
                result.kind, result.status = "protocol_error", "empty_replay_output"
                result.error = "replay exited 0 but produced no stdout"
                return result
            try:
                payload = json.loads(text)
            except json.JSONDecodeError as exc:
                result.kind, result.status = "protocol_error", "non_json_replay_output"
                result.error = f"replay stdout is not JSON: {exc}"
                return result
            if not isinstance(payload, dict):
                result.kind, result.status = "protocol_error", "replay_payload_not_object"
                result.error = "replay stdout JSON is not an object"
                return result
            problem = _validate_replay_payload(payload, input_path)
            if problem is not None:
                result.kind, result.status = "protocol_error", "invalid_replay_payload"
                result.error = problem
                return result
            result.payload = payload
            result.kind, result.status = "semantic", "replayed"
            return result
        text = stdout.strip()
        if not text:
            result.kind, result.status = "protocol_error", "empty_output"
            result.error = "concir-backend produced no stdout"
            return result
        try:
            payload = json.loads(text)
        except json.JSONDecodeError as exc:
            result.kind, result.status = "protocol_error", "non_json_output"
            result.error = f"stdout is not JSON: {exc}"
            return result
        if not isinstance(payload, dict):
            result.kind, result.status = "protocol_error", "payload_not_object"
            result.error = "stdout JSON is not an object"
            return result
        result.payload = payload
        return _classify(result)

    # ---- commands --------------------------------------------------------
    def check(self, program: Path | str):
        call_dir = self._call_dir("check")
        program_path = self._materialize(program, call_dir / "program.json")
        return self._invoke("check", ["check", str(program_path)], call_dir=call_dir,
                            input_path=program_path, contract_path=None)

    def support(self, program: Path | str):
        call_dir = self._call_dir("support")
        program_path = self._materialize(program, call_dir / "program.json")
        return self._invoke("support", ["support", str(program_path)], call_dir=call_dir,
                            input_path=program_path, contract_path=None)

    def explore(self, program: Path | str, contract: Path | str, engine: str = "petri"):
        if engine not in {"petri", "interp"}:
            raise ValueError(f"engine must be 'petri' or 'interp', got {engine!r}")
        call_dir = self._call_dir("explore")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        return self._invoke(
            "explore", ["explore", str(program_path), str(contract_path), engine],
            call_dir=call_dir, input_path=program_path, contract_path=contract_path, engine=engine,
        )

    def repair(
        self,
        program: Path | str,
        contract: Path | str,
        *,
        strategy: str = "c",
        candidate_budget: int = 64,
        verification_budget: int = 64,
        max_depth: int = 4,
        max_total_edits: int = 4,
        artifact_name: str = "artifact.json",
    ):
        if strategy not in {"a", "b", "c"}:
            raise ValueError(f"strategy must be 'a'/'b'/'c', got {strategy!r}")
        call_dir = self._call_dir("repair")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        artifact_path = call_dir / artifact_name
        config = {
            "strategy": strategy,
            "candidate_budget": candidate_budget,
            "verification_budget": verification_budget,
            "max_depth": max_depth,
            "max_total_edits": max_total_edits,
        }
        result = self._invoke(
            "repair",
            [
                "repair", str(program_path), str(contract_path),
                "--strategy", strategy,
                "--candidate-budget", str(candidate_budget),
                "--verification-budget", str(verification_budget),
                "--max-depth", str(max_depth),
                "--max-total-edits", str(max_total_edits),
                "--artifact", str(artifact_path),
            ],
            call_dir=call_dir, input_path=program_path, contract_path=contract_path,
            config=config,
        )
        result.artifact_path = str(artifact_path) if artifact_path.exists() else None
        return result

    def repair_context(self, program: Path | str, contract: Path | str):
        call_dir = self._call_dir("repair-context")
        program_path = self._materialize(program, call_dir / "program.json")
        contract_path = self._materialize(contract, call_dir / "contract.json")
        return self._invoke(
            "repair-context",
            ["repair-context", str(program_path), str(contract_path),
             "--artifact", str(call_dir / "context.json")],
            call_dir=call_dir, input_path=program_path, contract_path=contract_path,
        )

    def evaluate_patch(self, context: Path | str, candidate: Path | str):
        call_dir = self._call_dir("evaluate-patch")
        context_path = self._materialize(context, call_dir / "context.json")
        candidate_path = self._materialize(candidate, call_dir / "candidate.json")
        result = self._invoke(
            "evaluate-patch",
            ["evaluate-patch", str(context_path), str(candidate_path),
             "--artifact", str(call_dir / "external_artifact.json")],
            call_dir=call_dir, input_path=candidate_path, contract_path=None,
        )
        artifact = call_dir / "external_artifact.json"
        result.artifact_path = str(artifact) if artifact.exists() else None
        return result

    def replay(self, artifact: Path | str):
        call_dir = self._call_dir("replay")
        if isinstance(artifact, Path):
            artifact_path = Path(artifact).resolve()
        elif os.path.exists(str(artifact)) and "\n" not in str(artifact):
            artifact_path = Path(artifact).resolve()
        else:
            artifact_path = call_dir / "artifact.json"
            artifact_path.write_text(str(artifact), encoding="utf-8")
        return self._invoke("replay", ["replay", str(artifact_path)], call_dir=call_dir,
                            input_path=artifact_path, contract_path=None)


def _classify(result: ConcirResult) -> ConcirResult:
    """Map a parsed payload + exit code to the documented semantic outcome."""
    payload = result.payload or {}
    command = result.command
    exit_code = result.exit_code

    if command == "check":
        valid = payload.get("valid")
        status = "valid" if valid else "invalid"
        expected = CHECK_EXIT[status]
        if not isinstance(valid, bool):
            return _protocol(result, "check payload has no boolean 'valid'")
    elif command == "support":
        supported = payload.get("supported")
        status = "supported" if supported else "unsupported"
        expected = SUPPORT_EXIT[status]
        if not isinstance(supported, bool):
            return _protocol(result, "support payload has no boolean 'supported'")
    elif command == "explore":
        outcome = payload.get("outcome")
        status = str(outcome).lower() if outcome is not None else ""
        if status not in EXPLORE_EXIT:
            return _protocol(result, f"explore payload has unknown outcome {outcome!r}")
        expected = EXPLORE_EXIT[status]
    elif command == "repair":
        outcome = payload.get("outcome")
        status = str(outcome) if outcome is not None else ""
        if status not in REPAIR_EXIT:
            return _protocol(result, f"repair payload has unknown outcome {outcome!r}")
        expected = REPAIR_EXIT[status]
    elif command == "replay":
        # Replay has no outcome field in the result; exit 0 means success.
        if exit_code == 0:
            result.kind, result.status = "semantic", "replayed"
            return result
        result.kind, result.status = "semantic", "replay_failed"
        result.error = result.stderr or "artifact replay failed"
        return result
    elif command == "repair-context":
        if payload.get("schema_version") != "concir-repair-context-v1":
            return _protocol(result, f"unexpected context schema {payload.get('schema_version')!r}")
        if exit_code != 0:
            return _protocol(result, f"repair-context exited {exit_code}")
        result.kind, result.status = "semantic", "context"
        return result
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


# Required ReplayResult fields. Integers must be real ints (not bool); Option
# fields may be null.
_REPLAY_INTS = ("nodes", "chain_len")
_REPLAY_OPTIONAL_INTS = ("accepted_node",)
SEARCH_ARTIFACT_SCHEMA = "concir-repair-artifact-v1"
EXTERNAL_ARTIFACT_SCHEMA = "concir-external-patch-artifact-v1"
_EXTERNAL_OUTCOME_MAP = {
    "PASS": "repaired",
    "FAIL": "no_acceptable_candidate",
    "UNKNOWN": "analysis_unknown",
    "INVALID": "invalid",
    "UNSUPPORTED": "unsupported",
}


def _validate_replay_payload(payload: dict, artifact_path: Path | None) -> str | None:
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
        return (f"replay outcome {payload['outcome']!r} != artifact outcome "
                f"{artifact.get('outcome')!r}")
    chain = artifact.get("patch_chain")
    if isinstance(chain, list) and payload["chain_len"] != len(chain):
        return (f"replay chain_len {payload['chain_len']} != artifact patch_chain "
                f"length {len(chain)}")
    if nodes:
        root_outcome = ((nodes[0] or {}).get("report") or {}).get("outcome")
        if root_outcome is not None and payload["input_outcome"] != root_outcome:
            return (f"replay input_outcome {payload['input_outcome']!r} != artifact "
                    f"root outcome {root_outcome!r}")
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
        return (f"replay accepted_ok {payload['accepted_ok']!r} != artifact accepted "
                f"{expected_ok!r}")
    if expected_ok:
        if payload["accepted_node"] != 0:
            return "accepted external artifact replay must report accepted_node 0"
    elif payload["accepted_node"] is not None:
        return "non-accepted external artifact replay must report no accepted node"
    root = artifact.get("root_report") or {}
    if root.get("outcome") is not None and payload["input_outcome"] != root.get("outcome"):
        return (f"replay input_outcome {payload['input_outcome']!r} != artifact root "
                f"outcome {root.get('outcome')!r}")
    expected_outcome = _EXTERNAL_OUTCOME_MAP.get(str(verification.get("outcome")))
    if payload["outcome"] != expected_outcome:
        return (f"replay outcome {payload['outcome']!r} != verification outcome "
                f"{verification.get('outcome')!r} mapped to {expected_outcome!r}")
    return None
