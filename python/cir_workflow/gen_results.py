"""Shared aggregation/render for the §2 generation main batch and §3 probe.

Used by ``scripts/render_gen_main.py`` (standalone) and by
``cir_workflow.results`` so the RESULTS document and the LaTeX tables come from
one implementation.
"""

from __future__ import annotations

import json
import statistics
from collections import Counter
from pathlib import Path
from typing import Any

ARMS = ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir", "G3_codegen")
PROBE_ARMS = ("G0_direct", "G2_tools_iter", "G3_concir")
TIERS = ("Simple", "Medium", "Complex")

# Display-only arm labels for anonymity; data and aggregation keys are unchanged.
_ARM_LABELS = {"G3_concir": "G3"}


def _tex_arm(arm: str) -> str:
    return _ARM_LABELS.get(arm, arm).replace("_", "\\_")


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
    # Historical freeze-9 formula, kept so old tables still reproduce.
    # `c.get("rf") or 0` treats a missing accepted score as 0. The 2026-09-25
    # re-evaluation defines RF_acc as the mean over accepted cells that were
    # actually scored, and reports a 0/1 sensitivity bound when any remain
    # missing. See experiments/evidence-20260925/reeval/RF_SUMMARY.json.
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
        "", "## Generation (main) — composite (see Batches)", "",
        "Requirements -> verified CIR -> LLM code -> tool post-verification, "
        "24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the "
        "CIR exhaustively, the LLM writes the Rust, and conform/monitor "
        "post-verify; G3_codegen is the tool-codegen ablation (rep 0).",
    ]
    if batches:
        sources: dict[str, Counter] = {}
        for c in all_cells:
            sources.setdefault(c.get("source_run", "unknown"), Counter())[c["arm"]] += 1
        lines += ["", "Batches (per-cell `source_run`; earlier runs overridden by later):"]
        for run, counts in sorted(sources.items()):
            lines.append(f"- `{run}` — "
                         + ", ".join(f"{a}: {n}" for a, n in sorted(counts.items())))
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
            r"\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        main.append(f"{_tex_arm(arm)} & {a['n']} & {a['accept_rate']} & "
                    f"{a['rf_all']} & {a['rf_acc']} & {a['defect']} & {a['awp']} & "
                    f"{a['tokens']} \\\\")
    a = aggregate(all_cells)
    main += ["\\midrule", f"all & {a['n']} & {a['accept_rate']} & {a['rf_all']} & "
             f"{a['rf_acc']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\",
             r"\bottomrule", r"\end{tabular}"]
    arms = ["\\begin{tabular}{lrrr}", "\\toprule", "arm & acc. rate & RC & RF \\\\",
            r"\midrule"]
    for arm in ARMS:
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        arms.append(f"{_tex_arm(arm)} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf_all']} \\\\")
    arms += [r"\bottomrule", r"\end{tabular}"]
    tiers = ["\\begin{tabular}{l" + "r" * len(ARMS) + "}", "\\toprule",
             "tier & " + " & ".join(_tex_arm(a) for a in ARMS) + " \\\\",
             r"\midrule"]
    for tier in TIERS:
        row = [tier]
        for arm in ARMS:
            a = aggregate([c for c in all_cells if c["arm"] == arm and c.get("tier") == tier])
            row.append(str(a["rf_acc"]))
        tiers.append(" & ".join(row) + " \\\\")
    tiers += [r"\bottomrule", r"\end{tabular}"]

    fam_order = ["lock-order", "condvar", "channel", "semaphore", "atomic-data", "structure"]
    families = ["\\begin{tabular}{lrrrrr}", "\\toprule",
                "family & tasks & G0\\_direct & G1\\_self\\_iter & G2\\_tools\\_iter & " + _tex_arm("G3_concir") + " \\\\",
                r"\midrule"]
    for fam in fam_order:
        fcells = [c for c in all_cells if c["task"].split("/")[0] == fam]
        ntask = len({c["task"] for c in fcells})
        vals = []
        for arm in ("G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"):
            a = aggregate([c for c in fcells if c["arm"] == arm])
            vals.append(str(a["rf_all"]))
        families.append(" & ".join([fam, str(ntask)] + vals) + " \\\\")
    families += [r"\bottomrule", r"\end{tabular}"]

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
              r"\bottomrule", r"\end{tabular}"]

    ablation = ["\\begin{tabular}{lrrrrr}", "\\toprule",
                "arm & cells & acc. & RF\\_all & RF\\_acc & awp \\\\", r"\midrule"]
    for arm in ("G3_concir", "G3_codegen"):
        a = aggregate([c for c in all_cells if c["arm"] == arm])
        ablation.append(f"{_tex_arm(arm)} & {a['n']} & {a['accept_rate']} & "
                        f"{a['rf_all']} & {a['rf_acc']} & {a['awp']} \\\\")
    ablation += [r"\bottomrule", r"\end{tabular}"]

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
                     r"\bottomrule", r"\end{tabular}"]

    return {"gen_main.tex": "\n".join(main) + "\n",
            "gen_arms.tex": "\n".join(arms) + "\n",
            "gen_tiers.tex": "\n".join(tiers) + "\n",
            "gen_families.tex": "\n".join(families) + "\n",
            "gen_g3_stages.tex": "\n".join(stages) + "\n",
            "gen_codegen_ablation.tex": "\n".join(ablation) + "\n",
            "gen_conform_value.tex": "\n".join(conform_value) + "\n",
            "gen_ci.tex": ci_table(cells)}


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
    lines = ["", "## Generation with a frontier model", "",
             f"Model `{model}` (OpenCode Go, chat/completions), 1 rep, K=4, "
             f"temperature {temperature}.", "",
             "| arm | cells | not_run | accepted | accept rate | RC | RF_all | defect | awp | tokens |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for arm in PROBE_ARMS:
        a = probe_aggregate([c for c in cells if c["arm"] == arm])
        if a["n"] == 0:
            continue
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
        if a["n"] == 0:
            continue
        body.append(f"{_tex_arm(arm)} & {a['accept_rate']} & {a['rc']} & "
                    f"{a['rf_all']} & {a['defect']} & {a['awp']} & {a['tokens']} \\\\")
    body += [r"\bottomrule", r"\end{tabular}"]
    return "\n".join(body) + "\n"


def latest_probe_summary(probe_dir: Path) -> dict | None:
    runs = sorted(Path(probe_dir).glob("run-*"))
    if not runs:
        return None
    return json.loads((runs[-1] / "SUMMARY.json").read_text(encoding="utf-8"))


def cell_index(cells: dict) -> list[dict]:
    out = []
    for c in sorted(cells.values(), key=lambda c: (c.get("rep", 0), c.get("task", ""), c.get("arm", ""))):
        out.append({
            "rep": c.get("rep"), "task": c.get("task"), "arm": c.get("arm"),
            "tier": c.get("tier"), "source_run": c.get("source_run"),
            "source_stage": c.get("source_stage"),
            "source_cir": c.get("source_cir"), "source_code": c.get("source_code"),
            "accepted": c.get("accepted"), "rf": c.get("rf"), "rc": c.get("rc"),
            "status": c.get("status"),
        })
    return out


def render_provenance(cells: dict) -> str:
    lines = ["# PROVENANCE — per-cell source of the main generation table", "",
             "Generated by `python -m cir_workflow results`. `source_run` is the run a",
             "cell's verdict is taken from; for G3, `cir` and `code` stages may come",
             "from different runs.", "",
             "| rep | task | arm | source_run | source_stage | source_cir | source_code | accepted | rf |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for c in cell_index(cells):
        lines.append(f"| {c['rep']} | {c['task']} | {c['arm']} | {c['source_run']} | "
                     f"{c['source_stage']} | {c['source_cir'] or ''} | {c['source_code'] or ''} | "
                     f"{c['accepted']} | {c['rf']} |")
    return "\n".join(lines) + "\n"


# ───────────────────── paired bootstrap CIs (no scipy) ─────────────────────

def _task_means(cells: list[dict]) -> dict[tuple[str, str], dict]:
    """Per (arm, task) mean of RF_all, accept and RF_acc over repeats."""
    from collections import defaultdict
    acc: dict[tuple[str, str], list[float]] = defaultdict(list)
    accf: dict[tuple[str, str], list[float]] = defaultdict(list)
    rfs: dict[tuple[str, str], list[float]] = defaultdict(list)
    for c in cells:
        key = (c["arm"], c["task"])
        acc[key].append(1.0 if c.get("accepted") else 0.0)
        rfs[key].append((c.get("rf") or 0.0) if c.get("accepted") else 0.0)
        if c.get("accepted") and c.get("rf") is not None:
            accf[key].append(c["rf"])
    out = {}
    for key in rfs:
        out[key] = {"rf_all": sum(rfs[key]) / len(rfs[key]),
                    "accept": sum(acc[key]) / len(acc[key]),
                    "rf_acc": (sum(accf[key]) / len(accf[key])) if accf[key] else None}
    return out


def _bootstrap_ci(diffs: list[float], *, seed: int = 20260923, iters: int = 10000) -> tuple[float, float, float]:
    import random
    if not diffs:
        return (0.0, 0.0, 0.0)
    mean = sum(diffs) / len(diffs)
    rng = random.Random(seed)
    n = len(diffs)
    means = []
    for _ in range(iters):
        means.append(sum(diffs[rng.randrange(n)] for _ in range(n)) / n)
    means.sort()
    lo = means[int(0.025 * iters)]
    hi = means[min(iters - 1, int(0.975 * iters))]
    return (mean, lo, hi)


def _wilcoxon_p(diffs: list[float]) -> float | None:
    import math
    nonzero = [d for d in diffs if d != 0]
    n = len(nonzero)
    if n < 1:
        return None
    order = sorted(range(n), key=lambda i: abs(nonzero[i]))
    ranks = [0.0] * n
    i = 0
    while i < n:
        j = i
        while j + 1 < n and abs(nonzero[order[j + 1]]) == abs(nonzero[order[i]]):
            j += 1
        avg = (i + j) / 2 + 1
        for k in range(i, j + 1):
            ranks[order[k]] = avg
        i = j + 1
    w_plus = sum(ranks[i] for i in range(n) if nonzero[i] > 0)
    w_minus = sum(ranks[i] for i in range(n) if nonzero[i] < 0)
    w = min(w_plus, w_minus)
    mean_w = n * (n + 1) / 4
    sd = (n * (n + 1) * (2 * n + 1) / 24) ** 0.5
    if sd == 0:
        return None
    z = (w - mean_w) / sd
    return 2 * (1 - 0.5 * (1 + math.erf(abs(z) / (2 ** 0.5))))


def render_probe_ci(tex_name: str = "gen_ci.tex") -> str:
    return tex_name


def ci_table(cells: dict, tiers: dict[str, set[str]] | None = None) -> str:
    all_cells = list(cells.values())
    means = _task_means(all_cells)
    tasks = sorted({c["task"] for c in all_cells})
    comps = [("G3_concir", "G0_direct"), ("G3_concir", "G1_self_iter"),
             ("G3_concir", "G2_tools_iter"), ("G3_concir", "G3_codegen")]
    if tiers is None:
        tiers = {}
        for c in all_cells:
            if c.get("tier"):
                tiers.setdefault(c["tier"], set()).add(c["task"])
    groups = {"all": set(tasks)}
    groups.update({k: v for k, v in tiers.items() if v})
    rows = [r"% gen_ci.tex — generated by cir_workflow results (paired bootstrap, 10k, seed 20260923)",
            r"\begin{tabular}{llrrr}", r"\toprule",
            r"group & comparison & metric & delta [95\% CI] & $p$ \\", r"\midrule"]
    for gname, gtasks in groups.items():
        gtasks = sorted(t for t in gtasks if t in tasks)
        for a, b in comps:
            for metric in ("rf_all", "rf_acc", "accept"):
                diffs = []
                for t in gtasks:
                    x = means.get((a, t), {}).get(metric)
                    y = means.get((b, t), {}).get(metric)
                    if x is not None and y is not None:
                        diffs.append(x - y)
                if not diffs:
                    continue
                mean, lo, hi = _bootstrap_ci(diffs)
                p = _wilcoxon_p(diffs)
                ps = f"{p:.3f}" if p is not None else "--"
                rows.append(f"{gname} & {_tex_arm(a)} vs {_tex_arm(b)} & "
                            f"{metric.replace('_', chr(92)+'_')} & "
                            f"{mean:.3f} [{lo:.3f}, {hi:.3f}] & {ps} " + r"\\")
    rows += [r"\bottomrule", r"\end{tabular}"]
    return "\n".join(rows) + "\n"
