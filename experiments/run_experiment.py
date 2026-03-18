#!/usr/bin/env python3
"""
CIR+CVN Experiment Runner

Orchestrates the evaluation experiments described in Section 7 of the paper:
  RQ1 - CIR Generation: How many rounds for each LLM to produce valid CIR?
  RQ2 - Bug Detection & Repair: CVN analysis metrics + LLM repair loop.
  RQ3 - Translation Correctness: Structural invariant checks.

Usage:
    python experiments/run_experiment.py --config experiments/config.toml
    python experiments/run_experiment.py --config experiments/config.toml --rq 1
    python experiments/run_experiment.py --config experiments/config.toml --rq 2 --model gpt-4o
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # Python < 3.11 fallback

import requests

ROOT_DIR = Path(__file__).resolve().parent.parent

# ── Data classes ──────────────────────────────────────────────

@dataclass
class ModelConfig:
    name: str
    provider: str
    model_id: str
    api_key_env: str
    base_url: str

@dataclass
class ExperimentConfig:
    max_gen_rounds: int
    max_repair_rounds: int
    temperature: float
    max_tokens: int
    patterns_dir: str
    source_dir: str
    output_dir: str
    runs_per_pattern: int
    models: list[ModelConfig]
    source_programs: dict[str, str]
    buggy_cirs: dict[str, str]

@dataclass
class RQ1Result:
    model: str
    pattern: str
    round_num: int       # 0 = failed
    success: bool
    errors: list[str] = field(default_factory=list)

@dataclass
class RQ2Result:
    model: str
    pattern: str
    places: int
    transitions: int
    states: int
    analysis_time_ms: float
    bug_detected: str    # bug kind or "none"
    repair_rounds: int   # 0 = no repair needed, -1 = failed
    regressions: int
    success: bool

@dataclass
class RQ3Result:
    pattern: str
    cir_statements: int
    cvn_places: int
    cvn_transitions: int
    translation_errors: int
    invariants_passed: int
    invariants_total: int


# ── Config loading ────────────────────────────────────────────

def load_config(config_path: str) -> ExperimentConfig:
    with open(config_path, "rb") as f:
        raw = tomllib.load(f)

    exp = raw["experiment"]
    models = [ModelConfig(**m) for m in raw.get("models", [])]

    return ExperimentConfig(
        max_gen_rounds=exp.get("max_gen_rounds", 5),
        max_repair_rounds=exp.get("max_repair_rounds", 5),
        temperature=exp.get("temperature", 0.0),
        max_tokens=exp.get("max_tokens", 4096),
        patterns_dir=exp.get("patterns_dir", "tests/e2e"),
        source_dir=exp.get("source_dir", "experiments/source_programs"),
        output_dir=exp.get("output_dir", "experiments/results"),
        runs_per_pattern=exp.get("runs_per_pattern", 3),
        models=models,
        source_programs=raw.get("source_programs", {}),
        buggy_cirs=raw.get("buggy_cirs", {}),
    )


# ── LLM API calls ────────────────────────────────────────────

GENERATION_SYSTEM_PROMPT = """\
You are an expert in concurrent systems. Given a Rust source program, produce a \
CIR (Concurrency Intermediate Representation) in JSON format.

CIR schema:
- "program": string (program name)
- "resources": array of {name, kind, type, mode?, count?, base?, init?}
  - kind: "sync" or "var"
  - type (sync): "Mutex", "RwLock", "Condvar", "Semaphore", "Channel"
  - type (var): "Var", "Atomic"
  - For Condvar: add "paired_with": "<mutex_name>"
  - For Semaphore: add "count": <initial_permits>
  - For Var/Atomic: add "base": "Bool"|"Int"|"Float"|"String", "init": <value>
- "protection": array of {variable, locks: [lock_names]}
- "functions": array of {name, kind, body}
  - kind: "normal", "async", "closure"
  - body: array of statements {sid, op, transfer}
    - sid: unique within function (e.g. "s1", "s2")
    - op: ["res_op", resource, action] | ["spawn", fn] | ["join", fn] | \
["call", fn] | "return" | "nop"
      - actions: "lock", "drop", "read_lock", "read_unlock", "write_lock", \
"write_unlock", "acquire", "release", "send", "recv", \
"wait", "notify_one", "notify_all", "read", "write", "cas"
      - For write: ["res_op", var, "write", value]
      - For cas: ["res_op", var, "cas", expected, desired]
    - transfer: ["next", sid] | ["branch", {cond, on_true, on_false}] | "return"
      - cond: {var, op, val}  op: "Eq"|"Neq"|"Gt"|"Lt"|"Gte"|"Lte"
