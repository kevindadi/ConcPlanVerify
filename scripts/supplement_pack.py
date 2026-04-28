#!/usr/bin/env python3
from __future__ import annotations

import ast
import io
import re
import shutil
import tokenize
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BUILD_PARENT = REPO_ROOT / "_supplement_build"
OUT_DIR = BUILD_PARENT / "anonymous_supplement"

EXCLUDE_TOP_LEVEL = {
    ".git",
    "target",
    "node_modules",
    "paper",
    "cpn-gui",
    "_supplement_build",
    ".venv",
    "scripts",
    ".cursor",
    ".vscode",
}

EXCLUDE_SUFFIXES = (".pyc", ".pdf", ".DS_Store")


def ignore_supplement(dirpath: str, names: list[str]) -> set[str]:
    ignored: set[str] = set()
    for name in names:
        if name in EXCLUDE_TOP_LEVEL:
            ignored.add(name)
            continue
        if name.startswith(".env") and name != ".env.example":
            ignored.add(name)
            continue
        if name == "__pycache__":
            ignored.add(name)
            continue
        if name.endswith(EXCLUDE_SUFFIXES):
            ignored.add(name)
            continue
    return ignored


def strip_python(src: str) -> str:
    tree = ast.parse(src)

    class R(ast.NodeTransformer):
        def _drop_doc(self, body: list) -> list:
            if (
                body
                and isinstance(body[0], ast.Expr)
                and isinstance(body[0].value, ast.Constant)
                and isinstance(body[0].value.value, str)
            ):
                return body[1:]
            return body

        def visit_FunctionDef(self, node: ast.FunctionDef) -> ast.FunctionDef:
            self.generic_visit(node)
            node.body = self._drop_doc(node.body)
            return node

        def visit_AsyncFunctionDef(
            self, node: ast.AsyncFunctionDef
        ) -> ast.AsyncFunctionDef:
            self.generic_visit(node)
            node.body = self._drop_doc(node.body)
            return node

        def visit_ClassDef(self, node: ast.ClassDef) -> ast.ClassDef:
            self.generic_visit(node)
            node.body = self._drop_doc(node.body)
            return node

        def visit_Module(self, node: ast.Module) -> ast.Module:
            self.generic_visit(node)
            node.body = self._drop_doc(node.body)
            return node

    tree = R().visit(tree)
    ast.fix_missing_locations(tree)
    out = ast.unparse(tree)
    buf = io.StringIO(out)
    tokens: list[tokenize.TokenInfo] = []
    for t in tokenize.generate_tokens(buf.readline):
        if t.type != tokenize.COMMENT:
            tokens.append(t)
    return tokenize.untokenize(tokens)


def patch_workspace_cargo(toml_text: str) -> str:
    return re.sub(
        r"members\s*=\s*\[[^\]]*\]",
        'members = ["."]',
        toml_text,
        count=1,
    )


def anonymize_uni_llm_cargo(text: str) -> str:
    return re.sub(
        r"^repository\s*=\s*\"[^\"]*\"\s*\n",
        "",
        text,
        flags=re.MULTILINE,
    )


ANON_LICENSE = """MIT License

Copyright (c) 2026 Anonymous

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""

SUPPLEMENT_README = """# Supplemental code: reproduction

This archive contains the **cir2cvn** translator, CVN analysis backend, tests, and **experiment scripts** used in the paper. The GUI crate is omitted; behavior relevant to the evaluation is exercised via the CLI binary and Python driver.

## Prerequisites

- **Rust** (stable), **Cargo**
- **Python** 3.11+ (3.11 includes `tomllib`; on older versions install `tomli`)
- Network access to whichever LLM HTTP APIs you configure (see below)

## Configure APIs (`experiments/config.toml`)

Open **`experiments/config.toml`** and read the header comments. The runner uses an **OpenAI-compatible** Chat Completions endpoint: `POST {base_url}/chat/completions`.

- Replace placeholder `base_url` values (e.g. `REPLACE_WITH_YOUR_OPENAI_COMPAT_BASE`) with roots that your credentials can use for the listed `model_id` strings.
- **Do not** put API keys in the TOML file. Use environment variables named in each row’s `api_key_env` field.

## Environment variables

Export the keys your `[[models]]` rows expect, for example:

```bash
export ALL_API_KEY="..."
export DEEPSEEK_API_KEY="..."
export DASHSCOPE_API_KEY="..."
```

(Optional) A repo-root `.env` file is supported by the experiment script for the same keys if you prefer not to export manually in every shell.

## Python environment

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install -U pip
pip install requests
python3 -c "import tomllib" 2>/dev/null || pip install tomli
```

## Build and test (Rust)

```bash
cargo build --release
cargo test
```

The experiment script builds `target/release/cir2cvn` on demand if missing; a prior `cargo build --release` avoids the first-call compile delay.

## Run experiments

From the **repository root** of this archive:

```bash
python experiments/run_experiment.py --config experiments/config.toml --rq 3
python experiments/run_experiment.py --config experiments/config.toml --rq 4
python experiments/run_experiment.py --config experiments/config.toml
```

- `--rq 1` / `2` / `3` / `4` runs one research question; omit `--rq` to run all that the script schedules.

**RQ1 (CIR generation from Rust source)** expects the paths listed under `[source_programs]` in `experiments/config.toml`. If those `.rs` files are not present in your copy, RQ1 will skip or fail for missing inputs; add the sources or trim that section if you only reproduce RQ2–RQ4.

**RQ2 partial reruns / resume:** optional helper:

```bash
python experiments/rq2_run_missing.py
python experiments/rq2_run_missing.py --model gpt-5
```

See that script’s `--help` for `--partial`, `--intermediate`, and `--out-stem`.
"""


def main() -> None:
    if BUILD_PARENT.exists():
        shutil.rmtree(BUILD_PARENT)
    BUILD_PARENT.mkdir(parents=True)

    shutil.copytree(
        REPO_ROOT,
        OUT_DIR,
        ignore=ignore_supplement,
    )

    cargo_toml = OUT_DIR / "Cargo.toml"
    cargo_toml.write_text(patch_workspace_cargo(cargo_toml.read_text()))

    uni = OUT_DIR / "uni-llm" / "Cargo.toml"
    if uni.exists():
        uni.write_text(anonymize_uni_llm_cargo(uni.read_text()))

    lic = OUT_DIR / "LICENSE"
    if lic.exists():
        lic.write_text(ANON_LICENSE)

    for rel in (
        Path("experiments/run_experiment.py"),
        Path("experiments/rq2_run_missing.py"),
    ):
        p = OUT_DIR / rel
        if p.exists():
            p.write_text(strip_python(p.read_text()) + "\n")

    (OUT_DIR / "README.md").write_text(SUPPLEMENT_README)

    doc_dir = OUT_DIR / "doc"
    if doc_dir.exists():
        shutil.rmtree(doc_dir)

    uni_toml = OUT_DIR / "uni-llm" / "uni-llm.toml"
    if uni_toml.exists():
        uni_toml.unlink()

    zip_path = REPO_ROOT / "supplement_anonymous.zip"
    if zip_path.exists():
        zip_path.unlink()

    with zipfile.ZipFile(
        zip_path,
        "w",
        zipfile.ZIP_DEFLATED,
    ) as zf:
        for f in sorted(OUT_DIR.rglob("*")):
            if f.is_file():
                arc = f.relative_to(OUT_DIR)
                zf.write(f, arc.as_posix())

    print(f"Wrote {zip_path} ({zip_path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
