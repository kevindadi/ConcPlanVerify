#!/usr/bin/env python3
"""Render the strong-link-v23 phase-1 markdown deliverables from the JSON."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
P1 = REPO / "experiments/strong-link-v23-phase1"
ARMS = ["G0_direct", "G1_self_iter", "G2_tools_iter", "G3_concir"]


def load(name):
    return json.loads((P1 / name).read_text())


def unified_table(results, models):
    rows = ["| model | arm | accepted | delivered-eval | RC_all | RF_all | RF_acc | RF_last_generated | RF_last_build_ok |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for m in models:
        for a in ARMS:
            x = results["per_model_arm"].get(m, {}).get(a)
            if not x:
                continue
            rows.append(f"| {m} | {a} | {x['accepted']}/{x['planned']} | "
                        f"{x['delivered_evaluable']} | {x['rc_all']} | {x['rf_all']} | "
                        f"{x['rf_acc']} | {x['rf_last_generated']} | {x['rf_last_build_ok']} |")
    return "\n".join(rows)


def per_cell_table(rows):
    out = ["| source | model | task | arm | acc | status | delivered RF | last-gen RF | note |",
           "| --- | --- | --- | --- | :-: | --- | ---: | ---: | --- |"]
    for r in rows:
        note = ""
        if r["accepted"] and r["delivered_coverage"] is None:
            note = "instrumented build failed"
        out.append(f"| {r['source']} | {r['model_id']} | {r['task']} | {r['arm']} | "
                   f"{'Y' if r['accepted'] else 'n'} | {r.get('record_status') or ''} | "
                   f"{r['delivered_rf']} | {r['last_generated_rf']} | {note} |")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--notes", required=True)
    args = ap.parse_args()
    notes = Path(args.notes); notes.mkdir(parents=True, exist_ok=True)
    results = load("UNIFIED_RESULTS.json")
    rows = [json.loads(l) for l in (P1 / "PER_CELL.jsonl").read_text().splitlines()]
    new = [r for r in rows if r["source"] == "v22"]
    hist = [r for r in rows if r["source"] == "deepseek-history"]
    models = ["qwen3.8-flash", "gpt-6-luna", "glm-5.3-flash"]

    (notes / "UNIFIED_RESULTS.md").write_text(
        "# strong-link-v23 统一结果（零真实请求）\n\n"
        "评分器：`rust_oracle.evaluate`，同一合同、同一插桩、32 次原生运行、无 Miri、"
        "`run_timeout=10s`。`RF_all` 把未交付格计 0；`RF_acc` 只覆盖已交付且可评的格；"
        "`RF_last_generated`/`RF_last_build_ok` 是敏感性范围，不作为结论。\n\n"
        "## 新三模型 72 格\n\n" + unified_table(results, models) +
        "\n\n## 历史 DeepSeek 24 格（协议不同，仅供对照）\n\n" +
        unified_table({"per_model_arm": results["deepseek_history"]}, ["deepseek-flash-hist"]) +
        "\n\n## 逐格\n\n" + per_cell_table(rows) + "\n", encoding="utf-8")

    rec = load("REQUEST_RECONCILIATION.json")
    usage_lines = ["| model | calls | input | output | reasoning | missing fields | null usage |",
                   "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for m, u in rec["usage_by_model"].items():
        usage_lines.append(f"| {m} | {u['calls']} | {u['input']} | {u['output']} | "
                           f"{u['reasoning']} | {u['missing']} | {u['null_usage']} |")
    (notes / "TOKEN_LEDGER.md").write_text(
        "# strong-link-v23 TOKEN_LEDGER（旧三模型 72 格）\n\n"
        "只汇总格内 `audit.jsonl` 的服务端 usage。缺失字段记 null，不补零；"
        "reasoning 是 output 的子项，不重复相加。缓存回放不产生新用量。\n\n"
        + "\n".join(usage_lines) + "\n\n" +
        f"审计事件总数 {rec['audit_events_total']}，渠道计数 {rec['audit_events_by_channel']}，"
        f"身份确认 {rec['identity_confirmed']}，身份未确认/不符 "
        f"{rec['identity_unconfirmed_or_mismatch']}。\n\n"
        f"预算文件：`{rec['budget']}`。历史请求数与新批次数分别报告，不合并。\n", encoding="utf-8")

    (notes / "REQUEST_RECONCILIATION.md").write_text(
        "# strong-link-v23 请求/渠道/缓存对账（旧三模型）\n\n"
        "```json\n" + json.dumps(rec, ensure_ascii=False, indent=2) + "\n```\n", encoding="utf-8")

    g1 = load("G1_DIAGNOSIS.json")
    (notes / "G1_DIAGNOSIS.md").write_text(
        "# strong-link-v23 G1（self-review）诊断\n\n" + g1["rule"] + "\n\n" +
        f"轮次决定统计：`{g1['decision_totals']}`。\n\n**结论**：{g1['finding']}\n\n"
        "逐格决定序列见 `experiments/strong-link-v23-phase1/G1_DIAGNOSIS.json`。\n",
        encoding="utf-8")

    g3 = load("G3_DIAGNOSIS.json")
    (notes / "G3_DIAGNOSIS.md").write_text(
        "# strong-link-v23 G3 诊断\n\n" + g3["rule"] + "\n\n"
        f"阶段计数：`{g3['stage_counts']}`。\n\n**结论**：{g3['finding']}\n\n"
        "逐格 CIR 逐轮 `check/explore` 复算结果见 "
        "`experiments/strong-link-v23-phase1/G3_DIAGNOSIS.json`。\n", encoding="utf-8")

    man = load("COMPARISON_MANIFEST.json")
    (notes / "COMPARISON_MANIFEST.md").write_text(
        "# strong-link-v23 COMPARISON_MANIFEST\n\n" + man["note"] + "\n\n"
        f"共 {man['count']} 格。逐格来源、交付候选路径与 sha256、审计 provenance 见 "
        "`experiments/strong-link-v23-phase1/COMPARISON_MANIFEST.json`。\n", encoding="utf-8")
    print("rendered phase-1 markdown to", notes)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
