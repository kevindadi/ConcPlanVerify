"""LLM extraction oracle: candidate Rust -> CIR + ev-annotated Rust, then
validate the extraction by trace conformance before trusting its verdict.

The model must return ``{"cir": <CIR>, "rust": "<annotated source>"}``. The
annotated source is built and traced (native + Miri seeds); if every trace is
conformant against the extracted CIR, the extraction is *validated* and the CIR
may be explored. Otherwise the result is ``extract_unverified``.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any, Callable

from .conformance import TraceRun, collect_traces, conform_all
from .experiments_v2 import sha256_text
from .normalize import normalize as normalize_program


def schema_text(binary: Path | str) -> str:
    proc = subprocess.run([str(binary), "schema"], capture_output=True, text=True,
                          timeout=60)
    if proc.returncode != 0:
        return "{}"
    return proc.stdout


def extraction_prompt(schema: str, rust_source: str) -> str:
    return (
        "Machine schema for ConcIR (authoritative):\n```json\n"
        + schema.strip()
        + "\n```\n\nRust program to model and annotate:\n```rust\n"
        + rust_source.strip()
        + "\n```\n\nReply with exactly two fenced blocks: a ```json CIR block "
          "first, then a ```rust annotated-source block."
    )


def _fenced(text: str, lang: str) -> str | None:
    import re

    match = re.search(r"```" + lang + r"\s*\n(.*?)```", text, re.DOTALL | re.IGNORECASE)
    return match.group(1).strip() if match else None


def parse_extraction(text: str) -> dict[str, Any] | None:
    """Two-fence protocol: ```json CIR then ```rust annotated source."""

    stripped = text.strip()
    if "```" in stripped:
        json_block = _fenced(stripped, "json")
        rust_block = _fenced(stripped, "rust") or _fenced(stripped, "rs")
        if json_block and rust_block:
            try:
                cir = json.loads(json_block)
            except json.JSONDecodeError:
                cir = None
            if isinstance(cir, dict):
                return {"cir": cir, "rust": rust_block}
        # single-object fallback
        for part in stripped.split("```")[1::2]:
            candidate = part.lstrip()
            if candidate.lower().startswith("json"):
                candidate = candidate[4:]
            try:
                data = json.loads(candidate)
                if isinstance(data, dict) and "cir" in data:
                    return data
            except json.JSONDecodeError:
                continue
    try:
        data = json.loads(stripped)
        return data if isinstance(data, dict) and "cir" in data else None
    except json.JSONDecodeError:
        return None


def validate_extraction(extracted_cir: dict, annotated_rust: str, contract_path: Path,
                        work_dir: Path, *, binary: Path | str,
                        native_runs: int = 20, miri_seeds: int = 8,
                        client=None) -> dict[str, Any]:
    """Build the annotated Rust, trace it, and require every trace conformant."""

    from . import conformance
    from .concir_client import ConcirClient  # lazy: avoids cycles at import time

    work_dir = Path(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    program_path = work_dir / "extracted.cir.json"
    normalized, records, issues = normalize_program(extracted_cir)
    program_path.write_text(json.dumps(normalized, ensure_ascii=False, indent=2) + "\n",
                            encoding="utf-8")
    if issues:
        return {"extract_validated": False, "reason": "extracted CIR has invalid sids",
                "model_verdict": None}

    # Use the generated skeleton for the trace runtime, then replace its main
    # with the model's annotated Rust.
    try:
        skeleton = work_dir / "skeleton"
        conformance.codegen(program_path, skeleton, binary=binary)
    except Exception as exc:  # noqa: BLE001
        return {"extract_validated": False, "reason": f"extracted CIR not codegen-able: {exc}",
                "model_verdict": None}
    (skeleton / "src/main.rs").write_text(annotated_rust, encoding="utf-8")
    build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=skeleton,
                           capture_output=True, text=True, timeout=300)
    if build.returncode != 0:
        return {"extract_validated": False, "reason": "annotated Rust did not build",
                "model_verdict": None}
    tries = collect_traces(skeleton, native_runs=native_runs, miri_seeds=miri_seeds,
                           timeout_s=8.0, calls_dir=work_dir / "traces",
                           run_miri=miri_seeds > 0)
    agg = conform_all(program_path, tries, binary=binary)
    expected = native_runs + miri_seeds
    validated = agg["conformant"] == expected and agg["violation"] == 0
    if not validated:
        return {"extract_validated": False, "reason": "traces not all conformant",
                "conformance": agg, "model_verdict": None}

    concir = ConcirClient(binary, workdir=work_dir / "explore", timeout=30.0)
    explore = concir.explore(program_path, contract_path, "petri")
    verdict = explore.outcome
    return {
        "extract_validated": True,
        "conformance": agg,
        "model_verdict": verdict,
        "model_complete": explore.complete,
        "extracted_cir_sha256": sha256_text(json.dumps(normalized, sort_keys=True)),
    }
