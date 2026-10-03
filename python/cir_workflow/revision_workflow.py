"""Whole-artifact CIR revision loop (arm ``A3_ours_revision``).

Unlike :mod:`offline_workflow` (which freezes the first statically valid CIR and
then only allows the backend's deterministic patch search), this workflow lets
the provider rewrite the **whole** modular-CIR artifact between rounds. It is the
only place a provider may replace the program after freezing, and it is labelled
``repair_mode=llm_revision`` so it is never confused with the backend's
deterministic repair or with ``external_single_patch``.

Acceptance is still decided by the Rust backend: a complete ``PASS`` of the
frozen contract. Fidelity (the task-level structural check) is reported
separately and never changes acceptance. Every version is archived and hashed.

Ablation switches (protocol B6):

- ``diagnostics=False``  -> only ``{"outcome", "complete"}`` feedback is sent
  (``A3_nodiag``);
- ``contract`` without ``preserved`` -> acceptance only checks ``deadlock_free``
  (``A3_nopreserved``);
- ``check_fidelity=False`` -> no fidelity record (``A3_nofidelity``).
"""

from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient, sha256_file
from .experiments_v2 import Consumption, sha256_text
from .json_utils import extract_json
from .models import normalize_token_usage
from .normalize import normalize as normalize_program
from .prompts import build_explore_feedback, render_feedback
from .providers import CandidateProvider, CandidateRequest
from .structural import run_structural_check

SUCCESS_STATUSES = {"accepted"}


@dataclass
class RevisionVersion:
    version: int
    source: str
    raw_text: str | None = None
    program_sha256: str | None = None
    artifact_path: str | None = None
    parse_error: str | None = None
    provider_error: str | None = None
    check_status: str | None = None
    support_status: str | None = None
    explore_outcome: str | None = None
    explore_complete: bool | None = None
    fidelity: dict[str, Any] | None = None
    accepted: bool = False
    decision: str | None = None
    # measured per-round consumption / evidence
    request_sha256: str | None = None
    feedback_sha256: str | None = None
    prompt_tokens: int | None = None
    completion_tokens: int | None = None
    llm_wall_ms: int = 0
    tool_wall_ms: int = 0
    tool_output_sha256: str | None = None
    normalizations: list[dict[str, Any]] = field(default_factory=list)
    sid_issues: list[dict[str, Any]] = field(default_factory=list)
    local_patch_function: str | None = None


@dataclass
class RevisionResult:
    status: str
    task: str
    requirements: str
    contract_sha256: str
    out_dir: str
    max_rounds: int
    engine: str
    diagnostics: bool
    check_fidelity: bool
    fidelity_name: str | None = None
    versions: list[RevisionVersion] = field(default_factory=list)
    accepted_version: int | None = None
    repair_mode: str = "llm_revision"
    candidate_source: str | None = None
    consumption: Consumption = field(default_factory=Consumption)
    error: str | None = None

    @property
    def accepted(self) -> bool:
        return self.status in SUCCESS_STATUSES

    def as_dict(self) -> dict[str, Any]:
        payload = dict(self.__dict__)
        payload["versions"] = [v.__dict__ for v in self.versions]
        payload["consumption"] = self.consumption.as_dict()
        payload["accepted"] = self.accepted
        return payload


def _exclusive_run_dir(base: Path) -> Path:
    import os

    base.mkdir(parents=True, exist_ok=True)
    for _ in range(1000):
        token = f"{time.strftime('%Y%m%dT%H%M%S', time.gmtime())}-{os.getpid()}-{os.urandom(3).hex()}"
        path = base / f"run-{token}"
        try:
            path.mkdir(parents=True, exist_ok=False)
            return path
        except FileExistsError:
            continue
    raise RuntimeError(f"could not allocate a unique run directory under {base}")


