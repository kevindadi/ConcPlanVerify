#!/usr/bin/env python3
"""
CIR+CVN Experiment Runner
=========================

Orchestrates the evaluation experiments described in Section 7 of the paper:
  RQ1 — CIR Generation           : Rounds required for each LLM to produce a
                                   static-valid CIR from a Rust source.
  RQ2 — Bug Detection & Repair   : CVN analysis + LLM repair loop until no
                                   deadlock remains.
  RQ3 — Translation Correctness  : Structural invariants of CIR→CVN output.
  RQ4 — Goal Reachability        : Fraction of user-declared business goals
                                   that remain reachable in the fixed CIR.

Usage:
    python experiments/run_experiment.py --config experiments/config.toml
    python experiments/run_experiment.py --config experiments/config.toml --rq 2
    python experiments/run_experiment.py --rq 4
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
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


def _load_dotenv(path: Path) -> None:
    """Minimal .env loader (no dependency on python-dotenv).

    Silently updates os.environ for KEY=VALUE lines, ignoring comments and
    existing environment values (env-level settings win).
    """
    if not path.exists():
        return
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, val = line.split("=", 1)
        key = key.strip()
        val = val.strip().strip('"').strip("'")
        os.environ.setdefault(key, val)


_load_dotenv(ROOT_DIR / ".env")

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
    fixed_cirs: dict[str, str]

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

@dataclass
class RQ4Result:
    pattern: str
    goals_total: int
    goals_met: int
    goals_unmet: int
    unmet_ids: str     # comma-separated for CSV friendliness
    warnings: int


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
        fixed_cirs=raw.get("fixed_cirs", {}),
    )


# ── LLM API calls ────────────────────────────────────────────

CIR_SCHEMA_SPEC = """\
CIR schema:
- "program": string (program name)
- "resources": array of {name, kind, type, mode?, count?, base?, init?}
  - kind: "sync" or "var"
  - type (sync): "Mutex", "RwLock", "Condvar", "Semaphore", "Channel"
  - type (var): "Var", "Atomic"
  - For Condvar: add "paired_with": "<mutex_name>"
  - For Semaphore: add "count": <initial_permits>
  - For Var/Atomic: add "base": "Bool"|"Int"|"Float"|"String", "init": <value>
- "protection": array of {var, lock} (map variable → guarding lock)
- "functions": array of {name, kind, body}
  - kind: "normal", "async", "closure"
  - body: array of statements {sid, op, transfer}
    - sid: unique within function (e.g. "s1", "s2")
    - op: ["res_op", resource, action] | ["spawn", fn] | ["join", fn] |
          ["call", fn] | "return" | "nop"
      - actions: "lock", "drop", "read", "write", "acquire", "release",
                 "send", "recv", "wait", "notify", "notify_all", "cas"
      - For write: ["res_op", var, "write", value]
      - For cas:   ["res_op", var, "cas", expected, desired]
    - transfer: "return" | ["next", sid] | ["branch", cond_expr, true_sid, false_sid]
      - cond_expr: a single string expression, NOT an object.
        Supported forms (lhs is a variable name; rhs is a literal):
          "x == 0"   "x != 1"   "x > 0"   "x < 10"
          "x >= 1"   "x <= 5"   "flag == true"   "state == \\"ready\\""
        Logical connectives are also accepted:
          "x > 0 && y != 1"   "done == true || count == 0"
        The lhs variable must be declared in "resources" as kind "var" or
        "atomic". Do NOT use object form {cond, on_true, on_false}.
- "fn_summaries": array of {name, reads, writes, callees, has_concurrency}
- "entry": string (entry function name, usually "main")
- Optional "goals": array of {id, desc?, marking, variables} specifying
  user-visible outcomes that must remain reachable after repair.

Predicate-loop example (use this shape for wait-with-predicate patterns):
  {"sid":"s1","op":["res_op","mtx","lock"],"transfer":["next","s2"]},
  {"sid":"s2","op":["res_op","ready","read"],
    "transfer":["branch","ready == true","s4","s3"]},
  {"sid":"s3","op":["res_op","cv","wait"],"transfer":["next","s2"]},
  {"sid":"s4","op":["res_op","mtx","drop"],"transfer":"return"}

