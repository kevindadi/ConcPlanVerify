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

ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml", "A3_local", "A3_whole")
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


def expert_index(paths: list[Path]) -> tuple[dict[str, dict], dict[str, dict]]:
    """Return (per-(task|arm), per-(task|arm|rep)) expert label maps."""

    agg: dict[str, dict] = {}
    cell: dict[str, dict] = {}
    for path in paths:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        labels = data.get("labels", data if isinstance(data, list) else [])
        for item in labels:
            key = f"{item['task']}|{item['arm']}"
            agg[key] = item
            cells = item.get("cells")
            if cells:
                for c in cells:
                    task, arm, rep = (c if isinstance(c, list)
                                      else (c.get("task"), c.get("arm"), c.get("rep")))
                    cell[f"{task}|{arm}|{rep}"] = item
            else:
                for rep in range(3):
                    cell[f"{key}|{rep}"] = item
    return agg, cell


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
        for rep, rec in rep_recs:
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
        })
    return rows


def render(rows: list[dict[str, Any]], expert_agg: dict[str, dict],
           extract: dict[str, dict], trackd: dict[str, Any] | None,
           scale: dict[str, Any] | None, header: dict[str, Any],
           deviations: list[str]) -> str:
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
          "tokens/correct_accept | by source |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |"]
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
        L.append(f"| {arm} | {n} | {acc} | {acc / n:.2f} | {fa} | {cpr} | {tca} | "
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
    L += ["", f"Agreement with the automatic oracle: **{rate}** (unsure {unsure}); "
              f"`design_loss` (accepted ∧ design_preserved=no): **{design_loss}**.", ""]
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
    "Expert labels are LLM-assisted (agent-proxy); the human review queue "
    "(`HUMAN_REVIEW_QUEUE.md`) is left blank for the owner.",
    "All offline recomputation uses a single BIN_MAIN; any result on another binary "
    "sha is listed here.",
]


def build(batch_paths: list[Path], expert_paths: list[Path], extract_dirs: list[Path],
          trackd_path: Path | None, scale_path: Path | None,
          reclass_path: Path | None, command: str) -> str:
    summaries = load_summaries(batch_paths)
    expert_agg, expert_cell = expert_index(expert_paths)
    extract = extract_index(extract_dirs)
    reclass = json.loads(reclass_path.read_text()) if reclass_path and reclass_path.is_file() else None
    trackd = json.loads(trackd_path.read_text()) if trackd_path and trackd_path.is_file() else None
    scale = json.loads(scale_path.read_text()) if scale_path and scale_path.is_file() else None
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
    shas += [(f"protocol[{i}]", s.get("protocol_sha256") or "unknown")
             for i, s in enumerate(summaries)]
    header = {"command": command, "inputs": inputs, "shas": shas}
    return render(rows, expert_agg, extract, trackd, scale, header, DEFAULT_DEVIATIONS)


def main(argv: list[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(prog="cir_workflow results")
    parser.add_argument("--batch", action="append", default=[])
    parser.add_argument("--expert", action="append", default=[])
    parser.add_argument("--extraction", action="append", default=[])
    parser.add_argument("--trackd")
    parser.add_argument("--scale")
    parser.add_argument("--reclass")
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    command = "python -m cir_workflow results " + " ".join(shlex.quote(a) for a in argv)
    text = build([Path(p) for p in args.batch], [Path(p) for p in args.expert],
                 [Path(p) for p in args.extraction],
                 Path(args.trackd) if args.trackd else None,
                 Path(args.scale) if args.scale else None,
                 Path(args.reclass) if args.reclass else None, command)
    Path(args.out).write_text(text, encoding="utf-8")
    print(json.dumps({"out": args.out, "bytes": len(text)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
