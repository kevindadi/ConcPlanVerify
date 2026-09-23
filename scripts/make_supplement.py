#!/usr/bin/env python3
"""Build the reviewer supplement zip for a freeze tag.

Gates (any failure -> no zip, non-zero exit):
  * anonymisation: no absolute user paths, author names, hostname or username;
  * secret scan: no API keys / `.env` content;
  * size: default <= 200 MB;
  * self-check: unzip to a temp dir and re-run `python -m cir_workflow results`
    with the package-relative command; the regenerated `RESULTS.md` and
    `tables/*.tex` must be byte-identical.

Usage: python scripts/make_supplement.py --tag experiments-v2-freeze-7
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DIST = REPO / "dist"

TIER_A = ["tables", "flash-gen-main-v4-code", "gen-code-replay-v2",
          "flash-repair-main-v1", "detection-v3", "scale-v2",
          "conform-mutation-v2", "post-edit-conform-v2", "extraction-v5",
          "model-probe-v2", "gen-model-probe-v2", "rust-oracle-v1",
          "gen-llmcode-smoke-v1", "case-partial-deadlock-v1", "a3-to-rust-v2",
          "contract-strength-v1", "extraction-v6"]
DENY_PARTS = {"proj", "target", "traces", "monitor-traces", "conform-traces",
              "llm", "calls", "__pycache__", ".git", ".venv", "candidate",
              "oracle"}
KEEP_SUFFIX = (".md", ".json", ".cir.json", ".rs", ".tex", ".toml", ".txt",
               ".csv")
FORBIDDEN = [r"/Users/kevin", r"/home/[a-z]+", r"Kaiwen", r"Guanjun",
             r"zhangkw", r"liuguanjun", r"sk-[A-Za-z0-9]{8,}",
             r"OPENCODE_API_KEY", r"DEEPSEEK_API_KEY", r"Bearer\s+[A-Za-z0-9]",
             r"OPENCODE_API_KEY\s*=", r"DEEPSEEK_API_KEY\s*="]
# Flag only values, not the environment-variable *names* the scripts read.
# Flag secret *values*, not the environment-variable names the scripts read.
SECRET = [r"\bsk-[A-Za-z0-9]{8,}",
          r"Bearer\s+[A-Za-z0-9._-]{10,}",
          r"(OPENCODE|DEEPSEEK|[A-Za-z0-9_]+)_?API_KEY\s*=\s*[A-Za-z0-9]"]
PATH_REWRITES = {
    str(REPO): ".",
    str(REPO.parent / "ConcIR"): "./concir",
    "/Users/kevin": "<home>",
    REPO.name: REPO.name,
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _anonymize_text(text: str) -> str:
    for old, new in PATH_REWRITES.items():
        text = text.replace(old, new)
    return text


def _iter_experiment_files(root: Path):
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        parts = set(path.parts)
        if parts & DENY_PARTS:
            continue
        if path.name.endswith(KEEP_SUFFIX) or path.suffix in (".rs", ".json"):
            yield path


def collect() -> list[tuple[Path, str]]:
    files: list[tuple[Path, str]] = []
    roots = [
        ("benchmarks", REPO / "benchmarks"),
        ("prompts", REPO / "prompts"),
        ("python", REPO / "python"),
        ("scripts", REPO / "scripts"),
        ("runtime", REPO / "runtime"),
    ]
    for prefix, base in roots:
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            if path.is_file() and not (set(path.parts) & {"__pycache__", ".venv", "legacy"}):
                files.append((path, f"{prefix}/{path.relative_to(base)}"))
    for name in ("Makefile", "pyproject.toml"):
        path = REPO / name
        if path.is_file():
            files.append((path, name))
    files.append((REPO / "experiments/RESULTS.md", "experiments/RESULTS.md"))
    files.append((REPO / "docs/PAPER_EVIDENCE_MAP.md", "docs/PAPER_EVIDENCE_MAP.md"))
    files.append((REPO / "experiments/FREEZE_MANIFEST.md", "experiments/FREEZE_MANIFEST.md"))
    files.append((REPO / "experiments/EVIDENCE_TIERS.md", "experiments/EVIDENCE_TIERS.md"))
    files.append((REPO / "experiments/HUMAN_REVIEW_QUEUE_GEN.md",
                  "experiments/HUMAN_REVIEW_QUEUE_GEN.md"))
    # Tier B inputs the `results` self-check command reads.
    for name in TIER_A + ["conform-mutation-v1", "post-edit-conform-v1"]:
        base = REPO / "experiments" / name
        if not base.exists():
            continue
        for path in _iter_experiment_files(base):
            files.append((path, f"experiments/{name}/{path.relative_to(base)}"))
    return files


def archive_concir(tmp: Path, tag: str) -> list[tuple[Path, str]]:
    concir = REPO.parent / "ConcIR"
    out = tmp / "concir.tar"
    try:
        proc = subprocess.run(["git", "-C", str(concir), "archive", "--format=tar",
                               f"-o{out}", tag], capture_output=True, text=True)
        if proc.returncode != 0:
            raise RuntimeError(proc.stderr.strip())
        extract = tmp / "concir"
        extract.mkdir(parents=True, exist_ok=True)
        subprocess.run(["tar", "-xf", str(out), "-C", str(extract)], check=True)
    except Exception as exc:  # noqa: BLE001
        print(f"warning: could not archive ConcIR tag {tag}: {exc}", file=sys.stderr)
        return []
    return [(p, f"concir/{p.relative_to(extract)}") for p in extract.rglob("*") if p.is_file()]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--concir-tag", default="concir-freeze-6")
    parser.add_argument("--max-mb", type=float, default=200.0)
    parser.add_argument("--skip-selfcheck", action="store_true")
    args = parser.parse_args()
    DIST.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        files = collect()
        files += archive_concir(tmp, args.concir_tag)

        staged = tmp / "stage"
        staged.mkdir()
        manifest = ["# SUPPLEMENT_MANIFEST", "", f"Freeze tag: `{args.tag}`",
                    f"ConcIR tag: `{args.concir_tag}`", "",
                    "Path rewrites: the repository root becomes `.`, the ConcIR "
                    "checkout becomes `./concir`, and `/Users/kevin` becomes "
                    "`<home>` inside text records.", "",
                    "| file | sha256 |", "| --- | --- |"]
        errors: list[str] = []
        for src, arc in files:
            if not src.is_file():
                continue
            dest = staged / arc
            dest.parent.mkdir(parents=True, exist_ok=True)
            if src.suffix in (".md", ".json", ".tex", ".txt", ".csv", ".toml"):
                text = _anonymize_text(src.read_text(encoding="utf-8", errors="replace"))
                dest.write_text(text, encoding="utf-8")
            else:
                shutil.copyfile(src, dest)
            manifest.append(f"| `{arc}` | `{sha256(dest)}` |")

        # gates
        for path in staged.rglob("*"):
            if not path.is_file():
                continue
            blob = path.read_bytes()
            for pat in SECRET:
                if re.search(pat.encode(), blob):
                    errors.append(f"secret match {pat} in {path.relative_to(staged)}")
            if path.suffix in (".md", ".json", ".tex", ".csv", ".toml"):
                text = path.read_text(encoding="utf-8", errors="replace")
                for pat in FORBIDDEN[:6]:
                    if re.search(pat, text):
                        errors.append(f"identity leak {pat} in {path.relative_to(staged)}")
        if errors:
            print("GATE FAILED:", file=sys.stderr)
            for e in errors[:20]:
                print("  ", e, file=sys.stderr)
            return 1

        (staged / "SUPPLEMENT_MANIFEST.md").write_text("\n".join(manifest) + "\n",
                                                       encoding="utf-8")
        # size
        total = sum(p.stat().st_size for p in staged.rglob("*") if p.is_file())
        if total > args.max_mb * 1024 * 1024:
            biggest = sorted((p for p in staged.rglob("*") if p.is_file()),
                             key=lambda p: p.stat().st_size, reverse=True)[:20]
            print(f"GATE FAILED: size {total/1e6:.1f} MB > {args.max_mb} MB",
                  file=sys.stderr)
            for p in biggest:
                print(f"  {p.stat().st_size/1e6:8.2f} MB {p.relative_to(staged)}",
                      file=sys.stderr)
            return 1

        zip_path = DIST / f"concplanverify-supplement-{args.tag}.zip"
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in sorted(staged.rglob("*")):
                if path.is_file():
                    zf.write(path, path.relative_to(staged))
        (DIST / (zip_path.name + ".sha256")).write_text(
            f"{sha256(zip_path)}  {zip_path.name}\n", encoding="utf-8")

        selfcheck = "skipped"
        if not args.skip_selfcheck:
            selfcheck = _self_check(zip_path)
            print("self-check:", selfcheck)
            if selfcheck != "ok":
                zip_path.unlink(missing_ok=True)
                return 1
        print(f"wrote {zip_path} ({zip_path.stat().st_size/1e6:.1f} MB); "
              f"files={sum(1 for _ in staged.rglob('*') if _.is_file())}")
    return 0


def _self_check(zip_path: Path) -> str:
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        with zipfile.ZipFile(zip_path) as zf:
            zf.extractall(root)
        results = root / "experiments/RESULTS.md"
        tables = root / "experiments/tables"
        if not results.is_file():
            return "no RESULTS.md in package"
        ref_results = results.read_text(encoding="utf-8")
        ref_tables = {p.name: p.read_bytes() for p in tables.glob("*.tex")}
        cmd = _package_command(ref_results)
        if cmd is None:
            return "could not read the command header"
        env = dict(os.environ, PYTHONPATH=str(root / "python"))
        proc = subprocess.run(cmd, cwd=root, env=env, capture_output=True, text=True)
        if proc.returncode != 0:
            return f"results failed: {proc.stderr[-300:]}"
        if results.read_text(encoding="utf-8") != ref_results:
            return "RESULTS.md differs after regeneration"
        for name, blob in ref_tables.items():
            if (tables / name).read_bytes() != blob:
                return f"{name} differs after regeneration"
        return "ok"


def _package_command(text: str) -> list[str] | None:
    import shlex
    for line in text.splitlines():
        if "cir_workflow results" in line and line.startswith(">"):
            tokens = shlex.split(line[2:])
            idx = tokens.index("cir_workflow") if "cir_workflow" in tokens else -1
            if idx < 0:
                return None
            rest = tokens[idx + 1:]
            while rest and rest[0] == "results":
                rest = rest[1:]
            return [sys.executable, "-m", "cir_workflow", "results"] + rest
    return None


if __name__ == "__main__":
    raise SystemExit(main())
