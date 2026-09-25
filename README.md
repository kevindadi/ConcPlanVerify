# ConcPlanVerify

Python orchestration for LLM-driven construction and repair of concurrent
programs. It turns clause-numbered requirement documents into a
verification-oriented concurrency model (ConcIR), drives a verify–revise loop
against a frozen contract, then has the model implement the verified design in
Rust and post-verifies the result with instrumentation, conformance checking,
and a bounded trace monitor.

All formal work is delegated to the Rust **`concir-backend`** CLI from the
companion `ConcIR` repository: CIR validation, supportability, Petri-net
exploration/verification, structured diagnostics, deterministic repair, code
instrumentation, conformance, and monitoring. This package never re-implements
CIR semantics, Petri translation, verification, candidate enumeration, patch
legality, or repair acceptance.

## Pipeline

```
requirements ──▶ CIR generation ──▶ check / support ──▶ explore vs contract ──▶ revise
   (clause-          (LLM)              (Rust)          (exhaustive, Rust)     (local /
    numbered,                                                                    whole /
    contract                                                                     tiered)
    hidden)
                                                            │ complete PASS
                                                            ▼
                       accepted CIR ──▶ LLM writes Rust ──▶ instrument + conform ──▶ bounded monitor
```

- The contract is supplied by the caller, frozen, and **hidden from the
  generation model**; generation and revision are driven by the requirement
  document and contract-derived diagnostics only.
- The CIR is decided *exhaustively* by model checking; the generated Rust is
  checked by operation-bound conformance and a *bounded* monitor. The two words
  are never used interchangeably.
- `UNKNOWN` / `UNSUPPORTED` / budget-exhausted is reported as such, never as a
  verified success.

## Repositories

- **This repository** — LLM-facing orchestration, versioned prompts, benchmarks,
  experiment batches, evidence tiers, and the table/results generators.
- **`ConcIR`** — Rust tool code only: CIR definition and the tools that operate
  on it, exposed through the `concir` and `concir-backend` CLIs.

## Layout

```
python/cir_workflow/
├── concir_client.py      # typed subprocess client for concir-backend
├── providers.py          # CandidateProvider protocol + ScriptedProvider (+ LLM adapter)
├── offline_workflow.py   # generation -> check -> feedback -> verify -> tool repair
├── generation.py         # generation benchmark arms and task loading
├── revision_workflow.py  # local / whole / tiered CIR revision
├── patch_repair.py       # constrained single-patch repair loop
├── instrument.py         # concir-instrument driver (wrapper types, spawn events)
├── conformance.py        # operation-bound conform driver
├── bounded_monitor.py    # bounded trace monitor over observed runs
├── rust_arm.py           # direct Rust arms (G0/G1/G2) + tool baselines
├── rust_oracle.py        # reference-program oracle
├── detection.py, scale.py, structural.py, contract_strength.py
├── experiments_v2.py     # experiment batch orchestration
├── gen_results.py        # RESULTS.md and tables/*.tex generation
├── results.py, arms.py, models.py, llm.py, env.py, json_utils.py
└── __main__.py           # CLI entry point
prompts/                  # versioned modular-CIR generation / feedback / revision / Rust prompts
benchmarks/               # families (generation tasks), real-cases, legacy reference programs
experiments/              # raw experiment batches, tables, freeze manifest, evidence tiers
runtime/concir_sync/      # provided std-only sync crate used by generated Rust
scripts/                  # frozen experiment runners, renderers, and verifiers
docs/                     # repository boundaries, migration record, prompt docs
```

## Prerequisites

- A built ConcIR CLI:
  `cargo build --release --bin concir-backend` in the `ConcIR` repository.
- Python 3.10+. The workflow has no required third-party dependency; `openai`
  is only needed for live LLM providers.
- Point the client at the binary with `--binary` or the `CONCIR_BACKEND`
  environment variable.

## CLI usage

Thin wrappers over the backend (JSON on stdout; the process exit code mirrors
the backend's semantic exit):

```bash
export CONCIR_BACKEND=/path/to/ConcIR/target/release/concir-backend
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out check   program.json
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out support program.json
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out explore program.json contract.json petri
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out repair  program.json contract.json --strategy c
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-out replay  artifact.json
```

Offline workflow with the scripted provider (no API key, no network):

```bash
PYTHONPATH=python python3 -m cir_workflow --out /tmp/cir-offline offline \
  --requirements "Two tasks take the same two locks in opposite order." \
  --contract contract.json --provider scripted --script responses.json
```

`responses.json` is a list of `{"text": "<CIR JSON>"}` (or `{"error": "..."}`)
turns replayed in order.

Regenerate the result tables from committed artifacts:

```bash
PYTHONPATH=python python3 -m cir_workflow results \
  --batch experiments/flash-repair-main-v1/run-... \
  --gen experiments/flash-gen-main-v5-code/run-... \
  --genprobe experiments/gen-model-probe-v3-code \
  --latex experiments/tables --output experiments/RESULTS.md
```

## Workflow guarantees

- The contract is frozen and caller-supplied; the provider never writes or
  edits it.
- Generation candidates are retried only on parse/`check` failure, within a
  bounded number of rounds. Once a candidate passes static checking the initial
  CIR is frozen.
- After freezing, the only repair path is the backend's deterministic strategy
  search under the frozen contract and its `allowed_scope`; the provider cannot
  replace the whole program to manufacture a pass.
- A tool repair is labelled `tool`, never `LLM`.
- Every complete repair artifact is replayed by the backend before it is
  treated as evidence.

## Prompt assets

`prompts/` holds the versioned prompt templates used by the workflow: CIR
generation and revision, structured feedback, single-patch repair, direct Rust
generation and self-review, and Rust-to-CIR extraction. Prompts are data, not
code; they are loaded by `python/cir_workflow/prompts.py`.

## Benchmarks

`benchmarks/families/` contains the generation tasks (requirement documents with
an Entities section, frozen contracts with requirement-tagged clauses, and
reference material), grouped by concurrency family. `benchmarks/real-cases/`
holds larger reference programs, and `benchmarks/legacy-cir2cvn/` keeps the
retired translator benchmark for historical comparison.

## Experiments and evidence

`experiments/` holds raw batches, `tables/`, `RESULTS.md`, `PROVENANCE.md`, the
freeze manifest, and the evidence-tier policy:

- `experiments/EVIDENCE_TIERS.md` — which batches feed the current results.
- `experiments/RAW_POLICY.md` — which raw artifacts live on disk/freeze tags
  only and are not tracked.
- `experiments/FREEZE_MANIFEST.md` — frozen batch hashes and tags.

## Tests

```bash
PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .
```

Unit tests exercise the protocol mapping with a controllable stub; integration
tests (`test_concir_integration.py`, `test_offline_workflow.py`) call the real
`concir-backend` binary and skip with a clear message when it is absent.

## Model and provider constraints

Live LLM commands use only the DeepSeek API key from a local `.env` and are
pinned to `deepseek` / `deepseek-flash` with `thinking` disabled, a persisted
request budget, and no Pro/alias/automatic-fallback. The key is for runtime
authentication only and is never printed, committed, or written into
experiment artifacts. Offline and generation commands remain keyless and
network-free.
