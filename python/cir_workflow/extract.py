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


def extraction_prompt_v4(schema: str, rust_source: str, labels_markdown: str) -> str:
    return (
        "Machine schema for ConcIR (authoritative):\n```json\n"
        + schema.strip()
        + "\n```\n\nRust program (annotated by the tool; do not edit):\n```rust\n"
        + rust_source.strip()
        + "\n```\n\nConcurrency labels (each is the `sid` of its operation):\n"
        + labels_markdown.strip()
        + "\n\nReply with exactly one ```json block containing the CIR model."
    )


def parse_cir_only(text: str) -> dict[str, Any] | None:
    """Parse a single-fence CIR reply (v4 protocol)."""

    stripped = text.strip()
    if "```" in stripped:
        for part in stripped.split("```")[1::2]:
            candidate = part.lstrip()
            if candidate.lower().startswith("json"):
                candidate = candidate[4:]
            try:
                data = json.loads(candidate)
            except json.JSONDecodeError:
                continue
            if isinstance(data, dict) and "modules" in data:
                return {"cir": data}
    try:
        data = json.loads(stripped)
    except json.JSONDecodeError:
        return None
    return {"cir": data} if isinstance(data, dict) and "modules" in data else None


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


LINE_RE = __import__("re").compile(r"line (\d+)")


def write_extraction_result(work_dir: Path | str, record: dict[str, Any]) -> dict[str, Any]:
    """Persist a per-cell extraction record (used for parse/transport stages)."""

    work_dir = Path(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    record.setdefault("extract_validated", False)
    record.setdefault("model_verdict", None)
    (work_dir / "extraction_result.json").write_text(
        json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return record


def _ensure_cir_trace_mod(rust: str) -> tuple[str, bool]:
    """The skeleton provides ``src/cir_trace.rs``; models often omit the
    ``mod`` declaration when they rewrite ``main.rs``. Prepend it if absent."""

    import re

    if re.search(r"^\s*(pub\s+)?mod\s+cir_trace\s*;", rust, re.MULTILINE):
        return rust, False
    return "mod cir_trace;\n" + rust, True


def validate_extraction(extracted_cir: dict, annotated_rust: str, contract_path: Path,
                        work_dir: Path, *, binary: Path | str,
                        native_runs: int = 20, miri_seeds: int = 8,
                        client=None, lenient_unlock: bool = False,
                        attempt_events: bool = False) -> dict[str, Any]:
    """Build the annotated Rust, trace it, and require every trace conformant.

    Every exit writes ``extraction_result.json`` in ``work_dir`` with a
    ``stage`` in {parse, normalize, codegen, build, trace, conform, explore,
    harness}, a JSON pointer when one is known, and the stderr artefact path.
    """

    from . import conformance
    from .concir_client import ConcirClient  # lazy: avoids cycles at import time

    work_dir = Path(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    program_path = work_dir / "extracted.cir.json"
    result_path = work_dir / "extraction_result.json"

    def emit(record: dict[str, Any]) -> dict[str, Any]:
        record.setdefault("extract_validated", False)
        record.setdefault("model_verdict", None)
        result_path.write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n",
                               encoding="utf-8")
        return record

    def harness(stage: str, exc: BaseException, **extra: Any) -> dict[str, Any]:
        return emit({"stage": "harness", "where": stage,
                     "reason": f"{type(exc).__name__}: {exc}",
                     "harness_error": True, **extra})

    try:
        normalized, records, issues = normalize_program(extracted_cir,
                                                        strict_arrays=True)
    except Exception as exc:  # noqa: BLE001
        return harness("normalize", exc)
    program_path.write_text(json.dumps(normalized, ensure_ascii=False, indent=2) + "\n",
                            encoding="utf-8")
    if issues:
        first = issues[0]
        return emit({"stage": "normalize", "reason": "extracted CIR failed normalisation",
                     "pointer": first.get("pointer"), "detail": first,
                     "normalizations": records})

    rust_source, added_mod = _ensure_cir_trace_mod(annotated_rust)
    normalizations = list(records)
    if added_mod:
        normalizations.append({"rule": "prepend_mod_cir_trace"})

    # Use the generated skeleton for the trace runtime, then replace its main
    # with the model's annotated Rust.
    skeleton = work_dir / "skeleton"
    try:
        conformance.codegen(program_path, skeleton, binary=binary)
    except Exception as exc:  # noqa: BLE001
        stderr_path = work_dir / "codegen.stderr.txt"
        stderr_path.write_text(str(exc), encoding="utf-8")
        return emit({"stage": "codegen", "reason": f"extracted CIR not codegen-able: {exc}",
                     "stderr_path": str(stderr_path),
                     "normalizations": normalizations})
    (skeleton / "src/main.rs").write_text(rust_source, encoding="utf-8")
    try:
        build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=skeleton,
                               capture_output=True, text=True, timeout=300)
    except Exception as exc:  # noqa: BLE001
        return harness("build", exc, normalizations=normalizations)
    (work_dir / "build.stderr.txt").write_text(build.stderr, encoding="utf-8")
    if build.returncode != 0:
        pointer = None
        match = LINE_RE.search(build.stderr)
        if match:
            pointer = f"source:line:{match.group(1)}"
        return emit({"stage": "build", "reason": "annotated Rust did not build",
                     "pointer": pointer, "stderr_path": str(work_dir / "build.stderr.txt"),
                     "normalizations": normalizations})
    try:
        tries = collect_traces(skeleton, native_runs=native_runs, miri_seeds=miri_seeds,
                               timeout_s=8.0, calls_dir=work_dir / "traces",
                               run_miri=miri_seeds > 0)
    except Exception as exc:  # noqa: BLE001
        return harness("trace", exc, normalizations=normalizations)
    try:
        agg = conform_all(program_path, tries, binary=binary,
                          lenient_unlock=lenient_unlock,
                          attempt_events=attempt_events)
    except Exception as exc:  # noqa: BLE001
        return harness("conform", exc, normalizations=normalizations)
    expected = native_runs + miri_seeds
    validated = agg["conformant"] == expected and agg["violation"] == 0
    if not validated:
        bad = next((d for d in agg["details"]
                    if d.get("status") not in ("conformant",)), None)
        return emit({"stage": "conform", "reason": "traces not all conformant",
                     "pointer": f"traces/{bad['kind']}-{bad['index']}" if bad else None,
                     "conformance": agg, "normalizations": normalizations})

    try:
        concir = ConcirClient(binary, workdir=work_dir / "explore", timeout=30.0)
        explore = concir.explore(program_path, contract_path, "petri")
    except Exception as exc:  # noqa: BLE001
        return harness("explore", exc, conformance=agg, normalizations=normalizations)
    verdict = explore.outcome
    return emit({
        "stage": "explore",
        "extract_validated": True,
        "conformance": agg,
        "model_verdict": verdict,
        "model_complete": explore.complete,
        "normalizations": normalizations,
        "extracted_cir_sha256": sha256_text(json.dumps(normalized, sort_keys=True)),
    })
