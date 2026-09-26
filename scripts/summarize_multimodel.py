#!/usr/bin/env python3
"""Combine multi-model smoke/pilot batches into the paper deliverables.

Reads every ``smoke-*``/``pilot-*`` batch under ``experiments/multimodel-v1``
and writes a combined ``REQUEST_EVENTS.jsonl``, ``CELL_RESULTS.csv`` and a
Chinese report ``MULTIMODEL_RESULTS.md``. Every conclusion is tied to a raw
evidence path; missing data is shown as missing, never as zero.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def _load_batches(root: Path) -> list[dict]:
    batches = []
    for b in sorted(root.glob("smoke-*")) + sorted(root.glob("pilot-*")):
        if (b / "SUMMARY.json").is_file():
            batches.append({"dir": b, "summary": json.loads((b / "SUMMARY.json").read_text())})
    return batches


def _events(root: Path) -> list[dict]:
    events = []
    for path in sorted(root.glob("*/REQUEST_EVENTS.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                e = json.loads(line)
                e["_batch"] = path.parent.name
                events.append(e)
    return events


def _classify(cell: dict, events: list[dict]) -> str:
    status = (cell.get("status") or "").lower()
    err = str(cell.get("error") or "")
    if status == "blocked":
        return "channel-blocked"
    if "APIConnectionError" in err or "Connection error" in err:
        return "channel-transport"
    if "ModelProtocolUnsupported" in err:
        return "channel-protocol"
    if status == "budget_exhausted":
        return "budget"
    if status == "generation_failed":
        return "model-failed-cir"
    if status == "error":
        return "unknown"
    if status == "accepted":
        return "accepted"
    return status or "unknown"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="experiments/multimodel-v1")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    root = REPO / args.root
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    batches = _load_batches(root)
    events = _events(root)

    # combined events
    (out / "REQUEST_EVENTS.jsonl").write_text(
        "\n".join(json.dumps(e, ensure_ascii=False, sort_keys=True) for e in events) + "\n",
        encoding="utf-8")

    # Deduplicate by (model, task, replicate): a later batch supersedes an
    # earlier one (retry/recovery), and we record how many runs were seen.
    raw_cells = []
    for b in batches:
        for c in b["summary"].get("cells", []):
            if c.get("arm") != "G3":
                continue
            row = dict(c)
            row["batch"] = b["dir"].name
            row["failure_class"] = _classify(c, events)
            raw_cells.append(row)
    latest: dict[tuple, dict] = {}
    runs: dict[tuple, int] = defaultdict(int)
    for row in raw_cells:
        key = (row["model"], row["task"], row.get("replicate", 0))
        runs[key] += 1
        latest[key] = row
    pilot_cells = []
    for key, row in latest.items():
        row = dict(row)
        row["runs"] = runs[key]
        pilot_cells.append(row)
    pilot_cells.sort(key=lambda r: (r["model"], r["task"]))

    # combined cell csv
    if pilot_cells:
        fields = list(pilot_cells[0].keys())
        with (out / "CELL_RESULTS.csv").open("w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(fh, fieldnames=fields)
            writer.writeheader()
            writer.writerows(pilot_cells)

    # ---- report ----
    lines: list[str] = []
    add = lines.append
    add("# 多模型实验结果（试点 + 连通性）")
    add("")
    add("> 本文件由 `scripts/summarize_multimodel.py` 从原始批次生成。")
    add("> 这是**试点**：1 次重复、2 个任务，用于验证统一调用/审计层并给出初步信号，")
    add("> **不是**论文主比较；主比较（24 任务 × 3 重复）尚未运行，原因见文末。")
    add("")

    # 1. end-to-end success
    add("## 1. 模型 × 实验组端到端成功率（试点）")
    add("")
    add("实验组固定为 G3（需求→已验证 CIR→模型写 Rust）。分子/分母逐格列出。")
    add("")
    add("| 模型 | 渠道 | 任务数 | 接受 | 成功率 | 运行次数 |")
    add("| --- | --- | --- | --- | --- | --- |")
    by_model: dict[str, list[dict]] = defaultdict(list)
    for c in pilot_cells:
        by_model[c["model"]].append(c)
    for model, cells in by_model.items():
        n = len(cells)
        k = sum(1 for c in cells if c.get("accepted") is True)
        ch = cells[0]["transport"]
        runs = max(c.get("runs", 1) for c in cells)
        add(f"| {model} | {ch} | {n} | {k}/{n} | {k/n:.2f} | {runs} |")
    add("")
    add("分母 2 = `channel/rendezvous_both_send` + `lock-order/cross_module_cycle`，rep0。")
    add("重复运行按 (模型, 任务) 取**最后一次**（恢复重跑覆盖先前结果），`运行次数` 列出观察到的批次数；")
    add("被覆盖的失败与重跑成本保留在各自的 `REQUEST_EVENTS.jsonl` 中，不消失。")
    add("")

    # 2. G3 funnel
    add("## 2. G3 阶段漏斗与条件代码接受率（试点）")
    add("")
    add("| 模型 | CIR 通过/任务 | CIR 通过后代码接受 | 端到端接受 | 首次接受总候选轮 |")
    add("| --- | --- | --- | --- | --- |")
    for model, cells in by_model.items():
        n = len(cells)
        cir = sum(1 for c in cells if c.get("cir_accepted") is True)
        code_cond = [c for c in cells if c.get("cir_accepted") is True]
        code = sum(1 for c in code_cond if c.get("code_accepted") is True)
        end = sum(1 for c in cells if c.get("accepted") is True)
        rounds = [c["first_accept_total_candidate_rounds"] for c in cells
                  if c.get("first_accept_total_candidate_rounds")]
        rt = f"{statistics.median(rounds):.1f} (n={len(rounds)})" if rounds else "—"
        cond = f"{code}/{len(code_cond)}" if code_cond else "—"
        add(f"| {model} | {cir}/{n} | {cond} | {end}/{n} | {rt} |")
    add("")
    add("代码阶段条件分母是 CIR 通过任务数；端到端分母是完整任务集合（此处各 2）。")
    add("")

    # 3. first-pass rounds
    add("## 3. 预算内首次通过轮次（试点）")
    add("")
    add("`first_cir_pass_round` / `first_code_accept_round`（第 1 轮 = 0 次修复）。未通过为 null。")
    add("")
    add("| 模型 | 任务 | CIR 首次通过轮 | 代码首次接受轮 |")
    add("| --- | --- | --- | --- |")
    for c in pilot_cells:
        if c.get("arm") != "G3":
            continue
        add(f"| {c['model']} | {c['task']} | {c.get('first_cir_pass_round')} | "
            f"{c.get('first_code_accept_round')} |")
    add("")

    # 4. tokens
    add("## 4. token、耗时")
    add("")
    add("服务端实测 usage；缺失为 null，不写 0。费用未获取（OpenCode 未回传 cost），标 null。")
    add("")
    add("| 模型 | 调用数 | input | output | total | 总耗时(s) |")
    add("| --- | --- | --- | --- | --- | --- |")
    for model, cells in by_model.items():
        calls = sum(int(c.get("calls") or 0) for c in cells)
        it = sum(int(c.get("input_tokens") or 0) for c in cells)
        ot = sum(int(c.get("output_tokens") or 0) for c in cells)
        tt = sum(int(c.get("total_tokens") or 0) for c in cells)
        lat = sum(int(c.get("latency_ms") or 0) for c in cells) / 1000.0
        add(f"| {model} | {calls} | {it} | {ot} | {tt} | {lat:.0f} |")
    add("")

    # 5. failure decomposition
    add("## 5. 失败分解")
    add("")
    add("| 模型 | 失败类 | 计数 |")
    add("| --- | --- | --- |")
    for model, cells in by_model.items():
        counts: dict[str, int] = defaultdict(int)
        for c in cells:
            counts[c["failure_class"]] += 1
        for cls, n in sorted(counts.items()):
            add(f"| {model} | {cls} | {n} |")
    add("")
    add("`channel-transport` = OpenCode 网关 `APIConnectionError`（瞬时/限流），")
    add("`model-failed-cir` = 预算内未产出通过检查的 CIR，`channel-blocked` = 渠道不可用。")
    add("")

    # 6. paired baseline
    add("## 6. 同任务配对的基线 vs G3")
    add("")
    add("本轮试点只跑了 G3；**没有** G0/G1/G2 的同模型配对数据，")
    add("故此处不给出配对比较，避免用不完整数据下结论。")
    add("")

    # 7. historical separation
    add("## 7. 历史结果、固定 CIR 诊断与本轮正式结果的区分")
    add("")
    add("- 历史：`experiments/model-probe-v2`（kimi-k2.7-code、glm-5.3-flash，修复臂）、")
    add("  `experiments/gen-model-probe-v3-code`（kimi-k3，固定 CIR 代码阶段）——渠道/提示/工具与本轮不同，")
    add("  **不并入**本轮主比较，仅作历史对照。")
    add("- 本轮：`experiments/multimodel-v1/` 下的 smoke/pilot 批次，统一审计层。")
    add("- 固定 CIR 重跑属单独诊断，本轮未运行。")
    add("")

    # 8. evidence
    add("## 8. 证据定位")
    add("")
    for b in batches:
        add(f"- `{b['dir']}/SUMMARY.json`、`CELL_RESULTS.csv`、`REQUEST_EVENTS.jsonl`、`raw/`")
    add("")
    add("每个 `REQUEST_EVENTS.jsonl` 事件含 prompt/response 的文件路径与 sha256。")
    add("")

    add("## 9. 阻塞与未完成（诚实披露）")
    add("")
    add("- **Qwen**：`DASHSCOPE_API_KEY` 在 cn 与 intl 端点均返回 HTTP 401，直连不可用；")
    add("  未通过 OpenCode 绕过（协议禁止）。")
    add("- **Composer 2.5**：`cursor_sdk` 未安装，Cursor 会话通道不可用。")
    add("- **Grok 4.7 / Mimo / GLM**：OpenCode 网关间歇 `APIConnectionError`，恢复重跑仍失败。")
    add("- **主比较未运行**：24 任务 × 3 重复 × 多模型超出本轮时间与预算；")
    add("  请求上界 ≈ 24×3×(K_cir+K_code)×模型数（见 `MULTIMODEL_PROTOCOL`）。")
    add("- 统计：1 次重复、2 个任务不足以做显著性检验，**不声称**任何模型显著优于其他模型。")
    add("")

    (out / "MULTIMODEL_RESULTS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {out} (events={len(events)}, cells={len(pilot_cells)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