class WholeArtifactRevisionWorkflow:
    def __init__(
        self,
        client: ConcirClient,
        provider: CandidateProvider,
        *,
        out_dir: Path | str,
        max_rounds: int = 4,
        engine: str = "petri",
        diagnostics: bool = True,
        check_fidelity: bool = True,
        fidelity_name: str | None = None,
        reply_format: str = "whole",
        tolerate_invalid: bool = False,
    ) -> None:
        self.client = client
        self.provider = provider
        self.out_dir = Path(out_dir).expanduser().resolve()
        self.max_rounds = max(1, max_rounds)
        self.engine = engine
        self.diagnostics = diagnostics
        self.check_fidelity = check_fidelity
        self.fidelity_name = fidelity_name
        self.reply_format = reply_format
        # Generation starts from nothing, so a first-pass invalid/unknown CIR is
        # fed back as feedback instead of ending the loop (repair keeps the
        # default terminal behaviour).
        self.tolerate_invalid = tolerate_invalid

    def run(self, requirements: str, contract: dict[str, Any], *,
            task_id: str = "task", initial_program: Path | str | None = None) -> RevisionResult:
        run_dir = _exclusive_run_dir(self.out_dir)
        contract_path = run_dir / "contract.json"
        contract_path.write_text(json.dumps(contract, ensure_ascii=False, indent=2) + "\n",
                                 encoding="utf-8")
        result = RevisionResult(
            status="generation_failed", task=task_id, requirements=requirements,
            contract_sha256=sha256_file(contract_path), out_dir=str(run_dir),
            max_rounds=self.max_rounds, engine=self.engine,
            diagnostics=self.diagnostics, check_fidelity=self.check_fidelity,
            fidelity_name=self.fidelity_name,
        )

        feedback_dict: dict[str, Any] | None = None
        previous_text: str | None = None
        next_program: Path | None = None
        last_norm_sha: str | None = None
        last_normalized: dict[str, Any] | None = None
        stall_count = 0
        local_patch_function: str | None = None
        if initial_program is not None:
            src = Path(initial_program).expanduser().resolve()
            next_program = run_dir / "revision-0.cir.json"
            next_program.write_bytes(src.read_bytes())

        for version in range(1, self.max_rounds + 1):
            record = RevisionVersion(version=version, source="llm")
            result.versions.append(record)

            if next_program is not None:
                record.source = "frozen"
                parsed = json.loads(next_program.read_text(encoding="utf-8"))
                parsed, norm_records, sid_issues = normalize_program(parsed)
                record.normalizations = norm_records
                record.sid_issues = sid_issues
                program_path = run_dir / f"revision-{version}.normalized.cir.json"
                program_path.write_text(
                    json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8")
                next_program = None
            else:
                request = CandidateRequest(
                    requirements=requirements, contract=contract,
                    feedback=render_feedback(feedback_dict) if feedback_dict else None,
                    attempt=version, previous_candidate=previous_text,
                    current_program=(json.dumps(last_normalized, ensure_ascii=False)
                                     if last_normalized is not None else None),
                )
                response = self.provider.propose(request)
                result.candidate_source = response.source
                self._count_usage(result, response)
                record.request_sha256 = sha256_text(json.dumps({
                    "requirements": requirements,
                    "contract_sha256": result.contract_sha256,
                    "attempt": version,
                    "feedback": feedback_dict,
                    "previous_sha256": (sha256_text(previous_text)
                                        if previous_text else None),
                }, sort_keys=True, default=str))
                record.llm_wall_ms = response.wall_ms
                record.prompt_tokens, record.completion_tokens = _tokens(response)
                if response.error:
                    record.provider_error = response.error
                    record.decision = "provider_error"
                    result.status = "model_error"
                    result.error = response.error
                    _write_result(run_dir, result)
                    return result
                record.raw_text = response.text
                previous_text = response.text
                extracted = extract_json(response.text)
                if self.reply_format == "local":
                    reply = _parse_local_reply(extracted)
                    if reply is None:
                        record.parse_error = "not a {functions,...} object"
                        record.decision = "parse_error"
                        feedback_dict = {"stage": "parse",
                                         "error": "reply must be a JSON object with "
                                                  "'functions' and 'new_resources'"}
                        record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                        continue
                    if last_normalized is None:
                        result.status = "tool_error"
                        result.error = "local revision requires an initial program"
                        _write_result(run_dir, result)
                        return result
                    merged, merge_errors = _merge_local(last_normalized, reply)
                    if merge_errors:
                        record.decision = "merge_error"
                        feedback_dict = {"stage": "merge", "errors": merge_errors}
                        record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                        continue
                    parsed = merged
                else:
                    try:
                        parsed = json.loads(extracted)
                    except json.JSONDecodeError as exc:
                        record.parse_error = f"invalid JSON: {exc}"
                        record.decision = "parse_error"
                        feedback_dict = {"stage": "parse", "error": f"invalid JSON: {exc}"}
                        record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                        continue
                    if not isinstance(parsed, dict):
                        record.parse_error = "candidate is not a JSON object"
                        record.decision = "parse_error"
                        feedback_dict = {"stage": "parse", "error": "candidate is not a JSON object"}
                        record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                        continue
                # Local-patch reply: {function, body:[...]} merged into the last
                # accepted program instead of a whole-program replacement.
                if (local_patch_function and "modules" not in parsed
                        and isinstance(parsed.get("body"), list)
                        and last_normalized is not None):
                    merged = json.loads(json.dumps(last_normalized))
                    for module in merged.get("modules", []):
                        for function in module.get("functions", []):
                            if function.get("name") == local_patch_function:
                                function["body"] = parsed["body"]
                    parsed = merged
                    record.decision = "local_patch"

                # Tier-1 deterministic normalisation (recorded, sid-safe).
                parsed, norm_records, sid_issues = normalize_program(parsed)
                record.normalizations = norm_records
                record.sid_issues = sid_issues
                program_path = run_dir / f"revision-{version}.normalized.cir.json"
                program_path.write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                                        encoding="utf-8")
                (run_dir / f"revision-{version}.cir.json").write_text(
                    json.dumps(parsed, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

            record.program_sha256 = sha256_file(program_path)
            record.artifact_path = str(program_path)

            if record.sid_issues:
                record.decision = "sid_invalid"
                feedback_dict = {
                    "stage": "schema",
                    "sid_issues": record.sid_issues,
                    "expected": "every statement sid must match ^s[0-9]+$ (s1, s2, ...)",
                }
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue

            # Stall detection: identical normalised program -> local-patch, then stop.
            if last_norm_sha is not None and record.program_sha256 == last_norm_sha:
                stall_count += 1
                if stall_count >= 2:
                    record.decision = "stalled"
                    result.status = "stalled"
                    result.error = "candidate unchanged twice; stopping"
                    _write_result(run_dir, result)
                    return result
                local_patch_function = record.local_patch_function
                feedback_dict = {
                    "stage": "stalled",
                    "message": "the program is byte-identical to the previous attempt; "
                               "change only the offending function body",
                    "function": local_patch_function,
                    "expected": "reply with {\"function\": name, \"body\": [ ...statements... ]}",
                }
                record.decision = "stalled_local_patch"
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue
            stall_count = 0
            last_norm_sha = record.program_sha256
            last_normalized = parsed

            check = self.client.check(program_path)
            record.check_status = check.status
            record.tool_wall_ms += check.wall_ms
            result.consumption.tool_wall_ms += check.wall_ms
            record.tool_output_sha256 = _payload_hash("check", check.status, check.payload)
            if check.kind != "semantic":
                # A malformed candidate (usage/protocol/parse) is a schema error
                # the LLM can fix: feed it back and spend the round instead of
                # stopping the batch. Only a missing/broken binary is a tool error.
                if check.kind in ("usage_error", "protocol_error"):
                    record.decision = "check_schema_error"
                    feedback_dict = {
                        "stage": "check",
                        "schema_error": check.error or check.stderr,
                        "detail": "the program JSON did not parse or had an invalid shape; "
                                  "fix the schema (e.g. `expr` must be a string) and "
                                  "resend the whole program",
                    }
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                result.status = "tool_error"
                result.error = check.error or "check failed"
                _write_result(run_dir, result)
                return result
            if check.status == "invalid":
                record.decision = "check_invalid"
                record.local_patch_function = _first_function(check)
                local_patch_function = record.local_patch_function
                feedback_dict = (build_schema_feedback(check, record.normalizations, parsed)
                                 if self.diagnostics
                                 else {"stage": "check", "outcome": "INVALID"})
                record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                continue

            support = self.client.support(program_path)
            record.support_status = support.status
            record.tool_wall_ms += support.wall_ms
            result.consumption.tool_wall_ms += support.wall_ms
            if support.kind != "semantic":
                result.status = "tool_error"
                result.error = support.error or "support failed"
                _write_result(run_dir, result)
                return result
            if support.status == "unsupported":
                record.decision = "unsupported"
                result.status = "unsupported"
                if self.check_fidelity:
                    self._fidelity(program_path, record)
                _write_result(run_dir, result)
                return result

            explore = self.client.explore(program_path, contract_path, self.engine)
            record.explore_outcome = explore.outcome
            record.explore_complete = explore.complete
            record.tool_wall_ms += explore.wall_ms
            result.consumption.tool_wall_ms += explore.wall_ms
            record.tool_output_sha256 = _payload_hash("explore", explore.status,
                                                      explore.payload)
            if explore.kind != "semantic":
                result.status = "tool_error"
                result.error = explore.error or "explore failed"
                _write_result(run_dir, result)
                return result

            if self.check_fidelity:
                self._fidelity(program_path, record)

            if explore.outcome == "PASS" and explore.complete is True:
                record.accepted = True
                record.decision = "accepted"
                result.status = "accepted"
                result.accepted_version = version
                result.consumption.first_correct_round = version
                _write_result(run_dir, result)
                return result

            if explore.status == "unknown":
                if self.tolerate_invalid:
                    record.decision = "unknown"
                    feedback_dict = (build_explore_feedback(explore) if self.diagnostics
                                     else {"stage": "explore", "outcome": "UNKNOWN"})
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                record.decision = "unknown"
                result.status = "unknown"
                _write_result(run_dir, result)
                return result
            if explore.status in ("invalid", "unsupported"):
                if self.tolerate_invalid:
                    record.decision = explore.status
                    feedback_dict = (build_explore_feedback(explore) if self.diagnostics
                                     else {"stage": "explore",
                                           "outcome": explore.status.upper()})
                    record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))
                    continue
                record.decision = explore.status
                result.status = explore.status
                _write_result(run_dir, result)
                return result

            record.decision = "explore_fail"
            feedback_dict = (build_explore_feedback(explore) if self.diagnostics
                             else {"stage": "explore",
                                   "outcome": explore.outcome,
                                   "complete": explore.complete})
            record.feedback_sha256 = sha256_text(render_feedback(feedback_dict))

        result.status = "exhausted"
        result.error = f"no complete PASS within {self.max_rounds} rounds"
        _write_result(run_dir, result)
        return result

    def _fidelity(self, program_path: Path, record: RevisionVersion) -> None:
        if not self.fidelity_name:
            return
        try:
            program = json.loads(program_path.read_text(encoding="utf-8"))
            record.fidelity = run_structural_check(self.fidelity_name, program)
        except (OSError, json.JSONDecodeError) as exc:
            record.fidelity = {"ok": None, "reason": f"unreadable: {exc}"}

    @staticmethod
    def _count_usage(result: RevisionResult, response) -> None:
        result.consumption.rounds += 1
        result.consumption.http_requests += 1 if response.source == "llm" else 0
        result.consumption.llm_wall_ms += int(getattr(response, "wall_ms", 0) or 0)
        usage = response.usage
        if usage is None and response.input_tokens is not None:
            usage = {"prompt_tokens": response.input_tokens,
                     "completion_tokens": response.output_tokens or 0}
        result.consumption.add_usage(usage)
        # llm wall is recorded by the live provider's own evidence log; the
        # workflow only counts model-level calls here.


