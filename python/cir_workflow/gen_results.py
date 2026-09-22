"""Shared aggregation/render for the §2 generation main batch and §3 probe.

Used by ``scripts/render_gen_main.py`` (standalone) and by
``cir_workflow.results`` so the RESULTS document and the LaTeX tables come from
one implementation.
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path
from typing import Any

ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir")
PROBE_ARMS = ("G0_direct", "G2_tools_iter", "G3_concir")
TIERS = ("Simple", "Medium", "Complex")


def load_cells(batches: list[Path]) -> dict[tuple, dict]:
    cells: dict[tuple, dict] = {}
    for batch in batches:
        summary = json.loads((Path(batch) / "SUMMARY.json").read_text(encoding="utf-8"))
        for cell in summary["cells"]:
            if cell.get("status") == "not_run":
                continue
            cells[(cell["rep"], cell["task"], cell["arm"])] = cell
    return cells


def _mean(values):
    values = [v for v in values if v is not None]
    return round(statistics.mean(values), 3) if values else None


def _tokens(cell: dict) -> int:
    return (cell.get("prompt_tokens") or 0) + (cell.get("completion_tokens") or 0)


def _defect(cell: dict) -> bool:
    if not cell.get("accepted"):
        return False
    rec = cell.get("record") or {}
    if cell["arm"] == "G3_concir":
        conform = rec.get("conform") or {}
        return bool(rec.get("behavior_hang") or (conform.get("violation") or 0) > 0
                    or (rec.get("model") or {}).get("outcome") not in ("PASS", None))
    return bool(cell.get("hang") or cell.get("monitor_fail"))


def aggregate(cells: list[dict]) -> dict:
    accepted = [c for c in cells if c.get("accepted")]
    return {
        "n": len(cells), "accepted": len(accepted),
        "accept_rate": round(len(accepted) / len(cells), 3) if cells else None,
        "rc": _mean([c.get("rc") for c in cells]),
        "rf": _mean([c.get("rf") for c in cells]),
        "defect": sum(1 for c in cells if _defect(c)),
        "awp": sum(1 for c in cells if c.get("accepted_with_proof")),
        "tokens": sum(_tokens(c) for c in cells),
    }


def render_md(cells: dict, batches: list[Path] | None = None) -> str:
    all_cells = list(cells.values())
    lines = [
        "", "## Generation (main) — `flash-gen-main-v1`", "",
        "Requirements -> program, four arms, 24 tasks, K=4, 3 reps. The contract",
        "is hidden from every model prompt. Rust arms are scored by the bounded",
        "monitor (§1); G3 by the exhaustive model verdict + conform.",
    ]
    if batches:
        lines += ["", "Batches (later overrides earlier): "
                  + ", ".join(f"`{Path(b).name}`" for b in batches) + "."]
    lines += ["", "| arm | cells | accepted | accept rate | RC | RF | defect | awp | tokens |",
              "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        lines.append(f"| {arm} | {a['n']} | {a['accepted']} | {a['accept_rate']} | "
                     f"{a['rc']} | {a['rf']} | {a['defect']} | {a['awp']} | {a['tokens']} |")
    a = aggregate(all_cells)
    lines.append(f"| **all** | {a['n']} | {a['accepted']} | {a['accept_rate']} | "
                 f"{a['rc']} | {a['rf']} | {a['defect']} | {a['awp']} | {a['tokens']} |")
    lines += ["", "Per tier (RF):", "",
              "| tier | " + " | ".join(ARMS) + " |",
              "| --- | " + " | ".join("---" for _ in ARMS) + " |"]
    for tier in TIERS:
        row = [tier]
        for arm in ARMS:
            agg = aggregate([c for c in all_cells if c["arm"] == arm and c.get("tier") == tier])
            row.append(str(agg["rf"]))
        lines.append("| " + " | ".join(row) + " |")
    lines += ["", "`defect` = accepted and (hang/behavior or monitor/conform "
              "violation). `awp` is G3-only (model PASS and conform PASS)."]
    return "\n".join(lines) + "\n"


def render_tex(cells: dict) -> dict[str, str]:
    all_cells = list(cells.values())
    main = ["\\begin{tabular}{lrrrrrr}", "\\toprule",
            "arm & cells & acc. & RF & defect & awp & tokens \\\\", "\\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        main.append(f"{arm.replace('_', chr(92)+'_')} & {a['n']} & {a['accept_rate']} & "
                    f"{a['rf']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\")
    a = aggregate(all_cells)
    main += ["\\midrule", f"all & {a['n']} & {a['accept_rate']} & {a['rf']} & "
             f"{a['defect']} & {a['awp']} & {a['tokens']} \\\\", "\\bottomrule",
             "\\end{tabular}"]
    arms = ["\\begin{tabular}{lrrr}", "\\toprule", "arm & acc. rate & RC & RF \\\\",
            "\\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        arms.append(f"{arm.replace('_', chr(92)+'_')} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf']} \\\\")
    arms += ["\\bottomrule", "\\end{tabular}"]
    tiers = ["\\begin{tabular}{l" + "r" * len(ARMS) + "}", "\\toprule",
             "tier & " + " & ".join(ARMS) + " \\\\", "\\midrule"]
    for tier in TIERS:
        row = [tier]
        for arm in ARMS:
            a = aggregate([c for c in all_cells if c["arm"] == arm and c.get("tier") == tier])
            row.append(str(a["rf"]))
        tiers.append(" & ".join(row) + " \\\\")
    tiers += ["\\bottomrule", "\\end{tabular}"]
    return {"gen_main.tex": "\n".join(main) + "\n",
            "gen_arms.tex": "\n".join(arms) + "\n",
            "gen_tiers.tex": "\n".join(tiers) + "\n"}


def probe_aggregate(cells: list[dict]) -> dict:
    ran = [c for c in cells if c.get("status") != "not_run"]
    accepted = [c for c in ran if c.get("accepted")]
    return {
        "n": len(ran), "not_run": len(cells) - len(ran), "accepted": len(accepted),
        "accept_rate": round(len(accepted) / len(ran), 3) if ran else None,
        "rc": _mean([c.get("rc") for c in ran]),
        "rf": _mean([c.get("rf") for c in ran]),
        "defect": sum(1 for c in ran if _defect(c)),
        "awp": sum(1 for c in ran if c.get("accepted_with_proof")),
        "tokens": sum(_tokens(c) for c in ran),
    }


def render_probe_md(model: str, cells: list[dict], temperature: Any) -> str:
    lines = ["", "## Generation with a frontier model — `gen-model-probe-v1`", "",
             f"Model `{model}` (OpenCode Go, chat/completions), 1 rep, K=4, "
             f"temperature {temperature}.", "",
             "| arm | cells | not_run | accepted | accept rate | RC | RF | defect | awp | tokens |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in PROBE_ARMS:
        a = probe_aggregate([c for c in cells if c["arm"] == arm])
        lines.append(f"| {arm} | {a['n']} | {a['not_run']} | {a['accepted']} | "
                     f"{a['accept_rate']} | {a['rc']} | {a['rf']} | {a['defect']} | "
                     f"{a['awp']} | {a['tokens']} |")
    a = probe_aggregate(cells)
    lines.append(f"| **all** | {a['n']} | {a['not_run']} | {a['accepted']} | "
                 f"{a['accept_rate']} | {a['rc']} | {a['rf']} | {a['defect']} | "
                 f"{a['awp']} | {a['tokens']} |")
    return "\n".join(lines) + "\n"


def render_probe_tex(cells: list[dict]) -> str:
    body = ["\\begin{tabular}{lrrrrrr}", "\\toprule",
            "arm & acc. & RC & RF & defect & awp & tokens \\\\", "\\midrule"]
    for arm in PROBE_ARMS:
        a = probe_aggregate([c for c in cells if c["arm"] == arm])
        body.append(f"{arm.replace('_', chr(92)+'_')} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\")
    body += ["\\bottomrule", "\\end{tabular}"]
    return "\n".join(body) + "\n"


def latest_probe_summary(probe_dir: Path) -> dict | None:
    runs = sorted(Path(probe_dir).glob("run-*"))
    if not runs:
        return None
    return json.loads((runs[-1] / "SUMMARY.json").read_text(encoding="utf-8"))
