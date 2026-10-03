# ConcPlanVerify: anonymous source artifact

This repository contains the Python orchestration code, a local copy of the
ConcIR Rust CLI, benchmark inputs, and scripts for fresh runs. **Historical
experimental outputs are not included.** Reviewers provide their own API keys.

## Contents

| Directory | Contents |
|---|---|
| `ConcIR/` | CIR parsing, validation, Petri-net analysis, repair, instrumentation, code generation, binding and trace conformance |
| `python/cir_workflow/` | LLM orchestration, feedback, requirement scoring and audit logging |
| `prompts/` | Prompt templates used by the workflow |
| `runtime/concir_sync/` | Local Rust synchronization support for generated implementations |
| `benchmarks/` | Requirements, contracts, reference CIRs, faulty/correct Rust inputs and task labels |
| `configs/` | Model IDs, task selection, arm budgets and repetition settings |
| `scripts/` and `reproduce.py` | Fresh generation, repair, scoring, mutation and offline analysis entry points |

The generation benchmark has **24 tasks: 8 Simple, 8 Medium and 8 Complex**.
`benchmarks/GENERATION_MANIFEST.json` defines this selection. The broader
`benchmarks/MANIFEST.json` also includes detector fixtures, boundary cases and
legacy/real-case inputs; those are not additional generation cells. Reference
programs and expected labels are benchmark inputs, not recorded LLM outputs.

## Installation

Use Linux or macOS, Python 3.10 or newer, Git, and Rustup. The toolchain is pinned
in `rust-toolchain.toml`. Rust candidates are built and executed locally.

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -e ./python
rustup toolchain install nightly-2026-09-04 --component miri --component rust-src
python reproduce.py build
cargo +nightly-2026-09-04 miri setup
```

The Python package only requires the OpenAI-compatible SDK for the three shipped
model configurations. Cursor SDK is not required. ConcIR is compiled from the
**included `ConcIR/` sources** into `.build/backend/debug/`; no external ConcIR
checkout, prebuilt author binary, or author filesystem path is required.
`build --offline` works after Cargo dependencies have been downloaded.

For the G2 baseline and the Rust repair baseline, install
[Lockbud](https://github.com/BurtonQin/lockbud) at the pinned revision:

```bash
git clone https://github.com/BurtonQin/lockbud.git tools/lockbud
git -C tools/lockbud checkout cc78cb72cb85cb80e596717339cca95ef50c8fe0
rustup toolchain install nightly-2026-02-07 --component rust-src --component rustc-dev --component llvm-tools-preview
cargo +nightly-2026-02-07 build --release --manifest-path tools/lockbud/Cargo.toml
export LOCKBUD_BIN="$PWD/tools/lockbud/target/release/lockbud"
```

The wrapper refuses to start a G2 generation batch when Lockbud or Miri is
missing. A missing detector or a detector execution error is not a clean result.
Third-party tools remain under their upstream licenses and are downloaded
separately; their Git repositories and binaries are not part of this artifact.

## Offline entry points

These commands make **no LLM requests**. Each run creates its own output directory;
choose a new directory when repeating an offline command.

```bash
# Check input/source hashes (no compilation).
python reproduce.py verify

# Show cells and request caps without spending API requests.
python reproduce.py plan --config configs/gpt_glm_all_arms.json

# A small model-to-code example, including four native trace replays.
python reproduce.py smoke --out results/smoke

# Run both model engines over the supplied detector fixtures.
python reproduce.py benchmark --out results/benchmark