def _tokens(response) -> tuple[int | None, int | None]:
    if response.usage:
        prompt, completion = normalize_token_usage(response.usage)
        return prompt, completion
    if response.input_tokens is not None or response.output_tokens is not None:
        return response.input_tokens, response.output_tokens
    return None, None


def _payload_hash(stage: str, status: str | None, payload: Any) -> str:
    return sha256_text(json.dumps(
        {"stage": stage, "status": status, "payload": payload},
        sort_keys=True, default=str))


def _parse_local_reply(text: str) -> dict[str, Any] | None:
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return None
    if not isinstance(data, dict) or "functions" not in data:
        return None
    if not isinstance(data["functions"], dict):
        return None
    return data


def _norm_ref(module: str, ref: str) -> str:
    return ref if "::" in ref else f"{module}::{ref}"


def _merge_local(base: dict[str, Any], reply: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    """Merge a local revision into the previous program; return (program, errors)."""

    import copy

    out = copy.deepcopy(base)
    errors: list[str] = []
    known_fns = {f"{m['name']}::{f['name']}" for m in out.get("modules", [])
                 for f in m.get("functions", [])}
    new_fns: list[str] = []
    for fqn, body in (reply.get("functions") or {}).items():
        if "::" not in fqn or not isinstance(body, list):
            errors.append(f"function {fqn!r} must be module::function with a body list")
            continue
        module_name, fn_name = fqn.split("::", 1)
        module = next((m for m in out.get("modules", []) if m.get("name") == module_name),
                      None)
        if module is None:
            module = {"name": module_name,
                      "provides": {"resources": [], "functions": []},
                      "requires": {"resources": [], "functions": []},
                      "resources": [], "protection": [], "functions": []}
            out.setdefault("modules", []).append(module)
        fn = next((f for f in module.setdefault("functions", []) if f.get("name") == fn_name),
                  None)
        if fn is None:
            fn = {"name": fn_name, "kind": "normal", "form": "closure", "body": []}
            module["functions"].append(fn)
            module.setdefault("provides", {}).setdefault("functions", []).append(fn_name)
            new_fns.append(fqn)
        fn["body"] = body
    # new resources go to the entry module
    entry_mod = str(out.get("entry", "main::main")).split("::")[0]
    module = next((m for m in out.get("modules", []) if m.get("name") == entry_mod),
                  out.get("modules", [{}])[0])
    for res in reply.get("new_resources") or []:
        name = res.get("name") if isinstance(res, dict) else None
        if not name:
            errors.append("new resource entry without a name")
            continue
        if any(r.get("name") == name for r in module.get("resources", [])):
            continue
        module.setdefault("resources", []).append(res)
        module.setdefault("provides", {}).setdefault("resources", []).append(name)
    for fqn in reply.get("removed_resources") or []:
        if not isinstance(fqn, str) or "::" not in fqn:
            continue
        mod_name, res_name = fqn.split("::", 1)
        m = next((x for x in out.get("modules", []) if x.get("name") == mod_name), None)
        if m:
            m["resources"] = [r for r in m.get("resources", []) if r.get("name") != res_name]
    # every newly added function must be reachable
    referenced: set[str] = set()
    for m in out.get("modules", []):
        for f in m.get("functions", []):
            for stmt in f.get("body", []) or []:
                for ref in stmt.get("funcs", []) or []:
                    referenced.add(_norm_ref(m["name"], str(ref)))
                if stmt.get("kind") in ("spawn", "call", "async_call") and stmt.get("func"):
                    referenced.add(_norm_ref(m["name"], str(stmt["func"])))
    entry = str(out.get("entry"))
    for fqn in new_fns:
        if fqn not in referenced and fqn != entry:
            errors.append(f"new function {fqn} is not reachable from any scope/spawn/call")
    return out, errors


SHAPE_HINTS = {
    "E001": "a Var/Atomic resource requires both `base` (e.g. \"Bool\"/\"Int\") and "
            "`init`; a Channel requires `base` and `capacity`",
    "E005": "every statement `sid` must match ^s[0-9]+$ (s1, s2, ...)",
    "E008": "sync resource `type` must be one of Mutex, Condvar, Semaphore, Channel",
    "E205": "atomic_cas `dst` receives the old value and must have the Atomic `base` "
            "type, not a Bool",
    "E510": "mutex_unlock requires holding that mutex",
    "E511": "condvar_wait requires holding the paired `lock`",
    "E208": "a resource's `init` must match its declared `base` type: `base: \"Int\"` "
            "needs an integer init, `base: \"Bool\"` a boolean; example "
            "{\"name\":\"done\",\"kind\":\"var\",\"type\":\"Var\",\"base\":\"Int\",\"init\":0}",
    "E931": "every name used in an expression must be a declared param/local of the "
            "function or a shared resource; declare new locals in the function's "
            "`locals` list",
    "E102": "call only a function that appears in some module's functions list. "
            "Do not invent println, print, or io::/stdio:: helpers. Exact terminal "
            "text is implemented later in Rust, not as a CIR call.",
    "E920": "`args` must be a JSON array of strings whose length matches the callee's "
            "params. A bare string is not an argument list.",
    "E108": "a cross-module function reference must be declared and listed in requires.functions",
}


def _function_symbols(program: dict[str, Any] | None, location: str) -> dict[str, Any]:
    """Params/locals declared for the function named in a diagnostic location."""

    if not program or "::" not in location:
        return {}
    fqn = location.split(".")[0]
    for module in program.get("modules", []) or []:
        for fn in module.get("functions", []) or []:
            full = f"{module.get('name')}::{fn.get('name')}"
            if full == fqn:
                params = [p.get("name") for p in fn.get("params", []) or []]
                locals_ = [l.get("name") for l in fn.get("locals", []) or []]
                return {"function": full, "params": params, "locals": locals_}
    return {}


def _first_function(check) -> str | None:
    for diag in ((check.payload or {}).get("diagnostics") or []):
        location = str(diag.get("location") or "")
        if "::" in location:
            return location.split(".")[0]
    return None


_UNKNOWN_FIELD = re.compile(
    r"unknown field `([^`]+)`, expected (?:one of )?(.*)")
_INVALID_TYPE = re.compile(
    r"invalid type: ([^,]+), expected (.+?)(?: at |$)")
_MISSING_FIELD = re.compile(r"missing field `([^`]+)`")
_PATH = re.compile(r"(?:JSON parse error in )?'[^']+'")


def _sanitize_backend_text(text: str) -> str:
    """Drop absolute paths. Keep the serde message."""
    cleaned = _PATH.sub("'program.json'", text or "")
    cleaned = re.sub(r"/[^\s'\"]+", "<path>", cleaned)
    return cleaned.strip()


def _declared_functions(program: dict[str, Any] | None) -> list[str]:
    if not program:
        return []
    names = []
    for module in program.get("modules", []) or []:
        for fn in module.get("functions", []) or []:
            names.append(f"{module.get('name')}::{fn.get('name')}")
    return names


def parse_schema_stderr(stderr: str) -> dict[str, Any]:
    """Turn a backend schema/serde error into structured feedback.

    Serde reports a line/column, not a JSON pointer. ``json_path`` stays null
    unless the message itself contains one.
    """
    message = _sanitize_backend_text(stderr)
    entry: dict[str, Any] = {
        "stage": "check",
        "error_class": "schema_parse",
        "message": message,
        "json_path": None,
        "fix_hint": "Resend one complete CIR JSON object that matches the schema.",
    }
    unknown = _UNKNOWN_FIELD.search(message)
    if unknown:
        allowed = re.findall(r"`([^`]+)`", unknown.group(2))
        entry["actual_value"] = unknown.group(1)
        entry["allowed_fields"] = allowed
        field = unknown.group(1)
        if field == "base":
            entry["fix_hint"] = (
                "params and locals use `name`, `type`, and `modeled` "
                "(locals may also have `init`). `base` belongs on a Var, Atomic, "
                "or Channel resource, not on a parameter.")
        else:
            entry["fix_hint"] = (
                f"remove unknown field `{field}`; allowed fields: {', '.join(allowed) or 'see schema'}.")
        return entry
    invalid = _INVALID_TYPE.search(message)
    if invalid:
        entry["actual_value"] = invalid.group(1).strip()
        entry["expected_type"] = invalid.group(2).strip()
        if "sequence" in entry["expected_type"]:
            entry["fix_hint"] = (
                "`args` and `funcs` are JSON arrays of strings, for example "
                '[\"main::worker\"]. A single string is rejected.')
        else:
            entry["fix_hint"] = f"value has type {entry['actual_value']}; expected {entry['expected_type']}."
        return entry
    missing = _MISSING_FIELD.search(message)
    if missing:
        entry["actual_value"] = None
        entry["fix_hint"] = f"add required field `{missing.group(1)}`."
    return entry


def build_schema_feedback(check, normalizations: list[dict[str, Any]],
                          program: dict[str, Any] | None = None) -> dict[str, Any]:
    """Schema or static-check feedback. Never drops a serde error into empty diagnostics."""

    if getattr(check, "kind", None) in {"process_error", "protocol_error"}:
        return {
            "stage": "check",
            "error_class": "tool_error",
            "status": getattr(check, "status", None),
            "message": _sanitize_backend_text(getattr(check, "error", None) or getattr(check, "stderr", "") or ""),
            "diagnostics": [],
            "json_path": None,
            "fix_hint": "The checker failed internally. This is not a CIR fault. Resend the same program.",
            "normalizations": normalizations,
        }
    if getattr(check, "kind", None) == "usage_error":
        stderr = getattr(check, "stderr", "") or getattr(check, "error", "") or ""
        parsed = parse_schema_stderr(stderr or "schema parse error")
        parsed["normalizations"] = normalizations
        parsed["diagnostics"] = [{
            "error_class": "schema_parse",
            "message": parsed["message"],
            "json_path": None,
            "expected_type": parsed.get("expected_type"),
            "allowed_fields": parsed.get("allowed_fields"),
            "actual_value": parsed.get("actual_value"),
            "fix_hint": parsed["fix_hint"],
        }]
        return parsed

    payload = check.payload or {}
    diagnostics = []
    hints: list[str] = []
    declared = _declared_functions(program)
    for diag in payload.get("diagnostics", []) or []:
        code = str(diag.get("code"))
        entry = {k: diag.get(k) for k in ("code", "severity", "message", "path", "location", "fix_hint")
                 if diag.get(k) is not None}
        entry["json_path"] = diag.get("path")
        source = ""
        if code in {"E931", "E102", "E920", "E108"}:
            symbols = _function_symbols(program, str(diag.get("location") or ""))
            if symbols:
                entry["declared_symbols"] = symbols
                source = (f" (declared in {symbols['function']}: params "
                          f"{symbols['params']}, locals {symbols['locals']})")
            if declared:
                entry["declared_functions"] = declared
        diagnostics.append(entry)
        hint = SHAPE_HINTS.get(code)
        if hint and hint + source not in hints:
            hints.append(hint + source)
        if diag.get("fix_hint") and diag["fix_hint"] not in hints:
            hints.append(str(diag["fix_hint"]))
    return {
        "stage": "check",
        "error_class": "static_check",
        "status": "INVALID",
        "diagnostics": diagnostics,
        "expected_shapes": hints,
        "declared_functions": declared,
        "normalizations": normalizations,
        "note": "The backend decides acceptance. `path` is a JSON pointer when the checker provided one. "
                "Resend the whole program.",
    }


def _write_result(run_dir: Path, result: RevisionResult) -> None:
    (run_dir / "result.json").write_text(
        json.dumps(result.as_dict(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
