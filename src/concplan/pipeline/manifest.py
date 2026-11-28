"""Traceability manifest datatypes and assembly for E2E verification."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any

from ..code_verify.static_verify import StaticVerificationReport
from ..core.concir_client import ConcirClient
from ..formalize import explain_contract_data

CLAUSE_RE = re.compile(r"\b(R\d+)\b")


@dataclass
class EscalationEvent:
    index: int
    source: str
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
    formalization: dict[str, Any] | None = None
    nl_explanations: dict[str, Any] | None = None
    vacuity_status: dict[str, Any] | None = None
    modular_reports: dict[str, Any] | None = None
    req_net_check: dict[str, Any] | None = None
    design_refinement: dict[str, Any] | None = None
    error: str | None = None

    @property
    def ok(self) -> bool:
        return self.status == "verified"

    def as_dict(self) -> dict[str, Any]:
        out = {
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
            "formalization": self.formalization,
            "nl_explanations": self.nl_explanations,
            "vacuity_status": self.vacuity_status,
            "modular_reports": self.modular_reports,
            "error": self.error,
        }
        if self.req_net_check is not None:
            out["req_net_check"] = self.req_net_check
        if self.design_refinement is not None:
            out["design_refinement"] = self.design_refinement
        return out


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
    formalization: dict[str, Any] | None = None,
    nl_explanations: dict[str, Any] | None = None,
    req_net_check: dict[str, Any] | None = None,
    design_refinement: dict[str, Any] | None = None,
    client: ConcirClient | None = None,
) -> dict[str, Any]:
    """Assemble ``concir-traceability-manifest/1`` linking clauses -> contract -> CIR -> holes -> Rust -> verdicts."""
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

    tm_entries: dict[str, dict[str, Any]] = {}
    if isinstance(trace_map, dict):
        for fqn, entry in (trace_map.get("entries") or {}).items():
            if isinstance(entry, dict):
                tm_entries[str(fqn)] = entry

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

    hole_records: list[dict[str, Any]] = []
    hole_ranges = (codegen_filled or {}).get("hole_ranges") or {}
    for hole in (codegen_filled or {}).get("holes", []) or []:
        if not isinstance(hole, dict) or not hole.get("id"):
            continue
        hid = str(hole["id"])
        rng = hole_ranges.get(hid) or {}
        rfile = str(rng.get("file") or hole.get("file") or "src/main.rs")
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

    type_hole_records: list[dict[str, Any]] = []
    type_hole_ranges = (codegen_filled or {}).get("type_hole_ranges") or {}
    for th in (codegen_filled or {}).get("type_holes", []) or []:
        if not isinstance(th, dict) or not th.get("id"):
            continue
        th_id = str(th["id"])
        rng = type_hole_ranges.get(th_id) or {}
        rfile = str(rng.get("file") or th.get("file") or "src/domain.rs")
        sline = rng.get("start_line", th.get("line"))
        eline = rng.get("end_line", sline)
        rust_loc = f"{rfile}:{sline}-{eline}" if sline is not None else rfile
        reqs = [str(r) for r in (th.get("req") or [])]
        for r in reqs:
            clause_descriptions.setdefault(r, "")
        type_hole_records.append({
            "type_hole_id": th_id,
            "type_name": th.get("type_name"),
            "module": th.get("module"),
            "rust_file": rfile,
            "start_line": sline,
            "end_line": eline,
            "rust_span": rust_loc,
            "req": reqs,
        })

    vacuous_props: list[dict[str, Any]] = []
    for prop in (model_report or {}).get("properties", []) or []:
        if isinstance(prop, dict) and (prop.get("vacuous") or prop.get("vacuous_antecedents")):
            vacuous_props.append({
                "id": prop.get("id"),
                "outcome": prop.get("outcome"),
                "vacuous": True,
                "vacuous_antecedents": list(prop.get("vacuous_antecedents") or []),
            })
    vacuity_status = {"has_vacuous": bool(vacuous_props), "vacuous_properties": vacuous_props}

    explanations_payload = (
        nl_explanations
        if nl_explanations is not None
        else (explain_contract_data(contract, client=client).as_dict() if isinstance(contract, dict) else None)
    )
    modular_reports = (static_report.metadata or {}).get("modular_reports") if static_report is not None else None
    model_verdict = (model_report or {}).get("outcome") or "NOT_RUN"
    static_verdict = static_report.status if static_report else "not_run"
    violations_list = [v.as_dict() for v in (static_report.violations if static_report else [])]

    clauses_out: list[dict[str, Any]] = []
    links_out: list[dict[str, Any]] = []
    all_clause_ids = sorted(clause_descriptions.keys())
    if not all_clause_ids and (cir_nodes or hole_records or type_hole_records):
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
            if (cid in h["req"]) or (h["hole_id"] in matched_node_holes) or (cid == "UNATTRIBUTED" and not h["req"])
        ]
        matched_type_holes = [
            th for th in type_hole_records
            if (cid in th["req"]) or (cid == "UNATTRIBUTED" and not th["req"])
        ]
        matched_hole_ids = {h["hole_id"] for h in matched_holes} | {th["type_hole_id"] for th in matched_type_holes}
        matched_violations = [
            v for v in violations_list
            if (cid in (v.get("req_clauses") or [])) or (v.get("hole_id") in matched_hole_ids)
        ]
        clause_static = "fail" if matched_violations else static_verdict
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
            "type_holes": [
                {
                    "type_hole_id": th["type_hole_id"],
                    "type_name": th["type_name"],
                    "rust_file": th["rust_file"],
                    "start_line": th["start_line"],
                    "end_line": th["end_line"],
                    "rust_span": th["rust_span"],
                }
                for th in matched_type_holes
            ],
            "model_verdict": model_verdict,
            "static_verdict": clause_static,
            "violations": matched_violations,
        })

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
        for th in matched_type_holes:
            mod_label = th.get("module") or "main"
            tname = th.get("type_name") or th["type_hole_id"]
            links_out.append({
                "clause_id": cid,
                "contract_property_ids": list(contract_prop_ids),
                "cir_node": f"{mod_label}::type::{tname}",
                "structured_id": None,
                "hole_id": th["type_hole_id"],
                "rust_location": th["rust_span"],
                "model_verdict": model_verdict,
                "static_verdict": clause_static,
            })

    out_manifest = {
        "schema": "concir-traceability-manifest/1",
        "status": status,
        "program": program_name,
        "formalization": formalization,
        "nl_explanations": explanations_payload,
        "vacuity_status": vacuity_status,
        "type_holes": type_hole_records,
        "modular_reports": modular_reports,
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
    if req_net_check is not None:
        out_manifest["req_net_check"] = req_net_check
    if design_refinement is not None:
        out_manifest["design_refinement"] = design_refinement
    return out_manifest
