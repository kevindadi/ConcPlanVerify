"""End-to-end traceable, verifiable, bi-level iterative pipeline (Section 3.4).

Orchestrates:
- **Stage 1/2 (Loop A)**: Generate/revise v3.5 CIR or v4.0 SCIR (auto-lowering
  ``structured_body`` via ``concir-backend lower`` to capture ``TraceMap``)
  until ``explore`` returns complete ``PASS``.
- **Stage 3**: Run ``codegen`` and prompt the hole provider to fill all
  sequential holes (supporting explicit cross-layer escalation replies
  ``{"escalate": {"reason": "...", "module": "..."}}``).
- **Stage 4**: Run :class:`~concplan.static_verify.CompositeStaticVerifier`
  (and optional dynamic conformance):
  - **Loop B (Hole-local repair)**: On ``fail_hole`` or hole-attributable
    ``fail_refinement``, re-prompt ONLY for the offending hole(s) with
    structured ``StaticViolation`` diagnostics up to ``max_hole_rounds``.
  - **Loop C (Cross-layer escalation)**: When the hole provider emits an
    ``escalate`` payload, Loop B exhausts ``max_hole_rounds``, or a
    non-hole-local refinement failure occurs, route structured escalation
    diagnostics back to Stage 2 CIR revision (up to ``max_escalations``)
    and regenerate the skeleton + holes.
- **Traceability**: Emits ``traceability_manifest.json``
  (``concir-traceability-manifest/1``) linking
  ``Clause ID -> Contract Property ID -> Module::Function::SID (Structured ID) ->
  Hole ID -> Rust File:LineRange -> Model & Static Verification Verdict``.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

from ..code_verify.conformance import (
    codegen as default_codegen,
    collect_traces,
    conform_all,
    holes_of,
)
from ..code_verify.static_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    SpnMirRefinementPlugin,
    StaticVerificationReport,
    StaticViolation,
    augment_codegen_map_with_hole_ranges,
    fill_holes_marked,
)
from ..core.concir_client import ConcirClient, ConcirResult, sha256_file
from ..core.json_utils import extract_json
from ..core.prompts import (
    build_check_feedback,
    build_explore_feedback,
    render_feedback,
)
from ..core.providers import CandidateProvider, CandidateRequest

CLAUSE_RE = re.compile(r"\b(R\d+)\b")


@dataclass
class EscalationEvent:
    index: int
    source: str  # "hole_provider_explicit" | "hole_repair_exhausted" | "refinement_non_hole" | "skeleton_fail"
    reason: str
    module: str | None = None
    offending_holes: list[str] = field(default_factory=list)
    req_clauses: list[str] = field(default_factory=list)
    violations: list[dict[str, Any]] = field(default_factory=list)

    def as_dict(self) -> dict[str, Any]:
        return {
            "index": self.index,
            "source": self.source,
            "reason": self.reason,
            "module": self.module,
            "offending_holes": list(self.offending_holes),
            "req_clauses": list(self.req_clauses),
            "violations": list(self.violations),
        }


@dataclass
class E2EResult:
    status: str  # "verified" | "unknown" | "failed" | "tool_error"
    out_dir: str
    verified_cir_path: str | None = None
    skeleton_dir: str | None = None
    filled_dir: str | None = None
    manifest_path: str | None = None
    trace_map_path: str | None = None
    cir_rounds: list[dict[str, Any]] = field(default_factory=list)
    hole_rounds: list[dict[str, Any]] = field(default_factory=list)
    escalations: list[dict[str, Any]] = field(default_factory=list)
    model_report: dict[str, Any] | None = None
    static_report: dict[str, Any] | None = None
    dynamic_report: dict[str, Any] | None = None
    fills: dict[str, str] = field(default_factory=dict)
    manifest: dict[str, Any] | None = None
    error: str | None = None

    @property
    def ok(self) -> bool:
        return self.status == "verified"

    def as_dict(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "ok": self.ok,
            "out_dir": self.out_dir,
            "verified_cir_path": self.verified_cir_path,
            "skeleton_dir": self.skeleton_dir,
            "filled_dir": self.filled_dir,
            "manifest_path": self.manifest_path,
            "trace_map_path": self.trace_map_path,
            "cir_rounds": list(self.cir_rounds),
            "hole_rounds": list(self.hole_rounds),
            "escalations": list(self.escalations),
            "model_report": self.model_report,
            "static_report": self.static_report,
            "dynamic_report": self.dynamic_report,
            "fills": dict(self.fills),
            "error": self.error,
        }


def _exclusive_run_dir(base: Path) -> Path:
    base.mkdir(parents=True, exist_ok=True)
    for _ in range(1000):
        token = f"{time.strftime('%Y%m%dT%H%M%S', time.gmtime())}-{os.getpid()}-{os.urandom(3).hex()}"
        path = base / f"e2e-{token}"
        try:
            path.mkdir(parents=True, exist_ok=False)
            return path
        except FileExistsError:
            continue
    raise RuntimeError(f"could not allocate unique e2e run dir under {base}")


def _has_structured_body(program: dict[str, Any]) -> bool:
    for mod in program.get("modules", []) or []:
        if not isinstance(mod, dict):
            continue
        for fn in mod.get("functions", []) or []:
            if isinstance(fn, dict) and fn.get("structured_body"):
                return True
    return False


def parse_hole_reply(text: str) -> tuple[dict[str, str] | None, dict[str, Any] | None]:
    """Parse a hole-provider JSON reply into ``(fills, escalation)``.

    Escalation shapes recognized:
    - ``{"escalate": {"reason": "...", "module": "..."}}``
    - ``{"escalate": true, "reason": "...", "module": "..."}``

    Otherwise returns ``({hole_id: code}, None)`` (or ``(None, None)`` if invalid JSON).
    """

    extracted = extract_json(text)
    try:
        data = json.loads(extracted)
    except json.JSONDecodeError:
        return None, None
    if not isinstance(data, dict):
        return None, None

    if "escalate" in data and data["escalate"]:
        esc = data["escalate"]
        if isinstance(esc, dict):
            return None, {
                "reason": str(esc.get("reason") or "hole provider requested CIR escalation"),
                "module": esc.get("module"),
                "details": esc,
            }
        return None, {
            "reason": str(data.get("reason") or esc if isinstance(esc, str) else "hole provider requested CIR escalation"),
            "module": data.get("module"),
            "details": data,
        }

    return {str(k): str(v) for k, v in data.items()}, None


def build_hole_fill_prompt(
    skeleton_dir: Path,
    requirements: str,
    *,
    hole_ids: list[str] | None = None,
    context_lines: int = 8,
) -> str:
    """Build a prompt to fill all (or selected) sequential holes, with escalation protocol."""

    source = (skeleton_dir / "src/main.rs").read_text(encoding="utf-8").splitlines()
    all_holes = holes_of(skeleton_dir)
    selected = (
        [h for h in all_holes if h.get("id") in set(hole_ids)]
        if hole_ids is not None
        else all_holes
    )
    parts = [
        "Fill ONLY the specified sequential hole(s) in the generated Rust skeleton.",
        "Rules:",
        "- Return a JSON object mapping each hole id to valid pure sequential Rust code.",
        "- Do NOT introduce synchronization, threads, channels, atomics, unsafe, `loop`/`while`, or `cir_trace`.",
        "- If a hole CANNOT be implemented purely without changing the concurrency architecture "
        "(e.g. missing shared variable, lock, or channel in CIR), return an escalation JSON instead:",
        '  {"escalate": {"reason": "<why CIR must change>", "module": "<module_name>"}}',
        "",
        "<requirements>",
        requirements.strip(),
        "</requirements>",
        "",
    ]
    for hole in selected:
        hid = hole["id"]
        idx = next((i for i, line in enumerate(source) if f"HOLE({hid})" in line), None)
        if idx is None and isinstance(hole.get("line"), int):
            idx = max(0, min(len(source) - 1, int(hole["line"]) - 1))
        parts.append(
            f"### hole `{hid}` (expected `{hole.get('expected', '()')}`, "
            f"sid `{hole.get('sid')}`, req `{hole.get('req', [])}`)"
        )
        if idx is not None:
            lo = max(0, idx - context_lines)
            hi = min(len(source), idx + context_lines + 1)
            parts.append("```rust")
            parts.extend(f"{n + 1:>4}: {source[n]}" for n in range(lo, hi))
            parts.append("```")
        parts.append("")
    parts.append("Reply with ONLY the JSON object.")
    return "\n".join(parts)


def build_hole_repair_prompt(
    skeleton_dir: Path,
    requirements: str,
    offending_holes: list[str],
    current_fills: dict[str, str],
    violations: list[StaticViolation],
    *,
    context_lines: int = 8,
) -> str:
    """Build a Loop B repair prompt targeting ONLY the offending hole(s)."""

    base = build_hole_fill_prompt(
        skeleton_dir, requirements, hole_ids=offending_holes, context_lines=context_lines
    )
    relevant_violations = [
        v.as_dict() for v in violations if v.hole_id in offending_holes or v.hole_id is None
    ]
    current_subset = {hid: current_fills.get(hid, "") for hid in offending_holes}
    repair_block = [
        "",
        "<offending_holes>",
        json.dumps(offending_holes, ensure_ascii=False),
        "</offending_holes>",
        "",
        "<previous_hole_fills>",
        json.dumps(current_subset, ensure_ascii=False, indent=2),
        "</previous_hole_fills>",
        "",
        "<static_verification_violations>",
        json.dumps(relevant_violations, ensure_ascii=False, indent=2),
        "</static_verification_violations>",
        "",
        "Repair ONLY the listed offending hole(s). Return a JSON object containing ONLY "
        "those hole id(s), OR return "
        '{"escalate": {"reason": "...", "module": "..."}} if the CIR design itself must change.',
    ]
    return base + "\n" + "\n".join(repair_block)


def build_traceability_manifest(
    *,
    status: str,
    requirements: str,
    contract: dict[str, Any],
    program_dict: dict[str, Any] | None,
    trace_map: dict[str, Any] | None,
    codegen_filled: dict[str, Any] | None,
    model_report: dict[str, Any] | None,
    static_report: StaticVerificationReport | None,
    dynamic_report: dict[str, Any] | None,
    escalations: list[dict[str, Any]],
    hole_rounds: list[dict[str, Any]],
    requirements_clauses: dict[str, str] | None = None,
) -> dict[str, Any]:
    """Assemble ``concir-traceability-manifest/1`` linking clauses -> contract -> CIR -> holes -> Rust -> verdicts."""

    # 1. Collect contract property IDs.
    contract_prop_ids: list[str] = []
    if isinstance(contract, dict):
        if contract.get("deadlock_free", True):
            contract_prop_ids.append("deadlock_free")
        for p in contract.get("properties", []) or []:
            if isinstance(p, dict) and p.get("id"):
                contract_prop_ids.append(str(p["id"]))
        for p in contract.get("preserved", []) or []:
            if isinstance(p, dict):
                pid = p.get("id") or p.get("function") or p.get("kind")
                if pid:
                    contract_prop_ids.append(f"preserved:{pid}")
            elif isinstance(p, str):
                contract_prop_ids.append(f"preserved:{p}")
    if model_report:
        for prop in model_report.get("properties", []) or []:
            if isinstance(prop, dict) and prop.get("id"):
                pid = str(prop["id"])
                if pid not in contract_prop_ids:
                    contract_prop_ids.append(pid)

    # 2. Index trace_map entries (module::fn::sid -> {structured_id, hole_id, req}).
    tm_entries: dict[str, dict[str, Any]] = {}
    if isinstance(trace_map, dict):
        for fqn, entry in (trace_map.get("entries") or {}).items():
            if isinstance(entry, dict):
                tm_entries[str(fqn)] = entry

    # 3. Walk CIR/SCIR program nodes to gather req tags & statement metadata.
    cir_nodes: list[dict[str, Any]] = []
    clause_descriptions: dict[str, str] = dict(requirements_clauses or {})
    for m in CLAUSE_RE.finditer(requirements or ""):
        clause_descriptions.setdefault(m.group(1), "")

    program_name = None
    if isinstance(program_dict, dict):
        program_name = program_dict.get("program")
        for mod in program_dict.get("modules", []) or []:
            if not isinstance(mod, dict):
                continue
            mname = str(mod.get("name", ""))
            mod_reqs = [str(r) for r in (mod.get("req") or [])]
            for r in mod_reqs:
                clause_descriptions.setdefault(r, "")
            for res in mod.get("resources", []) or []:
                if isinstance(res, dict):
                    for r in res.get("req") or []:
                        clause_descriptions.setdefault(str(r), "")
            for fn in mod.get("functions", []) or []:
                if not isinstance(fn, dict):
                    continue
                fname = str(fn.get("name", ""))
                fn_reqs = [str(r) for r in (fn.get("req") or [])]
                for r in fn_reqs:
                    clause_descriptions.setdefault(r, "")
                for stmt in fn.get("body", []) or []:
                    if not isinstance(stmt, dict):
                        continue
                    sid = str(stmt.get("sid", ""))
                    fqn = f"{mname}::{fname}::{sid}"
                    tm = tm_entries.get(fqn, {})
                    reqs = list(
                        dict.fromkeys(
                            [str(r) for r in (stmt.get("req") or [])]
                            + [str(r) for r in (tm.get("req") or [])]
                            + fn_reqs
                            + mod_reqs
                        )
                    )
                    for r in reqs:
                        clause_descriptions.setdefault(r, "")
                    cir_nodes.append({
                        "fqn": fqn,
                        "module": mname,
                        "function": fname,
                        "sid": sid,
                        "structured_id": tm.get("structured_id"),
                        "kind": stmt.get("kind"),
                        "hole_id": stmt.get("id") if stmt.get("kind") == "seq_hole" else tm.get("hole_id"),
                        "req": reqs,
                    })

    # Also ensure trace_map entries not in flat body (e.g. if program_dict was unlowered SCIR) are included.
    known_fqns = {n["fqn"] for n in cir_nodes}
    for fqn, entry in tm_entries.items():
        if fqn not in known_fqns:
            reqs = [str(r) for r in (entry.get("req") or [])]
            for r in reqs:
                clause_descriptions.setdefault(r, "")
            cir_nodes.append({
                "fqn": fqn,
                "module": entry.get("module"),
                "function": entry.get("function"),
                "sid": entry.get("sid"),
                "structured_id": entry.get("structured_id"),
                "kind": None,
                "hole_id": entry.get("hole_id"),
                "req": reqs,
            })

    # 4. Gather hole entries + Rust line ranges from codegen_filled.json.
    hole_records: list[dict[str, Any]] = []
    hole_ranges = (codegen_filled or {}).get("hole_ranges") or {}
    for hole in (codegen_filled or {}).get("holes", []) or []:
        if not isinstance(hole, dict) or not hole.get("id"):
            continue
        hid = str(hole["id"])
        rng = hole_ranges.get(hid) or {}
        rfile = str(rng.get("file") or "src/main.rs")
        sline = rng.get("start_line", hole.get("line"))
        eline = rng.get("end_line", sline)
        rust_loc = f"{rfile}:{sline}-{eline}" if sline is not None else rfile
        reqs = [str(r) for r in (hole.get("req") or [])]
        for r in reqs:
            clause_descriptions.setdefault(r, "")
        hole_records.append({
            "hole_id": hid,
            "module": hole.get("module"),
            "function": hole.get("function"),
            "sid": hole.get("sid"),
            "expected": hole.get("expected"),
            "rust_file": rfile,
            "start_line": sline,
            "end_line": eline,
            "rust_span": rust_loc,
            "req": reqs,
        })

    model_verdict = (model_report or {}).get("outcome") or "NOT_RUN"
    static_verdict = static_report.status if static_report else "not_run"
    violations_list = [v.as_dict() for v in (static_report.violations if static_report else [])]

    # 5. Build per-clause records and flat end-to-end links.
    clauses_out: list[dict[str, Any]] = []
    links_out: list[dict[str, Any]] = []

    all_clause_ids = sorted(clause_descriptions.keys())
    if not all_clause_ids and (cir_nodes or hole_records):
        all_clause_ids = ["UNATTRIBUTED"]

    hole_by_id = {h["hole_id"]: h for h in hole_records}

    for cid in all_clause_ids:
        matched_nodes = [
            n for n in cir_nodes
            if (cid in n["req"]) or (cid == "UNATTRIBUTED" and not n["req"])
        ]
        matched_node_holes = {n["hole_id"] for n in matched_nodes if n.get("hole_id")}
        matched_holes = [
            h for h in hole_records
            if (cid in h["req"])
            or (h["hole_id"] in matched_node_holes)
            or (cid == "UNATTRIBUTED" and not h["req"])
        ]
        matched_hole_ids = {h["hole_id"] for h in matched_holes}
        matched_violations = [
            v for v in violations_list
            if (cid in (v.get("req_clauses") or []))
            or (v.get("hole_id") in matched_hole_ids)
        ]
        clause_static = (
            "fail" if matched_violations else static_verdict
        )
        clauses_out.append({
            "clause_id": cid,
            "description": clause_descriptions.get(cid, ""),
            "contract_properties": list(contract_prop_ids),
            "cir_nodes": [
                {
                    "fqn": n["fqn"],
                    "module": n["module"],
                    "function": n["function"],
                    "sid": n["sid"],
                    "structured_id": n["structured_id"],
                    "kind": n["kind"],
                    "hole_id": n["hole_id"],
                }
                for n in matched_nodes
            ],
            "holes": [
                {
                    "hole_id": h["hole_id"],
                    "sid": h["sid"],
                    "rust_file": h["rust_file"],
                    "start_line": h["start_line"],
                    "end_line": h["end_line"],
                    "rust_span": h["rust_span"],
                }
                for h in matched_holes
            ],
            "model_verdict": model_verdict,
            "static_verdict": clause_static,
            "violations": matched_violations,
        })

        # Emit flat traceability links for every matched node/hole.
        emitted_holes: set[str] = set()
        for n in matched_nodes:
            hid = n.get("hole_id")
            hrec = hole_by_id.get(hid) if hid else None
            if hid:
                emitted_holes.add(hid)
            links_out.append({
                "clause_id": cid,
                "contract_property_ids": list(contract_prop_ids),
                "cir_node": n["fqn"],
                "structured_id": n["structured_id"],
                "hole_id": hid,
                "rust_location": hrec["rust_span"] if hrec else None,
                "model_verdict": model_verdict,
                "static_verdict": clause_static,
            })
        for h in matched_holes:
            if h["hole_id"] in emitted_holes:
                continue
            fn_label = h.get("function") or ""
            mod_label = h.get("module") or ""
            if "::" in fn_label:
                cir_ref = f"{fn_label}::{h.get('sid')}"
            elif mod_label:
                cir_ref = f"{mod_label}::{fn_label}::{h.get('sid')}"
            else:
                cir_ref = f"{fn_label}::{h.get('sid')}"
            links_out.append({
                "clause_id": cid,
                "contract_property_ids": list(contract_prop_ids),
                "cir_node": cir_ref,
                "structured_id": None,
                "hole_id": h["hole_id"],
                "rust_location": h["rust_span"],
                "model_verdict": model_verdict,
                "static_verdict": clause_static,
            })

    return {
        "schema": "concir-traceability-manifest/1",
        "status": status,
        "program": program_name,
        "model_verification": {
            "outcome": model_verdict,
            "complete": (model_report or {}).get("complete"),
            "properties": (model_report or {}).get("properties", []),
        },
        "static_verification": static_report.as_dict() if static_report else None,
        "dynamic_conformance": dynamic_report,
        "escalations": list(escalations),
        "hole_repair_rounds": list(hole_rounds),
        "clauses": clauses_out,
        "links": links_out,
    }


def run_e2e_pipeline(
    *,
    requirements: str,
    contract: dict[str, Any],
    client: ConcirClient,
    cir_provider: CandidateProvider,
    hole_provider: CandidateProvider | None = None,
    out_dir: Path | str,
    static_verifier: CompositeStaticVerifier | None = None,
    max_cir_rounds: int = 4,
    max_hole_rounds: int = 3,
    max_escalations: int = 2,
    engine: str = "petri",
    initial_program: Path | str | None = None,
    run_summarize: bool = False,
    run_dynamic_conform: bool = False,
    native_runs: int = 10,
    miri_seeds: int = 0,
    requirements_clauses: dict[str, str] | None = None,
    codegen_fn: Callable[[Path, Path], dict[str, Any]] | None = None,
) -> E2EResult:
    """Run the complete traceable bi-level verification pipeline (Loops A, B, and C)."""

    run_dir = _exclusive_run_dir(Path(out_dir).expanduser().resolve())
    contract_path = run_dir / "contract.json"
    contract_path.write_text(
        json.dumps(contract, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    hole_prov = hole_provider or cir_provider
    verifier = static_verifier or CompositeStaticVerifier([
        AstHolePurityPlugin(),
        SpnMirRefinementPlugin(client=client),
    ])

    def _run_codegen(prog: Path, skel: Path) -> dict[str, Any]:
        if codegen_fn is not None:
            return codegen_fn(prog, skel)
        return default_codegen(prog, skel, binary=client.binary)

    result = E2EResult(status="failed", out_dir=str(run_dir))

    escalation_feedback: dict[str, Any] | None = None
    last_verified_program_dict: dict[str, Any] | None = None
    last_verified_text: str | None = None
    last_trace_map: dict[str, Any] | None = None
    last_codegen_filled: dict[str, Any] | None = None
    last_static_report: StaticVerificationReport | None = None
    use_initial = initial_program

    def _finalize(status: str, error: str | None = None) -> E2EResult:
        result.status = status
        result.error = error
        manifest = build_traceability_manifest(
            status=status,
            requirements=requirements,
            contract=contract,
            program_dict=last_verified_program_dict,
            trace_map=last_trace_map,
            codegen_filled=last_codegen_filled,
            model_report=result.model_report,
            static_report=last_static_report,
            dynamic_report=result.dynamic_report,
            escalations=result.escalations,
            hole_rounds=result.hole_rounds,
            requirements_clauses=requirements_clauses,
        )
        manifest_path = run_dir / "traceability_manifest.json"
        manifest_path.write_text(
            json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        result.manifest = manifest
        result.manifest_path = str(manifest_path)
        (run_dir / "e2e_result.json").write_text(
            json.dumps(result.as_dict(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        return result

    # Outer Loop C: cross-layer escalation epochs (0 .. max_escalations inclusive).
    for epoch in range(max_escalations + 1):
        epoch_dir = run_dir / f"epoch-{epoch}"
        epoch_dir.mkdir(parents=True, exist_ok=True)

        # ---- Stage 1/2: Loop A (CIR/SCIR generation + verification) ----
        verified_cir_path: Path | None = None
        verified_program_dict: dict[str, Any] | None = None
        epoch_trace_map: dict[str, Any] | None = None
        epoch_trace_map_path: Path | None = None
        cir_feedback: dict[str, Any] | None = escalation_feedback
        previous_cir_text: str | None = last_verified_text

        for cir_attempt in range(1, max(1, max_cir_rounds) + 1):
            attempt_dir = epoch_dir / f"cir-{cir_attempt}"
            attempt_dir.mkdir(parents=True, exist_ok=True)
            round_rec: dict[str, Any] = {
                "epoch": epoch,
                "attempt": cir_attempt,
                "escalated": escalation_feedback is not None and cir_attempt == 1,
            }
            result.cir_rounds.append(round_rec)

            if use_initial is not None:
                raw_text = Path(use_initial).expanduser().resolve().read_text(encoding="utf-8")
                round_rec["source"] = "initial"
                use_initial = None
            else:
                req = CandidateRequest(
                    requirements=requirements,
                    contract=contract,
                    feedback=render_feedback(cir_feedback) if cir_feedback else None,
                    attempt=cir_attempt,
                    previous_candidate=previous_cir_text,
                    current_program=(
                        json.dumps(last_verified_program_dict, ensure_ascii=False)
                        if last_verified_program_dict is not None
                        else None
                    ),
                )
                resp = cir_provider.propose(req)
                round_rec["source"] = resp.source
                if resp.error:
                    round_rec["result"] = "provider_error"
                    round_rec["error"] = resp.error
                    cir_feedback = {"stage": "cir_provider", "error": resp.error}
                    continue
                raw_text = resp.text

            (attempt_dir / "raw.txt").write_text(raw_text, encoding="utf-8")
            previous_cir_text = raw_text

            try:
                parsed = json.loads(extract_json(raw_text))
            except json.JSONDecodeError as exc:
                round_rec["result"] = "parse_error"
                cir_feedback = {"stage": "parse", "error": f"invalid JSON: {exc}"}
                continue
            if not isinstance(parsed, dict):
                round_rec["result"] = "parse_error"
                cir_feedback = {"stage": "parse", "error": "candidate must be a JSON object"}
                continue

            raw_cir_path = attempt_dir / "candidate.cir.json"
            raw_cir_path.write_text(
                json.dumps(parsed, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
            active_cir_path = raw_cir_path
            active_program_dict = parsed

            # Lower SCIR v4.0 structured_body -> flat CIR + TraceMap.
            if _has_structured_body(parsed):
                lowered_path = attempt_dir / "lowered.cir.json"
                tm_path = attempt_dir / "trace_map.json"
                lower_res: ConcirResult = client.lower(
                    raw_cir_path, out_path=lowered_path, trace_map_path=tm_path
                )
                round_rec["lower_status"] = lower_res.status
                if lower_res.kind != "semantic":
                    if lower_res.kind in ("usage_error", "protocol_error"):
                        cir_feedback = {
                            "stage": "lower",
                            "error": lower_res.error or lower_res.stderr,
                        }
                        continue
                    return _finalize("tool_error", lower_res.error or "lower failed")
                if lower_res.status != "pass":
                    round_rec["result"] = f"lower_{lower_res.status}"
                    cir_feedback = {
                        "stage": "lower",
                        "status": lower_res.status,
                        "payload": lower_res.payload,
                    }
                    continue
                if lowered_path.is_file():
                    active_cir_path = lowered_path
                    active_program_dict = json.loads(lowered_path.read_text(encoding="utf-8"))
                elif isinstance(lower_res.payload, dict) and "modules" in lower_res.payload:
                    lowered_path.write_text(
                        json.dumps(lower_res.payload, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8",
                    )
                    active_cir_path = lowered_path
                    active_program_dict = lower_res.payload
                if tm_path.is_file():
                    epoch_trace_map = json.loads(tm_path.read_text(encoding="utf-8"))
                    epoch_trace_map_path = tm_path

            # Static check
            check_res = client.check(active_cir_path)
            round_rec["check_status"] = check_res.status
            if check_res.kind != "semantic":
                if check_res.kind in ("usage_error", "protocol_error"):
                    round_rec["result"] = "check_schema_error"
                    cir_feedback = {
                        "stage": "check",
                        "error": check_res.error or check_res.stderr,
                    }
                    continue
                return _finalize("tool_error", check_res.error or "check failed")
            if check_res.status != "valid":
                round_rec["result"] = "check_invalid"
                cir_feedback = build_check_feedback(check_res)
                continue

            # Support check
            sup_res = client.support(active_cir_path)
            round_rec["support_status"] = sup_res.status
            if sup_res.kind != "semantic":
                return _finalize("tool_error", sup_res.error or "support failed")
            if sup_res.status != "supported":
                round_rec["result"] = "unsupported"
                cir_feedback = {
                    "stage": "support",
                    "status": sup_res.status,
                    "unsupported": sup_res.unsupported,
                }
                continue

            # Optional modular lock-cycle summary check
            if run_summarize:
                sum_res = client.summarize(
                    active_cir_path, out_path=attempt_dir / "module_summary.json"
                )
                round_rec["summarize_status"] = sum_res.status
                if sum_res.kind == "semantic" and sum_res.status == "fail":
                    round_rec["result"] = "cross_module_lock_cycle"
                    cir_feedback = {
                        "stage": "summarize",
                        "status": "fail",
                        "summary": sum_res.payload,
                    }
                    continue

            # Exhaustive model exploration
            exp_res = client.explore(active_cir_path, contract_path, engine)
            round_rec["explore_outcome"] = exp_res.outcome
            round_rec["explore_complete"] = exp_res.complete
            if exp_res.kind != "semantic":
                return _finalize("tool_error", exp_res.error or "explore failed")

            result.model_report = exp_res.payload
            if exp_res.outcome == "PASS" and exp_res.complete is True:
                round_rec["result"] = "accepted"
                verified_cir_path = active_cir_path
                verified_program_dict = active_program_dict
                break

            round_rec["result"] = f"explore_{exp_res.status}"
            cir_feedback = build_explore_feedback(exp_res)

        if verified_cir_path is None or verified_program_dict is None:
            return _finalize(
                "failed",
                f"Stage 2 CIR verification did not reach complete PASS within {max_cir_rounds} rounds (epoch {epoch})",
            )

        last_verified_program_dict = verified_program_dict
        last_verified_text = json.dumps(verified_program_dict, ensure_ascii=False, indent=2)
        last_trace_map = epoch_trace_map
        result.verified_cir_path = str(verified_cir_path)
        result.trace_map_path = str(epoch_trace_map_path) if epoch_trace_map_path else None

        # ---- Stage 3: Codegen + Initial Hole Fill ----
        skeleton_dir = epoch_dir / "skeleton"
        filled_dir = epoch_dir / "filled"
        try:
            _run_codegen(verified_cir_path, skeleton_dir)
        except Exception as exc:  # noqa: BLE001
            return _finalize("tool_error", f"codegen failed: {exc}")

        result.skeleton_dir = str(skeleton_dir)
        result.filled_dir = str(filled_dir)

        holes = holes_of(skeleton_dir)
        current_fills: dict[str, str] = {}
        escalate_now: EscalationEvent | None = None

        if holes:
            prompt_text = build_hole_fill_prompt(skeleton_dir, requirements)
            hole_req = CandidateRequest(
                requirements=prompt_text,
                contract=contract,
                feedback=None,
                attempt=1,
                current_program=last_verified_text,
            )
            hole_resp = hole_prov.propose(hole_req)
            fill_rec: dict[str, Any] = {
                "epoch": epoch,
                "round": 0,
                "kind": "initial_fill",
                "target_holes": [h["id"] for h in holes],
                "source": hole_resp.source,
            }
            result.hole_rounds.append(fill_rec)
            if hole_resp.error:
                return _finalize("failed", f"hole provider error: {hole_resp.error}")

            parsed_fills, esc = parse_hole_reply(hole_resp.text)
            if esc is not None:
                fill_rec["result"] = "escalate"
                fill_rec["escalation"] = esc
                escalate_now = EscalationEvent(
                    index=len(result.escalations) + 1,
                    source="hole_provider_explicit",
                    reason=esc["reason"],
                    module=esc.get("module"),
                    offending_holes=[h["id"] for h in holes],
                    req_clauses=sorted({str(r) for h in holes for r in (h.get("req") or [])}),
                )
            elif parsed_fills is None:
                return _finalize("failed", "hole provider returned invalid JSON")
            else:
                current_fills = parsed_fills
                fill_rec["result"] = "filled"
                fill_rec["applied"] = sorted(current_fills.keys())

            if escalate_now is not None:
                result.escalations.append(escalate_now.as_dict())
                if epoch < max_escalations:
                    escalation_feedback = {
                        "stage": "cross_layer_escalation",
                        "escalation": escalate_now.as_dict(),
                        "instruction": (
                            "Hole implementation requested a CIR/SCIR architecture revision. "
                            "Update the CIR program to address the escalation reason while "
                            "preserving all contract properties."
                        ),
                    }
                    continue
                return _finalize(
                    "failed",
                    f"cross-layer escalation exhausted ({max_escalations}): {escalate_now.reason}",
                )

            fill_info = fill_holes_marked(skeleton_dir, filled_dir, current_fills)
            cmap_path = Path(fill_info["codegen_filled_path"])
            last_codegen_filled = json.loads(cmap_path.read_text(encoding="utf-8"))
        else:
            if filled_dir.exists():
                shutil.rmtree(filled_dir)
            shutil.copytree(skeleton_dir, filled_dir)
            cmap_path = filled_dir / "codegen_filled.json"
            last_codegen_filled = augment_codegen_map_with_hole_ranges(
                skeleton_dir, filled_dir / "src/main.rs", {}, out_path=cmap_path
            )

        result.fills = dict(current_fills)

        # ---- Stage 4: Static Verification + Loop B (Hole-Local Repair) ----
        static_report = verifier.verify(
            skeleton_dir=skeleton_dir,
            filled_dir=filled_dir,
            spec_path=verified_cir_path,
            contract_path=contract_path,
            codegen_map_path=cmap_path,
            fills=current_fills,
        )
        last_static_report = static_report
        result.static_report = static_report.as_dict()
        if static_report.metadata.get("codegen_filled_path"):
            cf_path = Path(static_report.metadata["codegen_filled_path"])
            if cf_path.is_file():
                last_codegen_filled = json.loads(cf_path.read_text(encoding="utf-8"))

        # Loop B: Hole-local repair when violations are attributable to holes.
        repair_round = 0
        while (
            static_report.status in ("fail_hole", "fail_refinement")
            and static_report.offending_holes
            and repair_round < max_hole_rounds
        ):
            repair_round += 1
            offending = list(static_report.offending_holes)
            repair_prompt = build_hole_repair_prompt(
                skeleton_dir,
                requirements,
                offending,
                current_fills,
                static_report.violations,
            )
            repair_req = CandidateRequest(
                requirements=repair_prompt,
                contract=contract,
                feedback=json.dumps(
                    [v.as_dict() for v in static_report.violations],
                    ensure_ascii=False,
                    indent=2,
                ),
                attempt=repair_round,
                previous_candidate=json.dumps(
                    {h: current_fills.get(h, "") for h in offending}, ensure_ascii=False
                ),
                current_program=last_verified_text,
            )
            repair_resp = hole_prov.propose(repair_req)
            rep_rec: dict[str, Any] = {
                "epoch": epoch,
                "round": repair_round,
                "kind": "hole_repair",
                "target_holes": offending,
                "source": repair_resp.source,
            }
            result.hole_rounds.append(rep_rec)
            if repair_resp.error:
                rep_rec["result"] = "provider_error"
                rep_rec["error"] = repair_resp.error
                break

            parsed_patch, esc = parse_hole_reply(repair_resp.text)
            if esc is not None:
                rep_rec["result"] = "escalate"
                rep_rec["escalation"] = esc
                escalate_now = EscalationEvent(
                    index=len(result.escalations) + 1,
                    source="hole_provider_explicit",
                    reason=esc["reason"],
                    module=esc.get("module"),
                    offending_holes=offending,
                    req_clauses=sorted({
                        r for v in static_report.violations for r in v.req_clauses
                    }),
                    violations=[v.as_dict() for v in static_report.violations],
                )
                break

            if parsed_patch is None:
                rep_rec["result"] = "parse_error"
                continue

            # Update ONLY the offending holes; passing holes remain frozen!
            updated: list[str] = []
            for hid in offending:
                if hid in parsed_patch:
                    current_fills[hid] = parsed_patch[hid]
                    updated.append(hid)
            rep_rec["updated_holes"] = updated
            result.fills = dict(current_fills)

            fill_info = fill_holes_marked(skeleton_dir, filled_dir, current_fills)
            cmap_path = Path(fill_info["codegen_filled_path"])
            last_codegen_filled = json.loads(cmap_path.read_text(encoding="utf-8"))

            static_report = verifier.verify(
                skeleton_dir=skeleton_dir,
                filled_dir=filled_dir,
                spec_path=verified_cir_path,
                contract_path=contract_path,
                codegen_map_path=cmap_path,
                fills=current_fills,
            )
            last_static_report = static_report
            result.static_report = static_report.as_dict()
            rep_rec["verify_status"] = static_report.status
            if static_report.status == "pass":
                rep_rec["result"] = "repaired"
                break
            rep_rec["result"] = static_report.status

        # Evaluate Stage 4 post-Loop-B outcome.
        if escalate_now is None and static_report.status == "pass":
            if run_dynamic_conform:
                traces = collect_traces(
                    filled_dir,
                    native_runs=native_runs,
                    miri_seeds=miri_seeds,
                    run_miri=miri_seeds > 0,
                    calls_dir=epoch_dir / "dynamic-traces",
                )
                dyn = conform_all(verified_cir_path, traces, binary=client.binary)
                result.dynamic_report = dyn
                if dyn.get("violation", 0) > 0:
                    return _finalize(
                        "failed",
                        f"dynamic conformance reported {dyn['violation']} violation(s)",
                    )
            return _finalize("verified")

        if escalate_now is None and static_report.status == "unknown":
            return _finalize(
                "unknown",
                static_report.error or "static verification returned unknown",
            )

        # ---- Loop C: Cross-Layer Escalation back to Stage 2 ----
        if escalate_now is None:
            if static_report.offending_holes:
                src = "hole_repair_exhausted"
                reason = (
                    f"hole-local repair exhausted {max_hole_rounds} round(s) with "
                    f"unresolved violations in holes {static_report.offending_holes}"
                )
            elif static_report.status == "fail_skeleton":
                src = "skeleton_fail"
                reason = "skeleton integrity check failed outside holes"
            else:
                src = "refinement_non_hole"
                reason = (
                    static_report.error
                    or f"static verification failed with status '{static_report.status}'"
                )
            escalate_now = EscalationEvent(
                index=len(result.escalations) + 1,
                source=src,
                reason=reason,
                module=None,
                offending_holes=list(static_report.offending_holes),
                req_clauses=sorted({
                    r for v in static_report.violations for r in v.req_clauses
                }),
                violations=[v.as_dict() for v in static_report.violations],
            )

        result.escalations.append(escalate_now.as_dict())
        if epoch < max_escalations:
            escalation_feedback = {
                "stage": "cross_layer_escalation",
                "escalation": escalate_now.as_dict(),
                "instruction": (
                    "Post-codegen static verification could not be resolved by hole-local repair. "
                    "Revise the CIR/SCIR architecture to address the violations/escalation."
                ),
            }
            continue

        return _finalize(
            "failed",
            f"pipeline failed after {max_escalations} escalation(s): {escalate_now.reason}",
        )

    return _finalize("failed", "pipeline terminated without verification")
