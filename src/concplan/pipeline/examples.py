"""Clean Stage 0-first example loader for ``examples/<name>``.

Discovers runnable ConcPlanVerify v5 examples and loads their free-form natural
language specifications (``spec.md``), Stage 0 structured requirements
(``REQUIREMENTS.md``, ``requirements.json``), frozen contracts
(``contract.json``, ``contract_explanation.json``), and verified SCIR models
(``model.scir.json``).
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[3]
DEFAULT_EXAMPLES_DIR = REPO / "examples"


@dataclass(frozen=True)
class ExampleTask:
    """One Stage 0-driven example discovered under ``examples/<name>``."""

    example_id: str
    directory: Path
    spec_md_path: Path | None = None
    requirements_md_path: Path | None = None
    requirements_json_path: Path | None = None
    contract_path: Path | None = None
    contract_explanation_path: Path | None = None
    model_scir_path: Path | None = None
    traceability_manifest_path: Path | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def load_spec_text(self) -> str:
        if self.spec_md_path is None or not self.spec_md_path.is_file():
            raise FileNotFoundError(f"spec.md not found for example {self.example_id}")
        return self.spec_md_path.read_text(encoding="utf-8")

    def load_requirements_text(self) -> str:
        if self.requirements_md_path is not None and self.requirements_md_path.is_file():
            return self.requirements_md_path.read_text(encoding="utf-8")
        if self.spec_md_path is not None and self.spec_md_path.is_file():
            return self.spec_md_path.read_text(encoding="utf-8")
        raise FileNotFoundError(f"no requirements document found for example {self.example_id}")

    def load_requirements_json(self) -> dict[str, Any] | None:
        if self.requirements_json_path is None or not self.requirements_json_path.is_file():
            return None
        return json.loads(self.requirements_json_path.read_text(encoding="utf-8"))

    def load_requirements_clauses(self) -> dict[str, str]:
        data = self.load_requirements_json()
        if not data:
            return {}
        raw_clauses = data.get("clauses")
        if isinstance(raw_clauses, list):
            out: dict[str, str] = {}
            for idx, item in enumerate(raw_clauses, start=1):
                if isinstance(item, dict):
                    cid = str(item.get("id") or f"R{idx}").strip()
                    text = str(item.get("text") or item.get("description") or "").strip()
                    out[cid] = text
                elif isinstance(item, str):
                    out[f"R{idx}"] = item
            return out
        raw_reqs = data.get("requirements")
        if isinstance(raw_reqs, list):
            out = {}
            for idx, item in enumerate(raw_reqs, start=1):
                if isinstance(item, dict):
                    cid = str(item.get("id") or f"R{idx}").strip()
                    text = str(item.get("text") or item.get("description") or "").strip()
                    out[cid] = text
                else:
                    out[f"R{idx}"] = str(item)
            return out
        return {}

    def load_contract(self) -> dict[str, Any]:
        if self.contract_path is None or not self.contract_path.is_file():
            raise FileNotFoundError(f"contract.json not found for example {self.example_id}")
        return json.loads(self.contract_path.read_text(encoding="utf-8"))

    def load_model_scir(self) -> dict[str, Any]:
        if self.model_scir_path is None or not self.model_scir_path.is_file():
            raise FileNotFoundError(f"model.scir.json not found for example {self.example_id}")
        return json.loads(self.model_scir_path.read_text(encoding="utf-8"))

    def as_summary(self) -> dict[str, Any]:
        return {
            "example_id": self.example_id,
            "has_spec_md": self.spec_md_path is not None and self.spec_md_path.is_file(),
            "has_requirements_md": (
                self.requirements_md_path is not None and self.requirements_md_path.is_file()
            ),
            "has_requirements_json": (
                self.requirements_json_path is not None and self.requirements_json_path.is_file()
            ),
            "has_contract": self.contract_path is not None and self.contract_path.is_file(),
            "has_contract_explanation": (
                self.contract_explanation_path is not None
                and self.contract_explanation_path.is_file()
            ),
            "has_model_scir": self.model_scir_path is not None and self.model_scir_path.is_file(),
            "has_traceability_manifest": (
                self.traceability_manifest_path is not None
                and self.traceability_manifest_path.is_file()
            ),
            "metadata": dict(self.metadata),
            "directory": str(self.directory),
        }


def _existing(path: Path) -> Path | None:
    return path if path.is_file() else None


def _load_manifest_metadata(examples_dir: Path) -> dict[str, dict[str, Any]]:
    manifest_path = examples_dir / "MANIFEST.json"
    if not manifest_path.is_file():
        return {}
    try:
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    out: dict[str, dict[str, Any]] = {}
    for entry in data.get("examples") or []:
        if isinstance(entry, dict) and entry.get("example_id"):
            out[str(entry["example_id"])] = dict(entry)
    return out


def list_examples(examples_dir: Path | str | None = None) -> list[ExampleTask]:
    """Discover all examples under ``examples/<name>``."""
    root = Path(examples_dir).expanduser().resolve() if examples_dir else DEFAULT_EXAMPLES_DIR
    if not root.is_dir():
        return []
    manifest_meta = _load_manifest_metadata(root)
    examples: list[ExampleTask] = []
    for ex_dir in sorted(p for p in root.iterdir() if p.is_dir() and not p.name.startswith(".")):
        spec_p = _existing(ex_dir / "spec.md")
        req_md_p = _existing(ex_dir / "REQUIREMENTS.md")
        req_json_p = _existing(ex_dir / "requirements.json")
        contract_p = _existing(ex_dir / "contract.json")
        if not any((spec_p, req_md_p, req_json_p, contract_p)):
            continue
        meta: dict[str, Any] = dict(manifest_meta.get(ex_dir.name) or {})
        vsum = ex_dir / "verification_summary.json"
        if vsum.is_file():
            try:
                meta["verification_summary"] = json.loads(vsum.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                pass
        examples.append(
            ExampleTask(
                example_id=ex_dir.name,
                directory=ex_dir,
                spec_md_path=spec_p,
                requirements_md_path=req_md_p,
                requirements_json_path=req_json_p,
                contract_path=contract_p,
                contract_explanation_path=_existing(ex_dir / "contract_explanation.json"),
                model_scir_path=_existing(ex_dir / "model.scir.json"),
                traceability_manifest_path=_existing(ex_dir / "traceability_manifest.json"),
                metadata=meta,
            )
        )
    return examples


def load_example(
    example_id: str,
    examples_dir: Path | str | None = None,
) -> ExampleTask:
    """Look up one example by ``example_id``."""
    normalized = example_id.strip().replace("\\", "/").strip("/")
    for ex in list_examples(examples_dir):
        if ex.example_id == normalized:
            return ex
    raise KeyError(f"example not found: {example_id!r}")


def examples_summary(examples_dir: Path | str | None = None) -> dict[str, Any]:
    """Return a JSON-serializable summary of all active Stage 0 examples."""
    examples = list_examples(examples_dir)
    return {
        "schema": "concir-examples-summary/1",
        "total": len(examples),
        "examples": [ex.as_summary() for ex in examples],
    }
