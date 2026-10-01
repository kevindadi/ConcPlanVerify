"""§2 generation benchmark: requirements -> program, four arms.

- ``G0_direct``   one-shot Rust from the requirements (contract hidden)
- ``G1_self_iter`` generate -> self-review rounds (contract hidden)
- ``G2_tools_iter`` generate -> build + Miri + Lockbud feedback (contract hidden)
- ``G3_concir``    generate CIR from the requirements (contract hidden) ->
                   normalize/check -> explore vs the contract -> whole revision
                   -> codegen -> build -> conform -> behavior

The Rust arms are scored by the bounded monitor (§1); G3 by the exhaustive
model verdict. The distinction is recorded per cell.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import bounded_monitor, rust_oracle
from .arms import ARM_GEN_DIRECT, ARM_GEN_SELF, ARM_GEN_TOOLS, run_rust_arm
from .conformance import codegen, collect_traces, conform_all, holes_of

RANK = ["FAIL", "unmapped", "unsupported", "not_observed", "deferred",
        "PASS_bounded", "PASS"]


@dataclass
class GenTask:
    id: str
    family: str
    case: str
    tier: str
    requirements: list[str]
    unverifiable: list[int]
    requirements_md: str
    contract_path: Path
    reference_cir_path: Path
    directory: Path

    @property
    def contract(self) -> dict[str, Any]:
        return json.loads(self.contract_path.read_text(encoding="utf-8"))

    @property
    def reqjson(self) -> dict[str, Any]:
        return {"requirements": self.requirements, "unverifiable": self.unverifiable}


def load_gen_tasks(repo: Path, manifest_path: Path | None = None) -> list[GenTask]:
    manifest = json.loads((manifest_path or repo / "benchmarks/GENERATION_MANIFEST.json")
                          .read_text(encoding="utf-8"))
    tasks: list[GenTask] = []
    for entry in manifest["tasks"]:
        directory = repo / "benchmarks/families" / entry["task"]
        reqjson = json.loads(
            (directory / "generation_input/requirements.json").read_text(encoding="utf-8"))
        requirements_md = (directory / "generation_input/REQUIREMENTS.md").read_text(
            encoding="utf-8")
        ref = entry.get("reference_variant") or "fixed"
        reference = directory / f"{ref}.cir.json"
        if not reference.is_file():
            for name in ("fixed.cir.json", "correct.cir.json", "buggy.cir.json"):
                if (directory / name).is_file():
                    reference = directory / name
                    break
        tasks.append(GenTask(
            id=entry["task"], family=entry["family"], case=entry["case"],
            tier=entry["tier"], requirements=reqjson["requirements"],
            unverifiable=reqjson["unverifiable"], requirements_md=requirements_md,
            contract_path=directory / "contract.json",
            reference_cir_path=reference, directory=directory))
    return tasks


def _clause_statuses(contract: dict[str, Any]) -> list[tuple[str, list[str]]]:
    out: list[tuple[str, list[str]]] = []
    for clause in contract.get("properties", []):
        out.append(("properties", list(clause.get("req", []))))
    for clause in contract.get("preserved", []):
        out.append(("preserved", list(clause.get("req", []))))
    return out


def _aggregate(by_req: dict[str, list[str]], requirements: list[str],
               unverifiable: list[int]) -> dict[str, Any]:
    statuses: dict[str, str] = {}
    unv = {i for i in unverifiable}
    for i in range(1, len(requirements) + 1):
        rid = f"R{i}"
        vals = by_req.get(rid, [])
        if i in unv and not vals:
            statuses[rid] = "unverifiable"
        elif not vals:
            statuses[rid] = "unverifiable" if i in unv else "not_observed"
        elif "FAIL" in vals:
            statuses[rid] = "FAIL"
        elif all(v == "PASS" for v in vals):
            statuses[rid] = "PASS"
        elif "PASS" in vals and all(v in ("PASS", "PASS_bounded") for v in vals):
            statuses[rid] = "PASS_bounded"
        else:
            statuses[rid] = next((v for v in RANK if v in vals), "not_observed")
    decidable = {k: v for k, v in statuses.items()
                 if v not in ("unmapped", "unsupported", "deferred", "unverifiable")}
    satisfied = [k for k, v in statuses.items() if v in ("PASS", "PASS_bounded")]
    return {
        "total": len(requirements), "decidable": len(decidable),
        "satisfied": len(satisfied), "failed": [k for k, v in statuses.items()
                                                if v == "FAIL"],
        "statuses": statuses,
        "rc": len(decidable) / len(requirements) if requirements else 0.0,
        "rf": len(satisfied) / len(requirements) if requirements else 0.0,
    }


def model_coverage(contract: dict[str, Any], explore_properties: list[dict[str, Any]],
                   requirements: list[str], unverifiable: list[int]) -> dict[str, Any]:
    """Exhaustive coverage from `explore` per-property outcomes."""
    clauses = _clause_statuses(contract)
    by_req: dict[str, list[str]] = {f"R{i}": [] for i in range(1, len(requirements) + 1)}
    for index, (_, reqs) in enumerate(clauses):
        outcome = explore_properties[index].get("outcome") if index < len(explore_properties) else None
        status = "PASS" if outcome == "PASS" else "FAIL" if outcome == "FAIL" else "not_observed"
        for rid in reqs:
            by_req.setdefault(rid, []).append(status)
    return _aggregate(by_req, requirements, unverifiable)


def bounded_coverage(contract: dict[str, Any], report: dict[str, Any],
                     requirements: list[str], unverifiable: list[int],
                     behavior_ok: bool | None) -> dict[str, Any]:
    cov = bounded_monitor.coverage(contract, report, requirements, unverifiable,
                                   behavior_ok=behavior_ok)
    return cov.as_dict()


class CirGenProvider:
    """CIR generation provider: requirements only, contract never shown."""

    name = "llm"

    def __init__(self, client, prompt_version: str | None = None) -> None:
        import os
        from .prompts import (concir_generation_v3_system_prompt,
                              concir_generation_v4_system_prompt,
                              requirements_only_user_prompt)
        version = prompt_version or os.environ.get("CIR_PROMPT_VERSION", "v3")
        if version == "v4":
            self.system = concir_generation_v4_system_prompt()
        else:
            self.system = concir_generation_v3_system_prompt()
        self.prompt_version = version
        self.client = client
        self._prompt = requirements_only_user_prompt
        self.calls: list[dict[str, Any]] = []

    def propose(self, request):
        user = self._prompt(
            request.requirements,
            previous_candidate=request.previous_candidate or request.current_program,
            feedback=request.feedback)
        outcome = self.client.complete(self.system, user)
        self.calls.append({
            "attempt": request.attempt, "request_id": outcome.request_id,
            "response_model": outcome.response_model, "usage": outcome.usage,
            "wall_ms": outcome.wall_ms, "prompt_sha256": outcome.prompt_sha256,
        })
        from .providers import CandidateResponse
        response = CandidateResponse.from_usage(
            outcome.text, "llm", "deepseek",
            model_id=outcome.response_model or "deepseek-flash", usage=outcome.usage)
        response.wall_ms = outcome.wall_ms
        return response


def _explore(binary: Path, program: Path, contract: Path) -> dict[str, Any]:
    proc = subprocess.run([str(binary), "explore", str(program), str(contract), "petri"],
                          capture_output=True, text=True, timeout=180)
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"outcome": "PROTOCOL_ERROR", "stderr": proc.stderr[-400:],
                "properties": []}


def _explore_signature(payload: dict[str, Any]) -> str:
    """A stable signature of an explore failure, for no-progress detection."""

    parts: list[str] = []
    for d in payload.get("diagnostics") or []:
        parts.append(str(d.get("property")))
        parts.extend(str(n) for n in (d.get("counterexample_names") or []))
        for b in d.get("blocked") or []:
            parts.append(str(b.get("resource_name") or b.get("resource")))
    return hashlib.sha256(json.dumps(parts, sort_keys=True).encode()).hexdigest()


def _map_name(value: str, mapping: dict[str, str], module: str) -> str:
    if value in mapping:
        return mapping[value]
    prefix = f"{module}::"
    if value.startswith(prefix) and value[len(prefix):] in mapping:
        return prefix + mapping[value[len(prefix):]]
    return value


def align_resource_names(program: dict[str, Any],
                         reference: dict[str, Any]) -> list[dict[str, str]]:
    """Rename the candidate's resources to the reference's, by type and order.

    The model never sees the contract, so it picks its own resource names; the
    frozen contract names resources (`main::a`, ...). Aligning by type and
    declaration order makes the oracle applicable without leaking names into
    the requirements.
    """
    ref_mods = {m.get("name"): m for m in reference.get("modules", [])}
    changes: list[dict[str, str]] = []
    for module in program.get("modules", []):
        ref = ref_mods.get(module.get("name"))
        if not ref:
            continue
        by_type: dict[str, list[str]] = {}
        for res in ref.get("resources", []):
            by_type.setdefault(res.get("type", ""), []).append(res["name"])
        seen: dict[str, int] = {}
        mapping: dict[str, str] = {}
        for res in module.get("resources", []):
            t = res.get("type", "")
            k = seen.get(t, 0)
            seen[t] = k + 1
            targets = by_type.get(t, [])
            if k < len(targets) and res["name"] != targets[k]:
                mapping[res["name"]] = targets[k]
        if not mapping:
            continue
        for old, new in mapping.items():
            module_name = module.get("name", "main")
            changes.append({"module": module_name, "old": old, "new": new})
        for res in module.get("resources", []):
            res["name"] = _map_name(res["name"], mapping, module.get("name", "main"))
        provides = module.setdefault("provides", {})
        provides["resources"] = [_map_name(r, mapping, module.get("name", "main"))
                                 for r in provides.get("resources", [])]
        for prot in module.get("protection", []):
            for key in ("var", "lock"):
                if isinstance(prot.get(key), str):
                    prot[key] = _map_name(prot[key], mapping, module.get("name", "main"))
        for fn in module.get("functions", []):
            for st in fn.get("body", []):
                for key in ("resource", "condvar", "channel", "lock"):
                    if isinstance(st.get(key), str):
                        st[key] = _map_name(st[key], mapping, module.get("name", "main"))
        changes.extend(_align_functions(module, ref))
    return changes


def _align_functions(module: dict[str, Any], ref_module: dict[str, Any]) -> list[dict[str, str]]:
    """Rename the candidate's worker functions to the reference's, by order.

    The contract references functions by FQN (`main::t1`); the model names its
    own workers. Aligning by declaration order (main excluded) lets the
    contract apply.
    """
    ref_names = [f.get("name") for f in ref_module.get("functions", [])
                 if f.get("name") != "main"]
    candidates = [f for f in module.get("functions", []) if f.get("name") != "main"]
    mapping: dict[str, str] = {}
    for fn, target in zip(candidates, ref_names):
        if fn.get("name") and target and fn["name"] != target:
            mapping[fn["name"]] = target
    if not mapping:
        return []
    mod = module.get("name", "main")
    for fn in module.get("functions", []):
        fn["name"] = _map_name(fn["name"], mapping, mod)
    provides = module.setdefault("provides", {})
    if isinstance(provides.get("functions"), list):
        provides["functions"] = [_map_name(x, mapping, mod) for x in provides["functions"]]
    for fn in module.get("functions", []):
        for st in fn.get("body", []):
            kind = st.get("kind")
            if kind == "scope" and isinstance(st.get("funcs"), list):
                st["funcs"] = [_map_name(x, mapping, mod) for x in st["funcs"]]
            elif kind in ("spawn", "call") and isinstance(st.get("func"), str):
                st["func"] = _map_name(st["func"], mapping, mod)
    return [{"module": mod, "old": old, "new": new, "kind": "function"}
            for old, new in mapping.items()]


def run_g3(llm_client, binary: Path, task: GenTask, out_dir: Path, *,
           k: int = 4, prompt_asset: str | None = None,
           with_codegen: bool = True) -> dict[str, Any]:
    from .concir_client import ConcirClient
    from .json_utils import extract_json
    from .normalize import normalize as normalize_program
    from .prompts import build_explore_feedback, render_feedback
    from .providers import CandidateRequest
    from .revision_workflow import build_schema_feedback

    out_dir.mkdir(parents=True, exist_ok=True)
    if hasattr(llm_client, "set_stage"):
        llm_client.set_stage("cir")
    backend = ConcirClient(str(binary), workdir=out_dir / "calls", timeout=60.0)
    provider = CirGenProvider(llm_client)
    contract = task.contract

    record: dict[str, Any] = {"arm": "G3_concir", "task": task.id,
                              "accepted": False, "status": "generation_failed",
                              "rounds": [], "llm_calls": provider.calls,
                              "accepted_with_proof": False,
                              "name_alignment": [], "cir_path": None}
    feedback: str | None = None
    previous: str | None = None
    accepted_path: Path | None = None
    for round_no in range(1, k + 1):
        response = provider.propose(CandidateRequest(
            requirements=task.requirements_md, contract=contract,
            feedback=feedback, attempt=round_no, previous_candidate=previous))
        previous = response.text
        round_info: dict[str, Any] = {"round": round_no, "decision": None,
                                      "check": None, "explore": None,
                                      "prompt_tokens": response.input_tokens,
                                      "completion_tokens": response.output_tokens,
                                      "llm_ms": response.wall_ms}
        record["rounds"].append(round_info)
        if response.error:
            record["status"] = "model_error"
            record["error"] = response.error
            break
        try:
            parsed = json.loads(extract_json(response.text))
        except Exception as exc:  # noqa: BLE001
            round_info["decision"] = "parse_error"
            feedback = f"Reply was not one JSON object: {exc}. Output only the JSON object."
            continue
        parsed, norm_records, _ = normalize_program(parsed)
        program_path = out_dir / f"revision-{round_no}.cir.json"
        program_path.write_text(json.dumps(parsed, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8")
        check = backend.check(program_path)
        round_info["check"] = check.status
        round_info["check_kind"] = check.kind
        round_info["normalizations"] = norm_records
        round_info["program_sha256"] = hashlib.sha256(
            program_path.read_bytes()).hexdigest()
        raw_path = out_dir / f"revision-{round_no}.raw.txt"
        raw_path.write_text(response.text, encoding="utf-8")
        round_info["raw_sha256"] = hashlib.sha256(response.text.encode()).hexdigest()
        if check.kind in {"process_error", "protocol_error"}:
            round_info["decision"] = "tool_error"
            round_info["error_class"] = "tool_error"
            fb = build_schema_feedback(check, norm_records, parsed)
            feedback = render_feedback(fb)
            (out_dir / f"revision-{round_no}.feedback.json").write_text(
                feedback, encoding="utf-8")
            continue
        if check.status != "valid":
            fb = build_schema_feedback(check, norm_records, parsed)
            error_class = fb.get("error_class") or "static_check"
            round_info["decision"] = "schema_error" if error_class == "schema_parse" else "check_invalid"
            round_info["error_class"] = error_class
            sig = json.dumps(fb.get("diagnostics"), sort_keys=True)
            round_info["diagnostic_signature"] = hashlib.sha256(sig.encode()).hexdigest()
            prev = record["rounds"][-2] if len(record["rounds"]) > 1 else None
            round_info["repeated_diagnostic"] = bool(
                prev and prev.get("diagnostic_signature") == round_info["diagnostic_signature"])
            round_info["program_changed"] = bool(
                prev and prev.get("program_sha256") != round_info["program_sha256"])
            feedback = render_feedback(fb)
            (out_dir / f"revision-{round_no}.feedback.json").write_text(
                feedback, encoding="utf-8")
            continue
        support = backend.support(program_path)
        if support.status == "unsupported":
            round_info["decision"] = "unsupported"
            record["status"] = "unsupported"
            break
        explore = backend.explore(program_path, task.contract_path, "petri")
        round_info["explore"] = explore.outcome
        record["model"] = {"outcome": explore.outcome, "complete": explore.complete}
        record["coverage"] = model_coverage(contract, explore.payload.get("properties", []),
                                            task.requirements, task.unverifiable)
        if explore.outcome == "PASS" and explore.complete is True:
            round_info["decision"] = "accepted"
            record["accepted"] = True
            record["status"] = "accepted"
            accepted_path = program_path
            break
        round_info["decision"] = "explore_fail"
        # No-progress detection: an unchanged program or an identical
        # counterexample means the model must edit the implicated statements,
        # not resend the same design.
        prev = record["rounds"][-2] if len(record["rounds"]) > 1 else None
        sig = _explore_signature(explore.payload)
        same_program = bool(prev and prev.get("program_sha256")
                            == round_info.get("program_sha256"))
        same_sig = bool(prev and prev.get("explore_signature") == sig)
        round_info["explore_signature"] = sig
        round_info["same_program_as_previous"] = same_program
        round_info["repeated_explore_signature"] = same_sig
        stagnation = ""
        if same_program:
            stagnation = ("Your previous revision was byte-identical to this one, so "
                          "it cannot resolve the counterexample. Edit the statements "
                          "named in the counterexample. ")
        elif same_sig:
            stagnation = ("The same counterexample persists; the structure it names "
                          "did not change. Modify those statements directly. ")
        if explore.outcome == "INVALID":
            feedback = (stagnation
                        + "The design cannot be evaluated against the contract because "
                        "it does not use the entity names given in the requirements "
                        "(see the Entities section): every role and shared resource "
                        "must appear with exactly that name in the design. "
                        + render_feedback(build_explore_feedback(explore)))
        else:
            feedback = stagnation + render_feedback(build_explore_feedback(explore))

    if accepted_path is not None:
        record["cir_path"] = str(accepted_path)
    else:
        last = out_dir / f"revision-{len(record['rounds'])}.cir.json"
        if last.is_file():
            record["cir_path"] = str(last)
    cir_path = Path(record["cir_path"]) if record.get("cir_path") else None
    if cir_path is None or not cir_path.is_file():
        return record
    explore = _explore(binary, cir_path, task.contract_path)
    record["model"] = {"outcome": explore.get("outcome"),
                       "complete": explore.get("complete")}
    record["coverage"] = model_coverage(
        task.contract, explore.get("properties", []), task.requirements,
        task.unverifiable)
    # codegen + build + conform + behavior on the accepted CIR (ablation arm)
    if with_codegen and record["accepted"] and explore.get("outcome") == "PASS":
        try:
            skeleton = out_dir / "rust"
            codegen(cir_path, skeleton, binary=binary)
            holes = holes_of(skeleton)
            record["holes"] = [h["id"] for h in holes]
            traces = collect_traces(skeleton, native_runs=32, miri_seeds=0,
                                    calls_dir=out_dir / "calls", run_miri=False)
            aggregate = conform_all(cir_path, traces, binary=binary)
            record["conform"] = {kk: aggregate.get(kk) for kk in
                                 ("traces_total", "conformant", "violation",
                                  "timeout", "missing")}
            record["behavior_hang"] = any(t.hang for t in traces)
            record["accepted_with_proof"] = bool(
                aggregate.get("violation") == 0
                and aggregate.get("conformant", 0) > 0
                and not record["behavior_hang"])
        except Exception as exc:  # noqa: BLE001
            record["codegen_error"] = f"{type(exc).__name__}: {exc}"
    return record


def run_rust_generation(client, arm: str, task: GenTask, out_dir: Path, *,
                        k: int = 4, miri_seeds: int = 16) -> tuple[Any, dict[str, Any]]:
    from .flash_smoke import RustLiveProvider
    from .experiments_v2 import ARM_DIRECT, ARM_SELF_ITER, ARM_TOOLS_ITER

    mode = {ARM_GEN_DIRECT: "direct", ARM_GEN_SELF: "self", ARM_GEN_TOOLS: "tools"}[arm]
    internal = {ARM_GEN_DIRECT: ARM_DIRECT, ARM_GEN_SELF: ARM_SELF_ITER,
                ARM_GEN_TOOLS: ARM_TOOLS_ITER}[arm]
    provider = RustLiveProvider(client, mode)
    # G2 tools: build + Miri 16 (per-seed, bounded) + Lockbud. The many-seeds
    # pass is skipped: on non-terminating programs it is unbounded, and each
    # bounded seed already counts as an observation.
    run = run_rust_arm(provider, arm=internal, task=task.id,
                       spec=task.requirements_md, contract=task.contract,
                       out_dir=out_dir, k=k, tool_timeout_s=15.0,
                       run_miri_many_seeds=False, miri_seed_count=miri_seeds)
    record = run.as_dict()
    record["arm"] = arm
    record["llm_calls"] = provider.calls
    return run, record


# ───────────────────── §1 G3 v2: LLM code from a verified CIR ─────────────────

_DROP_OPS = {"spawn", "join", "scope", "complete", "ev"}


def _cir_json_text(path: Path) -> str:
    return path.read_text(encoding="utf-8").strip()


def _llmcode_system() -> str:
    from .prompts import PROMPT_ASSET_DIR
    return (PROMPT_ASSET_DIR / "rust_from_cir_v2.md").read_text(encoding="utf-8")


def _cir_plan(cir: dict[str, Any]) -> str:
    """A compact, authoritative thread/step plan derived from the CIR."""
    lines = ["Thread plan (authoritative; mirror it exactly):"]
    for mod in cir.get("modules", []):
        lines.append(f"module {mod.get('name')}:")
        for fn in mod.get("functions", []):
            steps = []
            for st in fn.get("body", []):
                kind = st.get("kind")
                if kind == "spawn":
                    steps.append(f"spawn {st.get('func')}")
                elif kind == "scope":
                    steps.append("start [" + ", ".join(st.get("funcs", [])) + "]")
                elif kind == "join":
                    steps.append("join")
                elif kind in ("mutex_lock", "mutex_unlock", "semaphore_acquire",
                              "semaphore_release"):
                    steps.append(f"{kind} {st.get('resource')}")
                elif kind == "channel_send" or kind == "channel_recv":
                    steps.append(f"{kind} {st.get('channel')}")
                elif kind == "condvar_wait":
                    steps.append(f"condvar_wait {st.get('condvar')} on {st.get('lock')}")
                elif kind in ("condvar_notify", "condvar_notify_all"):
                    steps.append(f"{kind} {st.get('condvar')}")
            lines.append(f"- {fn.get('name')}: " + ("; ".join(steps) or "no operations"))
    return "\n".join(lines)


def _llmcode_user(task: GenTask, cir_path: Path, feedback: str | None,
                  previous_rust: str | None = None) -> str:
    cir = json.loads(cir_path.read_text(encoding="utf-8"))
    parts = [
        "Requirement document:",
        task.requirements_md.strip(),
        "",
        _cir_plan(cir),
        "",
        "Verified ConcIR design (authoritative):",
        "```json",
        _cir_json_text(cir_path),
        "```",
    ]
    if previous_rust:
        parts += ["", "Previous Rust program:", "```rust", previous_rust.rstrip(), "```"]
    if feedback:
        parts += ["", "Feedback on that Rust program:", feedback,
                  "Fix the Rust; do not change the design."]
    parts += ["", "Output only the complete Rust program in one ```rust fence."]
    return "\n".join(parts)


def _compiler_errors(log: str) -> str:
    lines = [ln for ln in log.splitlines()
             if ln.startswith("error") or ln.startswith("error[")]
    return "\n".join(lines) if lines else log[-2000:]


def _stage_row() -> dict[str, Any]:
    return {
        "format": "not_run", "source_build": "not_run", "instrument": "not_run",
        "instrumented_build": "not_run", "execution": "not_run",
        "conformance": "not_run", "requirements": "not_run",
    }


def _binding_base(name: str) -> str:
    import re
    return re.sub(r"_(mutex|condvar|semaphore|channel|atomic|var)\d+$", "", name)


_CHANNEL_ENDPOINT = re.compile(r"^(?P<chan>.+?)(?:_?(?:tx|rx|sender|receiver)\d*)$")


def _channel_base(name: str) -> str:
    """`ch1_tx`/`ch1_rx` -> `ch1`; `tx` -> `''` (no channel token)."""

    m = _CHANNEL_ENDPOINT.match(name)
    return m.group("chan") if m else name


def mapping_to_cir(rust_resources: list[dict[str, str]], cir: dict[str, Any]
                   ) -> tuple[dict[str, str], dict[str, Any]]:
    """Bind runtime resources to CIR FQNs by a layered, explainable rule.

    Only structural matches are bindings:

    1. ``exact`` — the runtime short name equals the CIR resource name;
    2. ``channel-name`` — a channel endpoint's channel token equals a CIR
       channel name (``ch1_tx`` -> ``ch1``);
    3. ``spawn-entry`` — the instrumenter's structural thread entry names a CIR
       function, and that entry is unambiguous.

    Everything else — module prefix, "only one object remains", handle-name
    substrings — is a *suggestion*, never a binding. Name collisions (two
    runtime resources with one CIR target) are unresolved. Anything unresolved
    is reported with its candidate list and construction site.
    """

    kinds = rust_oracle.reference_kinds(cir)
    by_kind: dict[str, list[str]] = {}
    by_short: dict[tuple[str, str], list[str]] = {}
    for fqn, kind in kinds.items():
        by_kind.setdefault(kind, []).append(fqn)
        by_short.setdefault((fqn.rsplit("::", 1)[-1], kind), []).append(fqn)
    modules = [m["name"] for m in cir.get("modules", [])]
    mapping: dict[str, str] = {}
    rules: dict[str, str] = {}
    ambiguous: list[dict[str, Any]] = []

    def unmapped(kind: str) -> list[str]:
        return [f for f in by_kind.get(kind, []) if f not in mapping.values()]

    from collections import Counter as _Counter
    # Match on the display name (field / binding / channel token); the runtime
    # `name` carries the construction site and is what trace events use.
    def _display(r):
        return r.get("display") or r.get("name", "")
    name_counts = _Counter(_display(r) for r in rust_resources)
    channel_resources = [r for r in rust_resources if r.get("kind") == "Channel"]
    for res in rust_resources:
        kind = res.get("kind", "")
        name = res.get("name", "")
        if kind in {"Spawn", "ChannelWrapper"}:
            continue
        # Two runtime instances with the same name cannot both be one identity,
        # and their trace events are indistinguishable.
        if name_counts[_display(res)] > 1:
            ambiguous.append({"rust": name, "kind": kind, "candidates": [],
                              "site": res.get("site"),
                              "reason": "duplicate runtime resource name"})
            continue
        short = _display(res).rsplit("::", 1)[-1]
        # 1. exact short-name match
        cands = [c for c in by_short.get((_binding_base(short), kind), [])
                 if c not in mapping.values()]
        if len(cands) == 1:
            mapping[name] = cands[0]; rules[name] = "exact"; continue
        # 2. channel endpoint token. A CIR channel has several runtime endpoints
        #    (sender and receiver), so both may map to the same channel; do not
        #    exclude an already-mapped channel here.
        if kind == "Channel":
            token = _channel_base(short)
            if token:
                hits = [f for f in by_kind.get("Channel", [])
                        if f.rsplit("::", 1)[-1] == token]
                if len(hits) == 1:
                    mapping[name] = hits[0]; rules[name] = "channel-name"; continue
            # A single CIR channel absorbing every endpoint is elimination, not
            # identity; leave it unresolved with a suggestion.
            all_channels = by_kind.get("Channel", [])
            ambiguous.append({"rust": name, "kind": kind,
                              "candidates": all_channels,
                              "suggestion": all_channels[0] if len(all_channels) == 1 else None})
            continue
        # A binding prefix naming a module, or "only one object remains", is NOT
        # a proven identity: the program may have omitted a modelled resource
        # and added another, or renamed it. Suggestion only.
        cands = unmapped(kind)
        ambiguous.append({"rust": name, "kind": kind, "candidates": cands,
                          "site": res.get("site"),
                          "suggestion": cands[0] if len(cands) == 1 else None})

    # Spawn identity comes from the instrumenter's structural `entry` (the
    # function the closure runs), and only when that entry is unambiguous.
    workers: list[str] = []
    for mod in cir.get("modules", []):
        for fn in mod.get("functions", []):
            if fn.get("name") != "main":
                workers.append(f"{mod['name']}::{fn['name']}")
    used: set[str] = set()
    for res in rust_resources:
        if res.get("kind") != "Spawn":
            continue
        name = res.get("name", "")
        entry = res.get("entry")
        unique = res.get("unique_entry")
        if unique is True and entry:
            hits = [w for w in workers if w.rsplit("::", 1)[-1] == entry and w not in used]
            if len(hits) == 1:
                mapping[name] = hits[0]; rules[name] = "spawn-entry"; used.add(hits[0])
                continue
            ambiguous.append({"rust": name, "kind": "Spawn", "candidates": hits,
                              "site": res.get("site"), "entry": entry,
                              "reason": "entry names no unique CIR thread"})
            continue
        ambiguous.append({"rust": name, "kind": "Spawn",
                          "candidates": [w for w in workers if w not in used],
                          "site": res.get("site"), "entry": entry,
                          "reason": "thread entry is not unambiguous (closure has "
                                    "several or no calls)"})

    # Collision: two runtime resources bound to the same CIR FQN is a name
    # collision, not two identities. Channels are exempt (endpoints share one).
    targets: dict[str, list[str]] = {}
    for rust, fqn in mapping.items():
        targets.setdefault(fqn, []).append(rust)
    for fqn, rusts in targets.items():
        if len(rusts) > 1 and kinds.get(fqn) != "Channel":
            for rust in rusts:
                mapping.pop(rust, None); rules.pop(rust, None)
                ambiguous.append({"rust": rust, "kind": kinds.get(fqn),
                                  "candidates": [fqn],
                                  "reason": "name collision: another runtime resource "
                                            "binds the same CIR resource"})
    spawn_names = [r["name"] for r in rust_resources if r.get("kind") == "Spawn"]
    return mapping, {"by_kind": {k: v for k, v in by_kind.items()},
                     "rules": rules,
                     "threads": {src: mapping.get(src) for src in spawn_names},
                     "ambiguous": ambiguous}


def _rewrite_traces(src_dir: Path, dst_dir: Path, mapping: dict[str, str],
                    drop_ops: set[str], wrapper_names: set[str] | None = None) -> None:
    dst_dir.mkdir(parents=True, exist_ok=True)
    for trace in sorted(src_dir.glob("*.jsonl")):
        out = []
        for line in trace.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            event = json.loads(line)
            if event.get("op") in drop_ops:
                continue
            if (wrapper_names and event.get("op") in ("mutex_lock", "mutex_unlock")
                    and event.get("r") in wrapper_names):
                continue
            event["r"] = mapping.get(event.get("r", ""), event.get("r", ""))
            # `n` and `api` stay on the event. Projection must not drop a count.
            out.append(json.dumps(event))
        (dst_dir / trace.name).write_text("\n".join(out) + ("\n" if out else ""),
                                          encoding="utf-8")


def _conform_all_op_resource(binary: Path, cir_path: Path, traces_dir: Path) -> dict[str, Any]:
    from collections import Counter
    statuses = Counter()
    first: dict[str, Any] | None = None
    violations: list[dict[str, Any]] = []
    traces = sorted(traces_dir.glob("*.jsonl"))
    raw: list[dict[str, Any]] = []
    for trace in traces:
        proc = subprocess.run(
            [str(binary), "conform", str(cir_path), str(trace), "--op-resource"],
            capture_output=True, text=True, timeout=120)
        raw.append({"trace": str(trace),
                    "trace_sha256": hashlib.sha256(trace.read_bytes()).hexdigest(),
                    "stdout": proc.stdout})
        try:
            result = json.loads(proc.stdout)
        except json.JSONDecodeError:
            result = {"status": "error", "detail": proc.stderr[-200:]}
        status = result.get("status")
        statuses[status] += 1
        if status != "conformant":
            # `got` is "<op>:<resource>"; keep the structured resource.
            got = str(result.get("got", ""))
            resource = got.split(":", 1)[1] if ":" in got else None
            detail = result.get("detail")
            kind = "count" if str(detail or "").startswith("kind=count") else status
            violations.append({"status": status, "got": got, "resource": resource,
                               "event_index": result.get("event_index"),
                               "expected": result.get("expected"), "detail": detail,
                               "kind": kind})
            if first is None:
                first = result
    return {"traces": len(traces), "statuses": dict(statuses),
            "conformant": statuses.get("conformant", 0), "first_violation": first,
            "violations": violations, "raw": raw}


def _cir_props_for(binary: Path, cir_path: Path, contract_path: Path
                   ) -> tuple[dict[str, str], bool | None]:
    """Run the real CVN explore and normalise property ids (preserved: prefix)."""

    try:
        proc = subprocess.run([str(binary), "explore", str(cir_path), str(contract_path),
                               "petri"], capture_output=True, text=True, timeout=120)
        payload = json.loads(proc.stdout)
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError):
        return {}, None
    props = {str(p["id"]).replace("preserved: ", ""): p.get("outcome")
             for p in payload.get("properties", [])}
    return props, payload.get("complete")


def run_llmcode_from_cir(llm_client, binary: Path, task: GenTask, cir_path: Path,
                         out_dir: Path, *, k_code: int = 3, instrument_binary=None,
                         functional_spec: dict | None = None, n_runs: int = 32
                         ) -> dict[str, Any]:
    """Generate Rust, then evaluate each candidate with the shared core.

    The model is called only to generate or to repair. Tool failures and
    capability gaps do not request another completion. Every round stores an
    evaluation record. Semantic repairs and protocol repairs are counted apart
    from tool failures.
    """
    from . import candidate_eval

    system = _llmcode_system()
    out_dir.mkdir(parents=True, exist_ok=True)
    if hasattr(llm_client, "set_stage"):
        llm_client.set_stage("code")
    record: dict[str, Any] = {
        "arm": "G3_concir_llmcode", "task": task.id, "cir_path": str(cir_path),
        "prompt": "prompts/rust_from_cir_v2.md",
        "rounds": [], "accepted": False, "accepted_with_proof": False,
        "status": "not_started", "instrument_limit": None,
        "semantic_repairs": 0, "protocol_repairs": 0, "tool_failures": 0,
        "llm_calls": 0,
        "proof_meaning": "32 observed traces conformed and the bounded monitor did not FAIL; not a proof of every execution",
    }
    feedback: str | None = None
    previous_rust: str | None = None
    for round_no in range(1, k_code + 1):
        info: dict[str, Any] = {"round": round_no, "decision": None, "stages": _stage_row(),
                                "reasons": [], "evaluation": None}
        record["rounds"].append(info)
        user = _llmcode_user(task, cir_path, feedback, previous_rust)
        sent = {"stage": "code", "system_sha256": hashlib.sha256(system.encode()).hexdigest(),
                "user_sha256": hashlib.sha256(user.encode()).hexdigest()}
        (out_dir / f"sent-round-{round_no}.json").write_text(
            json.dumps(sent, indent=2) + "\n", encoding="utf-8")
        outcome = llm_client.complete(system, user)
        record["llm_calls"] += 1
        if getattr(outcome, "sent", None):
            (out_dir / f"sent-round-{round_no}.txt").write_text(
                outcome.sent.get("message", ""), encoding="utf-8")
        info["prompt_tokens"] = _usage_get(outcome, "prompt_tokens")
        info["completion_tokens"] = _usage_get(outcome, "completion_tokens")
        info["llm_ms"] = getattr(outcome, "wall_ms", None)
        info["message_sha256"] = getattr(outcome, "prompt_sha256", None)
        rust = _extract_rust_body(outcome.text)
        if rust is None:
            rejection = candidate_eval.protocol_rejection(
                "format_error",
                "Reply was not one Rust program in a ```rust fence. Output the complete file.")
            info["stages"]["format"] = "format_error"
            info["decision"] = "protocol_noncompliance"
            info["reasons"].append("format_error")
            info["evaluation"] = rejection
            feedback = rejection["feedback"]
            record["protocol_repairs"] += 1
            record["status"] = "protocol_noncompliance"
            record["action"] = "protocol_noncompliance"
            continue
        info["stages"]["format"] = "ok"
        previous_rust = rust
        (out_dir / f"round-{round_no}.rs").write_text(rust, encoding="utf-8")
        evaluated = candidate_eval.evaluate_candidate(
            rust, Path(cir_path), task.contract_path, out_dir / f"round-{round_no}",
            binary=binary, instrument=instrument_binary, n_runs=n_runs,
            functional_spec=functional_spec, cell_id=task.id, round_no=round_no,
            candidate_kind="online")
        follow = evaluated["followup"]
        info["stages"] = {**info["stages"], **evaluated.get("stages", {})}
        info["evaluation"] = {
            "ledger": evaluated.get("ledger"),
            "action": follow["action"],
            "category": follow["category"],
            "feedback": follow.get("feedback") or "",
            "status": follow["status"],
            "evidence_path": evaluated.get("evidence_path"),
            "stages": evaluated.get("stages"),
        }
        info["reasons"] = list((evaluated.get("ledger") or {}).get("reasons") or [])
        record["ledger"] = evaluated.get("ledger")
        record["acceptance_policy"] = candidate_eval.ACCEPTANCE_POLICY
        if follow["action"] == "accept":
            info["decision"] = "accepted"
            record["accepted"] = True
            record["status"] = "accepted"
            record["action"] = "accepted"
            record["final_rust"] = str(out_dir / f"round-{round_no}.rs")
            break
        if follow["action"] == "repair":
            info["decision"] = "candidate_error"
            record["action"] = "candidate_error"
            record["status"] = "candidate_error"
            record["semantic_repairs"] += 1
            feedback = follow["feedback"]
            if not feedback.strip():
                info["decision"] = "capability_gap"
                record["action"] = "capability_gap"
                record["status"] = "capability_gap"
                break
            continue
        if follow["action"] == "repair_protocol":
            info["decision"] = "protocol_noncompliance"
            record["action"] = "protocol_noncompliance"
            record["status"] = "protocol_noncompliance"
            record["protocol_repairs"] += 1
            feedback = follow["feedback"]
            continue
        info["decision"] = follow["status"]
        record["status"] = follow["status"]
        record["action"] = follow["category"]
        if follow["category"] == "tool_failure":
            record["tool_failures"] += 1
        break
    return record


def code_stage_view(code: dict[str, Any] | None) -> dict[str, Any]:
    """Read hang, run counts, and coverage from the shared code-stage ledger.

    Hang is only a timeout. Tool failures and capability gaps stay in
    ``run_state`` / ``failure_stage``. Coverage counts bounded monitor
    statuses. A model-check PASS is not implementation coverage. When the
    monitor did not decide any property, coverage is null and ``coverage_reason``
    says why.
    """

    code = code or {}
    ledger = code.get("ledger") or {}
    run = ledger.get("run") or {}
    state = run.get("state")
    if not ledger:
        return {"hang": None, "run_state": None, "runs_started": None,
                "runs_completed": None, "failure_stage": code.get("status"),
                "coverage": None,
                "coverage_reason": "code stage has no shared evaluation ledger"}
    hang = bool(state == "timeout" or run.get("hang"))
    failure = None if code.get("accepted") else (code.get("status") or state)
    props = ledger.get("properties") or []
    decidable = [p for p in props
                 if p.get("independent_requirement_result") in {"PASS_bounded", "FAIL"}]
    blocked = state in {"not_run", "source_build_failed", "instrument_failed",
                        "instrument_build_failed", "tool_error"} or ledger.get(
                            "current_evaluation") in {"tool_error", "instrument_failed",
                                                      "instrument_build_failed"}
    requirement_ids = []
    for prop in props:
        for req in prop.get("requirements") or []:
            if req not in requirement_ids:
                requirement_ids.append(req)
    if blocked or not props:
        coverage = None
        reason = (f"metrics not computed: run state {state or ledger.get('current_evaluation')}"
                  if blocked else "no monitor properties on the ledger")
    else:
        satisfied_props = [p for p in decidable
                           if p.get("independent_requirement_result") == "PASS_bounded"
                           and p.get("obligation_state") == "satisfied"]
        # A requirement is satisfied only when every property that cites it is
        # satisfied. unsupported, unknown and missing properties are not passes.
        def req_ok(req: str) -> bool:
            cited = [p for p in props if req in (p.get("requirements") or [])]
            return bool(cited) and all(
                p.get("obligation_state") == "satisfied"
                and p.get("independent_requirement_result") == "PASS_bounded"
                for p in cited)
        satisfied_reqs = [req for req in requirement_ids if req_ok(req)]

        def req_covered(req: str) -> bool:
            cited = [p for p in props if req in (p.get("requirements") or [])]
            return bool(cited) and all(p in decidable for p in cited)

        covered_reqs = [req for req in requirement_ids if req_covered(req)]
        coverage = {
            "basis": "bounded_monitor_statuses",
            "property_total": len(props),
            "property_decidable": len(decidable),
            "property_satisfied": len(satisfied_props),
            "property_coverage": (len(decidable) / len(props)) if props else None,
            "requirement_coverage": (
                len(covered_reqs) / len(requirement_ids) if requirement_ids else None),
            "decidable_satisfaction_ratio": (
                len(satisfied_props) / len(decidable) if decidable else None),
            "requirement_total": len(requirement_ids),
            "requirement_satisfied": len(satisfied_reqs),
            "requirement_satisfaction_rate": (
                len(satisfied_reqs) / len(requirement_ids) if requirement_ids else None),
        }
        reason = None if decidable or requirement_ids else (
            "monitor did not return PASS_bounded or FAIL; "
            "a model-check PASS is not implementation coverage")
        if reason:
            coverage = None
    return {"hang": hang, "run_state": state,
            "runs_started": run.get("started"), "runs_completed": run.get("completed"),
            "failure_stage": failure, "coverage": coverage, "coverage_reason": reason}


def run_g3_v2(llm_client, binary: Path, task: GenTask, out_dir: Path, *,
              k_cir: int = 4, k_code: int = 3, instrument_binary=None) -> dict[str, Any]:
    """§1: verified CIR, then LLM-generated Rust post-verified by tools."""
    cir_rec = run_g3(llm_client, binary, task, out_dir / "cir", k=k_cir,
                     with_codegen=False)
    rounds = list(cir_rec.get("rounds", []))
    for rnd in rounds:
        rnd["stage"] = "cir"
    combined: dict[str, Any] = {
        "arm": "G3_concir", "task": task.id,
        "cir_stage": {k: cir_rec.get(k) for k in
                      ("accepted", "status", "model", "coverage")},
        "cir_rounds": len(cir_rec.get("rounds", [])), "code_stage": None,
        "rounds": rounds, "accepted": False, "accepted_with_proof": False,
        "status": cir_rec.get("status"),
    }
    if not cir_rec.get("accepted") or not cir_rec.get("cir_path"):
        return combined
    code = run_llmcode_from_cir(llm_client, binary, task,
                                Path(cir_rec["cir_path"]), out_dir / "code",
                                k_code=k_code, instrument_binary=instrument_binary)
    for rnd in code.get("rounds", []):
        rnd["stage"] = "code"
    combined["rounds"].extend(code.get("rounds", []))
    combined["code_stage"] = code
    combined["accepted"] = bool(cir_rec.get("accepted") and code.get("accepted"))
    combined["accepted_with_proof"] = bool(code.get("accepted_with_proof"))
    view = code_stage_view(code)
    combined["coverage"] = view["coverage"]
    combined["coverage_reason"] = view["coverage_reason"]
    combined["oracle"] = {"hang": view["hang"], "run_state": view["run_state"],
                          "runs_started": view["runs_started"],
                          "runs_completed": view["runs_completed"],
                          "failure_stage": view["failure_stage"]}
    last = (code.get("rounds") or [{}])[-1]
    combined["monitor_fail"] = last.get("monitor_fail")
    combined["status"] = "accepted" if combined["accepted"] else code.get(
        "status", cir_rec.get("status"))
    combined["model"] = cir_rec.get("model")
    return combined


def _usage_get(outcome, key: str):
    usage = getattr(outcome, "usage", None) or {}
    return usage.get(key)


_VIOLATION_EXPLAIN = {
    "extra_op": "You introduced a synchronization operation the design does not "
                "have. A CIR `var` must live inside the mutex that protects it, "
                "not behind a new lock, and `main` must not add shared-state access.",
    "order": "The synchronization operations are out of the design's order; replay "
             "the CIR statements in order.",
    "resource": "An operation used a resource that does not match the design at "
                "this point.",
    "unmapped_resource": "A design resource could not be recognised in the code; "
                         "use the exact entity names and the provided primitives.",
}


def _conform_kind(violation: dict[str, Any] | None) -> str | None:
    if not violation:
        return None
    if violation.get("status") == "unknown_sid":
        return "unmapped_resource"
    got = violation.get("got") or ""
    op = got.split(":")[0]
    expected = violation.get("expected") or []
    if not expected:
        return "extra_op" if op in ("mutex_lock", "mutex_unlock") else "order"
    if any(str(e).split(":")[0] == op for e in expected):
        return "resource"
    return "order"


def _explain_violation(violation: dict[str, Any], kind: str | None) -> str:
    return (f"Conformance violation at event {violation.get('event_index')}: "
            f"observed {violation.get('got')}, expected one of "
            f"{violation.get('expected')}. "
            f"{_VIOLATION_EXPLAIN.get(kind or '', '')} "
            f"[detail: {violation.get('detail')}]")


def _extract_rust_body(text: str) -> str | None:
    if "```" in text:
        blocks = text.split("```")[1::2]
        if not blocks:
            return None
        best = max(blocks, key=len)
        lines = best.splitlines()
        if lines and lines[0].strip().lower() in ("rust", "rs"):
            lines = lines[1:]
        body = "\n".join(lines).strip()
        return body + "\n" if "fn main" in body else None
    return text.strip() + "\n" if "fn main" in text else None


def rust_arm_oracle(artifact: Path, task: GenTask, work_dir: Path, *, binary) -> dict[str, Any]:
    result = rust_oracle.evaluate(
        artifact.read_text(encoding="utf-8"), task.contract_path,
        task.reference_cir_path, work_dir, n_native=32, miri_seeds=0,
        run_timeout=10.0, binary=binary)
    if result.get("status") == "build_failed":
        return {"status": "build_failed", "coverage": None}
    cov = result.get("coverage")
    report = result.get("monitor") or {}
    monitor_fail = [p["id"] for p in report.get("properties", [])
                    if p.get("status") == "FAIL"]
    result["monitor_fail"] = monitor_fail
    return result
