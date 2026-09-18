"""B5 scale and cost: generated lock-chain CIR over threads x chain length.

No LLM. For each ``(threads, chain_len)`` configuration the runner generates a
modular CIR program in which every thread acquires the chain in a single global
order (so it is deadlock-free) and records ``explore`` outcome, completeness,
states, transitions and wall-clock. The contract's ``max_states`` is varied so
the UNKNOWN boundary is observable. ConcIR per-call wall-clock is recorded next
to the (Track D) Rust-tool wall-clock in the report.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from .concir_client import ConcirClient


def _default_binary() -> Path | None:
    env = os.environ.get("CONCIR_BACKEND")
    candidates = [Path(env)] if env else []
    repo = Path(__file__).resolve().parents[2]
    candidates.append(repo.parent / "ConcIR/target/release/concir-backend")
    candidates.append(repo.parent / "ConcIR/target/debug/concir-backend")
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def lock_chain_program(threads: int, chain_len: int) -> dict[str, Any]:
    names = [f"m{i}" for i in range(1, chain_len + 1)]
    resources = [{"name": n, "kind": "sync", "type": "Mutex", "mode": "Sync"}
                 for n in names]
    functions = [{"name": "main", "kind": "normal", "body": [
        {"sid": "s1", "kind": "scope",
         "funcs": [f"main::t{i}" for i in range(1, threads + 1)]},
        {"sid": "s2", "kind": "return"},
    ]}]
    for i in range(1, threads + 1):
        body = [{"sid": f"s{j+1}", "kind": "mutex_lock", "resource": f"main::{n}"}
                for j, n in enumerate(names)]
        body += [{"sid": f"s{chain_len+j+1}", "kind": "mutex_unlock",
                  "resource": f"main::{n}"} for j, n in enumerate(reversed(names))]
        body.append({"sid": f"s{2*chain_len+1}", "kind": "return"})
        functions.append({"name": f"t{i}", "kind": "normal", "form": "closure",
                          "body": body})
    return {
        "program": f"lock_chain_{threads}x{chain_len}", "version": "3.5.0",
        "entry": "main::main",
        "modules": [{
            "name": "main",
            "provides": {"resources": names,
                         "functions": ["main"] + [f"t{i}" for i in range(1, threads + 1)]},
            "requires": {"resources": [], "functions": []},
            "resources": resources, "protection": [], "functions": functions,
        }],
    }


def lock_chain_contract(threads: int, *, max_states: int, max_threads: int = 64) -> dict[str, Any]:
    preserved = [{"kind": "reachable", "description": f"main::t{i} completes",
                  "goal": {"kind": "function_completed", "function": f"main::t{i}"}}
                 for i in range(1, threads + 1)]
    return {
        "name": f"scale-{threads}",
        "properties": [{"kind": "deadlock_free", "id": "no-deadlock"}],
        "preserved": preserved,
        "assumptions": {"sequential_consistency": True, "no_spurious_wakeups": True},
        "bounds": {"max_threads": max_threads, "max_frames_per_thread": 8,
                   "max_states": max_states, "max_depth": 128,
                   "max_boundary_events": 256},
        "allowed_scope": {"allow_lock_reorder": True},
    }


def render_markdown(result: dict[str, Any]) -> str:
    lines = ["# B5 — scale and cost (no LLM)", "",
             f"- ConcIR binary sha256: `{result['binary_sha256']}`", "",
             "| threads | chain | max_states | outcome | complete | states | transitions | wall_ms |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for r in result["runs"]:
        lines.append(
            f"| {r['threads']} | {r['chain_len']} | {r['max_states']} | {r['outcome']} | "
            f"{r['complete']} | {r['states_explored']} | {r['transitions_explored']} | "
            f"{r['wall_ms']} |")
    lines += ["", "UNKNOWN marks the analysis bound being reached; it is not a safety verdict."]
    return "\n".join(lines) + "\n"


def run_scale(out_dir: Path | str, *, matrix: list[tuple[int, int]] | None = None,
              max_states_values: list[int] | None = None) -> dict[str, Any]:
    binary = _default_binary()
    if binary is None:
        raise FileNotFoundError("concir-backend not found (set CONCIR_BACKEND)")
    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    matrix = matrix or [(2, 2), (3, 2), (4, 2), (5, 2), (3, 3), (4, 3), (5, 3), (6, 3)]
    max_states_values = max_states_values or [5000, 20000, 200000]
    client = ConcirClient(binary, workdir=out / "calls", timeout=120.0)
    runs = []
    for threads, chain_len in matrix:
        program = lock_chain_program(threads, chain_len)
        program_path = out / f"chain_{threads}x{chain_len}.cir.json"
        program_path.write_text(json.dumps(program, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8")
        for max_states in max_states_values:
            contract = lock_chain_contract(threads, max_states=max_states)
            contract_path = out / f"chain_{threads}x{chain_len}_ms{max_states}.contract.json"
            contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
            result = client.explore(program_path, contract_path, "petri")
            runs.append({
                "threads": threads, "chain_len": chain_len, "max_states": max_states,
                "outcome": result.outcome, "complete": result.complete,
                "status": result.status, "kind": result.kind,
                "states_explored": (result.payload or {}).get("states_explored"),
                "transitions_explored": (result.payload or {}).get("transitions_explored"),
                "wall_ms": result.wall_ms,
            })
    payload = {
        "schema_version": "scale-v1",
        "binary_sha256": __import__("hashlib").sha256(binary.read_bytes()).hexdigest(),
        "runs": runs,
    }
    (out / "SCALE.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
                                    encoding="utf-8")
    (out / "SCALE.md").write_text(render_markdown(payload), encoding="utf-8")
    return payload
