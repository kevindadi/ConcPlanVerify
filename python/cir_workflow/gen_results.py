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

ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir", "G3_codegen")
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
    """Unified across arms: accepted and (behavior hang or monitor FAIL).

    Miri-detected cells are added by the batch that runs Miri; the cell record
    carries `miri_detected` when available.
    """
    if not cell.get("accepted"):
        return False
    rec = cell.get("record") or {}
    return bool(cell.get("hang") or cell.get("monitor_fail")
                or rec.get("miri_detected"))


def aggregate(cells: list[dict]) -> dict:
    accepted = [c for c in cells if c.get("accepted")]
    # RF_all: a non-accepted cell counts as 0 (delivery quality including
    # failures). RF_acc: only accepted cells. RF_run: mean over the cells that
    # carry a monitor value (kept for comparison with freeze-5).
    rf_all = [(c.get("rf") or 0.0) if c.get("accepted") else 0.0 for c in cells]
    return {
        "n": len(cells), "accepted": len(accepted),
        "accept_rate": round(len(accepted) / len(cells), 3) if cells else None,
        "rc": _mean([c.get("rc") for c in cells]),
        "rf_all": round(sum(rf_all) / len(rf_all), 3) if rf_all else None,
        "rf_acc": _mean([c.get("rf") for c in accepted]),
        "rf_run": _mean([c.get("rf") for c in cells]),
        "defect": sum(1 for c in cells if _defect(c)),
        "awp": sum(1 for c in cells if c.get("accepted_with_proof")),
        "tokens": sum(_tokens(c) for c in cells),
    }


def render_md(cells: dict, batches: list[Path] | None = None) -> str:
    all_cells = list(cells.values())
    lines = [
        "", "## Generation (main) — `flash-gen-main-v2`", "",
        "Requirements -> verified CIR -> LLM code -> tool post-verification, "
        "24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the "
        "CIR exhaustively, the LLM writes the Rust, and conform/monitor "
        "post-verify; G3_codegen is the tool-codegen ablation (rep 0).",
    ]
    if batches:
        lines += ["", "Batches (later overrides earlier): "
                  + ", ".join(f"`{Path(b).name}`" for b in batches) + "."]
    lines += ["", "| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | RF_run | defect | awp | tokens |",
              "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
              "`RF_all` counts non-accepted cells as 0; `RF_acc` covers accepted cells; "
              "`RF_run` averages cells that carry a monitor value (freeze-5 wording)."]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        lines.append(f"| {arm} | {a['n']} | {a['accepted']} | {a['accept_rate']} | "
                     f"{a['rc']} | {a['rf_all']} | {a['rf_acc']} | {a['rf_run']} | "
                     f"{a['defect']} | {a['awp']} | {a['tokens']} |")
    a = aggregate(all_cells)
    lines.append(f"| **all** | {a['n']} | {a['accepted']} | {a['accept_rate']} | "
                 f"{a['rc']} | {a['rf_all']} | {a['rf_acc']} | {a['rf_run']} | "
                 f"{a['defect']} | {a['awp']} | {a['tokens']} |")
    lines += ["", "Per tier (RF):", "",
              "| tier | " + " | ".join(ARMS) + " |",
              "| --- | " + " | ".join("---" for _ in ARMS) + " |"]
    for tier in TIERS:
        row = [tier]
        for arm in ARMS:
            agg = aggregate([c for c in all_cells if c["arm"] == arm and c.get("tier") == tier])
            row.append(str(agg["rf_all"]))
        lines.append("| " + " | ".join(row) + " |")
    lines += ["", "`defect` = accepted and (behavior hang or monitor FAIL or conform violation). `awp` is G3-only (model PASS, conform PASS, no monitor FAIL)."]
    return "\n".join(lines) + "\n"


