"""Local (slice-based) CIR repair: regenerate only the implicated functions.

Slice policy
------------
The verifier's bug reports name the functions involved in each defect
(deadlock participants, dead-transition owners, signal-loss waiter/notifier).
The slice is that function set. The LLM receives:

* the global declarations verbatim (resources, protection, goals),
* the full bodies of the slice functions, and
* a one-line synchronization summary of every other function (lock/wait/
  notify/send/spawn order) so cross-thread lock ordering stays visible,

and must answer with replacement bodies for slice functions only. Python
splices the replacements into the original CIR, so every non-slice byte is
guaranteed unchanged — this structurally rules out the "silent semantic
drift" observed in whole-CIR repair (e.g. a Var quietly becoming an Atomic).

Expansion and fallback
----------------------
If a repair round produces bugs implicating functions outside the current
slice, the slice is expanded and local repair continues. When the slice
cannot be determined (no implicated functions) or ``max_slice_rounds`` is
exhausted, the workflow falls back to whole-CIR repair with full feedback.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from typing import Any

from .json_utils import extract_json
from .llm import LlmClient
from .models import RustCliResult, normalize_token_usage
from .prompts import repair_system_prompt, verification_feedback
from .repair import RepairWorkflow
from .rust_cli import RustCli


@dataclass
class LocalRepairRound:
    round: int
    phase: str  # "slice" or "full"
    slice_functions: list[str] = field(default_factory=list)
    candidate_cir_json: str | None = None
    parse_error: str | None = None
    verification: RustCliResult | None = None
    llm_error: str | None = None
    accepted: bool = False
    rejection_reason: str | None = None
    duration_ms: float = 0.0
    input_tokens: int = 0
    output_tokens: int = 0


@dataclass
class LocalRepairResult:
    fixed_cir_json: str | None
    rounds: list[LocalRepairRound] = field(default_factory=list)
    initial_verification: RustCliResult | None = None
    initial_slice: list[str] = field(default_factory=list)
    final_slice: list[str] = field(default_factory=list)
    total_functions: int = 0
    slice_expanded: bool = False
    fell_back: bool = False
    fallback_reason: str | None = None
    error: str | None = None

    @property
    def success(self) -> bool:
        return self.fixed_cir_json is not None and self.error is None

    @property
    def total_input_tokens(self) -> int:
        return sum(r.input_tokens for r in self.rounds)

    @property
    def total_output_tokens(self) -> int:
        return sum(r.output_tokens for r in self.rounds)

    @property
    def total_tokens(self) -> int:
        return self.total_input_tokens + self.total_output_tokens

    @property
    def repair_rounds(self) -> int:
        return len(self.rounds) if self.success else -1


def implicated_functions(payload: dict[str, Any] | None) -> list[str]:
    """Functions named by any bug report (slice seed)."""

    if not payload:
        return []
    names: list[str] = []

    def add(name: str | None) -> None:
        if name and name not in names:
            names.append(name)

    for bug in payload.get("bugs") or []:
        for fn in bug.get("involved_functions") or []:
            add(fn)
        for entry in bug.get("cir_slice") or []:
            add(entry.get("function"))
        kind = bug.get("kind")
        if isinstance(kind, dict):
            for detail in kind.values():
                if isinstance(detail, dict):
                    for participant in detail.get("participants") or []:
                        if isinstance(participant, dict):
                            add(participant.get("function"))
    return names


def _describe_op(stmt: dict[str, Any]) -> str | None:
    """Sync-relevant one-token description of a statement, or None."""

    op = stmt.get("op")
    if isinstance(op, list) and op:
        head = op[0]
        if head == "res_op" and len(op) >= 3:
            return f"{op[2]}({op[1]})"
        if head in ("spawn", "join", "call") and len(op) >= 2:
            return f"{head}({op[1]})"
    transfer = stmt.get("transfer")
    if isinstance(transfer, list) and transfer and transfer[0] == "branch":
        return f"branch[{transfer[1]}]"
    return None


def function_sync_summary(fn: dict[str, Any]) -> str:
    steps = [d for stmt in fn.get("body", []) if (d := _describe_op(stmt))]
    ops = " -> ".join(steps) if steps else "(no sync operations)"
    return f"- {fn.get('name')} ({fn.get('kind', 'normal')}): {ops}"


def build_slice_prompt(
    program: dict[str, Any],
    slice_names: list[str],
    feedback: str,
) -> str:
    globals_view = {
        key: program.get(key)
        for key in ("program", "resources", "protection", "fn_summaries", "entry", "goals")
    }
    slice_fns = [fn for fn in program.get("functions", []) if fn.get("name") in slice_names]
    other_fns = [fn for fn in program.get("functions", []) if fn.get("name") not in slice_names]

    sections = [
        "# Local CIR Repair Request",
        "Verification found a concurrency defect. Repair it by rewriting ONLY "
        "the functions listed under 'Functions you may modify'. Every other "
        "part of the program (declarations and all other functions) is FROZEN "
        "and will be kept verbatim, so your fix must work against the frozen "
        "synchronization order shown in the summaries.",
        "## Verification Feedback\n\n" + feedback,
        "## Global declarations (frozen)\n\n```json\n"
        + json.dumps(globals_view, ensure_ascii=False, indent=2)
        + "\n```",
        "## Functions you may modify\n\n```json\n"
        + json.dumps(slice_fns, ensure_ascii=False, indent=2)
        + "\n```",
    ]
    if other_fns:
        sections.append(
            "## Other functions (frozen, synchronization summary)\n\n"
            + "\n".join(function_sync_summary(fn) for fn in other_fns)
        )
    sections.append(
        "## Output contract (overrides any earlier instruction)\n\n"
        "Answer with ONE JSON object of the form "
        '{"functions": [{"name": ..., "kind": ..., "body": [...]}, ...]} '
        "containing complete replacement definitions for the functions you "
        "changed (a subset of 'Functions you may modify'). Do not output the "
        "full CIR, do not include frozen functions, no explanatory text."
    )
    return "\n\n".join(sections)


def splice_functions(
    program: dict[str, Any],
    replacements: list[dict[str, Any]],
    allowed: list[str],
) -> tuple[dict[str, Any], list[str], list[str]]:
    """Replace slice function bodies; returns (new_program, applied, rejected)."""

    by_name = {
        fn.get("name"): fn
        for fn in replacements
        if isinstance(fn, dict) and fn.get("name")
    }
    applied: list[str] = []
    rejected = [name for name in by_name if name not in allowed]

    new_program = json.loads(json.dumps(program))
    for index, fn in enumerate(new_program.get("functions", [])):
        name = fn.get("name")
        if name in by_name and name in allowed:
            replacement = by_name[name]
            new_program["functions"][index] = {
                "name": name,
                "kind": replacement.get("kind", fn.get("kind", "normal")),
                "body": replacement.get("body", fn.get("body")),
            }
            applied.append(name)
    return new_program, applied, rejected


class LocalRepairWorkflow:
    def __init__(
        self,
        client: LlmClient,
        rust_cli: RustCli,
        *,
        max_slice_rounds: int = 3,
        max_full_rounds: int = 2,
        temperature: float = 0.0,
        max_tokens: int = 8192,
    ) -> None:
        self.client = client
        self.rust_cli = rust_cli
        self.max_slice_rounds = max(0, max_slice_rounds)
        self.max_full_rounds = max(0, max_full_rounds)
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.system_prompt = repair_system_prompt()

    def run(self, cir_json: str) -> LocalRepairResult:
        initial = self.rust_cli.analyze(cir_json)
        program = json.loads(cir_json)
        total_functions = len(program.get("functions", []))

        if initial.status == "verified_safe":
            return LocalRepairResult(
                fixed_cir_json=json.dumps(program, ensure_ascii=False, indent=2),
                initial_verification=initial,
                total_functions=total_functions,
            )

        slice_names = implicated_functions(initial.payload)
        result = LocalRepairResult(
            fixed_cir_json=None,
            initial_verification=initial,
            initial_slice=list(slice_names),
            total_functions=total_functions,
        )

        feedback = verification_feedback(initial.payload, initial.error or initial.stderr)

        if not slice_names:
            result.fallback_reason = "no implicated functions in bug reports"
        else:
            for round_number in range(1, self.max_slice_rounds + 1):
                round_record = LocalRepairRound(
                    round=round_number,
                    phase="slice",
                    slice_functions=list(slice_names),
                )
                started = time.perf_counter()
                try:
                    content, usage = self.client.chat(
                        self.system_prompt,
                        build_slice_prompt(program, slice_names, feedback),
                        temperature=self.temperature,
                        max_tokens=self.max_tokens,
                    )
                except Exception as error:
                    round_record.llm_error = str(error)
                    round_record.duration_ms = _elapsed(started)
                    result.rounds.append(round_record)
                    continue

                tokens = normalize_token_usage(usage)
                round_record.input_tokens, round_record.output_tokens = tokens

                try:
                    parsed = json.loads(extract_json(content))
                    replacements = parsed.get("functions") if isinstance(parsed, dict) else None
                    if not isinstance(replacements, list) or not replacements:
                        raise ValueError('expected {"functions": [...]} with >= 1 entry')
                except (json.JSONDecodeError, ValueError) as error:
                    round_record.parse_error = str(error)
                    round_record.rejection_reason = str(error)
                    round_record.duration_ms = _elapsed(started)
                    result.rounds.append(round_record)
                    feedback += f"\n\nYour previous answer was rejected: {error}"
                    continue

                candidate, applied, rejected_names = splice_functions(
                    program, replacements, slice_names
                )
                if rejected_names:
                    # Frozen-function edits are dropped, not merged.
                    feedback += (
                        "\n\nNote: your previous answer tried to modify frozen "
                        f"functions {rejected_names}; those edits were discarded."
                    )
                if not applied:
                    round_record.parse_error = "no replacement matched a slice function"
                    round_record.rejection_reason = round_record.parse_error
                    round_record.duration_ms = _elapsed(started)
                    result.rounds.append(round_record)
                    continue

                candidate_json = json.dumps(candidate, ensure_ascii=False, indent=2)
                verification = self.rust_cli.analyze(candidate_json)
                accepted = verification.status == "verified_safe"
                round_record.candidate_cir_json = candidate_json
                round_record.verification = verification
                round_record.accepted = accepted
                round_record.duration_ms = _elapsed(started)
                result.rounds.append(round_record)

                if accepted:
                    result.fixed_cir_json = candidate_json
                    result.final_slice = list(slice_names)
                    return result

                feedback = verification_feedback(
                    verification.payload, verification.error or verification.stderr
                )
                round_record.rejection_reason = feedback
                # Adopt the candidate: it may be partially closer to a fix.
                program = candidate

                new_names = implicated_functions(verification.payload)
                outside = [name for name in new_names if name not in slice_names]
                if outside:
                    slice_names = slice_names + outside
                    result.slice_expanded = True

            result.fallback_reason = (
                f"exhausted {self.max_slice_rounds} slice rounds"
            )

        result.final_slice = list(slice_names)
        result.fell_back = True
        full = RepairWorkflow(
            self.client,
            self.rust_cli,
            max_rounds=self.max_full_rounds,
            temperature=self.temperature,
            max_tokens=self.max_tokens,
            feedback_mode="full",
        ).run(json.dumps(program, ensure_ascii=False, indent=2))

        offset = len(result.rounds)
        for full_round in full.rounds:
            result.rounds.append(LocalRepairRound(
                round=offset + full_round.round,
                phase="full",
                candidate_cir_json=full_round.candidate_cir_json,
                parse_error=full_round.parse_error,
                verification=full_round.verification,
                llm_error=full_round.llm_error,
                accepted=full_round.accepted,
                rejection_reason=full_round.rejection_reason,
                duration_ms=full_round.duration_ms,
                input_tokens=full_round.input_tokens,
                output_tokens=full_round.output_tokens,
            ))
        if full.success:
            result.fixed_cir_json = full.fixed_cir_json
        else:
            result.error = full.error or "local repair and full fallback both failed"
        return result


def _elapsed(started: float) -> float:
    return (time.perf_counter() - started) * 1000
