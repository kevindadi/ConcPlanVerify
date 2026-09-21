"""Deterministic RESULTS.md generator.

`python -m cir_workflow results --batch ... [--batch ...]` renders the
paper-facing table from the committed JSON artefacts (batch SUMMARY.json files,
expert labels, extraction CELLS.json, Track D, scale). RESULTS.md is
**generated**: never edit it by hand (see the header it writes with the exact
command and the sha256 of every input).
"""

from __future__ import annotations

import hashlib
import json
import shlex
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml", "A3_local", "A3_whole",
        "A3_tiered")
RUST_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml")


def sha256_file(path: Path | str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_summaries(paths: list[Path]) -> list[dict[str, Any]]:
    out = []
    for path in paths:
        path = Path(path)
        if path.is_dir():
            path = path / "SUMMARY.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        if "reps" in data:
            for rep in data["reps"]:
                merged = dict(rep)
                merged.setdefault("binary_sha256", data.get("binary_sha256"))
                merged.setdefault("protocol_sha256", data.get("protocol_sha256"))
                out.append(merged)
        else:
            out.append(data)
    return out


def expert_index(paths: list[Path]
                 ) -> tuple[dict[str, dict], dict[str, dict], list[dict]]:
    """Return (per-(task|arm), per-(task|arm|rep), per-candidate) expert maps.

    Per-candidate labels (rubric v2) carry `cells`; the (task|arm) map is the
    union (bug_present=yes if any candidate is yes, design_preserved=no if any
    candidate loses the design).
    """

    agg: dict[str, dict] = {}
    cell: dict[str, dict] = {}
    candidates: list[dict] = []
    for path in paths:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        labels = data.get("labels", data if isinstance(data, list) else [])
        for item in labels:
            candidates.append(item)
            cells = item.get("cells")
            if cells:
                for c in cells:
                    task, arm, rep = (c if isinstance(c, list)
                                      else (c.get("task"), c.get("arm"), c.get("rep")))
                    key = f"{task}|{arm}"
                    cell[f"{key}|{rep}"] = item
                    slot = agg.setdefault(key, {"task": task, "arm": arm,
                                                "bug_present": "no",
                                                "design_preserved": "yes"})
                    if item.get("bug_present") == "yes":
                        slot["bug_present"] = "yes"
                    elif item.get("bug_present") == "unsure" and slot["bug_present"] == "no":
                        slot["bug_present"] = "unsure"
                    if item.get("design_preserved") == "no":
                        slot["design_preserved"] = "no"
            else:
                key = f"{item['task']}|{item['arm']}"
                agg[key] = item
                for rep in range(3):
                    cell[f"{key}|{rep}"] = item
    return agg, cell, candidates


def extract_index(dirs: list[Path]) -> dict[str, dict[str, Any]]:
    idx: dict[str, dict[str, Any]] = {}
    for d in dirs:
        cells_file = Path(d) / "CELLS.json"
        if cells_file.is_file():
            data = json.loads(cells_file.read_text(encoding="utf-8"))
            for item in data.get("cells", data if isinstance(data, list) else []):
                idx[f"{item['task']}|{item['arm']}|{item['rep']}"] = item
            continue
        for result in Path(d).glob("*/extraction_result.json"):
            data = json.loads(result.read_text(encoding="utf-8"))
            idx[result.parent.name] = data
    return idx


def _extract_verdict(rec: dict[str, Any] | None) -> str:
    if not rec:
        return "—"
    if rec.get("contract_invalid"):
        return "INVALID"
    if not rec.get("validated", rec.get("extract_validated")):
        return "—"
    verdict = rec.get("model_verdict")
    return str(verdict) if verdict else "—"


def _bug(rec: dict[str, Any], expert: dict[str, Any] | None) -> tuple[bool, list[str]]:
    sources: list[str] = []
    oracle = rec.get("oracle") or {}
    if oracle.get("behavior_status") == "hang":
        sources.append("behavior")
    if oracle.get("verify_pass") is False:
        sources.append("model")
    if oracle.get("conform") == "FAIL":
        sources.append("conform")
    if (rec.get("notes") or {}).get("bug_present_reason") == "by_construction":
        sources.append("by_construction")
    if expert and expert.get("bug_present") == "yes":
        sources.append("expert")
    return bool(sources), sources


def _counts_display(counter: Counter, n: int, empty: str = "—") -> str:
    if not counter:
        return empty
    total = sum(counter.values())
    parts = []
    for key, count in sorted(counter.items(), key=lambda kv: -kv[1]):
        parts.append(f"{key} {count}/{total}")
    return " · ".join(parts)


def _mean_range(values: list[float]) -> str:
    vals = [v for v in values if v is not None]
    if not vals:
        return "—"
    mean = sum(vals) / len(vals)
    lo, hi = min(vals), max(vals)
    return f"{mean:.0f}" if lo == hi else f"{mean:.0f}±{hi - lo:.0f}"


def _a3_decisions(rec: dict[str, Any]) -> Counter:
    d = rec.get("decision_distribution")
    if d:
        return Counter(d)
    path = rec.get("final_cir")
    if path:
        result_path = Path(path).parent / "result.json"
        if result_path.is_file():
            data = json.loads(result_path.read_text(encoding="utf-8"))
            return Counter(v.get("decision") for v in data.get("versions", []))
    return Counter()


def aggregate(summaries: list[dict[str, Any]], expert_agg: dict[str, dict],
              expert_cell: dict[str, dict], extract: dict[str, dict],
              reclass: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """One aggregated row per (task, arm) across all reps."""

    buckets: dict[tuple[str, str], list[tuple[int, dict]]] = {}
    for summary in summaries:
        rep = summary.get("rep", 0)
        for task in summary.get("tasks", []):
            for arm, rec in (task.get("arms") or {}).items():
                buckets.setdefault((task["task"], arm), []).append((rep, rec))

    rows = []
    for (task, arm), rep_recs in buckets.items():
        key = f"{task}|{arm}"
        overridden = reclass.get("cells", {}).get(key) if reclass else None
        accepted_count = fa_count = 0
        fa_sources: Counter = Counter()
        rounds, tokens, llm_ms, tool_ms, verify_ms = [], [], [], [], []
        build_c: Counter = Counter()
        behavior_c: Counter = Counter()
        miri_c: Counter = Counter()
        expert_c: Counter = Counter()
        extract_c: Counter = Counter()
        conform_c: Counter = Counter()
        decisions: Counter = Counter()
        escalated = 0
        for rep, rec in rep_recs:
            escalated += 1 if rec.get("escalated") else 0
            accepted = bool(rec.get("accepted"))
            if overridden is not None and arm in ("A0_direct", "A1_self_iter"):
                accepted = bool(overridden.get("accepted"))
            conn = rec.get("consumption") or {}
            if accepted:
                accepted_count += 1
                if overridden and overridden.get("accepted_round"):
                    rounds.append(overridden["accepted_round"])
                elif rec.get("accepted_round"):
                    rounds.append(rec["accepted_round"])
                tokens.append(conn.get("total_tokens"))
                llm_ms.append(conn.get("llm_wall_ms"))
                tool_ms.append(conn.get("tool_wall_ms"))
            oracle = rec.get("oracle") or {}
            exp = expert_cell.get(f"{key}|{rep}") or expert_agg.get(key)
            bug, sources = _bug(rec, exp)
            if overridden and overridden.get("by_construction"):
                bug = True
                sources.append("by_construction")
            if rec.get("extract_validated") or (rec.get("oracle") or {}).get("extract") == "FAIL":
                pass
            if accepted and bug:
                fa_count += 1
                for source in set(sources):
                    fa_sources[source] += 1
            if oracle.get("build_ok") is not None:
                build_c[str(oracle.get("build_ok"))] += 1
            if oracle.get("behavior_status"):
                behavior_c[oracle["behavior_status"]] += 1
            if oracle.get("miri_detected") is not None:
                miri_c["detected" if oracle["miri_detected"] else "clean"] += 1
            if oracle.get("conform"):
                conform_c[str(oracle["conform"])] += 1
            if exp:
                expert_c[exp.get("bug_present", "?")] += 1
            ext = extract.get(f"{key}|{rep}")
            if ext:
                extract_c[_extract_verdict(ext)] += 1
            evidence = oracle.get("evidence") or {}
            if evidence.get("wall_ms") is not None:
                verify_ms.append(evidence.get("wall_ms"))
            if arm in ("A3_local", "A3_whole"):
                decisions += _a3_decisions(rec)
        n = len(rep_recs)
        rows.append({
            "task": task, "arm": arm, "n": n,
            "accepted": accepted_count, "accepted_display": f"{accepted_count}/{n}",
            "round": _mean_range(rounds), "tokens": _mean_range(tokens),
            "llm_ms": _mean_range(llm_ms), "tool_ms": _mean_range(tool_ms),
            "verify_ms": _mean_range(verify_ms),
            "build": _counts_display(build_c, n),
            "behavior": _counts_display(behavior_c, n),
            "miri": _counts_display(miri_c, n),
            "expert": _counts_display(expert_c, n),
            "extract": _counts_display(extract_c, n),
            "conform": _counts_display(conform_c, n),
            "false_accept": fa_count, "fa_display": f"{fa_count}/{n}",
            "fa_sources": dict(fa_sources), "decisions": dict(decisions),
            "escalated": escalated,
        })
    return rows


def render(rows: list[dict[str, Any]], expert_agg: dict[str, dict],
           candidates: list[dict], extract: dict[str, dict],
           trackd: dict[str, Any] | None,
           scale: dict[str, Any] | None, header: dict[str, Any],
           deviations: list[str], mutation_v1: dict | None = None,
           postedit_v1: dict | None = None) -> str:
    L: list[str] = ["# RESULTS — ConcIR repair/extraction benchmark", "",
                    "> **Generated file — do not edit by hand.** Regenerate with:",
                    "> ```", f"> {header['command']}", "> ```", ""]
    L += ["## Provenance", ""]
    for label, value in header.get("inputs", []):
        L.append(f"- {label}: `{value}`")
    for label, digest in header.get("shas", []):
        L.append(f"- {label} sha256: `{digest}`")
    L.append("")

    L += ["## 1. Main table", "",
          "| task | arm | accepted | round | tokens | llm_ms | tool_ms | verify_ms | "
          "false_accept | build | behavior | miri | conform | expert | extract |",
          "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | "
          "--- | --- | --- |"]
    for r in rows:
        L.append(f"| {r['task']} | {r['arm']} | {r['accepted_display']} | {r['round']} | "
                 f"{r['tokens']} | {r['llm_ms']} | {r['tool_ms']} | {r['verify_ms']} | "
                 f"{r['fa_display']} | {r['build']} | {r['behavior']} | {r['miri']} | "
                 f"{r['conform']} | {r['expert']} | {r['extract']} |")
    L += ["", "`accepted`/`false_accept` are k/n over repeats; `round`/`tokens`/`*_ms` "
              "are mean±range over accepted cells; oracle columns are per-rep counts. "
              "`verify_ms` is the ConcIR `explore`/`conform` time. **A0's `accepted` only "
              "means it produced a parseable program** (a single round with no "
              "rejection); it is not a correctness claim. `extract` is the validated "
              "extraction verdict (`INVALID` = contract could not be evaluated).", ""]

    L += ["### Per-arm aggregates", "",
          "| arm | cells | accepted | accept_rate | false_accept | conform_pass_rate | "
          "escalation_rate | tokens/correct_accept | by source |",
          "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in ARMS:
        cells = [r for r in rows if r["arm"] == arm]
        if not cells:
            continue
        n = sum(r["n"] for r in cells)
        acc = sum(r["accepted"] for r in cells)
        fa = sum(r["false_accept"] for r in cells)
        src: Counter = Counter()
        for r in cells:
            for k, v in r["fa_sources"].items():
                src[k] += v
        correct = acc - fa
        toks = sum(_sum_tokens(r) for r in cells)
        tca = f"{toks / correct:.0f}" if correct > 0 else "∞"
        conf_n = conf_pass = 0
        for r in cells:
            for label, count in _parse_counts(r["conform"]).items():
                conf_n += count
                if label == "PASS":
                    conf_pass += count
        cpr = f"{conf_pass}/{conf_n} = {conf_pass / conf_n:.2f}" if conf_n else "—"
        esc = sum(r.get("escalated", 0) for r in cells)
        er = f"{esc}/{n} = {esc / n:.2f}" if esc else "—"
        L.append(f"| {arm} | {n} | {acc} | {acc / n:.2f} | {fa} | {cpr} | {er} | {tca} | "
                 f"{dict(src) or '—'} |")
    L += ["", "`tokens/correct_accept = Σ tokens / (accepted − false_accept)`; `∞` when "
              "the denominator is 0. `conform_pass_rate` is over A3 cells with a "
              "conform verdict.", ""]

    L += ["### A3 decision distribution", "",
          "| task | arm | decisions |", "| --- | --- | --- |"]
    for r in rows:
        if r["arm"] in ("A3_local", "A3_whole") and r["decisions"]:
            L.append(f"| {r['task']} | {r['arm']} | {r['decisions']} |")
    L.append("")

    # ---- expert
    L += ["### Expert labels", "",
          "| task | arm | bug_present | design_preserved | design_loss | auto | agree |",
          "| --- | --- | --- | --- | --- | --- | --- |"]
    agree = compared = unsure = design_loss = 0
    disagreements = []
    for key, item in sorted(expert_agg.items()):
        task, arm = key.split("|")
        row = next((r for r in rows if r["task"] == task and r["arm"] == arm), None)
        auto = bool(row and row["false_accept"] > 0)
        dl = bool(row and row["accepted"] > 0 and item.get("design_preserved") == "no")
        design_loss += 1 if dl else 0
        match = None
        if item.get("bug_present") != "unsure" and row:
            compared += 1
            match = bool(item["bug_present"] == "yes") == auto
            agree += 1 if match else 0
        if item.get("bug_present") == "unsure":
            unsure += 1
        L.append(f"| {task} | {arm} | {item.get('bug_present')} | "
                 f"{item.get('design_preserved')} | {dl} | {auto} | {match} |")
        if match is False:
            disagreements.append((task, arm, item.get("bug_present"), auto))
    rate = f"{agree}/{compared} = {agree / compared:.3f}" if compared else "n/a"
    design_loss_by_arm: Counter = Counter()
    for cand in candidates:
        if cand.get("design_preserved") == "no":
            for c in cand.get("cells", []) or []:
                arm = c.get("arm") if isinstance(c, dict) else c[1]
                design_loss_by_arm[arm] += 1
    L += ["", f"Agreement with the automatic oracle: **{rate}** (unsure {unsure}); "
              f"`design_loss` (accepted ∧ design_preserved=no): **{design_loss}** "
              f"{dict(design_loss_by_arm)}.", ""]
    if candidates:
        L += ["#### Expert labels (per candidate)",
              "", "| sha256 | task | arm | kind | bug_present | design_preserved | human | cells |",
              "| --- | --- | --- | --- | --- | --- | --- | --- |"]
        for cand in sorted(candidates, key=lambda c: (c.get("task", ""), c.get("arm", ""))):
            cells = cand.get("cells", []) or []
            human = cand.get("human_label") if cand.get("human_reviewed") else "—"
            L.append(f"| `{str(cand.get('sha256',''))[:12]}` | {cand.get('task')} | "
                     f"{cand.get('arm')} | {cand.get('kind','—')} | "
                     f"{cand.get('bug_present')} | {cand.get('design_preserved')} | "
                     f"{human} | {len(cells)} |")
        L.append("")
        reviewed = [c for c in candidates if c.get("human_reviewed")]
        if reviewed:
            ag_cmp = ag_agree = ag_unsure = 0
            au_cmp = au_agree = 0
            au_disagree = []
            for c in reviewed:
                h = c.get("human_label")
                if h not in ("yes", "no"):
                    continue
                a = c.get("bug_present")
                if a == "unsure":
                    ag_unsure += 1
                else:
                    ag_cmp += 1
                    ag_agree += 1 if (h == "yes") == (a == "yes") else 0
                row = next((r for r in rows if r["task"] == c.get("task")
                            and r["arm"] == c.get("arm")), None)
                if row is not None:
                    auto = row["false_accept"] > 0
                    au_cmp += 1
                    if (h == "yes") == auto:
                        au_agree += 1
                    else:
                        au_disagree.append((c.get("task"), c.get("arm"),
                                            str(c.get("sha256"))[:12], h, auto))
            L += [f"Human review: {len(reviewed)} candidates; agent vs human "
                  f"**{ag_agree}/{ag_cmp}** (agent unsure {ag_unsure}); human vs auto "
                  f"**{au_agree}/{au_cmp}**.", ""]
            if au_disagree:
                L += ["##### Human vs automatic-oracle disagreements", "",
                      "| task | arm | sha | human | auto |", "| --- | --- | --- | --- | --- |"]
                for t, a, sha, h, auto in au_disagree:
                    L.append(f"| {t} | {a} | `{sha}` | {h} | {auto} |")
                L.append("")
    if disagreements:
        L += ["### Oracle disagreements", "",
              "| task | arm | expert bug_present | automatic false_accept |",
              "| --- | --- | --- | --- |"]
        for task, arm, bug, auto in disagreements:
            L.append(f"| {task} | {arm} | {bug} | {auto} |")
        L.append("")

    # ---- extraction
    stages: Counter = Counter()
    reasons: Counter = Counter()
    validated = contract_invalid = 0
    for rec in extract.values():
        stages[rec.get("stage", "?")] += 1
        validated += 1 if rec.get("validated", rec.get("extract_validated")) else 0
        contract_invalid += 1 if rec.get("contract_invalid") else 0
        if rec.get("stage") in ("codegen", "labels", "conform"):
            reasons[rec.get("reason", "unclassified")] += 1
    L += ["### Extraction oracle", "",
          f"- cells: {len(extract)}; **validated: {validated}**; "
          f"contract_invalid: {contract_invalid}",
          f"- stage distribution: `{dict(stages)}`",
          f"- failure reason distribution: `{dict(reasons)}`", "",
          "| task | arm | rep | stage | validated | verdict | reason |",
          "| --- | --- | --- | --- | --- | --- | --- |"]
    for key, rec in sorted(extract.items()):
        task, arm, rep = key.split("|")
        L.append(f"| {task} | {arm} | {rep} | {rec.get('stage')} | "
                 f"{rec.get('validated', rec.get('extract_validated'))} | "
                 f"{_extract_verdict(rec)} | {rec.get('reason','')} |")
    L.append("")

    # ---- conform recall (v1 vs v2)
    if trackd is not None or True:
        pass
    mut_v2 = header.get("mutation")
    post_v2 = header.get("postedit")
    if mut_v2 or postedit_v1:
        L += ["### Conform recall (mutation, v1 vs v2)", "",
              "| op | v1 recall | v2 recall | v2 failure reason |",
              "| --- | --- | --- | --- |"]
        ops_v1 = (mutation_v1 or {}).get("ops", {})
        ops_v2 = (mut_v2 or {}).get("ops", {})
        for op in sorted(set(ops_v1) | set(ops_v2)):
            r1 = ops_v1.get(op, {}).get("conform_recall", "—")
            r2 = ops_v2.get(op, {}).get("conform_recall", "—")
            reason = (mut_v2 or {}).get("reasons", {}).get(op, "—")
            L.append(f"| {op} | {r1} | {r2} | {reason} |")
        L.append("")
    if post_v2 or postedit_v1:
        L += ["### Post-edit drift (v1 vs v2)", "",
              "| edit | v1 conform PASS | v1 drift-only-conform | v2 conform PASS | v2 drift-only-conform |",
              "| --- | --- | --- | --- | --- |"]
        p1 = (postedit_v1 or {}).get("ops", {})
        p2 = (post_v2 or {}).get("ops", {})
        for eid in sorted(set(p1) | set(p2)):
            a = p1.get(eid, {})
            b = p2.get(eid, {})
            b_pass = b.get("conform_pass_real", b.get("conform_pass", "—"))
            b_cells = b.get("real_edits", b.get("cells", "—"))
            b_drift = b.get("drift_only_conform_real",
                            b.get("drift_caught_only_by_conform", "—"))
            L.append(f"| {eid} | {a.get('conform_pass','—')}/{a.get('cells','—')} | "
                     f"{a.get('drift_caught_only_by_conform','—')} | "
                     f"{b_pass}/{b_cells} | {b_drift} |")
        L.append("")

    # ---- Model probe
    mp = header.get("modelprobe")
    if mp:
        L += ["### Model probe (OpenCode Go)", "",
              "| model | arm | cells | accepted | false_accept |",
              "| --- | --- | --- | --- | --- |"]
        for model, v in mp.get("models", {}).items():
            cells = v.get("cells", [])
            for arm in ("A0_direct", "A2_tools_iter_ml", "A3_local"):
                rs = [c for c in cells if c.get("arm") == arm and not c.get("error")]
                acc = sum(1 for c in rs if c.get("accepted"))
                fa = sum(1 for c in rs if c.get("accepted")
                         and (c.get("oracle") or {}).get("bug_present") is True)
                L.append(f"| {model} | {arm} | {len(rs)} | {acc} | {fa} |")
        L += ["", "Not part of the main table; 8 SMOKE tasks, 1 rep, K=4.", ""]

    # ---- Track D
    if trackd:
        records = trackd.get("records") or trackd.get("tasks") or []
        rust_rows, cir_rows = [], []
        for rec in records:
            task = rec.get("task") or rec.get("id")
            for side in ("buggy", "fixed"):
                entry = rec.get(side) if isinstance(rec.get(side), dict) else {}
                if "concir_petri" in entry:
                    petri, interp = entry.get("concir_petri"), entry.get("concir_interp")
                    miri = _miri_counts(entry.get("miri"))
                    lockbud = (entry.get("lockbud") or {}).get("status")
                else:
                    concir = rec.get("concir") or {}
                    c = concir.get(side) or {}
                    petri = (c.get("petri") or {}).get("outcome")
                    interp = (c.get("interp") or {}).get("outcome")
                    miri = rec.get(f"miri_{side}")
                    lockbud = rec.get(f"lockbud_{side}")
                row = (task, side, petri, interp, miri, lockbud)
                if miri == "—" and lockbud is None:
                    cir_rows.append(row)
                else:
                    rust_rows.append(row)
        L += ["### Track D — tasks with a Rust reference", "",
              "| task | side | concir.petri | concir.interp | miri | lockbud |",
              "| --- | --- | --- | --- | --- | --- |"]
        for row in rust_rows:
            L.append("| " + " | ".join(str(x) for x in row) + " |")
        L.append("")
        if cir_rows:
            L += ["### Track D (CIR-only)", "",
                  "| task | side | concir.petri | concir.interp |", "| --- | --- | --- | --- |"]
            for task, side, petri, interp, _m, _l in cir_rows:
                L.append(f"| {task} | {side} | {petri} | {interp} |")
            L.append("")
        lockbud = trackd.get("lockbud") or {}
        avail = trackd.get("lockbud_available")
        if avail is None and lockbud:
            avail = bool(lockbud.get("available", lockbud.get("path")))
        L += [f"Lockbud: available={bool(avail)} commit=`{str(lockbud.get('commit'))[:12]}` "
              f"(miri seeds={trackd.get('miri_seeds', '?')}).", ""]

    # ---- scale
    if scale:
        L += ["### Scale", "",
              "| threads | chain_len | max_states | outcome | complete | states | wall_ms |",
              "| --- | --- | --- | --- | --- | --- | --- |"]
        for run in scale.get("runs", [])[:40]:
            L.append(f"| {run.get('threads')} | {run.get('chain_len')} | "
                     f"{run.get('max_states')} | {run.get('outcome')} | "
                     f"{run.get('complete')} | {run.get('states_explored')} | {run.get('wall_ms')} |")
        L.append("")

    L += ["## Deviations and threats to validity", ""]
    for d in deviations:
        L.append(f"- {d}")
    L.append("")
    return "\n".join(L)


def _miri_counts(miri: dict | None) -> str:
    if not miri:
        return "—"
    statuses = Counter(miri.get("statuses") or [])
    if not statuses:
        return "—"
    return " · ".join(f"{k} {v}" for k, v in statuses.most_common())


def _parse_counts(text: str) -> dict[str, int]:
    out: dict[str, int] = {}
    if not text or text == "—":
        return out
    for part in text.split("·"):
        bits = part.strip().rsplit(" ", 1)
        if len(bits) == 2 and "/" in bits[1]:
            count = bits[1].split("/")[0]
            try:
                out[bits[0]] = int(count)
            except ValueError:
                pass
    return out


def _sum_tokens(row: dict[str, Any]) -> float:
    text = row.get("tokens", "—")
    if text == "—":
        return 0.0
    try:
        return float(text.split("±")[0])
    except ValueError:
        return 0.0


DEFAULT_DEVIATIONS = [
    "D-19: the main batch is not gated on Rust-arm oracle completeness; missing "
    "`oracle.model` cells are `inconclusive` and backfilled by extraction/expert labels.",
    "Expert labels are LLM-assisted (agent-proxy) with the owner's human review "
    "merged (17 rows / 11 candidates); 2 human/auto disagreements await the owner "
    "(`HUMAN_DISAGREEMENT_EVIDENCE.md`).",
    "All offline recomputation uses BIN_MAIN (main table) or BIN_V2 (conform/"
    "a3-to-rust/mutation/post-edit); both shas are listed above.",
]


def build(batch_paths: list[Path], expert_paths: list[Path], extract_dirs: list[Path],
          trackd_path: Path | None, scale_path: Path | None,
          reclass_path: Path | None, command: str,
          mutation_v1_path: Path | None = None, postedit_v1_path: Path | None = None,
          mutation_path: Path | None = None, postedit_path: Path | None = None,
          modelprobe_dir: Path | None = None) -> str:
    summaries = load_summaries(batch_paths)
    expert_agg, expert_cell, candidates = expert_index(expert_paths)
    extract = extract_index(extract_dirs)
    reclass = json.loads(reclass_path.read_text()) if reclass_path and reclass_path.is_file() else None
    trackd = json.loads(trackd_path.read_text()) if trackd_path and trackd_path.is_file() else None
    scale = json.loads(scale_path.read_text()) if scale_path and scale_path.is_file() else None
    mutation_v1 = json.loads(mutation_v1_path.read_text()) if mutation_v1_path and mutation_v1_path.is_file() else None
    postedit_v1 = json.loads(postedit_v1_path.read_text()) if postedit_v1_path and postedit_v1_path.is_file() else None
    mutation_v2 = json.loads(mutation_path.read_text()) if mutation_path and mutation_path.is_file() else None
    modelprobe = None
    if modelprobe_dir and Path(modelprobe_dir).exists():
        runs = sorted(Path(modelprobe_dir).glob("run-*"))
        if runs:
            modelprobe = json.loads((runs[-1] / "SUMMARY.json").read_text())
    postedit_v2 = json.loads(postedit_path.read_text()) if postedit_path and postedit_path.is_file() else None
    rows = aggregate(summaries, expert_agg, expert_cell, extract, reclass)
    inputs = []
    for p in batch_paths:
        inputs.append(("batch", str(p)))
    for p in expert_paths:
        inputs.append(("expert", str(p)))
    for p in extract_dirs:
        inputs.append(("extraction", str(p)))
    if trackd_path:
        inputs.append(("track D", str(trackd_path)))
    if scale_path:
        inputs.append(("scale", str(scale_path)))
    if reclass_path and reclass_path.is_file():
        inputs.append(("reclass overlay", str(reclass_path)))
    shas = [("binary", s.get("binary_sha256") or "unknown") for s in summaries[:1]]
    if mutation_v2 and mutation_v2.get("binary_sha256"):
        shas.append(("binary v2 (conform)", mutation_v2["binary_sha256"]))
    shas += [(f"protocol[{i}]", s.get("protocol_sha256") or "unknown")
             for i, s in enumerate(summaries)]
    header = {"command": command, "inputs": inputs, "shas": shas,
              "mutation": mutation_v2, "postedit": postedit_v2,
              "modelprobe": modelprobe}
    return render(rows, expert_agg, candidates, extract, trackd, scale, header,
                  DEFAULT_DEVIATIONS, mutation_v1, postedit_v1)


def _tex_header(name: str, command: str, shas: list[tuple[str, str]]) -> str:
    lines = [f"% {name} — generated by cir_workflow results --latex",
             f"% command: {command}"]
    for label, digest in shas:
        lines.append(f"% {label} sha256: {digest}")
    lines.append("% requires \\usepackage{booktabs}; no \\begin{table} wrapper")
    return "\n".join(lines) + "\n"


def _tex_escape(text: Any) -> str:
    out = str(text).replace("\\", "\\textbackslash{}").replace("_", "\\_")
    out = out.replace("%", "\\%").replace("±", "$\\pm$").replace("—", "--")
    return out


def latex_tables(rows: list[dict[str, Any]], candidates: list[dict],
                 trackd: dict[str, Any] | None, scale: dict[str, Any] | None,
                 mutation: dict[str, Any] | None, postedit: dict[str, Any] | None,
                 out_dir: Path, command: str, shas: list[tuple[str, str]],
                 mutation_v1: dict[str, Any] | None = None) -> list[str]:
    out_dir.mkdir(parents=True, exist_ok=True)
    written = []

    def write(name: str, body: str):
        (out_dir / name).write_text(_tex_header(name, command, shas) + body + "\n",
                                    encoding="utf-8")
        written.append(name)

    hdr = "\\toprule\n" + " & ".join("\\textbf{" + c + "}" for c in
        ["task", "arm", "accepted", "round", "tokens", "false-acc"]) + " \\\\\n\\midrule\n"
    body = hdr
    for r in rows:
        body += (f"{_tex_escape(r['task'])} & {_tex_escape(r['arm'])} & "
                 f"{_tex_escape(r['accepted_display'])} & {_tex_escape(r['round'])} & "
                 f"{_tex_escape(r['tokens'])} & {_tex_escape(r['fa_display'])} \\\\\n")
    body += "\\bottomrule"
    write("main.tex", "\\begin{tabular}{llrrrr}\n" + body + "\n\\end{tabular}")

    body = ("\\toprule\n\\textbf{arm} & \\textbf{cells} & \\textbf{accepted} & "
            "\\textbf{rate} & \\textbf{false-acc} & \\textbf{conform} & "
            "\\textbf{esc.} & \\textbf{tok/correct} \\\\\n\\midrule\n")
    for arm in ARMS:
        cells = [r for r in rows if r["arm"] == arm]
        if not cells:
            continue
        n = sum(r["n"] for r in cells)
        acc = sum(r["accepted"] for r in cells)
        fa = sum(r["false_accept"] for r in cells)
        conf_n = conf_pass = 0
        for r in cells:
            for label, count in _parse_counts(r["conform"]).items():
                conf_n += count
                conf_pass += count if label == "PASS" else 0
        cpr = f"{conf_pass}/{conf_n}" if conf_n else "--"
        esc = sum(r.get("escalated", 0) for r in cells)
        correct = acc - fa
        toks = sum(_sum_tokens(r) for r in cells)
        tca = f"{toks / correct:.0f}" if correct > 0 else "$\\infty$"
        body += (f"{_tex_escape(arm)} & {n} & {acc} & {acc / n:.2f} & {fa} & {cpr} & "
                 f"{esc} & {tca} \\\\\n")
    body += "\\bottomrule"
    write("arms.tex", "\\begin{tabular}{lrrrrrrr}\n" + body + "\n\\end{tabular}")

    if trackd:
        body = ("\\toprule\n\\textbf{task} & \\textbf{side} & \\textbf{petri} & "
                "\\textbf{interp} & \\textbf{miri} & \\textbf{lockbud} \\\\\n\\midrule\n")
        for rec in trackd.get("tasks", []):
            for side in ("buggy", "fixed"):
                e = rec.get(side) or {}
                if e.get("miri") is None:
                    continue
                m = Counter((e.get("miri") or {}).get("statuses") or [])
                body += (f"{_tex_escape(rec['task'])} & {side} & {e.get('concir_petri')} & "
                         f"{e.get('concir_interp')} & {dict(m)} & "
                         f"{(e.get('lockbud') or {}).get('status')} \\\\\n")
        body += "\\bottomrule"
        write("trackd.tex", "\\begin{tabular}{llllll}\n" + body + "\n\\end{tabular}")

    if candidates:
        body = ("\\toprule\n\\textbf{sha} & \\textbf{task} & \\textbf{arm} & "
                "\\textbf{bug} & \\textbf{design} & \\textbf{human} \\\\\n\\midrule\n")
        for c in sorted(candidates, key=lambda x: (x.get("task", ""), x.get("arm", ""))):
            human = c.get("human_label") if c.get("human_reviewed") else "--"
            body += (f"\\texttt{{{str(c.get('sha256',''))[:8]}}} & "
                     f"{_tex_escape(c.get('task'))} & {_tex_escape(c.get('arm'))} & "
                     f"{c.get('bug_present')} & {c.get('design_preserved')} & "
                     f"{human} \\\\\n")
        body += "\\bottomrule"
        write("expert.tex", "\\begin{tabular}{lllll}\n" + body + "\n\\end{tabular}")

    if scale:
        body = ("\\toprule\n\\textbf{threads} & \\textbf{chain} & \\textbf{max-states} & "
                "\\textbf{outcome} & \\textbf{complete} & \\textbf{states} & "
                "\\textbf{ms} \\\\\n\\midrule\n")
        for run in scale.get("runs", [])[:40]:
            body += (f"{run.get('threads')} & {run.get('chain_len')} & {run.get('max_states')} & "
                     f"{run.get('outcome')} & {run.get('complete')} & "
                     f"{run.get('states_explored')} & {run.get('wall_ms')} \\\\\n")
        body += "\\bottomrule"
        write("scale.tex", "\\begin{tabular}{rrrrrrr}\n" + body + "\n\\end{tabular}")

    if mutation and mutation.get("ops"):
        body = ("\\toprule\n\\textbf{op} & \\textbf{mutants} & \\textbf{conform FAIL} & "
                "\\textbf{recall} & \\textbf{reasons} \\\\\n\\midrule\n")
        for op, s in mutation["ops"].items():
            reasons = mutation.get("reasons", {}).get(op, {})
            rtext = ", ".join(f"{k}: {v}" for k, v in reasons.items()) or "--"
            body += (f"{op} & {s['mutants']} & {s['conform_fail']} & "
                     f"{s['conform_recall']} & {_tex_escape(rtext)} \\\\\n")
        body += "\\bottomrule"
        write("mutation.tex", "\\begin{tabular}{lrrrl}\n" + body + "\n\\end{tabular}")

    mp = (header_modelprobe := None)
    if mutation and mutation.get("ops"):
        ops1 = (mutation_v1 or {}).get("ops", {})
        body = ("\\toprule\n\\textbf{op} & \\textbf{v1 recall} & \\textbf{v2 recall} \\\\\n\\midrule\n")
        for op, s2 in mutation["ops"].items():
            body += f"{op} & {ops1.get(op, {}).get('conform_recall', '--')} & {s2['conform_recall']} \\\\\n"
        body += "\\bottomrule"
        write("conform_recall_v1v2.tex", "\\begin{tabular}{lrr}\n" + body + "\n\\end{tabular}")

    if postedit and postedit.get("ops"):
        body = ("\\toprule\n\\textbf{edit} & \\textbf{cells} & \\textbf{build} & "
                "\\textbf{conform PASS} & \\textbf{drift-only-conform} \\\\\n\\midrule\n")
        for eid, s in postedit["ops"].items():
            body += (f"{eid} & {s.get('cells')} & {s.get('real_edits', s.get('build_ok'))} & "
                     f"{s.get('conform_pass_real', s.get('conform_pass'))} & "
                     f"{s.get('drift_only_conform_real', s.get('drift_caught_only_by_conform'))} "
                     f"\\\\\n")
        body += "\\bottomrule"
        write("postedit.tex", "\\begin{tabular}{lrrrr}\n" + body + "\n\\end{tabular}")
    return written


def main(argv: list[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(prog="cir_workflow results")
    parser.add_argument("--batch", action="append", default=[])
    parser.add_argument("--expert", action="append", default=[])
    parser.add_argument("--extraction", action="append", default=[])
    parser.add_argument("--trackd")
    parser.add_argument("--scale")
    parser.add_argument("--reclass")
    parser.add_argument("--mutation")
    parser.add_argument("--postedit")
    parser.add_argument("--latex")
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    command = "python -m cir_workflow results " + " ".join(shlex.quote(a) for a in argv)
    text = build([Path(p) for p in args.batch], [Path(p) for p in args.expert],
                 [Path(p) for p in args.extraction],
                 Path(args.trackd) if args.trackd else None,
                 Path(args.scale) if args.scale else None,
                 Path(args.reclass) if args.reclass else None, command)
    Path(args.out).write_text(text, encoding="utf-8")
    written = []
    if args.latex:
        summaries = load_summaries([Path(p) for p in args.batch])
        expert_agg, expert_cell, candidates = expert_index(
            [Path(p) for p in args.expert])
        extract = extract_index([Path(p) for p in args.extraction])
        rows = aggregate(summaries, expert_agg, expert_cell, extract)
        trackd = json.loads(Path(args.trackd).read_text()) if args.trackd else None
        scale = json.loads(Path(args.scale).read_text()) if args.scale else None
        mutation = json.loads(Path(args.mutation).read_text()) if args.mutation else None
        postedit = json.loads(Path(args.postedit).read_text()) if args.postedit else None
        shas = [("binary", summaries[0].get("binary_sha256") if summaries else "unknown")]
        written = latex_tables(rows, candidates, trackd, scale, mutation, postedit,
                               Path(args.latex), command, shas)
    print(json.dumps({"out": args.out, "bytes": len(text), "latex": written}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