# Generated lock-chain models with varying threads, locks and analysis bounds.
python reproduce.py scale --out results/scale
```

The model checker distinguishes `PASS`, `FAIL`, `UNKNOWN`, invalid input and
unsupported operations. `UNKNOWN` indicates an analysis bound and must not be
reported as verification success. Scaling measures the abstract concurrency
model; it does not measure source-code line count or arbitrary computation.

Source tests are also included. The optional command
`python reproduce.py test --offline` runs the portable Python test selection and
Rust tests. It does not request LLM responses. Legacy tests that depend on
historical runs are outside `configs/portable_tests.json`.

## Fresh LLM generation

Copy `.env.example` to `.env` and fill in `DEEPSEEK_API_KEY` and/or
`OPENCODE_API_KEY`. Keys are read at runtime and are not distributed. Commands
below send real requests and may incur charges.

| Configuration | Systems | Tasks per system | Repetitions | Arms | Total arm cells |
|---|---|---:|---:|---|---:|
| `deepseek_all_arms.json` | DeepSeek Flash, direct API | 24 | 3 | G0–G3 | 288 |
| `gpt_glm_all_arms.json` | GPT 6 Luna and GLM 5.3 Flash, OpenCode | 24 | 1 | G0–G3 | 192 |
| `gpt_glm_g3.json` | GPT 6 Luna and GLM 5.3 Flash, OpenCode | 24 | 1 | G3 | 48 |

An arm cell is one `(model, task, arm, repetition)` run. Thus DeepSeek has
72 cells **per arm**, while GPT and GLM have 24 cells per arm each.

- **G0:** direct Rust generation.
- **G1:** generation followed by model self-review.
- **G2:** generation and iteration with build, Miri and Lockbud feedback.
- **G3:** CIR generation and validation, requirement-contract checking, then Rust
  generation with the verified CIR and code-stage feedback.

```bash
python reproduce.py generate --config configs/deepseek_all_arms.json --out results/deepseek
python reproduce.py generate --config configs/gpt_glm_all_arms.json --out results/gpt-glm
# Alternative focused run; do not count it again in the all-arm comparison.
python reproduce.py generate --config configs/gpt_glm_g3.json --out results/gpt-glm-g3
```

Generation uses concurrency **3 per launched batch**; feedback within a cell is
serial. Launch batches sequentially to maintain total concurrency 3. Completed
generation cells are cached. Repeating the same command with the same output,
configuration and toolchain resumes it; workflow mismatches are refused.
Preflight verifies the returned model identity. Unavailable models remain
blocked; no alternate model is silently substituted.

DeepSeek is configured as `deepseek-flash`, with thinking disabled, four CIR
rounds and three code rounds for G3. GPT/GLM enable thinking, with three CIR and
three code rounds for G3. Their `max_output_tokens: null` omits an explicit output
token ceiling; providers may impose limits. G0 uses one round. Exact send
parameters and physical request caps are recorded in each JSON configuration.
The caps are upper bounds, including transport retries; they are not token
budgets and are not expected consumption.

## Requirement scoring and accounting

Use the same independent bounded Rust oracle for every arm:

```bash
python reproduce.py score --batch results/deepseek --out results/deepseek-scored
python reproduce.py score --batch results/gpt-glm --out results/gpt-glm-scored
```

`REPORT.json` contains per-cell, per-requirement and tier-level results. The
scorer instruments the final emitted Rust candidate, binds its resources to the
reference CIR, and checks the frozen contract over 32 native executions by
default. `--native-runs` can change this value and should be reported explicitly.
This is a bounded execution verdict; it does not replace exhaustive CIR model
checking or prove equivalence between a source Petri net and CVN.

Method acceptance, requirement fulfillment, missing candidates, incomplete
matrices and unavailable oracle scores are reported separately. `RF_all`
includes zero contribution from no emitted candidate or a build failure;
unknown oracle scores make that aggregate `null`. The explicitly labelled
`RF_all_known_lower_bound` is conservative and is not an exact fulfillment
estimate. Only compare completed matrices under declared protocols.

Per-call `audit.jsonl` stores requested/returned model IDs, stage, round, retries,
raw prompts/responses, and provider-reported token usage. The scorer preserves
each step and reports known token sums with the number of missing fields.
Missing usage remains `null`. Reasoning tokens may already be included in output
tokens and must not be added twice. Preflight usage remains in the separate
preflight/transport logs, rather than being attributed to a task cell.

## Repair and controlled mutations

The fresh repair runner uses the ten tasks in `configs/repair.json`, three
repetitions and six arms: direct Rust, self-review, tool iteration, local CIR
revision, whole CIR revision, and tiered local-then-whole CIR revision. Its
round count denotes LLM proposals; inspection of the supplied initial faulty
program does not consume a proposal. Tiered repair shares the four-proposal
limit across two local and two whole proposals. This runner is serial and uses
one persistent request budget. It does not resume a partial repair batch.

```bash
# Paid DeepSeek Flash calls; use a fresh output directory.
python reproduce.py repair --config configs/repair.json --out results/repair

# Offline mutation of a fresh mechanically generated implementation.
python reproduce.py mutation --program benchmarks/families/lock-order/abba_2lock/fixed.cir.json --out results/mutation
```

Mutations are applied only when the relevant operation exists. Operators cover
lock-order swaps, removed unlocks, notification changes, channel operation
moves, synchronization removal, spawn order and trace-label corruption. The
unmodified implementation is checked alongside mutants. Build failures,
timeouts, missing traces and conformance violations remain distinct; a mutated
trace label is instrumentation corruption, not an independently established
concurrency defect. `--miri` adds Miri runs. To examine newly generated CIRs,
pass their CIR paths rather than the reference path.

## Scope and reproducibility

No paper files, historical experiment directories, generated candidates,
review records, raw response logs, binaries, credentials or original Git
history are distributed in this snapshot. Expected unit-test fixtures and
benchmark ground truth are retained. `SOURCE_SHA256.json` identifies the shipped
files. The ConcIR build revision string is anonymized; source hashes identify
the exact artifact. Packaging changes relocate dependency paths, provide fresh
entry points and preserve full per-cell records; they do not tune model verdicts.

These scripts perform **fresh re-executions**, not replay of the undistributed
historical responses. Model sampling, provider availability, model revisions,
toolchain/platform behavior and the declared round settings can change outcomes.
All shipped generation configurations use this one source snapshot; they do
not recreate historical runs that used earlier backend revisions. The fresh
repair entry point states its proposal-count convention explicitly. No new
paid LLM experiments were run while preparing this package.

All runtime output belongs under `results/`; `.build/`, `.env`, third-party
`tools/`, caches and experimental outputs are Git-ignored. For review, expose
only the `artifact-anonymous` branch through an anonymous repository service;
do not publish the development branch or its original Git history.