def render_tex(cells: dict) -> dict[str, str]:
    all_cells = list(cells.values())
    main = ["\\begin{tabular}{lrrrrrrr}", "\\toprule",
            "arm & cells & acc. & RF\\_all & RF\\_acc & defect & awp & tokens \\\\",
            "\\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        main.append(f"{arm.replace('_', chr(92)+'_')} & {a['n']} & {a['accept_rate']} & "
                    f"{a['rf_all']} & {a['rf_acc']} & {a['defect']} & {a['awp']} & "
                    f"{a['tokens']} \\\\")
    a = aggregate(all_cells)
    main += ["\\midrule", f"all & {a['n']} & {a['accept_rate']} & {a['rf_all']} & "
             f"{a['rf_acc']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\",
             "\\bottomrule", "\\end{tabular}"]
    arms = ["\\begin{tabular}{lrrr}", "\\toprule", "arm & acc. rate & RC & RF \\\\",
            "\\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        arms.append(f"{arm.replace('_', chr(92)+'_')} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf_all']} \\\\")
    arms += ["\\bottomrule", "\\end{tabular}"]
    tiers = ["\\begin{tabular}{l" + "r" * len(ARMS) + "}", "\\toprule",
             "tier & " + " & ".join(a.replace("_", "\\_") for a in ARMS) + " \\\\",
             "\\midrule"]
    for tier in TIERS:
        row = [tier]
        for arm in ARMS:
            a = aggregate([c for c in all_cells if c["arm"] == arm and c.get("tier") == tier])
            row.append(str(a["rf_acc"]))
        tiers.append(" & ".join(row) + " \\\\")
    tiers += ["\\bottomrule", "\\end{tabular}"]

    g3 = [c for c in all_cells if c["arm"] == "G3_concir"]
    v2recs = [c.get("record") or {} for c in g3]
    cir_rounds = [r.get("cir_rounds") for r in v2recs if r.get("cir_rounds") is not None]
    code_rounds = [len((r.get("code_stage") or {}).get("rounds", []))
                   for r in v2recs if (r.get("code_stage") or {}).get("rounds")]
    cir_tokens = sum((rnd.get("prompt_tokens") or 0) + (rnd.get("completion_tokens") or 0)
                     for r in v2recs for rnd in (r.get("rounds") or [])
                     if rnd.get("stage") == "cir")
    code_tokens = sum((rnd.get("prompt_tokens") or 0) + (rnd.get("completion_tokens") or 0)
                      for r in v2recs for rnd in (r.get("rounds") or [])
                      if rnd.get("stage") == "code")
    stages = ["\\begin{tabular}{lrrr}", "\\toprule",
              "stage & cells & mean rounds & tokens \\\\", "\\midrule",
              f"CIR & {len(cir_rounds)} & {_mean(cir_rounds)} & {cir_tokens} \\\\",
              f"code & {len(code_rounds)} & {_mean(code_rounds)} & {code_tokens} \\\\",
              "\\bottomrule", "\\end{tabular}"]

    ablation = ["\\begin{tabular}{lrrrrr}", "\\toprule",
                "arm & cells & acc. & RF\\_all & RF\\_acc & awp \\\\", "\\midrule"]
    for arm in ("G3_concir", "G3_codegen"):
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        ablation.append(f"{arm.replace('_', chr(92)+'_')} & {a['n']} & {a['accept_rate']} & "
                        f"{a['rf_all']} & {a['rf_acc']} & {a['awp']} \\\\")
    ablation += ["\\bottomrule", "\\end{tabular}"]

    conform_caught = 0
    code_cells = 0
    for r in v2recs:
        for rnd in (r.get("code_stage") or {}).get("rounds", []):
            code_cells += 1
            conform = rnd.get("conform") or {}
            if conform and conform.get("conformant", 0) < conform.get("traces", 0):
                conform_caught += 1
                break
    conform_value = ["\\begin{tabular}{lr}", "\\toprule",
                     "G3 code cells & " + str(len(code_rounds)) + " \\\\",
                     "conform rejected a round & " + str(conform_caught) + " \\\\",
                     "\\bottomrule", "\\end{tabular}"]

    return {"gen_main.tex": "\n".join(main) + "\n",
            "gen_arms.tex": "\n".join(arms) + "\n",
            "gen_tiers.tex": "\n".join(tiers) + "\n",
            "gen_g3_stages.tex": "\n".join(stages) + "\n",
            "gen_codegen_ablation.tex": "\n".join(ablation) + "\n",
            "gen_conform_value.tex": "\n".join(conform_value) + "\n"}


def probe_aggregate(cells: list[dict]) -> dict:
    ran = [c for c in cells if c.get("status") != "not_run"]
    accepted = [c for c in ran if c.get("accepted")]
    return {
        "n": len(ran), "not_run": len(cells) - len(ran), "accepted": len(accepted),
        "accept_rate": round(len(accepted) / len(ran), 3) if ran else None,
        "rc": _mean([c.get("rc") for c in ran]),
        "rf": _mean([c.get("rf") for c in ran]),
        "rf_all": round(sum((c.get("rf") or 0.0) if c.get("accepted") else 0.0 for c in ran) / len(ran), 3) if ran else None,
        "defect": sum(1 for c in ran if _defect(c)),
        "awp": sum(1 for c in ran if c.get("accepted_with_proof")),
        "tokens": sum(_tokens(c) for c in ran),
    }


def render_probe_md(model: str, cells: list[dict], temperature: Any) -> str:
    lines = ["", "## Generation with a frontier model — `gen-model-probe-v1`", "",
             f"Model `{model}` (OpenCode Go, chat/completions), 1 rep, K=4, "
             f"temperature {temperature}.", "",
             "| arm | cells | not_run | accepted | accept rate | RC | RF_all | defect | awp | tokens |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in PROBE_ARMS:
        a = probe_aggregate([c for c in cells if c["arm"] == arm])
        lines.append(f"| {arm} | {a['n']} | {a['not_run']} | {a['accepted']} | "
                     f"{a['accept_rate']} | {a['rc']} | {a['rf_all']} | {a['defect']} | "
                     f"{a['awp']} | {a['tokens']} |")
    a = probe_aggregate(cells)
    lines.append(f"| **all** | {a['n']} | {a['not_run']} | {a['accepted']} | "
                 f"{a['accept_rate']} | {a['rc']} | {a['rf_all']} | {a['defect']} | "
                 f"{a['awp']} | {a['tokens']} |")
    return "\n".join(lines) + "\n"


def render_probe_tex(cells: list[dict]) -> str:
    body = ["\\begin{tabular}{lrrrrrr}", "\\toprule",
            r"arm & acc. & RC & RF\_all & defect & awp & tokens \\", r"\midrule"]
    for arm in PROBE_ARMS:
        a = probe_aggregate([c for c in cells if c["arm"] == arm])
        body.append(f"{arm.replace('_', chr(92)+'_')} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf_all']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\")
    body += ["\\bottomrule", "\\end{tabular}"]
    return "\n".join(body) + "\n"


def latest_probe_summary(probe_dir: Path) -> dict | None:
    runs = sorted(Path(probe_dir).glob("run-*"))
    if not runs:
        return None
    return json.loads((runs[-1] / "SUMMARY.json").read_text(encoding="utf-8"))