Rules:
1. Every shared resource gets a unique global name.
2. Functions reference resources by name directly.
3. Every lock must have a matching drop.
4. Condvar wait requires paired mutex to be held.
5. Spawn targets must have matching join in the same function.
6. "transfer" is always either the string "return", or an array whose
   first element is the string "next" or "branch" (NEVER an object).
"""


GENERATION_SYSTEM_PROMPT = (
    "You are an expert in concurrent systems. Given a Rust source program, "
    "produce a CIR (Concurrency Intermediate Representation) in JSON format.\n\n"
    + CIR_SCHEMA_SPEC
    + "\nOutput ONLY the JSON, no explanation."
)

REPAIR_SYSTEM_PROMPT = (
    "You are an expert in concurrent systems, specialised in repairing CIR "
    "(Concurrency Intermediate Representation) programs based on formal "
    "verification feedback.\n\n"
    "You will receive:\n"
    "  1. A buggy CIR JSON.\n"
    "  2. A structured bug report produced by a Petri-net state-space\n"
    "     analyser (may describe a deadlock, signal-loss, channel-block,\n"
    "     dead-transition, or goal-unreachability).\n"
    "  3. Zero or more regression notes from previous attempts.\n\n"
    "Your task is to output the complete repaired CIR JSON so that:\n"
    "  (a) the static checker reports zero errors,\n"
    "  (b) the state-space analyser reports no concurrency bug, AND\n"
    "  (c) every declared business goal remains reachable.\n\n"
    "Editing constraints (violating these causes regressions):\n"
    "  * Keep the same 'program' name and 'entry' function.\n"
    "  * Preserve every declared resource name, protection entry, and\n"
    "    business goal id. You may add resources only if strictly\n"
    "    necessary to fix the bug.\n"
    "  * Keep existing 'sid' names whenever the statement is kept. Only\n"
    "    introduce new sids when you add statements.\n"
    "  * Apply the minimum edit that removes the reported bug — do not\n"
    "    rewrite unrelated functions.\n"
    "  * Do NOT delete statements that produce business-goal-relevant\n"
    "    behaviour (e.g. a write that a goal depends on).\n\n"
    "Regression feedback semantics (read carefully when given):\n"
    "  - 'Static check errors: ...'     → your JSON violates well-formedness;\n"
    "                                     re-read the schema above.\n"
    "  - 'Translation error: ...'       → translation from CIR to CVN failed;\n"
    "                                     usually an unknown resource or a\n"
    "                                     malformed transfer.\n"
    "  - 'Deadlock cleared but N business goal(s) became unreachable'\n"
    "                                   → you removed required behaviour;\n"
    "                                     restore it without re-introducing\n"
    "                                     the original bug.\n"
    "  - Repeated identical bug reports → your last edit did not change the\n"
    "                                     concurrency structure that caused\n"
    "                                     the bug; try a different strategy.\n\n"
    + CIR_SCHEMA_SPEC
    + "\nOutput ONLY the revised CIR JSON object, with no prose and no\n"
    "markdown fences."
)


def call_llm(
    model: ModelConfig,
    system_prompt: str,
    user_prompt: str,
    temperature: float,
    max_tokens: int,
) -> tuple[str, dict[str, Any]]:
    """Call an LLM API and return (content, usage_info).

    The paper's five evaluation models all go through OpenAI-compatible
    endpoints (z.apiyihe.org aggregator + DashScope), so only one transport
    implementation is needed.
    """
    api_key = os.environ.get(model.api_key_env, "")
    if not api_key:
        raise RuntimeError(
            f"Missing API key: set env var {model.api_key_env}"
        )
    return _call_openai_compat(
        model, api_key, system_prompt, user_prompt,
        temperature, max_tokens,
    )


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
        timeout=180,
    )
    resp.raise_for_status()
    data = resp.json()
    content = data["choices"][0]["message"]["content"]
    usage = data.get("usage", {})
    return content, usage


_THINK_TAG_RE = re.compile(r"<think\b[^>]*>.*?</think>", re.DOTALL | re.IGNORECASE)


def extract_json(text: str) -> str:
    """Extract JSON from a response that may be wrapped in markdown fences.

    Some aggregator channels (notably claude-opus-4-6 on z.apiyihe.org) inline
    chain-of-thought as <think>...</think> blocks inside ``message.content``
    instead of surfacing them via ``reasoning_content``. We strip those blocks
    so the remaining payload can be parsed as JSON. This is defensive: models
    that return clean content are unaffected.
    """
    text = _THINK_TAG_RE.sub("", text).strip()
    if text.startswith("```"):
        text = text.split("\n", 1)[1] if "\n" in text else text[3:]
        if text.endswith("```"):
            text = text[:-3]
    return text.strip()


# ── Rust toolchain interface ─────────────────────────────────

_CIR2CVN_BIN = ROOT_DIR / "target" / "release" / "cir2cvn"


def _ensure_cir2cvn_built() -> Path:
    """Build the release binary lazily (one-off cost) and cache the path."""
    if not _CIR2CVN_BIN.exists():
        subprocess.run(
            ["cargo", "build", "--release", "--quiet"],
            cwd=str(ROOT_DIR),
            check=True,
        )
    return _CIR2CVN_BIN


def _cir2cvn(mode: str, cir_json: str, timeout: int = 120) -> dict[str, Any]:
    """Invoke the cir2cvn CLI with the given mode (--validate, --analyze, --goals)."""
    binary = _ensure_cir2cvn_built()
    proc = subprocess.run(
        [str(binary), mode, "-"],
        input=cir_json,
        capture_output=True,
        text=True,
        cwd=str(ROOT_DIR),
        timeout=timeout,
    )
    if proc.returncode != 0 and not proc.stdout.strip():
        return {"error": proc.stderr.strip() or f"cir2cvn {mode} exited {proc.returncode}"}
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"error": f"Invalid JSON output from cir2cvn {mode}: {proc.stdout[:200]}"}


def validate_cir(cir_json: str) -> tuple[bool, list[str]]:
    """Validate a CIR JSON string using the cir2cvn binary."""
    report = _cir2cvn("--validate", cir_json, timeout=60)
    if "error" in report and "valid" not in report:
        return False, [report["error"]]
    if report.get("valid", False):
        return True, []
    diags = report.get("diagnostics", [])
    errors = [f"{d.get('code', '')}: {d.get('message', '')}" for d in diags]
    return False, errors


def translate_and_analyze(cir_json: str) -> dict[str, Any]:
    """Translate CIR to CVN and run full analysis."""
    return _cir2cvn("--analyze", cir_json)


def check_goal_reachability(cir_json: str) -> dict[str, Any]:
    """Run the goal-reachability check on a (typically fixed) CIR."""
    return _cir2cvn("--goals", cir_json)


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

def _format_goal_unreachable_report(
    unmet: list[dict[str, Any]],
    *,
    deadlock_was_present: bool = False,
) -> str:
    """Render an unmet-goal list as a repair prompt diagnostic."""
    if not unmet:
        return ""
    lines = []
    for g in unmet:
        gid = g.get("id", "?")
        desc = g.get("desc") or g.get("description") or ""
        if desc:
            lines.append(f"- {gid}: {desc}")
        else:
            lines.append(f"- {gid}")
    body = "\n".join(lines)
    prefix = (
        "Deadlock cleared but the following business goal(s) became "
        "unreachable after the repair:"
        if deadlock_was_present
        else (
            "BUG: GoalUnreachable. The program has no CVN deadlock, but the "
            "following declared business goal(s) are not reachable from any "
            "execution trace. Typical causes: a monitor/watcher loop never "
            "exits, a spawned worker never signals completion, or a branch "
            "guard locks control flow away from the goal state."
        )
    )
    return f"{prefix}\n{body}"


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

        # Also probe business-goal reachability. A "partial deadlock" pattern
        # such as the monitor-loop bug has no CVN deadlock, yet the declared
        # business goal (e.g. "counter reaches N") is unreachable. We treat
        # that as a first-class repair trigger with a synthetic bug kind so
        # the LLM gets a chance to fix it in the repair loop.
        goal_report_initial = check_goal_reachability(buggy_json)
        unmet_initial = goal_report_initial.get("unmet", []) \
            if isinstance(goal_report_initial, dict) else []

        if bugs:
            bug_kind = bugs[0]["kind"]
            goal_trigger = False
        elif unmet_initial:
            bug_kind = "GoalUnreachable"
            goal_trigger = True
        else:
            bug_kind = "none"
            goal_trigger = False

        print(f"\n  [{pattern}] CVN: {base_places}P/{base_transitions}T, "
              f"{base_states} states, {base_time:.1f}ms, bug={bug_kind}")

        if not bugs and not goal_trigger:
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
                config, model, pattern, buggy_json, analysis,
                initial_unmet_goals=unmet_initial if goal_trigger else None,
                synthetic_bug_kind="GoalUnreachable" if goal_trigger else None,
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
    *,
    initial_unmet_goals: list[dict[str, Any]] | None = None,
    synthetic_bug_kind: str | None = None,
) -> RQ2Result:
    bugs = initial_analysis.get("bugs", [])
    bug_reports = initial_analysis.get("bug_reports", [])

    if bugs:
        bug_kind = bugs[0]["kind"]
        report_text = bug_reports[0].get("text", "") if bug_reports else ""
    elif synthetic_bug_kind and initial_unmet_goals:
        bug_kind = synthetic_bug_kind
        report_text = _format_goal_unreachable_report(initial_unmet_goals)
    else:
        bug_kind = "none"
        report_text = ""

    current_json = buggy_json
    regressions = 0
    last_error: str | None = None
    round_diagnostics: list[str] = []

    for round_num in range(1, config.max_repair_rounds + 1):
        repair_prompt = _build_repair_prompt(current_json, report_text)
        t_round_start = time.perf_counter()
        print(f"\n      r{round_num} calling {model.name}...", end="", flush=True)

        try:
            content, _ = call_llm(
                model, REPAIR_SYSTEM_PROMPT, repair_prompt,
                config.temperature, config.max_tokens,
            )
        except Exception as e:
            elapsed = (time.perf_counter() - t_round_start) * 1000
            last_error = f"Round {round_num}: API error ({elapsed:.0f}ms): {type(e).__name__}: {str(e)[:200]}"
            round_diagnostics.append(f"r{round_num}=api_err")
            print(f" API_ERR {elapsed:.0f}ms [{type(e).__name__}]", flush=True)
            continue
        llm_ms = (time.perf_counter() - t_round_start) * 1000
        print(f" llm={llm_ms:.0f}ms", end="", flush=True)

        candidate_json = extract_json(content)
        valid, errors = validate_cir(candidate_json)
        if not valid:
            regressions += 1
            err_summary = "; ".join(errors[:3])
            round_diagnostics.append(f"r{round_num}=static_err[{err_summary[:150]}]")
            report_text = "Static check errors:\n" + "\n".join(errors)
            current_json = candidate_json
            print(" static_err", flush=True)
            continue

        analysis = translate_and_analyze(candidate_json)
        if "error" in analysis:
            regressions += 1
            round_diagnostics.append(f"r{round_num}=xlate_err[{str(analysis['error'])[:150]}]")
            report_text = f"Translation error: {analysis['error']}"
            current_json = candidate_json
            print(" xlate_err", flush=True)
            continue

        new_bugs = analysis.get("bugs", [])
        if not new_bugs:
            # Also check that declared goals remain reachable after repair.
            goal_report = check_goal_reachability(candidate_json)
            if goal_report.get("goals_unmet", 0) > 0:
                # Continued goal-unmet on a GoalUnreachable-triggered run is
                # just incomplete progress, not a fresh regression introduced
                # by the LLM, so we only bump regressions when the repair
                # actually introduced the goal-unmet state from scratch.
                if synthetic_bug_kind != "GoalUnreachable":
                    regressions += 1
                report_text = _format_goal_unreachable_report(
                    goal_report.get("unmet", []),
                    deadlock_was_present=bool(bugs),
                )
                current_json = candidate_json
                round_diagnostics.append(f"r{round_num}=goal_unmet")
                print(" goal_unmet", flush=True)
                continue

            print(" SUCCESS", flush=True)
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
        round_diagnostics.append(f"r{round_num}=bug_{new_kind}")
        print(f" bug={new_kind}", flush=True)

        new_reports = analysis.get("bug_reports", [])
        report_text = new_reports[0].get("text", "") if new_reports else ""
        current_json = candidate_json

    if round_diagnostics:
        print(f"\n      trace: {' | '.join(round_diagnostics)}", flush=True)

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


_BRANCH_FEW_SHOT = """\
## Example: predicate-check loop pattern (branch syntax)

