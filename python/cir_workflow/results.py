"""Deterministic RESULTS.md generator.

`python -m cir_workflow results --batch ... [--batch ...]` renders the
paper-facing table from the committed JSON artefacts (batch SUMMARY.json files,
expert labels, extraction CELLS, Track D, scale). RESULTS.md is **generated**:
never edit it by hand (see the header it writes with the exact command and the
sha256 of every input).
"""

from __future__ import annotations

import hashlib
import json
import shlex
import sys
from pathlib import Path
from typing import Any

ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml", "A3_local", "A3_whole")
RUST_ARMS = ("A0_direct", "A1_self_iter", "A2_tools_iter_ml")


def sha256_file(path: Path | str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_summaries(paths: list[Path]) -> list[dict[str, Any]]:
    """Each path is a batch directory (or a SUMMARY.json); expand `reps`."""

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


def expert_index(paths: list[Path]) -> dict[str, dict[str, Any]]:
    idx: dict[str, dict[str, Any]] = {}
    for path in paths:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        labels = data.get("labels", data if isinstance(data, list) else [])
        for item in labels:
            key = f"{item['task']}|{item['arm']}"
            idx[key] = item
    return idx


def extract_index(dirs: list[Path]) -> dict[str, dict[str, Any]]:
    idx: dict[str, dict[str, Any]] = {}
    for d in dirs:
        cells_file = Path(d) / "CELLS.json"
        if cells_file.is_file():
            data = json.loads(cells_file.read_text(encoding="utf-8"))
            for item in data.get("cells", data if isinstance(data, list) else []):
                idx[f"{item['task']}|{item['arm']}"] = item
            continue
        for result in Path(d).glob("*/extraction_result.json"):
            data = json.loads(result.read_text(encoding="utf-8"))
            key = result.parent.name
            idx[key] = data
    return idx


def _extract_verdict(rec: dict[str, Any] | None) -> str:
    if not rec or not rec.get("extract_validated"):
        return "—"
    verdict = rec.get("model_verdict")
    if verdict is None:
        return "—"
    return "PASS" if verdict == "PASS" else ("FAIL" if verdict == "FAIL" else str(verdict))


def _bug(rec: dict[str, Any], expert: dict[str, Any] | None) -> tuple[bool, list[str]]:
    sources: list[str] = []
    oracle = rec.get("oracle") or {}
    if oracle.get("behavior_status") == "hang":
        sources.append("behavior")
    if oracle.get("verify_pass") is False:
        sources.append("model")
    if (rec.get("notes") or {}).get("bug_present_reason") == "by_construction":
        sources.append("by_construction")
    if expert and expert.get("bug_present") == "yes":
        sources.append("expert")
    return bool(sources), sources


def _extract_source(rec: dict[str, Any], extract: dict[str, Any] | None) -> bool:
    if not extract or not extract.get("extract_validated"):
        return False
    return extract.get("model_verdict") == "FAIL"


def _mean_range(values: list[float]) -> str:
    vals = [v for v in values if v is not None]
    if not vals:
        return "—"
    mean = sum(vals) / len(vals)
    lo, hi = min(vals), max(vals)
    if lo == hi:
        return f"{mean:.0f}"
    return f"{mean:.0f}±{hi - lo:.0f}"


def aggregate(summaries: list[dict[str, Any]], expert: dict[str, dict[str, Any]],
              extract: dict[str, dict[str, Any]],
              reclass: dict[str, Any] | None = None) -> list[dict[str, Any]]:
    """One aggregated row per (task, arm) across all reps."""

    buckets: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for summary in summaries:
        for task in summary.get("tasks", []):
            for arm, rec in (task.get("arms") or {}).items():
                buckets.setdefault((task["task"], arm), []).append(rec)

    rows = []
    for (task, arm), recs in buckets.items():
        key = f"{task}|{arm}"
        exp = expert.get(key)
        ext = extract.get(key)
        overridden = reclass.get("cells", {}).get(key) if reclass else None
        accepted_count = 0
        fa_count = 0
        fa_sources: dict[str, int] = {}
        rounds: list[float] = []
        tokens: list[float] = []
        llm_ms: list[float] = []
        tool_ms: list[float] = []
        behavior = None
        miri = None
        build = None
        for rec in recs:
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
            bug, sources = _bug(rec, exp)
            if overridden and overridden.get("by_construction"):
                bug = True
                sources.append("by_construction")
            if _extract_source(rec, ext):
                bug = True
                sources.append("extract")
            if accepted and bug:
                fa_count += 1
                for source in set(sources):
                    fa_sources[source] = fa_sources.get(source, 0) + 1
            oracle = rec.get("oracle") or {}
            behavior = behavior or oracle.get("behavior_status")
            build = build if build is not None else oracle.get("build_ok")
            if oracle.get("miri_detected") is not None:
                miri = "detected" if oracle.get("miri_detected") else "clean"
        rows.append({
            "task": task, "arm": arm, "n": len(recs),
            "accepted": accepted_count, "accepted_display": f"{accepted_count}/{len(recs)}",
            "round": _mean_range(rounds), "tokens": _mean_range(tokens),
            "llm_ms": _mean_range(llm_ms), "tool_ms": _mean_range(tool_ms),
            "build": build, "behavior": behavior or "—", "miri": miri or "—",
            "expert": (exp or {}).get("bug_present", "—") if exp else "—",
            "extract": _extract_verdict(ext),
            "false_accept": fa_count, "fa_display": f"{fa_count}/{len(recs)}",
            "fa_sources": fa_sources,
        })
    return rows


def render(summaries: list[dict[str, Any]], rows: list[dict[str, Any]],
           expert: dict[str, dict[str, Any]], expert_paths: list[Path],
           extract: dict[str, dict[str, Any]], trackd: dict[str, Any] | None,
           scale: dict[str, Any] | None, header: dict[str, Any],
           deviations: list[str]) -> str:
    L: list[str] = ["# RESULTS — ConcIR repair/extraction benchmark", "",
                    "> **Generated file — do not edit by hand.** Regenerate with:",
                    "> ```",
                    f"> {header['command']}",
                    "> ```", ""]
    L += ["## Provenance", ""]
    for label, value in header.get("inputs", []):
        L.append(f"- {label}: `{value}`")
    for label, digest in header.get("shas", []):
        L.append(f"- {label} sha256: `{digest}`")
    L.append("")

    # ---- 1. main table
    L += ["## 1. Main table", "",
          "| task | arm | accepted | round | tokens | false_accept | build | behavior | miri | expert | extract |",
          "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for r in rows:
        L.append(f"| {r['task']} | {r['arm']} | {r['accepted_display']} | {r['round']} | "
                 f"{r['tokens']} | {r['fa_display']} | {r['build']} | {r['behavior']} | "
                 f"{r['miri']} | {r['expert']} | {r['extract']} |")
    L.append("")
    L.append("`accepted`/`false_accept` are k/n over repeats; `round`/`tokens` are "
             "mean±range over accepted cells. `expert` is the manual/proxy label, "
             "`extract` the validated extraction verdict.")
    L.append("")

    # ---- 2. per-arm
    L += ["### Per-arm aggregates", "",
          "| arm | cells | accepted | accept_rate | false_accept | by source |",
          "| --- | --- | --- | --- | --- | --- |"]
    for arm in ARMS:
        cells = [r for r in rows if r["arm"] == arm]
        if not cells:
            continue
        n = sum(r["n"] for r in cells)
        acc = sum(r["accepted"] for r in cells)
        fa = sum(r["false_accept"] for r in cells)
        src: dict[str, int] = {}
        for r in cells:
            for k, v in r["fa_sources"].items():
                src[k] = src.get(k, 0) + v
        L.append(f"| {arm} | {n} | {acc} | {acc / n:.2f} | {fa} | {src or '—'} |")
    L.append("")

    # ---- 3. A3 decision distribution
    dist: dict[str, dict[str, int]] = {}
    for summary in summaries:
        for task in summary.get("tasks", []):
            for arm in ("A3_local", "A3_whole"):
                rec = (task.get("arms") or {}).get(arm) or {}
                d = rec.get("decision_distribution")
                if d:
                    key = task["task"]
                    merged = dist.setdefault(key, {})
                    for k, v in d.items():
                        merged[k] = merged.get(k, 0) + v
    L += ["### A3 decision distribution", "",
          "| task | decisions |", "| --- | --- |"]
    for task, d in sorted(dist.items()):
        L.append(f"| {task} | {d} |")
    L.append("")

    # ---- 4. expert labels
    L += ["### Expert labels", "",
          "| task | arm | bug_present | design_preserved | auto | agree |",
          "| --- | --- | --- | --- | --- | --- |"]
    agree = compared = unsure = 0
    disagreements = []
    for key, item in sorted(expert.items()):
        task, arm = key.split("|")
        row = next((r for r in rows if r["task"] == task and r["arm"] == arm), None)
        auto = row and (row["false_accept"] > 0)
        match = None
        if item.get("bug_present") != "unsure" and auto is not None:
            compared += 1
            match = bool(item["bug_present"] == "yes") == bool(auto)
            agree += 1 if match else 0
        if item.get("bug_present") == "unsure":
            unsure += 1
        L.append(f"| {task} | {arm} | {item.get('bug_present')} | "
                 f"{item.get('design_preserved')} | {auto} | {match} |")
        if match is False:
            disagreements.append((task, arm, item.get("bug_present"), auto))
    rate = f"{agree}/{compared} = {agree / compared:.3f}" if compared else "n/a"
    L += ["", f"Agreement with the automatic oracle: **{rate}** (unsure {unsure}).", ""]
    if disagreements:
        L += ["### Oracle disagreements", "",
              "| task | arm | expert bug_present | automatic false_accept |", "| --- | --- | --- | --- |"]
        for task, arm, bug, auto in disagreements:
            L.append(f"| {task} | {arm} | {bug} | {auto} |")
        L.append("")

    # ---- 5. extraction
    stages: dict[str, int] = {}
    validated = 0
    for rec in extract.values():
        stages[rec.get("stage", "?")] = stages.get(rec.get("stage", "?"), 0) + 1
        validated += 1 if rec.get("extract_validated") else 0
    L += ["### Extraction oracle", "",
          f"- cells: {len(extract)}; validated: {validated}; stage distribution: `{stages}`",
          "", "| task|arm | stage | validated | verdict |", "| --- | --- | --- | --- |"]
    for key, rec in sorted(extract.items()):
        L.append(f"| {key} | {rec.get('stage')} | {rec.get('extract_validated')} | "
                 f"{_extract_verdict(rec)} |")
    L.append("")

    # ---- 6. Track D
    if trackd:
        L += ["### Track D (detection capability)", "",
              "| task | side | concir.petri | concir.interp | miri | lockbud |",
              "| --- | --- | --- | --- | --- | --- |"]
        records = trackd.get("records") or trackd.get("tasks") or []
        for rec in records:
            task = rec.get("task") or rec.get("id")
            concir = rec.get("concir") or {}
            for side in ("buggy", "fixed"):
                entry = concir.get(side) or {}
                petri = (entry.get("petri") or {}).get("outcome")
                interp = (entry.get("interp") or {}).get("outcome")
                miri = rec.get(f"miri_{side}")
                lockbud = rec.get(f"lockbud_{side}")
                L.append(f"| {task} | {side} | {petri} | {interp} | {miri} | {lockbud} |")
        L.append("")
        lockbud = trackd.get("lockbud") or {}
        if lockbud:
            L.append(f"Lockbud: available={trackd.get('lockbud_available')} "
                     f"commit=`{str(lockbud.get('commit'))[:12]}`.")
            L.append("")

    # ---- 7. scale
    if scale:
        L += ["### Scale", "",
              "| threads | chain_len | max_states | outcome | complete | states | wall_ms |",
              "| --- | --- | --- | --- | --- | --- | --- |"]
        for run in scale.get("runs", [])[:40]:
            L.append(f"| {run.get('threads')} | {run.get('chain_len')} | "
                     f"{run.get('max_states')} | {run.get('outcome')} | "
                     f"{run.get('complete')} | {run.get('states_explored')} | {run.get('wall_ms')} |")
        L.append("")

    # ---- 8. deviations
    L += ["## Deviations and threats to validity", ""]
    for d in deviations:
        L.append(f"- {d}")
    L.append("")
    return "\n".join(L)


DEFAULT_DEVIATIONS = [
    "D-19: the main batch is not gated on Rust-arm oracle completeness; missing "
    "`oracle.model` cells are `inconclusive` and backfilled by extraction/expert labels.",
    "Expert labels are LLM-assisted (agent-proxy); a random sample plus every "
    "disagreement is queued for human review (`expert-labels/HUMAN_REVIEW_QUEUE.md`).",
    "All offline recomputation uses a single BIN_MAIN; any result on another binary "
    "sha is listed here.",
]


def build(batch_paths: list[Path], expert_paths: list[Path], extract_dirs: list[Path],
          trackd_path: Path | None, scale_path: Path | None,
          reclass_path: Path | None, command: str) -> str:
    summaries = load_summaries(batch_paths)
    expert = expert_index(expert_paths)
    extract = extract_index(extract_dirs)
    reclass = json.loads(reclass_path.read_text()) if reclass_path and reclass_path.is_file() else None
    trackd = json.loads(trackd_path.read_text()) if trackd_path and trackd_path.is_file() else None
    scale = json.loads(scale_path.read_text()) if scale_path and scale_path.is_file() else None
    rows = aggregate(summaries, expert, extract, reclass)
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
    return render(summaries, rows, expert, expert_paths, extract, trackd, scale,
                  header, DEFAULT_DEVIATIONS)


def main(argv: list[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(prog="cir_workflow results")
    parser.add_argument("--batch", action="append", default=[],
                        help="batch directory containing SUMMARY.json (repeatable)")
    parser.add_argument("--expert", action="append", default=[],
                        help="expert labels JSON (repeatable)")
    parser.add_argument("--extraction", action="append", default=[],
                        help="extraction directory (repeatable)")
    parser.add_argument("--trackd")
    parser.add_argument("--scale")
    parser.add_argument("--reclass")
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    command = "python -m cir_workflow results " + " ".join(
        shlex.quote(a) for a in argv)
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
