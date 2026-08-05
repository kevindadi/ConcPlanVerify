"""Verified-CIR to Rust code generation (the last pipeline stage).

Workflow position: the user states a concurrency requirement in natural
language; the LLM first produces CIR; the CVN pipeline verifies it; only a
``verified_safe`` CIR reaches this stage, where the LLM writes the concrete
Rust program *from the verified plan* instead of directly from the prose.

The acceptance oracle here is ``cargo check`` on a scratch package: compiler
errors are fed back for up to ``max_rounds`` attempts. The result carries
code-size metrics (LOC, bytes, functions, thread spawns, sync-primitive
uses) so scaling experiments can relate CIR size -> CVN size -> code size.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .llm import LlmClient
from .models import normalize_token_usage

CARGO_TOML = """\
[package]
name = "codegen-check"
version = "0.0.0"
edition = "2021"

[workspace]

[[bin]]
name = "case"
path = "src/main.rs"
"""

_RUST_BLOCK_RE = re.compile(r"```(?:rust|rs)?\s*\n(.*?)```", re.DOTALL)


def codegen_system_prompt() -> str:
    return (
        "You are an expert Rust concurrency engineer. You receive a CIR "
        "(Concurrency Intermediate Representation) JSON plan that has ALREADY "
        "been formally verified as free of deadlocks and dead transitions, "
        "with all declared goals reachable. Your job is to implement that "
        "plan as a concrete Rust program, faithfully preserving its "
        "concurrency structure.\n\n"
        "Rules:\n"
        "- Produce one complete, standalone `main.rs` using only the Rust "
        "standard library (std::sync, std::thread, std::sync::mpsc).\n"
        "- Map CIR constructs literally: each non-main function becomes a "
        "closure run on its own thread; `spawn`/`join` map to "
        "`thread::spawn`/`JoinHandle::join`; Mutex -> std::sync::Mutex, "
        "Condvar -> std::sync::Condvar, Semaphore -> a small permit counter "
        "built from Mutex+Condvar, Channel -> std::sync::mpsc; `res_op` "
        "lock/drop pairs delimit guard scopes (use explicit `drop(guard)` "
        "when the CIR drops early or out of declaration order).\n"
        "- Preserve the CIR's acquisition ORDER exactly. Do not reorder, "
        "merge, or 'improve' the synchronization: the verified plan is the "
        "specification.\n"
        "- Branch statements become `if`/`else` on the same condition over "
        "the same shared variable; protected variables are accessed only "
        "under their protecting lock.\n"
        "- The program must terminate: main joins every spawned thread.\n\n"
        "Answer with exactly one fenced ```rust code block containing the "
        "full main.rs and nothing else."
    )


def codegen_user_prompt(cir_json: str, requirement: str | None) -> str:
    parts = []
    if requirement:
        parts.append(f"Original natural-language requirement:\n{requirement}\n")
    parts.append(f"Verified CIR plan:\n```json\n{cir_json}\n```")
    parts.append("Implement this plan as a complete Rust main.rs.")
    return "\n".join(parts)


def codegen_retry_prompt(cir_json: str, code: str, cargo_errors: str) -> str:
    return (
        "Your previous Rust implementation of the verified CIR plan failed "
        "`cargo check`. Fix every compiler error while keeping the "
        "concurrency structure of the plan unchanged.\n\n"
        f"Verified CIR plan:\n```json\n{cir_json}\n```\n\n"
        f"Previous code:\n```rust\n{code}\n```\n\n"
        f"cargo check errors:\n{cargo_errors}\n\n"
        "Answer with exactly one fenced ```rust code block."
    )


def extract_rust_code(content: str) -> str:
    matches = _RUST_BLOCK_RE.findall(content)
    if matches:
        # The last block is the final answer when the model narrates first.
        return matches[-1].strip() + "\n"
    return content.strip() + "\n"


def code_metrics(code: str) -> dict[str, Any]:
    lines = code.splitlines()
    loc = sum(
        1
        for line in lines
        if line.strip() and not line.strip().startswith("//")
    )
    return {
        "loc": loc,
        "total_lines": len(lines),
        "bytes": len(code.encode("utf-8")),
        "functions": len(re.findall(r"\bfn\s+\w+", code)),
        "thread_spawns": len(re.findall(r"thread::spawn|Builder::new\(\)", code)),
        "mutex_uses": len(re.findall(r"\bMutex::new\b", code)),
        "condvar_uses": len(re.findall(r"\bCondvar::new\b", code)),
        "channel_uses": len(re.findall(r"\bmpsc::channel\b|\bsync_channel\b", code)),
        "atomic_uses": len(re.findall(r"\bAtomic\w+::new\b", code)),
    }


def run_cargo_check(code: str, scratch: Path, timeout_s: float = 120.0) -> tuple[bool, str, float]:
    """Returns (ok, error_output, wall_s)."""

    src = scratch / "src"
    src.mkdir(parents=True, exist_ok=True)
    (scratch / "Cargo.toml").write_text(CARGO_TOML, encoding="utf-8")
    (src / "main.rs").write_text(code, encoding="utf-8")
    # Fingerprints can mask a re-check of identical-length edits; cheap to drop.
    shutil.rmtree(scratch / "target", ignore_errors=True)

    started = time.perf_counter()
    try:
        proc = subprocess.run(
            ["cargo", "check", "--quiet"],
            cwd=scratch,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_s,
            env=os.environ.copy(),
        )
        ok = proc.returncode == 0
        output = (proc.stdout or "") + (proc.stderr or "")
    except subprocess.TimeoutExpired:
        ok = False
        output = f"cargo check timed out after {timeout_s}s"
    wall_s = time.perf_counter() - started
    # Keep only error/warning lines to fit the retry prompt.
    interesting = [
        line
        for line in output.splitlines()
        if line.strip() and not line.startswith("    Checking")
    ]
    return ok, "\n".join(interesting[-60:]), wall_s


@dataclass
class CodegenRound:
    round: int
    code: str | None = None
    cargo_check_ok: bool | None = None
    cargo_errors: str | None = None
    cargo_wall_s: float | None = None
    llm_error: str | None = None
    duration_ms: float = 0.0
    input_tokens: int | None = None
    output_tokens: int | None = None


@dataclass
class CodegenResult:
    code: str | None
    rounds: list[CodegenRound] = field(default_factory=list)
    error: str | None = None

    @property
    def success(self) -> bool:
        return self.code is not None

    @property
    def total_input_tokens(self) -> int:
        return sum(r.input_tokens or 0 for r in self.rounds)

    @property
    def total_output_tokens(self) -> int:
        return sum(r.output_tokens or 0 for r in self.rounds)


class CodegenWorkflow:
    def __init__(
        self,
        client: LlmClient,
        *,
        scratch: Path,
        max_rounds: int = 3,
        temperature: float = 0.0,
        max_tokens: int = 8192,
        cargo_timeout_s: float = 120.0,
    ) -> None:
        self.client = client
        self.scratch = scratch
        self.max_rounds = max(1, max_rounds)
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.cargo_timeout_s = cargo_timeout_s

    def run(self, cir_json: str, requirement: str | None = None) -> CodegenResult:
        rounds: list[CodegenRound] = []
        user_prompt = codegen_user_prompt(cir_json, requirement)

        for round_number in range(1, self.max_rounds + 1):
            started = time.perf_counter()
            try:
                content, usage = self.client.chat(
                    codegen_system_prompt(),
                    user_prompt,
                    temperature=self.temperature,
                    max_tokens=self.max_tokens,
                )
            except Exception as error:
                rounds.append(CodegenRound(
                    round=round_number,
                    llm_error=str(error),
                    duration_ms=_elapsed(started),
                ))
                user_prompt = codegen_user_prompt(cir_json, requirement)
                continue

            input_tokens, output_tokens = normalize_token_usage(usage)
            code = extract_rust_code(content)
            ok, errors, cargo_wall = run_cargo_check(
                code, self.scratch, self.cargo_timeout_s
            )
            rounds.append(CodegenRound(
                round=round_number,
                code=code,
                cargo_check_ok=ok,
                cargo_errors=None if ok else errors,
                cargo_wall_s=cargo_wall,
                duration_ms=_elapsed(started),
                input_tokens=input_tokens,
                output_tokens=output_tokens,
            ))
            if ok:
                return CodegenResult(code=code, rounds=rounds)
            user_prompt = codegen_retry_prompt(cir_json, code, errors)

        return CodegenResult(
            code=None,
            rounds=rounds,
            error=f"exhausted {self.max_rounds} codegen rounds",
        )


def _elapsed(started: float) -> float:
    return (time.perf_counter() - started) * 1000
