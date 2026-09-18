"""LLM-driven modularity planning for ConcIR generation.

Given natural-language requirements, the planner decides whether the program
should be generated as one ConcIR artifact or split into modular fragments.
Modularity is a *generation strategy* only: small programs generate directly;
large programs are split into source-file-sized fragments that a later
``merge`` step assembles into a single Program for the Rust translator.
"""

from __future__ import annotations

import json
from typing import Any

from .json_utils import extract_json
from .llm import LlmClient, LlmError

_PLAN_SYSTEM_PROMPT = """\
You are an expert in concurrent systems modeling. Given natural-language \
requirements for a concurrent program, decide whether it should be generated \
as ONE ConcIR artifact or split into MODULAR fragments, and if modular, \
produce the module breakdown.

Modularity is a generation strategy for large projects. Use modular when a \
single pass would be error-prone: typically many functions, several shared \
resources, or clearly separable subsystems (a producer, workers, a monitor, \
a server/transport layer, ...). For small programs — a few functions and \
resources — answer modular: false and generate directly.

Every module owns a coherent slice of the project:
- "main": the entry module; owns the program's entry function and the \
deployment wiring (spawn/join of workers, start/stop).
- Other modules: self-contained subsystems (workers, monitor, transport). \
They may reference shared resources and call/spawn functions defined in \
other modules (call targets resolve after merge).
- Shared resources (mutexes, channels, semaphores, condvars, shared \
variables) are declared once, by the module that most naturally owns them \
(often main), and referenced by the others. List them in "shared_resources".

The body of each function must contain control flow and synchronization \
operations. Pure-computation helpers may be declared as body-less functions \
("body": [], with an optional "effects" {reads, writes} hint); they are \
placeholders, not call-chain elements.

Output ONLY a JSON object, no commentary:

{
  "modular": true,
  "rationale": "one sentence",
  "modules": [
    {"name": "main", "entry": true, "responsibility": "...",
     "functions": ["main", "start"], "resources": ["mtx", "ch"]},
    {"name": "worker", "entry": false, "responsibility": "...",
     "functions": ["worker", "helper"], "resources": []}
  ],
  "shared_resources": ["mtx", "ch"]
}

When "modular" is false, omit "modules" and "shared_resources".
"""


class PlanError(RuntimeError):
    """The planner returned an unusable plan."""


def run_plan(
    client: LlmClient,
    requirements: str,
    *,
    temperature: float = 0.0,
    max_tokens: int = 4096,
) -> dict[str, Any]:
    """Ask the planner for a modularity decision; return the parsed plan dict."""
    try:
        text, _usage = client.chat(
            _PLAN_SYSTEM_PROMPT,
            requirements,
            temperature=temperature,
            max_tokens=max_tokens,
        )
    except Exception as error:
        raise PlanError(f"planner request failed: {error}") from error

    candidate = extract_json(text)
    try:
        plan = json.loads(candidate)
    except json.JSONDecodeError as error:
        raise PlanError(f"planner returned non-JSON: {error}") from error

    _validate_plan(plan)
    return plan


def _validate_plan(plan: dict[str, Any]) -> None:
    if not isinstance(plan, dict):
        raise PlanError("plan must be a JSON object")
    modular = plan.get("modular")
    if modular is True:
        modules = plan.get("modules")
        if not isinstance(modules, list) or not modules:
            raise PlanError("modular plan requires a non-empty 'modules' array")
        entry_owners = [m["name"] for m in modules if m.get("entry")]
        if len(entry_owners) != 1:
            raise PlanError(
                f"modular plan must name exactly one entry module, got {entry_owners}"
            )
        for m in modules:
            if not isinstance(m.get("name"), str) or not m["name"]:
                raise PlanError("each module requires a non-empty 'name'")
            functions = m.get("functions")
            if not isinstance(functions, list) or not functions:
                raise PlanError(f"module '{m.get('name')}' requires non-empty 'functions'")
    elif modular is False:
        pass
    else:
        raise PlanError("plan requires 'modular' to be true or false")


def render_plan(plan: dict[str, Any]) -> str:
    """Human-readable summary of a plan for the terminal."""
    if plan.get("modular") is False:
        return "Direct generation (modular: false)"
    lines = [f"Modular generation ({len(plan.get('modules', []))} fragments)"]
    lines.append(f"  shared resources: {', '.join(plan.get('shared_resources', []))}")
    for m in plan.get("modules", []):
        entry = " [entry]" if m.get("entry") else ""
        functions = ", ".join(m.get("functions", []))
        resources = ", ".join(m.get("resources", []))
        lines.append(
            f"  - {m['name']}{entry}: {m.get('responsibility', '')}"
            f"  functions({functions})  resources({resources})"
        )
    if plan.get("rationale"):
        lines.append(f"  rationale: {plan['rationale']}")
    return "\n".join(lines)
