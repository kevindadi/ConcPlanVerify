"""Tool-driven Rust instrumentation for extraction (no LLM edits Rust).

`concir-instrument` parses a std-only single-file Rust program and inserts a
`cir_trace::ev` call before every concurrency operation, emitting a label map
(``L<n>`` -> line / op / receiver / thread). The extraction oracle then only
asks the model to map labels to CIR; the annotated Rust is produced by the tool.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any


def find_instrument_binary(explicit: Path | str | None = None) -> Path:
    if explicit:
        return Path(explicit).expanduser().resolve()
    env = os.environ.get("CONCIR_INSTRUMENT")
    candidates = [Path(env)] if env else []
    repo = Path(__file__).resolve().parents[2]
    candidates += [repo.parent / "ConcIR/target/release/concir-instrument",
                   repo.parent / "ConcIR/target/debug/concir-instrument"]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise FileNotFoundError("concir-instrument not found (set CONCIR_INSTRUMENT)")


def instrument(rust_source: str, out_dir: Path | str, *,
               binary: Path | str | None = None,
               timeout: float = 120.0) -> dict[str, Any]:
    """Instrument ``rust_source``; return ``{annotated, labels, labels_path}``."""

    out = Path(out_dir).expanduser().resolve()
    out.mkdir(parents=True, exist_ok=True)
    source_path = out / "input.rs"
    source_path.write_text(rust_source, encoding="utf-8")
    proc = subprocess.run(
        [str(find_instrument_binary(binary)), str(source_path), "--out", str(out)],
        capture_output=True, text=True, timeout=timeout)
    if proc.returncode != 0:
        raise RuntimeError(f"concir-instrument failed: {proc.stderr.strip()}")
    annotated = (out / "annotated.rs").read_text(encoding="utf-8")
    labels = json.loads((out / "labels.json").read_text(encoding="utf-8"))
    return {"annotated": annotated, "labels": labels,
            "labels_path": str(out / "labels.json")}


def labels_prompt_section(labels: dict[str, Any]) -> str:
    """Render the label map for the extraction prompt."""

    rows = ["| label | line | op | receiver | thread |",
            "| --- | --- | --- | --- | --- |"]
    for item in labels.get("labels", []):
        rows.append("| {label} | {line} | {op} | `{receiver}` | {thread} |".format(
            label=item.get("label"), line=item.get("line"), op=item.get("op"),
            receiver=item.get("receiver"), thread=item.get("thread")))
    return "\n".join(rows)