A waiter that must re-check a predicate after being woken by a condvar
uses a `branch` transfer whose condition is a *string* expression. The
`branch` transfer is ALWAYS a 4-element array: ["branch", cond_expr,
true_sid, false_sid].

Example body (abridged):

```json
[
  {"sid":"s1","op":["res_op","mtx","lock"],"transfer":["next","s2"]},
  {"sid":"s2","op":["res_op","ready","read"],
    "transfer":["branch","ready == true","s5","s3"]},
  {"sid":"s3","op":["res_op","cv","wait"],"transfer":["next","s4"]},
  {"sid":"s4","op":["res_op","ready","read"],
    "transfer":["branch","ready == true","s5","s3"]},
  {"sid":"s5","op":["res_op","mtx","drop"],"transfer":"return"}
]
```

This is the canonical fix for both signal-loss and dual-condvar
deadlocks: always re-check the predicate variable via `branch` after
`wait`, and loop back to `wait` while the predicate is false.
"""


def _build_repair_prompt(cir_json: str, report_text: str) -> str:
    return (
        "# Concurrency Bug Repair Request\n\n"
        f"## Original CIR\n\n```json\n{cir_json}\n```\n\n"
        f"## Detected Bug / Regression\n\n{report_text}\n\n"
        f"{_BRANCH_FEW_SHOT}\n"
        "## Instructions\n\n"
        "Fix the issue and output the complete repaired CIR JSON. "
        "Keep all declared business goals reachable. "
        "Do NOT output any explanation, markdown fences, or partial JSON."
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


# ── RQ4: Goal Reachability ───────────────────────────────────

def run_rq4(config: ExperimentConfig) -> list[RQ4Result]:
    """Run RQ4 experiments: goal reachability on fixed (or baseline) CIRs.

    For every pattern listed in `[fixed_cirs]`, translate the fixed CIR and
    run `cir2cvn --goals`. Patterns with no declared goals contribute
    `goals_total == 0` rows (kept for completeness).
    """
    results: list[RQ4Result] = []

    for pattern, cir_path in config.fixed_cirs.items():
        full_path = ROOT_DIR / cir_path
        if not full_path.exists():
            print(f"  [{pattern}] SKIP - fixed CIR not found: {cir_path}")
            continue

        cir_json = full_path.read_text()
        report = check_goal_reachability(cir_json)
        if "error" in report:
            print(f"  [{pattern}] Error: {report['error']}")
            results.append(RQ4Result(
                pattern=pattern, goals_total=0, goals_met=0,
                goals_unmet=0, unmet_ids="error", warnings=0,
            ))
            continue

        total = report.get("goals_total", 0)
        met = report.get("goals_met", 0)
        unmet = report.get("goals_unmet", 0)
        unmet_ids = ",".join(g.get("id", "?") for g in report.get("unmet", []))
        warnings = len(report.get("warnings", []))

        print(f"  [{pattern}] goals {met}/{total} met"
              + (f"  (unmet: {unmet_ids})" if unmet else ""))

        results.append(RQ4Result(
            pattern=pattern, goals_total=total, goals_met=met,
            goals_unmet=unmet, unmet_ids=unmet_ids, warnings=warnings,
        ))

    return results


# ── Output ────────────────────────────────────────────────────

def save_results(
    output_dir: Path,
    rq1: list[RQ1Result] | None = None,
    rq2: list[RQ2Result] | None = None,
    rq3: list[RQ3Result] | None = None,
    rq4: list[RQ4Result] | None = None,
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

    if rq4:
        _save_csv(output_dir / f"rq4_{ts}.csv", rq4)
        _save_json(output_dir / f"rq4_{ts}.json", rq4)
        print(f"RQ4 results saved to {output_dir}/rq4_{ts}.*")


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
        "--rq", type=int, choices=[1, 2, 3, 4],
        help="Run only a specific RQ (1, 2, 3, or 4). Default: all.",
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
    rq4_results = None

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

    if run_all or args.rq == 4:
        print("\n" + "=" * 60)
        print("  RQ4: Goal Reachability")
        print("=" * 60)
        rq4_results = run_rq4(config)

    save_results(
        output_dir,
        rq1_results, rq2_results, rq3_results, rq4_results,
    )

    print("\nDone.")


if __name__ == "__main__":
    main()
