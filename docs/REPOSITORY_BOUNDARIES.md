# Three-repository boundaries (rewritten 2026-09-18)

This replaces the earlier paper-side `REPOSITORY_BOUNDARIES.md`, which was moved
here during the 2026-09-18 migration. The three repositories have distinct,
non-overlapping roles.

## 1. `/Users/kevin/local-repos/ConcIR` — Rust tool code only

Owns the CIR definition and the tools that operate on it: static validation,
supportability, Petri/interpreter exploration and verification, structured
diagnostics, deterministic patch/repair, and repair artifacts with replay. It
exposes these through the `concir` and `concir-backend` CLIs and keeps its core
regression suite.

It does **not** contain experiment data, experiment scripts, or prompts. After
the 2026-09-18 migration the local `experiments/` tree and the pilot scripts are
gone; the `experiments/` ignore entry was removed.

Layout: `src/`, `tests/`, `doc/`, `examples/`, `Cargo.toml`, `Cargo.lock`,
`README.md`, `.cursor/`.

## 2. `/Users/kevin/local-repos/ConcPlanVerify` — experiment repository

Owns everything around the experiments:

- `python/cir_workflow/` — LLM-facing orchestration only. It calls the Rust CLI
  as a subprocess and never re-implements CIR semantics, verification, candidate
  enumeration, patch legality or repair acceptance.
- `prompts/` — versioned prompt assets (generation, feedback, patch).
- `benchmarks/` — frozen task inputs: `patterns/P1..P9` (to be authored),
  `real-cases/` (from `ConcIR/experiments/real-cases-v0`), and
  `legacy-cir2cvn/` (the retired `cir2cvn` benchmark, including the 20 Rust
  reference programs under `benchmarks/rust/`).
- `experiments/` — all raw experiment batches (paper history plus the ConcIR
  pilot batches), directory names unchanged.
- `reviews/` — backend review records (formerly `backend-review/`).
- `scripts/` — frozen experiment scripts (`run_pilot.py`, `pilot_analyze.py`,
  `pilot_audit.py`, `pilot_checks.py`), which are experiment tooling rather than
  Rust tool code.
- `docs/` — this document, the migration record, and `docs/prompts/`.

The exact source -> destination mapping and per-file sha256 are recorded in
[`MIGRATION_2026-09-18.md`](MIGRATION_2026-09-18.md).

## 3. `/Users/kevin/paper-review/papers/ConcPlanVerify` — paper sources only

Holds the manuscript and its build inputs: `paper.tex`, `checklist.tex`,
`refs.bib`, `neurips_2026.sty`, `figures/`, `graphviz.svg`, and LaTeX build
products (ignored by the paper repository's `.gitignore`). It no longer holds
`experiments/`, `backend-review/`, or `REPOSITORY_BOUNDARIES.md`.

The paper title remains ConcPlanVerify; the split between the two code
repositories does not change the paper's thesis.

## Real-model constraints (first round, unchanged)

Use only the DeepSeek API key in `/Users/kevin/local-repos/ConcPlanVerify/.env`;
use DeepSeek Flash only; Pro, aliases and automatic fallback to another model or
provider are forbidden. The key is for runtime authentication only: it is never
printed, committed, or written into experiment artifacts. As of 2026-09-18 the
Flash request identifier is `deepseek-flash`; every experiment must record both
the requested name and the model identity returned by the service.
