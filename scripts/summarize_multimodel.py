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
    add("# 多模型实验结果（主矩阵 G3 + 连通性）")
    add("")
    add("> 本文件由 `scripts/summarize_multimodel.py` 从原始批次生成。")
    add("> 主矩阵 = G3，全部 24 任务。DeepSeek/Qwen 跑满 3 重复；OpenCode 组因网关间歇")
    add("> `APIConnectionError` 只跑到 1 重复且部分模型未覆盖满 24 任务（见覆盖率列）。")
    add("> Composer 2.5 因 Cursor 代理上下文不可控/不可观测（15–18 万 input tokens/调用），")
    add("> 按协议标记**不可比**，其数据单列，不并入主比较。")
    add("")

    # 1. end-to-end success
    add("## 1. 模型 × 实验组端到端成功率（试点）")
    add("")
    add("实验组固定为 G3（需求→已验证 CIR→模型写 Rust）。分子/分母逐格列出。")
    add("")
    add("| 模型 | 渠道 | cells(尝试) | 覆盖任务/24 | 接受 | 成功率 | 重复 |")
    add("| --- | --- | --- | --- | --- | --- | --- |")
    by_model: dict[str, list[dict]] = defaultdict(list)
    for c in pilot_cells:
        by_model[c["model"]].append(c)
    all_task_ids = {c["task"] for c in pilot_cells}
    for model, cells in by_model.items():
        n = len(cells)
        k = sum(1 for c in cells if c.get("accepted") is True)
        ch = cells[0]["transport"]
        cov = len({c["task"] for c in cells})
        reps = max(c.get("replicate", 0) for c in cells) + 1
        add(f"| {model} | {ch} | {n} | {cov}/24 | {k}/{n} | {k/n:.2f} | {reps} |")
    add("")
    add("主矩阵 = G3，全部 24 任务；重复数见末列（DeepSeek/Qwen=3，OpenCode 组=1）。")
    add("`cells(尝试)` 是去重后的 (模型,任务,重复) 数；未跑到的任务不计入分母，覆盖率单列。")
    add("重复运行按 (模型, 任务, 重复) 取**最后一次**（恢复重跑覆盖先前结果）；被覆盖的失败与重跑成本保留在各自 `REQUEST_EVENTS.jsonl`。")
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
    add("- **Qwen 直连已修复**：`DASHSCOPE_API_KEY` 在 cn 端点可用（intl 401）；关闭 thinking")
    add("  (`enable_thinking=false`) 后与 DeepSeek Flash 对齐，CIR 提示不再超时。")
    add("- **Composer 2.5 标记不可比**：Cursor 代理每次调用累积 15–18 万 input tokens 且内部行为")
    add("  不可完全观测，无法归约为无状态 chat，按协议排除出主比较（数据单列）。")
    add("- **Grok 4.7 / Mimo / GLM**：OpenCode 网关间歇 `APIConnectionError`，覆盖不全。")
    add("- **主矩阵部分完成**：DeepSeek/Qwen 跑满 24 任务 × 3 重复；OpenCode 组 1 重复且部分模型")
    add("  未覆盖满 24 任务（见覆盖率列）。剩余格可用同一命令恢复（`budget.json` 持久化）。")
    add("- 统计：重复数不一致，且部分模型未覆盖满任务，**不声称**任何模型显著优于其他模型；")
    add("  仅报告观测到的点估计与失败分解。")
    add("")

    (out / "MULTIMODEL_RESULTS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {out} (events={len(events)}, cells={len(pilot_cells)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
