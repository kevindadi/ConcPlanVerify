#!/usr/bin/env python3
"""Build the reviewer supplement zip for a freeze tag.

The private word list (author names, accounts, host) and secret patterns live in
the gitignored `.supplement-private.json`; this script contains no real names.

Hard gates (any failure -> no zip, non-zero exit):
  * anonymisation of every text file (`PATH_REWRITES` + identity -> `<redacted>`);
  * post-unzip full-tree scan of every file, binary included, for identity and
    secret patterns;
  * size <= `--max-mb`;
  * self-check: unzip, re-run `python -m cir_workflow results` with the
    package-relative command; `RESULTS.md` and `tables/*.tex` must be identical.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DIST = REPO / "dist"

TIER_A = ["tables", "flash-gen-main-v5-code", "flash-gen-main-v4-code",
          "gen-code-replay-v2",
          "flash-repair-main-v1", "detection-v3", "scale-v2",
          "conform-mutation-v2", "post-edit-conform-v2", "extraction-v5",
          "model-probe-v2", "gen-model-probe-v2", "rust-oracle-v1",
          "gen-llmcode-smoke-v1", "gen-expert-labels-v1", "gen-model-probe-v3-code",
          "case-partial-deadlock-v1", "a3-to-rust-v2",
          "contract-strength-v1", "extraction-v6",
          "conform-mutation-v1", "post-edit-conform-v1"]
DENY_PARTS = {"proj", "target", "traces", "monitor-traces", "conform-traces",
              "llm", "calls", "__pycache__", ".git", ".venv", "candidate",
              "oracle", "raw", "legacy-cir2cvn", "reviews", "_archive"}
TEXT_SUFFIX = {".py", ".sh", ".toml", ".lock", ".md", ".json", ".rs", ".tex",
               ".txt", ".csv", ".jsonl"}
RAW_GLOBS = ("native-", "miri-")
EXCLUDE_NAMES = {"make_supplement.py", ".supplement-private.json",
                 "cir_trace.rs", "concir_sync.rs", "env.json", "wall_ms.txt",
                 "exit.txt", "build.stdout.txt", "build.stderr.txt"}


def _load_private() -> dict:
    for name in (".supplement-private.json", ".supplement-private.example.json"):
        path = REPO / name
        if path.is_file():
            return json.loads(path.read_text(encoding="utf-8"))
    return {"identity": [], "secrets": []}


def _rewrites(private: dict) -> list[tuple[str, str]]:
    rules = [(str(REPO), "."), (str(REPO.parent / "ConcIR"), "./concir")]
    for word in private.get("identity", []):
        if word == "/Users/kevin":
            rules.append((word, "<home>"))
        else:
            rules.append((word, "<redacted>"))
    return rules


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _anonymize(text: str, rules) -> str:
    for old, new in rules:
        text = text.replace(old, new)
    return text


def _is_raw(path: Path) -> bool:
    if path.name in EXCLUDE_NAMES:
        return True
    if path.suffix == ".jsonl" and path.name.startswith(RAW_GLOBS):
        return True
    return bool(set(path.parts) & DENY_PARTS)


def collect() -> list[tuple[Path, str]]:
    files: list[tuple[Path, str]] = []
    for prefix, base in [("benchmarks", REPO / "benchmarks"), ("prompts", REPO / "prompts"),
                         ("python", REPO / "python"), ("scripts", REPO / "scripts"),
                         ("runtime", REPO / "runtime")]:
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file():
                continue
            rel = path.relative_to(base)
            if set(path.parts) & {"__pycache__", ".venv", "legacy", "legacy-cir2cvn"}:
                continue
            if str(rel).startswith("real-cases/results"):
                continue
            if path.name in EXCLUDE_NAMES:
                continue
            files.append((path, f"{prefix}/{rel}"))
    for name, arc in [("experiments/RESULTS.md", "experiments/RESULTS.md"),
                      ("Makefile", "Makefile"), ("pyproject.toml", "pyproject.toml"),
                      ("docs/PAPER_EVIDENCE_MAP.md", "docs/PAPER_EVIDENCE_MAP.md"),
                      ("experiments/FREEZE_MANIFEST.md", "experiments/FREEZE_MANIFEST.md"),
                      ("experiments/EVIDENCE_TIERS.md", "experiments/EVIDENCE_TIERS.md"),
                      ("experiments/RAW_POLICY.md", "experiments/RAW_POLICY.md"),
                      ("experiments/HUMAN_REVIEW_QUEUE_GEN.md",
                       "experiments/HUMAN_REVIEW_QUEUE_GEN.md")]:
        path = REPO / name
        if path.is_file():
            files.append((path, arc))
    for name in TIER_A:
        base = REPO / "experiments" / name
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            if path.is_file() and not _is_raw(path):
                files.append((path, f"experiments/{name}/{path.relative_to(base)}"))
    return files


def archive_concir(tmp: Path, tag: str) -> list[tuple[Path, str]]:
    concir = REPO.parent / "ConcIR"
    out = tmp / "concir.tar"
    try:
        proc = subprocess.run(["git", "-C", str(concir), "archive", f"--format=tar",
                               f"--output={out}", tag], capture_output=True, text=True)
        if proc.returncode != 0:
            raise RuntimeError(proc.stderr.strip())
        extract = tmp / "concir"
        extract.mkdir(parents=True, exist_ok=True)
        with tarfile.open(out) as tar:
            tar.extractall(extract)
    except Exception as exc:  # noqa: BLE001
        print(f"warning: ConcIR archive failed: {exc}", file=sys.stderr)
        return []
    return [(p, f"concir/{p.relative_to(extract)}") for p in extract.rglob("*") if p.is_file()]


def _scan_tree(root: Path, private: dict) -> list[str]:
    identity = [w for w in private.get("identity", []) if w]
    secrets = [s["pattern"] for s in private.get("secrets", [])]
    hits: list[str] = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        blob = path.read_bytes()
        text = blob.decode("utf-8", errors="ignore")
        for word in identity:
            if word.encode() in blob:
                hits.append(f"{path.relative_to(root)}: identity {word!r}")
        for pat in secrets:
            m = re.search(pat.encode(), blob)
            if m:
                hits.append(f"{path.relative_to(root)}: secret {pat!r}")
    return hits


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--concir-tag", default="concir-freeze-7")
    parser.add_argument("--max-mb", type=float, default=200.0)
    parser.add_argument("--skip-selfcheck", action="store_true")
    args = parser.parse_args()
    private = _load_private()
    rules = _rewrites(private)
    DIST.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        staged = tmp / "stage"
        staged.mkdir()
        manifest = ["# SUPPLEMENT_MANIFEST", "", f"Freeze tag: `{args.tag}`",
                    f"ConcIR tag: `{args.concir_tag}`", "",
                    "Text records were rewritten so that absolute checkout paths "
                    "become package-relative paths; no absolute user path or "
                    "author/account identifier remains. Raw trace/build files are "
                    "excluded (see `experiments/RAW_POLICY.md`).", "",
                    "| file | sha256 |", "| --- | --- |"]
        for src, arc in collect() + archive_concir(tmp, args.concir_tag):
            if not src.is_file():
                continue
            dest = staged / arc
            dest.parent.mkdir(parents=True, exist_ok=True)
            if src.suffix in TEXT_SUFFIX or src.name == "Makefile":
                dest.write_text(_anonymize(src.read_text(encoding="utf-8",
                                                          errors="replace"), rules),
                                encoding="utf-8")
            else:
                shutil.copyfile(src, dest)
            manifest.append(f"| `{arc}` | `{sha256(dest)}` |")
        (staged / "SUPPLEMENT_MANIFEST.md").write_text("\n".join(manifest) + "\n",
                                                       encoding="utf-8")

        hits = _scan_tree(staged, private)
        if hits:
            print("GATE FAILED (pre-zip):", file=sys.stderr)
            for h in hits[:30]:
                print("  ", h, file=sys.stderr)
            return 1
        total = sum(p.stat().st_size for p in staged.rglob("*") if p.is_file())
        if total > args.max_mb * 1024 * 1024:
            biggest = sorted((p for p in staged.rglob("*") if p.is_file()),
                             key=lambda p: p.stat().st_size, reverse=True)[:20]
            print(f"GATE FAILED: size {total/1e6:.1f} MB > {args.max_mb} MB", file=sys.stderr)
            for p in biggest:
                print(f"  {p.stat().st_size/1e6:8.2f} MB {p.relative_to(staged)}", file=sys.stderr)
            return 1

        zip_path = DIST / f"concplanverify-supplement-{args.tag}.zip"
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in sorted(staged.rglob("*")):
                if path.is_file():
                    zf.write(path, path.relative_to(staged))

        # post-unzip full-tree gate
        with tempfile.TemporaryDirectory() as checkdir:
            with zipfile.ZipFile(zip_path) as zf:
                zf.extractall(checkdir)
            post = _scan_tree(Path(checkdir), private)
            if post:
                zip_path.unlink(missing_ok=True)
                print("GATE FAILED (post-unzip):", file=sys.stderr)
                for h in post[:30]:
                    print("  ", h, file=sys.stderr)
                return 1

        (DIST / (zip_path.name + ".sha256")).write_text(
            f"{sha256(zip_path)}  {zip_path.name}\n", encoding="utf-8")
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
        import os
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
