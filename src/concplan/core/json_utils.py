"""JSON extraction and content hashing utilities."""

from __future__ import annotations

import hashlib
import json
import os
import re
import time
from pathlib import Path
from typing import Any

_THINK_TAG_RE = re.compile(r"<think\b[^>]*>.*?</think>", re.DOTALL | re.IGNORECASE)


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path | str) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def extract_json(text: str) -> str:
    """Strip reasoning/fences and return the first decodable JSON value.

    The returned text is intentionally not parsed here. Keeping parse errors in
    the workflow lets the next LLM round receive the exact candidate that
    failed.
    """

    cleaned = _THINK_TAG_RE.sub("", text).strip()
    if cleaned.startswith("```"):
        first_newline = cleaned.find("\n")
        cleaned = cleaned[first_newline + 1 :] if first_newline >= 0 else cleaned[3:]
        if cleaned.rstrip().endswith("```"):
            cleaned = cleaned.rstrip()[:-3].rstrip()

    try:
        _, end = json.JSONDecoder().raw_decode(cleaned)
        return cleaned[:end].strip()
    except json.JSONDecodeError:
        start = cleaned.find("{")
        if start < 0:
            return cleaned
        try:
            _, end = json.JSONDecoder().raw_decode(cleaned[start:])
        except json.JSONDecodeError:
            return cleaned
        return cleaned[start : start + end].strip()


def write_json(path: Path | str, payload: Any) -> Path:
    """Serialize ``payload`` as pretty UTF-8 JSON to ``path``, creating parent dirs."""
    dest = Path(path).expanduser().resolve()
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return dest


def exclusive_run_dir(base: Path | str, *, prefix: str = "run") -> Path:
    """Create a unique per-run directory under ``base`` without overwriting prior runs."""
    root = Path(base).expanduser().resolve()
    root.mkdir(parents=True, exist_ok=True)
    for _ in range(1000):
        token = f"{time.strftime('%Y%m%dT%H%M%S', time.gmtime())}-{os.getpid()}-{os.urandom(3).hex()}"
        path = root / f"{prefix}-{token}"
        try:
            path.mkdir(parents=True, exist_ok=False)
            return path
        except FileExistsError:
            continue
    raise RuntimeError(f"could not allocate a unique run directory under {root}")
