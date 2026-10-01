#!/usr/bin/env python3
"""Freeze and validate the six-case feedback pilot. This command does not call a model."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "python"))

from cir_workflow.feedback_runner import PROMPT_PATH, validate_config  # noqa: E402
from cir_workflow.pilot_cases import CASES, FIXTURE_ROOT, evaluate_role, load_case  # noqa: E402


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _ref(path: Path) -> dict:
    return {"path": str(path), "sha256": _sha(path)}


def freeze_inputs() -> tuple[dict, list[dict], list[dict]]:
    backend = REPO.parent / "ConcIR/target/release/concir-backend"
    instrument = REPO.parent / "ConcIR/target/release/concir-instrument"
    preflight = FIXTURE_ROOT / "PREFLIGHT.json"
    oracle_py = REPO / "python/cir_workflow/pilot_oracle.py"
    cases_py = REPO / "python/cir_workflow/pilot_cases.py"
    config = {
        "version": "feedback-pilot-v10",
        "status": "frozen-dry-run",
        "question": "实现阶段的 CIR 一致性反馈消融：固定已验证 CIR 与初始 Rust 后，被测工具链的判定和结构化诊断是否改变修复结果。",
        "not_claimed": "工具诊断不全是 CVN 穷尽反例；独立 oracle 只评分，不进入反馈。",
        "models": ["DeepSeek Flash", "Qwen", "GPT 6 Luna", "GLM 5.3 Flash"],
        "model_ids": ["deepseek-flash", "qwen3.8-flash", "gpt-6-luna", "glm-5.3-flash"],
        "arms": ["verdict_only", "counterexample"],
        "max_repairs": 2,
        "task_set": "development-mechanism-pilot",
        "holdout": False,
        "prompt": str(PROMPT_PATH),
        "prompt_sha256": _sha(PROMPT_PATH),
        "tools": {
            "backend": {"path": str(backend), "sha256": _sha(backend) if backend.is_file() else None},
            "instrument": {"path": str(instrument),
                           "sha256": _sha(instrument) if instrument.is_file() else None},
            "n_runs": 2,
            "run_timeout_s": 8,
            "no_join_run_timeout_s": 2,
        },
        "preflight": _ref(preflight) if preflight.is_file() else {"path": str(preflight), "sha256": None},
        "oracle_sha256": _sha(oracle_py),
        "cases_sha256": _sha(cases_py),
        "only_difference": "feedback text",
    }
    rows = []
    oracle_rows = []
    for name in CASES:
        directory = FIXTURE_ROOT / name
        case = load_case(name)
        row = {
            "id": name,
            "class": case["class"],
            "origin": case["origin"],
            "origin_kind": "hand-authored",
            "defect": _ref(directory / "defect.rs"),
            "control": _ref(directory / "control.rs"),
            "cir": _ref(directory / "design.cir.json"),
            "contract": _ref(directory / "contract.json"),
            "requirements": _ref(directory / "requirements.md"),
            "prompt_sha256": config["prompt_sha256"],
            "oracle_sha256": config["oracle_sha256"],
            "cases_sha256": config["cases_sha256"],
            "preflight_sha256": config["preflight"]["sha256"],
        }
        rows.append(row)
    return config, rows, oracle_rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    parser.add_argument("--inputs", required=True)
    parser.add_argument("--freeze", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--out", default=None)
    parser.add_argument("--oracle-out", default=None)
    parser.add_argument("--execute", action="store_true",
                        help="Future paid entry. Not used in strong-link-v7.")
    args = parser.parse_args()
    config_path, inputs_path = Path(args.config), Path(args.inputs)
    if args.freeze:
        config, rows, _oracle = freeze_inputs()
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n",
                               encoding="utf-8")
        inputs_path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                               encoding="utf-8")
    else:
        config = json.loads(config_path.read_text(encoding="utf-8"))
        rows = [json.loads(line) for line in inputs_path.read_text(encoding="utf-8").splitlines()
                if line.strip()]
    if args.oracle_out:
        import tempfile
        report = []
        for name in CASES:
            case = load_case(name)
            with tempfile.TemporaryDirectory() as td:
                root = Path(td)
                defect = evaluate_role(case, case["defect"], root / "defect")
                control = evaluate_role(case, case["control"], root / "control")
            report.append({
                "id": name, "class": case["class"], "origin": case["origin"],
                "defect": {"requirement": defect["requirement"]["status"],
                           "design": defect["design"]["status"],
                           "requirement_error": defect["requirement_error"],
                           "cir_design_deviation": defect["cir_design_deviation"],
                           "supported": defect["support"]["supported"],
                           "in_repair_denominator": defect["requirement_error"]
                           and defect["support"]["supported"]
                           and not defect["tool_error"]},
                "control": {"requirement": control["requirement"]["status"],
                            "design": control["design"]["status"],
                            "model_requests": 0},
            })
        Path(args.oracle_out).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n",
                                        encoding="utf-8")
    if args.execute:
        print("refusing: this process was not started as the future paid run")
        return 2
    if not args.dry_run and not args.freeze:
        print("refusing to call a model; pass --dry-run")
        return 2
    plan = validate_config(config, rows)
    text = json.dumps({k: v for k, v in plan.items() if k != "matrix"},
                      ensure_ascii=False, indent=2)
    print(text)
    if args.out:
        Path(args.out).write_text(json.dumps(plan, ensure_ascii=False, indent=2) + "\n",
                                  encoding="utf-8")
    return 0 if plan["ok"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
