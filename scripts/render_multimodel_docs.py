#!/usr/bin/env python3
"""Render MODEL_TRANSPORT_MANIFEST.md and MULTIMODEL_PROTOCOL.md from the
versioned manifest and the latest connectivity smoke."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def _latest_smoke(root: Path) -> dict | None:
    for b in sorted(root.glob("smoke-*"), reverse=True):
        if (b / "SUMMARY.json").is_file():
            return json.loads((b / "SUMMARY.json").read_text())
    return None


def render_manifest(manifest: dict, smoke: dict | None) -> str:
    L = ["# A. MODEL_TRANSPORT_MANIFEST", "",
         f"Manifest `{manifest['manifest_version']}`，构建于 {manifest['built_at']}。",
         "展示名不等于 API ID；ID 来自实测 `models.list`。", "",
         "## 模型", "",
         "| 展示名 | 请求模型 ID | provider | 渠道 | surface | 状态 | 已发现 |",
         "| --- | --- | --- | --- | --- | --- | --- |"]
    for m in manifest["models"]:
        L.append(f"| {m['display_name']} | {m['model_id'] or '—'} | {m['provider']} | "
                 f"{m['channel']} | {m['surface']} | {m['status']} | "
                 f"{'yes' if m['discovered'] else 'no'} |")
    L += ["", "## 渠道", "",
          "| 渠道 | transport | base_url | key 环境变量 | 已发现模型数 |",
          "| --- | --- | --- | --- | --- |"]
    for c in manifest["channels"]:
        L.append(f"| {c['name']} | {c['transport']} | {c['base_url'] or '—'} | "
                 f"`{c['api_key_env']}` | {len(c['discovered_models'])} |")
    L += ["", "## 不可用模型（blocked）", ""]
    for m in manifest["models"]:
        if m["status"] == "blocked":
            L.append(f"- **{m['display_name']}**：{m['blocked_reason']}")
    L += ["", "## 可观测性与身份约束", "",
          "- 请求与返回模型 ID 均记录；`returned_model != requested_model` 时该结果",
          "  **不得**归到请求模型名下（`ModelIdentityError`）。",
          "- 禁止静默回退、自动替换、`auto` 路由。",
          "- OpenCode Responses surface（GPT 6 Luna、Grok 4.7）无法观测 SDK 内部调用次数，",
          "  事件中记 `sdk_calls_visible = null`。",
          "- Cursor（Composer）会话内部调用不可观测；本轮通道不可用。",
          "- 费用：OpenCode 未回传 `cost`，记为 null；DeepSeek 未提供价格依据。", ""]
    if smoke:
        L += ["## 连通性冒烟（真实调用，1 次/模型）", "",
              "| 模型 | 状态 | 返回模型 | input | output | total |",
              "| --- | --- | --- | --- | --- | --- |"]
        for r in smoke["rows"]:
            L.append(f"| {r['model']} | {r['status']} | {r['returned_model'] or '—'} | "
                     f"{r['input_tokens']} | {r['output_tokens']} | {r['total_tokens']} |")
        L += ["", f"原始事件：`{smoke['batch']}/REQUEST_EVENTS.jsonl`。"]
    return "\n".join(L) + "\n"


def render_protocol(manifest: dict) -> str:
    tasks = manifest["tasks"]
    fams: dict[str, int] = {}
    tiers: dict[str, int] = {}
    for t in tasks:
        fams[t["family"]] = fams.get(t["family"], 0) + 1
        tiers[t["tier"]] = tiers.get(t["tier"], 0) + 1
    n_models = sum(1 for m in manifest["models"] if m["status"] == "available")
    upper = len(tasks) * manifest["budgets"]["reps_generation_main"] * \
        (manifest["budgets"]["K_cir"] + manifest["budgets"]["K_code"]) * n_models
    L = ["# B. MULTIMODEL_PROTOCOL", "",
         "实验组取自 `python/cir_workflow/arms.py`（生成）与",
         "`python/cir_workflow/experiments_v2.py`（修复），不按名称猜测。", "",
         "## 任务", "",
         f"- 全部 **{len(tasks)}** 个生成任务，不删难例。",
         f"- 家族：{', '.join(f'{k}={v}' for k, v in sorted(fams.items()))}。",
         f"- 档位：{', '.join(f'{k}={v}' for k, v in sorted(tiers.items()))}。", "",
         "## 实验组", "",
         f"- 生成：{', '.join(manifest['arms']['generation'])}。",
         f"- 修复（第二研究）：{', '.join(manifest['arms']['repair'])}。",
         "- 依赖模型的组才跨模型重复；纯工具组（如 G3_codegen 的翻译器部分）不机械重复。", "",
         "## 预算（冻结，不因模型放宽）", "",
         f"- `K_cir = {manifest['budgets']['K_cir']}`，`K_code = {manifest['budgets']['K_code']}`。",
         "- CIR 未通过不进入代码阶段；代码阶段不得修改已验证 CIR。",
         f"- 首次生成算第 {manifest['budgets']['first_candidate_round']} 轮。",
         f"- 重复：主实验 {manifest['budgets']['reps_generation_main']} 次；消融 {manifest['budgets']['reps_ablation']} 次。", "",
         "## 接受标准", ""]
    for k, v in manifest["acceptance"].items():
        L.append(f"- `{k}`：{v}")
    L += ["", "## 公平条件", "",
          "- 同实验组在不同模型间使用相同任务、提示语义、反馈规则、候选预算与验证配置。",
          "- 保留各方法本身的协议差异，不强行统一成同一方法。",
          "- 成本差异如实报告；轮次相同不等于成本公平。", "",
          "## 主实验与诊断分离", "",
          "- 主比较：各模型从**原始需求**出发完成完整 G3 流程。",
          "- 固定 CIR 重跑：单独配对诊断（研究接口/工具修复影响），**不并入**主比较。", "",
          "## 历史结果复用", "",
          "只有任务、提示、模型版本、渠道、工具、预算与完整调用证据都符合本协议才可复用；",
          "旧渠道（OpenCode 旧提示/工具）结果保留为历史对照。", "",
          "## 请求上界", "",
          f"- 可用模型数 = {n_models}；每格上界 = K_cir + K_code = "
          f"{manifest['budgets']['K_cir'] + manifest['budgets']['K_code']}。",
          f"- 主比较请求上界 ≈ 任务 × 重复 × (K_cir+K_code) × 模型数 = "
          f"{len(tasks)} × {manifest['budgets']['reps_generation_main']} × "
          f"{manifest['budgets']['K_cir'] + manifest['budgets']['K_code']} × {n_models} = {upper}。",
          "- 运行严格受有限预算约束（`LiveBudget` 持久化，含传输重试与中断）。", ""]
    return "\n".join(L) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="experiments/multimodel-v1")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    root = REPO / args.root
    manifest = json.loads((root / "MANIFEST.json").read_text())
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "MODEL_TRANSPORT_MANIFEST.md").write_text(
        render_manifest(manifest, _latest_smoke(root)), encoding="utf-8")
    (out / "MULTIMODEL_PROTOCOL.md").write_text(
        render_protocol(manifest), encoding="utf-8")
    print(f"wrote {out}/MODEL_TRANSPORT_MANIFEST.md and MULTIMODEL_PROTOCOL.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
