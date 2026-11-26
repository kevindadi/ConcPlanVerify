"""Clean benchmark loader for ``benchmarks/families`` and ``benchmarks/real-cases``.

Discovers benchmark tasks and loads their requirement documents
(``REQUIREMENTS.md``, ``requirements.json``), frozen contracts (``contract.json``),
and reference ConcIR models (``buggy.cir.json``, ``fixed.cir.json``,
``correct.cir.json``, ``repair_input/input.cir.json``).
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[3]
DEFAULT_BENCHMARKS_DIR = REPO / "benchmarks"


@dataclass(frozen=True)
class BenchmarkTask:
    """One benchmark task discovered under ``benchmarks/families`` or ``benchmarks/real-cases``."""

    task_id: str
    suite: str  # "families" | "real-cases"
    family: str
    case: str
    directory: Path
    contract_path: Path | None = None
    requirements_md_path: Path | None = None
    requirements_json_path: Path | None = None
    buggy_cir_path: Path | None = None
    fixed_cir_path: Path | None = None
    correct_cir_path: Path | None = None
    repair_input_cir_path: Path | None = None
    tier: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def load_contract(self) -> dict[str, Any]:
        if self.contract_path is None or not self.contract_path.is_file():
            raise FileNotFoundError(f"contract.json not found for task {self.task_id}")
        return json.loads(self.contract_path.read_text(encoding="utf-8"))

    def load_requirements_text(self) -> str:
        if self.requirements_md_path is not None and self.requirements_md_path.is_file():
            return self.requirements_md_path.read_text(encoding="utf-8")
        for fallback in (
            self.directory / "spec.md",
            self.directory / "mapping.md",
            self.directory / "repair_input/requirements.txt",
        ):
            if fallback.is_file():
                return fallback.read_text(encoding="utf-8")
        raise FileNotFoundError(f"no requirements document found for task {self.task_id}")

    def load_requirements_json(self) -> dict[str, Any] | None:
        if self.requirements_json_path is None or not self.requirements_json_path.is_file():
            return None
        return json.loads(self.requirements_json_path.read_text(encoding="utf-8"))

    def load_requirements_clauses(self) -> dict[str, str]:
        data = self.load_requirements_json()
        if not data or not isinstance(data.get("requirements"), list):
            return {}
        return {f"R{i}": str(text) for i, text in enumerate(data["requirements"], start=1)}

    def load_reference_cir(self, variant: str | None = None) -> dict[str, Any]:
        """Load one reference CIR JSON (``fixed`` / ``correct`` / ``buggy`` / ``repair_input``)."""
        candidates: list[tuple[str, Path | None]]
        if variant == "buggy":
            candidates = [("buggy", self.buggy_cir_path)]
        elif variant == "fixed":
            candidates = [("fixed", self.fixed_cir_path)]
        elif variant == "correct":
            candidates = [("correct", self.correct_cir_path)]
        elif variant == "repair_input":
            candidates = [("repair_input", self.repair_input_cir_path)]
        else:
            candidates = [
                ("fixed", self.fixed_cir_path),
                ("correct", self.correct_cir_path),
                ("buggy", self.buggy_cir_path),
                ("repair_input", self.repair_input_cir_path),
            ]
        for _name, path in candidates:
            if path is not None and path.is_file():
                return json.loads(path.read_text(encoding="utf-8"))
        raise FileNotFoundError(
            f"no reference CIR ({variant or 'any'}) found for task {self.task_id}"
        )

    def as_summary(self) -> dict[str, Any]:
        return {
            "task_id": self.task_id,
            "suite": self.suite,
            "family": self.family,
            "case": self.case,
            "tier": self.tier,
            "has_contract": self.contract_path is not None and self.contract_path.is_file(),
            "has_requirements_md": (
                self.requirements_md_path is not None and self.requirements_md_path.is_file()
            ),
            "has_requirements_json": (
                self.requirements_json_path is not None and self.requirements_json_path.is_file()
            ),
            "reference_cirs": [
                name
                for name, p in (
                    ("buggy", self.buggy_cir_path),
                    ("fixed", self.fixed_cir_path),
                    ("correct", self.correct_cir_path),
                    ("repair_input", self.repair_input_cir_path),
                )
                if p is not None and p.is_file()
            ],
            "directory": str(self.directory),
        }


def _existing(path: Path) -> Path | None:
    return path if path.is_file() else None


def _load_tier_index(benchmarks_dir: Path) -> dict[str, str]:
    gen_manifest = benchmarks_dir / "GENERATION_MANIFEST.json"
    if not gen_manifest.is_file():
        return {}
    try:
        data = json.loads(gen_manifest.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    tiers: dict[str, str] = {}
    for tier_name, task_ids in (data.get("tiers") or {}).items():
        if isinstance(task_ids, list):
            for tid in task_ids:
                tiers[str(tid)] = str(tier_name)
    return tiers


def list_benchmarks(
    benchmarks_dir: Path | str | None = None,
    *,
    suite: str | None = None,
) -> list[BenchmarkTask]:
    """Discover all benchmark tasks in ``benchmarks/families`` and ``benchmarks/real-cases``."""
    root = Path(benchmarks_dir).expanduser().resolve() if benchmarks_dir else DEFAULT_BENCHMARKS_DIR
    tier_index = _load_tier_index(root)
    tasks: list[BenchmarkTask] = []

    if suite in (None, "families"):
        families_dir = root / "families"
        if families_dir.is_dir():
            for family_dir in sorted(p for p in families_dir.iterdir() if p.is_dir()):
                for case_dir in sorted(p for p in family_dir.iterdir() if p.is_dir()):
                    tid = f"{family_dir.name}/{case_dir.name}"
                    tasks.append(
                        BenchmarkTask(
                            task_id=tid,
                            suite="families",
                            family=family_dir.name,
                            case=case_dir.name,
                            directory=case_dir,
                            contract_path=_existing(case_dir / "contract.json"),
                            requirements_md_path=_existing(
                                case_dir / "generation_input/REQUIREMENTS.md"
                            ),
                            requirements_json_path=_existing(
                                case_dir / "generation_input/requirements.json"
                            ),
                            buggy_cir_path=_existing(case_dir / "buggy.cir.json"),
                            fixed_cir_path=_existing(case_dir / "fixed.cir.json"),
                            correct_cir_path=_existing(case_dir / "correct.cir.json"),
                            repair_input_cir_path=_existing(
                                case_dir / "repair_input/input.cir.json"
                            ),
                            tier=tier_index.get(tid),
                        )
                    )

    if suite in (None, "real-cases"):
        real_cases_dir = root / "real-cases/cases"
        if real_cases_dir.is_dir():
            for case_dir in sorted(p for p in real_cases_dir.iterdir() if p.is_dir()):
                tid = f"real-cases/{case_dir.name}"
                meta: dict[str, Any] = {}
                prov = case_dir / "provenance.json"
                if prov.is_file():
                    try:
                        meta["provenance"] = json.loads(prov.read_text(encoding="utf-8"))
                    except (OSError, json.JSONDecodeError):
                        pass
                tasks.append(
                    BenchmarkTask(
                        task_id=tid,
                        suite="real-cases",
                        family="real-cases",
                        case=case_dir.name,
                        directory=case_dir,
                        contract_path=_existing(case_dir / "contract.json"),
                        requirements_md_path=_existing(case_dir / "mapping.md"),
                        requirements_json_path=_existing(case_dir / "requirements.json"),
                        buggy_cir_path=_existing(case_dir / "buggy.cir.json"),
                        fixed_cir_path=_existing(case_dir / "fixed.cir.json"),
                        correct_cir_path=_existing(case_dir / "correct.cir.json"),
                        repair_input_cir_path=_existing(case_dir / "repair_input/input.cir.json"),
                        tier=None,
                        metadata=meta,
                    )
                )

    return tasks


def load_benchmark(
    task_id: str,
    benchmarks_dir: Path | str | None = None,
) -> BenchmarkTask:
    """Look up one benchmark task by ``family/case``, ``real-cases/<case>``, or unique case name."""
    tasks = list_benchmarks(benchmarks_dir)
    normalized = task_id.strip().replace("\\", "/")
    for t in tasks:
        if t.task_id == normalized:
            return t
    matches = [t for t in tasks if t.case == normalized]
    if len(matches) == 1:
        return matches[0]
    if len(matches) > 1:
        ids = ", ".join(m.task_id for m in matches)
        raise KeyError(f"ambiguous benchmark task {task_id!r}; matches: {ids}")
    raise KeyError(f"benchmark task not found: {task_id!r}")


def benchmarks_summary(benchmarks_dir: Path | str | None = None) -> dict[str, Any]:
    """Return a JSON-serializable summary of all active benchmark suites and tasks."""
    tasks = list_benchmarks(benchmarks_dir)
    families = [t for t in tasks if t.suite == "families"]
    real_cases = [t for t in tasks if t.suite == "real-cases"]
    return {
        "schema": "concir-benchmarks-summary/1",
        "total": len(tasks),
        "families_count": len(families),
        "real_cases_count": len(real_cases),
        "families": sorted({t.family for t in families}),
        "tasks": [t.as_summary() for t in tasks],
    }