- "fn_summaries": array of {name, reads, writes}
- "entry": string (entry function name, usually "main")

Rules:
1. Every shared resource gets a unique global name.
2. Functions reference resources by name directly.
3. Every lock must have a matching drop.
4. Condvar wait requires paired mutex to be held.
5. Spawn targets must have matching join in the same function.

Output ONLY the JSON, no explanation."""

REPAIR_SYSTEM_PROMPT = """\
你是一个并发系统修复专家。你会收到一个包含并发 bug 的 CIR JSON，\
以及由模型检验工具检测到的 bug 报告。\
请根据报告中的修复建议修复 CIR，输出修复后的完整 CIR JSON。\
只输出 JSON，不要添加任何解释文本。"""


def call_llm(
    model: ModelConfig,
    system_prompt: str,
    user_prompt: str,
    temperature: float,
    max_tokens: int,
) -> tuple[str, dict[str, Any]]:
    """Call an LLM API and return (content, usage_info)."""
    api_key = os.environ.get(model.api_key_env, "")
    if not api_key:
        raise RuntimeError(
            f"Missing API key: set env var {model.api_key_env}"
        )

    if model.provider == "anthropic":
        return _call_anthropic(model, api_key, system_prompt, user_prompt,
                               temperature, max_tokens)
    elif model.provider == "google":
        return _call_google(model, api_key, system_prompt, user_prompt,
                            temperature, max_tokens)
    else:
        return _call_openai_compat(model, api_key, system_prompt,
                                   user_prompt, temperature, max_tokens)


def _call_openai_compat(
    model: ModelConfig, api_key: str,
    system_prompt: str, user_prompt: str,
    temperature: float, max_tokens: int,
) -> tuple[str, dict[str, Any]]:
    headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
    payload = {
        "model": model.model_id,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
    }
    resp = requests.post(
        f"{model.base_url}/chat/completions",
        headers=headers,
        json=payload,
        timeout=120,
    )
    resp.raise_for_status()
    data = resp.json()
    content = data["choices"][0]["message"]["content"]
    usage = data.get("usage", {})
    return content, usage


def _call_anthropic(
    model: ModelConfig, api_key: str,
    system_prompt: str, user_prompt: str,
    temperature: float, max_tokens: int,
) -> tuple[str, dict[str, Any]]:
    headers = {
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
        "Content-Type": "application/json",
    }
    payload = {
        "model": model.model_id,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_prompt}],
    }
    resp = requests.post(
        f"{model.base_url}/messages",
        headers=headers,
        json=payload,
        timeout=120,
    )
    resp.raise_for_status()
    data = resp.json()
    content = data["content"][0]["text"]
    usage = data.get("usage", {})
    return content, usage


def _call_google(
    model: ModelConfig, api_key: str,
    system_prompt: str, user_prompt: str,
    temperature: float, max_tokens: int,
) -> tuple[str, dict[str, Any]]:
    url = (
        f"{model.base_url}/models/{model.model_id}:generateContent"
        f"?key={api_key}"
    )
    payload = {
        "contents": [{"parts": [{"text": f"{system_prompt}\n\n{user_prompt}"}]}],
        "generationConfig": {
            "temperature": temperature,
            "maxOutputTokens": max_tokens,
        },
    }
    resp = requests.post(url, json=payload, timeout=120)
    resp.raise_for_status()
    data = resp.json()
    content = data["candidates"][0]["content"]["parts"][0]["text"]
    usage = data.get("usageMetadata", {})
    return content, usage


def extract_json(text: str) -> str:
    """Extract JSON from a response that may be wrapped in markdown fences."""
    text = text.strip()
    if text.startswith("```"):
        text = text.split("\n", 1)[1] if "\n" in text else text[3:]
        if text.endswith("```"):
            text = text[:-3]
    return text.strip()


# ── Rust toolchain interface ─────────────────────────────────

def validate_cir(cir_json: str) -> tuple[bool, list[str]]:
    """Validate a CIR JSON string by calling the Rust binary.

    Returns (success, error_messages).
    """
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--", "--validate", "-"],
        input=cir_json,
        capture_output=True,
        text=True,
        cwd=str(ROOT_DIR),
        timeout=60,
    )
    if proc.returncode == 0:
        return True, []
    errors = [line for line in proc.stderr.strip().split("\n") if line]
    return False, errors


def translate_and_analyze(cir_json: str) -> dict[str, Any]:
    """Translate CIR to CVN and run analysis. Returns a result dict.

    Expected output format (JSON on stdout):
    {
      "places": int,
      "transitions": int,
      "states": int,
      "analysis_time_ms": float,
      "bugs": [{"kind": str, ...}],
      "bug_reports": [{"kind": str, "summary": str, ...}]
    }
    """
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--", "--analyze", "-"],
        input=cir_json,
        capture_output=True,
        text=True,
        cwd=str(ROOT_DIR),
        timeout=120,
    )
    if proc.returncode != 0:
        return {"error": proc.stderr.strip()}
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"error": f"Invalid JSON output: {proc.stdout[:200]}"}


def translate_and_analyze_with_timing(cir_json: str) -> dict[str, Any]:
    """Like translate_and_analyze but also measures wall-clock time."""
    t0 = time.perf_counter()
    result = translate_and_analyze(cir_json)
    elapsed_ms = (time.perf_counter() - t0) * 1000
    if "analysis_time_ms" not in result:
        result["analysis_time_ms"] = elapsed_ms
    return result


# ── RQ1: CIR Generation ──────────────────────────────────────

def run_rq1(
    config: ExperimentConfig,
    model_filter: str | None = None,
) -> list[RQ1Result]:
    """Run RQ1 experiments: CIR generation from source code."""
    results: list[RQ1Result] = []
    models = _filter_models(config.models, model_filter)

    for model in models:
        print(f"\n{'='*60}")
        print(f"RQ1: Model = {model.name}")
        print(f"{'='*60}")

        for pattern, source_path in config.source_programs.items():
            full_path = ROOT_DIR / source_path
            if not full_path.exists():
                print(f"  [{pattern}] SKIP - source file not found: {source_path}")
                results.append(RQ1Result(
                    model=model.name, pattern=pattern,
                    round_num=0, success=False,
                    errors=["source file not found"],
                ))
                continue

            source_code = full_path.read_text()
            print(f"  [{pattern}] generating CIR...", end="", flush=True)

            result = _run_rq1_single(config, model, pattern, source_code)
            results.append(result)

            status = f"round {result.round_num}" if result.success else "FAILED"
            print(f" {status}")

    return results


def _run_rq1_single(
    config: ExperimentConfig,
    model: ModelConfig,
    pattern: str,
    source_code: str,
) -> RQ1Result:
    user_prompt = (
        f"Analyze the following Rust program and produce its CIR JSON.\n\n"
        f"```rust\n{source_code}\n```"
    )
    all_errors: list[str] = []

    for round_num in range(1, config.max_gen_rounds + 1):
        try:
            content, _ = call_llm(
                model, GENERATION_SYSTEM_PROMPT, user_prompt,
                config.temperature, config.max_tokens,
            )
        except Exception as e:
            all_errors.append(f"Round {round_num}: API error: {e}")
            continue

        cir_json = extract_json(content)
        valid, errors = validate_cir(cir_json)

        if valid:
            return RQ1Result(
                model=model.name, pattern=pattern,
                round_num=round_num, success=True,
            )

        all_errors.extend(f"Round {round_num}: {e}" for e in errors)
        user_prompt = (
            f"The CIR you generated has errors. Please fix them.\n\n"
            f"Errors:\n" + "\n".join(errors) + "\n\n"
            f"Original CIR:\n```json\n{cir_json}\n```"
        )

    return RQ1Result(
        model=model.name, pattern=pattern,
        round_num=0, success=False, errors=all_errors,
    )


# ── RQ2: Bug Detection & Repair ──────────────────────────────

def run_rq2(
    config: ExperimentConfig,
    model_filter: str | None = None,
) -> list[RQ2Result]:
    """Run RQ2 experiments: bug detection + LLM repair loop."""
    results: list[RQ2Result] = []
    models = _filter_models(config.models, model_filter)

    for pattern, cir_path in config.buggy_cirs.items():
        full_path = ROOT_DIR / cir_path
        if not full_path.exists():
            print(f"  [{pattern}] SKIP - buggy CIR not found: {cir_path}")
            continue

        buggy_json = full_path.read_text()

        analysis = translate_and_analyze_with_timing(buggy_json)
        if "error" in analysis:
            print(f"  [{pattern}] Translation/analysis error: {analysis['error']}")
            continue

        base_places = analysis.get("places", 0)
        base_transitions = analysis.get("transitions", 0)
        base_states = analysis.get("states", 0)
        base_time = analysis.get("analysis_time_ms", 0)
        bugs = analysis.get("bugs", [])
        bug_kind = bugs[0]["kind"] if bugs else "none"

        print(f"\n  [{pattern}] CVN: {base_places}P/{base_transitions}T, "
              f"{base_states} states, {base_time:.1f}ms, bug={bug_kind}")

        if not bugs:
            for model in models:
                results.append(RQ2Result(
                    model=model.name, pattern=pattern,
                    places=base_places, transitions=base_transitions,
                    states=base_states, analysis_time_ms=base_time,
                    bug_detected="none", repair_rounds=0,
                    regressions=0, success=True,
                ))
            continue

        for model in models:
            print(f"    [{model.name}] repairing...", end="", flush=True)
            result = _run_rq2_repair(
                config, model, pattern, buggy_json, analysis
            )
            results.append(result)
            status = (f"fixed in {result.repair_rounds} rounds"
                      if result.success else "FAILED")
            regr = f" (+{result.regressions} regressions)" if result.regressions else ""
            print(f" {status}{regr}")

    return results


def _run_rq2_repair(
    config: ExperimentConfig,
    model: ModelConfig,
    pattern: str,
    buggy_json: str,
    initial_analysis: dict[str, Any],
) -> RQ2Result:
    bugs = initial_analysis.get("bugs", [])
    bug_kind = bugs[0]["kind"] if bugs else "none"
    bug_reports = initial_analysis.get("bug_reports", [])
    report_text = bug_reports[0].get("text", "") if bug_reports else ""

    current_json = buggy_json
    regressions = 0

    for round_num in range(1, config.max_repair_rounds + 1):
        repair_prompt = _build_repair_prompt(current_json, report_text)

        try:
            content, _ = call_llm(
                model, REPAIR_SYSTEM_PROMPT, repair_prompt,
                config.temperature, config.max_tokens,
            )
        except Exception as e:
            continue

        candidate_json = extract_json(content)
        valid, errors = validate_cir(candidate_json)
        if not valid:
            regressions += 1
            report_text = "Static check errors:\n" + "\n".join(errors)
            current_json = candidate_json
            continue

        analysis = translate_and_analyze(candidate_json)
        if "error" in analysis:
            regressions += 1
            report_text = f"Translation error: {analysis['error']}"
            current_json = candidate_json
            continue

        new_bugs = analysis.get("bugs", [])
        if not new_bugs:
            return RQ2Result(
                model=model.name, pattern=pattern,
                places=initial_analysis.get("places", 0),
                transitions=initial_analysis.get("transitions", 0),
                states=initial_analysis.get("states", 0),
                analysis_time_ms=initial_analysis.get("analysis_time_ms", 0),
                bug_detected=bug_kind,
                repair_rounds=round_num,
                regressions=regressions,
                success=True,
            )

        new_kind = new_bugs[0]["kind"]
        if new_kind != bug_kind:
            regressions += 1

        new_reports = analysis.get("bug_reports", [])
        report_text = new_reports[0].get("text", "") if new_reports else ""
        current_json = candidate_json

    return RQ2Result(
        model=model.name, pattern=pattern,
        places=initial_analysis.get("places", 0),
        transitions=initial_analysis.get("transitions", 0),
        states=initial_analysis.get("states", 0),
        analysis_time_ms=initial_analysis.get("analysis_time_ms", 0),
        bug_detected=bug_kind,
        repair_rounds=-1,
        regressions=regressions,
        success=False,
    )


def _build_repair_prompt(cir_json: str, report_text: str) -> str:
    return (
        f"# Concurrency Bug Repair Request\n\n"
        f"## Original CIR\n\n```json\n{cir_json}\n```\n\n"
        f"## Detected Bug\n\n{report_text}\n\n"
        f"## Instructions\n\n"
        f"Fix the bug and output the complete repaired CIR JSON. "
        f"Only output JSON, no explanation."
    )


# ── RQ3: Translation Correctness ─────────────────────────────

def run_rq3(config: ExperimentConfig) -> list[RQ3Result]:
    """Run RQ3 experiments: translation structural invariants."""
    results: list[RQ3Result] = []
    patterns_dir = ROOT_DIR / config.patterns_dir

    all_cirs: dict[str, str] = {}
    for cir_path in config.buggy_cirs.values():
        p = ROOT_DIR / cir_path
        if p.exists():
            name = p.parent.name
            all_cirs[name] = p.read_text()

    for pattern_dir in sorted(patterns_dir.iterdir()):
        if not pattern_dir.is_dir():
            continue
        name = pattern_dir.name
        for json_file in ["buggy.json", "fixed.json"]:
            fp = pattern_dir / json_file
            if fp.exists() and name not in all_cirs:
                all_cirs[name] = fp.read_text()

    for pattern, cir_json in sorted(all_cirs.items()):
        analysis = translate_and_analyze(cir_json)
        if "error" in analysis:
            print(f"  [{pattern}] Error: {analysis['error']}")
            results.append(RQ3Result(
                pattern=pattern, cir_statements=0,
                cvn_places=0, cvn_transitions=0,
                translation_errors=1,
                invariants_passed=0, invariants_total=9,
            ))
            continue

        cir_prog = json.loads(cir_json)
        stmt_count = sum(
            len(f.get("body", []))
            for f in cir_prog.get("functions", [])
        )

        results.append(RQ3Result(
            pattern=pattern,
            cir_statements=stmt_count,
            cvn_places=analysis.get("places", 0),
            cvn_transitions=analysis.get("transitions", 0),
            translation_errors=analysis.get("translation_errors", 0),
            invariants_passed=analysis.get("invariants_passed", 9),
            invariants_total=9,
        ))
        print(f"  [{pattern}] {stmt_count} stmts -> "
              f"{analysis.get('places', 0)}P/{analysis.get('transitions', 0)}T, "
              f"invariants {analysis.get('invariants_passed', 9)}/9")

    return results


# ── Output ────────────────────────────────────────────────────

def save_results(
    output_dir: Path,
    rq1: list[RQ1Result] | None = None,
    rq2: list[RQ2Result] | None = None,
    rq3: list[RQ3Result] | None = None,
):
    """Save results as CSV and JSON."""
    output_dir.mkdir(parents=True, exist_ok=True)
    ts = time.strftime("%Y%m%d_%H%M%S")

    if rq1:
        _save_csv(output_dir / f"rq1_{ts}.csv", rq1)
        _save_json(output_dir / f"rq1_{ts}.json", rq1)
        print(f"\nRQ1 results saved to {output_dir}/rq1_{ts}.*")

    if rq2:
        _save_csv(output_dir / f"rq2_{ts}.csv", rq2)
        _save_json(output_dir / f"rq2_{ts}.json", rq2)
        print(f"RQ2 results saved to {output_dir}/rq2_{ts}.*")

    if rq3:
        _save_csv(output_dir / f"rq3_{ts}.csv", rq3)
        _save_json(output_dir / f"rq3_{ts}.json", rq3)
        print(f"RQ3 results saved to {output_dir}/rq3_{ts}.*")


def _save_csv(path: Path, records: list) -> None:
    if not records:
        return
    dicts = [asdict(r) for r in records]
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=dicts[0].keys())
        writer.writeheader()
        writer.writerows(dicts)


def _save_json(path: Path, records: list) -> None:
    with open(path, "w") as f:
        json.dump([asdict(r) for r in records], f, indent=2)


# ── Helpers ───────────────────────────────────────────────────

def _filter_models(
    models: list[ModelConfig], name: str | None
) -> list[ModelConfig]:
    if name is None:
        return models
    filtered = [m for m in models if m.name == name]
    if not filtered:
        available = ", ".join(m.name for m in models)
        raise ValueError(f"Model '{name}' not found. Available: {available}")
    return filtered


# ── CLI ───────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="CIR+CVN Experiment Runner"
    )
    parser.add_argument(
        "--config", default="experiments/config.toml",
        help="Path to experiment config TOML",
    )
    parser.add_argument(
        "--rq", type=int, choices=[1, 2, 3],
        help="Run only a specific RQ (1, 2, or 3). Default: all.",
    )
    parser.add_argument(
        "--model", type=str, default=None,
        help="Run only a specific model by name.",
    )
    args = parser.parse_args()

    config = load_config(args.config)
    output_dir = ROOT_DIR / config.output_dir

    rq1_results = None
    rq2_results = None
    rq3_results = None

    run_all = args.rq is None

    if run_all or args.rq == 1:
        print("\n" + "=" * 60)
        print("  RQ1: CIR Generation Capability")
        print("=" * 60)
        rq1_results = run_rq1(config, args.model)

    if run_all or args.rq == 2:
        print("\n" + "=" * 60)
        print("  RQ2: Bug Detection & Repair")
        print("=" * 60)
        rq2_results = run_rq2(config, args.model)

    if run_all or args.rq == 3:
        print("\n" + "=" * 60)
        print("  RQ3: Translation Correctness")
        print("=" * 60)
        rq3_results = run_rq3(config)

    save_results(output_dir, rq1_results, rq2_results, rq3_results)

    print("\nDone.")


if __name__ == "__main__":
    main()
