#!/usr/bin/env python3
"""Aggregate the four-model offline re-evaluation into one derived report.

Reads each model's derived SUMMARY.json (built from per-cell ledgers) and emits
a combined SUMMARY.json and a markdown report. No manual edits; everything is
recomputable from the per-cell result.json files.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

MODELS = ["deepseekflash", "qwen", "gpt6luna", "kimi"]


def _model_row(root: Path, name: str) -> dict:
    summary = json.loads((root / f"reexec-{name}" / "SUMMARY.json").read_text())
    cells = summary["cells"]
    verdicts = Counter(c["current_evaluation"] for c in cells)
    trace = Counter(c["trace_state"] for c in cells)
    run = Counter(c["run_state"] for c in cells)
    kinds = Counter(c["candidate_kind"] for c in cells)
    hist_acc = sum(1 for c in cells if c.get("historical_acceptance"))
    # binding sufficiency: read per-cell ledgers for identity.unresolved
    binding_ok = 0
    binding_not_run = 0
    for c in cells:
        r = json.loads(Path(c["evidence_path"]).read_text())
        led = r["ledger"]
        ident = led.get("identity") or {}
        ran = (r.get("stages") or {}).get("binding") == "ok" \
            and (r.get("binding") or {}).get("source") == "rust-cli"
        if not ran:
            binding_not_run += 1
        elif not ident.get("relevant_unresolved") and not ident.get("declaration_error"):
            binding_ok += 1
    return {
        "model": name, "candidates": len(cells), "verdicts": dict(verdicts),
        "trace": dict(trace), "run": dict(run), "candidate_kind": dict(kinds),
        "historical_accepted": hist_acc,
        "executable": run.get("completed", 0),
        "binding_sufficient": binding_ok, "binding_not_run": binding_not_run,
        "trace_conformant": trace.get("observed_conformant", 0),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    args = ap.parse_args()
    root = Path(args.root)
    rows = [_model_row(root, m) for m in MODELS]
    combined = {"models": rows,
                "total_candidates": sum(r["candidates"] for r in rows),
                "verdicts": dict(sum((Counter(r["verdicts"]) for r in rows), Counter()))}
    (root / "SUMMARY.json").write_text(json.dumps(combined, ensure_ascii=False, indent=2) + "\n")

    lines = ["# 四模型离线重评（strong-link-v5）", "",
             "逐 cell 结果由 `scripts/reexecute_and_bind.py` 产生，本表由 `scripts/fourmodel_report.py` "
             "从各模型派生 SUMMARY 重算。**不作为新的生成成功率**；accepted/final 与历史接受分开。", "",
             "| 模型 | 候选 | 历史接受 | accepted/final | 可执行 | 绑定充分 | 轨迹符合 | verdicts |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for r in rows:
        lines.append(f"| {r['model']} | {r['candidates']} | {r['historical_accepted']} | "
                     f"{r['candidate_kind'].get('accepted',0)}/{r['candidate_kind'].get('final',0)} | "
                     f"{r['executable']} | {r['binding_sufficient']} | {r['trace_conformant']} | "
                     f"{r['verdicts']} |")
    lines += ["", "## 流失原因（trace/run 层）", "",
              "| 模型 | run | trace |", "| --- | --- | --- |"]
    for r in rows:
        lines.append(f"| {r['model']} | {r['run']} | {r['trace']} |")
    (root / "FOURMODEL_OFFLINE_REPORT.md").write_text("\n".join(lines) + "\n")
    for r in rows:
        print(f"{r['model']}: cand={r['candidates']} hist_acc={r['historical_accepted']} "
              f"exec={r['executable']} bind_ok={r['binding_sufficient']} "
              f"trace_ok={r['trace_conformant']} verdicts={r['verdicts']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
