#!/usr/bin/env python3
"""Recompute the strong-link-v6 four-model report from per-cell result.json files."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path

MODELS = ["deepseekflash", "qwen", "gpt6luna", "kimi"]
PROTOCOL = "strong-link-v6"
MISSING = {"cir_not_accepted", "cir_unknown_or_raw_error", "cir_accepted_no_rust"}
COUNTEREXAMPLE = {"explicit_failure", "functional_failure", "requirement_failure"}
TOOL = {"tool_error", "instrument_failed", "instrument_build_failed"}


def _load(root: Path) -> list[dict]:
    rows = []
    for path in sorted(root.glob("reexec-*/**/result.json")):
        # A second evaluation of a different round is not a second task.
        if path.parent.name == "final-only":
            continue
        row = json.loads(path.read_text(encoding="utf-8"))
        row["evidence_path"] = str(path)
        rows.append(row)
    return rows


def stage_class(row: dict) -> str:
    verdict = (row.get("ledger") or {}).get("current_evaluation")
    if verdict in MISSING or verdict == "source_build_failed" or verdict in TOOL:
        return verdict if verdict not in TOOL else "tool_failure"
    if verdict in COUNTEREXAMPLE:
        return "counterexample"
    if verdict == "satisfied_bounded":
        return "satisfied_bounded"
    if verdict == "binding_declaration_error":
        return "protocol_noncompliance"
    delivery = (row.get("ledger") or {}).get("delivery_status")
    if delivery == "withhold_tool":
        return "tool_failure"
    return "checker_or_insufficient"


def _slug(row: dict) -> str:
    model = row.get("model") or ""
    if model in MODELS:
        return model
    return {
        "DeepSeek Flash": "deepseekflash", "Qwen": "qwen",
        "GPT 6 Luna": "gpt6luna", "Kimi": "kimi",
    }.get(model, model)


def _matrix_row(row: dict) -> dict:
    ledger = row.get("ledger") or {}
    binding = row.get("binding_assessment") or {}
    functional = ledger.get("functional") or row.get("functional") or {}
    return {
        "model": _slug(row), "task": row.get("task"), "rep": row.get("rep", 0),
        "cell": row.get("cell"), "protocol": ledger.get("protocol") or row.get("protocol"),
        "origin_batch": row.get("origin_batch"),
        "candidate_kind": row.get("candidate_kind"),
        "accepted_round": row.get("accepted_round"), "final_round": row.get("final_round"),
        "same_round": row.get("same_round"), "round_no": row.get("round_no"),
        "historical_acceptance": ledger.get("historical_acceptance"),
        "current_evaluation": ledger.get("current_evaluation"),
        "evidence_grade": ledger.get("evidence_grade"),
        "delivery_status": ledger.get("delivery_status"),
        "needs_human_review": ledger.get("needs_human_review"),
        "needs_extra_check": ledger.get("needs_extra_check"),
        "stage_class": stage_class(row),
        "run_state": (ledger.get("run") or {}).get("state"),
        "trace_state": (ledger.get("trace") or {}).get("state"),
        "model_verified": (ledger.get("model") or {}).get("verified"),
        "binding_status": binding.get("status"),
        "binding_ran": bool(binding.get("ran")),
        "binding_sufficient": bool(binding.get("sufficient")),
        "functional_status": functional.get("status", "not_run"),
        "source_sha256": row.get("source_sha256"), "cir_sha256": row.get("cir_sha256"),
        "contract_sha256": row.get("contract_sha256"),
        "source_path": row.get("source_path"), "cir_path": row.get("cir_path"),
        "contract_path": row.get("contract_path"),
        "spot_rerun": row.get("spot_rerun") or [],
        "evidence_path": row.get("evidence_path"),
    }


def _model_block(rows: list[dict]) -> dict:
    verdicts = Counter(r["current_evaluation"] for r in rows)
    stages = Counter(r["stage_class"] for r in rows)
    binding = Counter(r["binding_status"] for r in rows)
    functional = Counter(r["functional_status"] for r in rows)
    kinds = Counter(r["candidate_kind"] for r in rows)
    with_source = [r for r in rows if r["stage_class"] not in MISSING]
    return {
        "model": rows[0]["model"] if rows else "",
        "candidates": len(rows),
        "historical_accepted": sum(1 for r in rows if r["historical_acceptance"]),
        "candidate_kind": dict(kinds),
        "verdicts": dict(verdicts),
        "stage_class": dict(stages),
        "with_source": len(with_source),
        "with_source_verdicts": dict(Counter(r["current_evaluation"] for r in with_source)),
        "binding_status": dict(binding),
        "binding_sufficient": sum(1 for r in rows if r["binding_sufficient"]),
        "functional": dict(functional),
        "satisfied_bounded": verdicts.get("satisfied_bounded", 0),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    root = Path(args.root)
    loaded = _load(root)
    matrix = [_matrix_row(r) for r in loaded]
    if len(matrix) != 96:
        print(f"expected 96 task rows, found {len(matrix)}")
    by_model = defaultdict(list)
    for row in matrix:
        by_model[row["model"]].append(row)
    blocks = [_model_block(by_model[name]) for name in MODELS]
    summary = {
        "protocol": PROTOCOL,
        "ledger_version": PROTOCOL,
        "note": "evidence grades are a re-interpretation of frozen candidates, not a new generation success rate",
        "models": blocks,
        "total_candidates": len(matrix),
        "verdicts": dict(sum((Counter(b["verdicts"]) for b in blocks), Counter())),
        "stage_class": dict(sum((Counter(b["stage_class"]) for b in blocks), Counter())),
        "binding_sufficient": sum(b["binding_sufficient"] for b in blocks),
        "functional": dict(sum((Counter(b["functional"]) for b in blocks), Counter())),
    }
    (root / "SUMMARY.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
                                       encoding="utf-8")
    (root / "FULL_MATRIX.jsonl").write_text(
        "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in matrix), encoding="utf-8")
    manifest = []
    for row in matrix:
        manifest.append({
            "model": row["model"], "task": row["task"], "rep": row["rep"],
            "cell": row["cell"], "origin_batch": row["origin_batch"],
            "protocol": PROTOCOL,
            "historical_accepted": row["historical_acceptance"],
            "candidate_kind": row["candidate_kind"],
            "accepted_round": row["accepted_round"], "final_round": row["final_round"],
            "evaluated_round": row["round_no"], "same_round": row["same_round"],
            "source_sha256": row["source_sha256"], "cir_sha256": row["cir_sha256"],
            "contract_sha256": row["contract_sha256"],
            "source_path": row["source_path"], "cir_path": row["cir_path"],
            "contract_path": row["contract_path"],
        })
    (root / "INPUT_MANIFEST.jsonl").write_text(
        "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in manifest), encoding="utf-8")

    lines = [
        f"# 四模型离线重评（{PROTOCOL}）",
        "",
        "本表由 `scripts/fourmodel_report.py` 从各 cell 的 `result.json` 重算。"
        "证据等级是对冻结候选的当前解释，不是新的生成成功率。"
        "有源码子集上的比例不能用来推断整体 CIR 不是瓶颈：没有进入代码阶段的 cell 仍在 96 的分母里。",
        "",
        "## 阶段流失（96 cells）",
        "",
        "| 模型 | cells | CIR 未通过 | CIR 未知/调用错误 | CIR 通过但无 Rust | 编译失败 | 工具失败 | 检查器不支持或证据不足 | 协议声明错误 | 有效反例 | 有限证据满足 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    keys = ["cir_not_accepted", "cir_unknown_or_raw_error", "cir_accepted_no_rust",
            "source_build_failed", "tool_failure", "checker_or_insufficient",
            "protocol_noncompliance", "counterexample", "satisfied_bounded"]
    for block in blocks:
        stage = block["stage_class"]
        lines.append("| " + " | ".join([
            block["model"], str(block["candidates"]),
            *[str(stage.get(k, 0)) for k in keys],
        ]) + " |")
    total_stage = Counter()
    for block in blocks:
        total_stage.update(block["stage_class"])
    lines.append("| 合计 | " + " | ".join([
        str(summary["total_candidates"]), *[str(total_stage.get(k, 0)) for k in keys],
    ]) + " |")
    lines += [
        "",
        "## 有源码候选的条件统计",
        "",
        "分母是已经有 Rust 源码的 cell。CIR 未通过、调用错误和有 CIR 但无源码的 cell 不在此表中，它们仍计入上一表。",
        "",
        "| 模型 | 有源码 | verdicts |",
        "| --- | ---: | --- |",
    ]
    for block in blocks:
        lines.append(f"| {block['model']} | {block['with_source']} | {block['with_source_verdicts']} |")
    lines += [
        "",
        "## accepted / final",
        "",
        "accepted 使用历史接受轮次的源码。final 是最后一轮。两者相同时复用同一次执行证据，任务仍计 1。",
        "",
        "| 模型 | 任务 | 历史接受 | accepted 记录 | final 记录 | 同轮复用 |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for name in MODELS:
        rows = by_model[name]
        acc = sum(1 for r in rows if r["candidate_kind"] == "accepted")
        fin = sum(1 for r in rows if r["candidate_kind"] == "final")
        same = sum(1 for r in rows if r["same_round"] and r["historical_acceptance"])
        hist = sum(1 for r in rows if r["historical_acceptance"])
        lines.append(f"| {name} | {len(rows)} | {hist} | {acc} | {fin} | {same} |")
    lines += [
        "",
        "## 绑定",
        "",
        "`sufficient` 要求绑定阶段实际成功、无 unresolved/violated、且所需对象被覆盖。"
        "`not_run` 与 `not_required` 都不是充分。编译失败不计入充分。",
        "",
        "| 模型 | not_run | not_required | insufficient | sufficient |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for name in MODELS:
        rows = by_model[name]
        counts = Counter(r["binding_status"] for r in rows)
        lines.append(f"| {name} | {counts.get('not_run', 0)} | {counts.get('not_required', 0)} | "
                     f"{counts.get('insufficient', 0)} | {counts.get('sufficient', 0)} |")
    lines += [
        "",
        "## 功能检查覆盖",
        "",
        "没有功能规格的 cell 记为 `not_run`，不记为通过。",
        "",
        "| 模型 | functional |",
        "| --- | --- |",
    ]
    for block in blocks:
        lines.append(f"| {block['model']} | {block['functional']} |")
    lines += [
        "",
        "## 当前结论 × 历史接受",
        "",
        "| 当前结论 | 历史接受 | 历史未接受 |",
        "| --- | ---: | ---: |",
    ]
    verdicts = sorted({r["current_evaluation"] for r in matrix})
    for verdict in verdicts:
        yes = sum(1 for r in matrix if r["current_evaluation"] == verdict and r["historical_acceptance"])
        no = sum(1 for r in matrix if r["current_evaluation"] == verdict and not r["historical_acceptance"])
        lines.append(f"| {verdict} | {yes} | {no} |")
    lines += [
        "",
        f"协议 `{PROTOCOL}`。Qwen `condvar/notify_one_multi_waiter_wrong_pick` 为源码编译失败，绑定状态为 not_run。",
        "",
    ]
    (root / "FOURMODEL_OFFLINE_REPORT.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"cells={len(matrix)} satisfied={summary['verdicts'].get('satisfied_bounded', 0)} "
          f"binding_sufficient={summary['binding_sufficient']}")
    return 0 if len(matrix) == 96 else 1


if __name__ == "__main__":
    raise SystemExit(main())
