# cir2cvn

Translator from **ConcIR** (Concurrency Intermediate Representation) to **CVN** (Concurrency Verification Net), with a Python CLI for LLM-driven ConcIR generation and repair.

## Overview

This crate implements a faithful 1:1 translation from ConcIR programs into CVN Petri nets
suitable for state-space exploration and deadlock/livelock detection. It is the bridge
between LLM-generated ConcIR and the CVN analysis back-end (which performs model checking).

```text
User Requirements --(Python + DeepSeek/Qwen)--> ConcIR JSON
                                              |
                              (Rust cir2cvn subprocess)
                                              v
                              validation / CVN / analysis
                                              |
                              structured feedback to Python
```

## Rust API

```rust
use cir2cvn::translate;

let program: cir::ast::Program = serde_json::from_str(&json)?;
let net = translate(&program)?;

// Use the CVN for analysis
let state = net.initial_state();
let enabled = net.enabled_transitions(&state);
```

## CLI Workflow

The default workflow is Python. Python owns LLM interaction, prompts, JSON extraction, and the generation/repair loop. Rust remains the source of truth for ConcIR schema validation, ConcIR-to-CVN translation, state-space analysis, and goal reachability. The boundary is JSON over stdin/stdout through the `cir2cvn` binary.

Install the Python dependencies from the repository root:

```bash
python3 -m venv python/.venv
python/.venv/bin/python -m pip install -r python/requirements.txt
```

The commands below run directly from the checkout, so an editable package
installation is not required. If you prefer the `cir-workflow` console script,
install the package with `python/.venv/bin/python -m pip install -e python`.

Put the API keys in the root `.env` file. The file is ignored by git and is never printed by the CLI:

```dotenv
DEEPSEEK_API_KEY=...
DASHSCOPE_API_KEY=...
```

Build the Rust verifier, or let the Python CLI build it when the release binary is missing:

```bash
cargo build --release --bin cir2cvn
```

Validate and analyze ConcIR without an LLM:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow validate tests/fixtures/canonical_schema.json
PYTHONPATH=python python/.venv/bin/python -m cir_workflow analyze tests/e2e/mutex_deadlock/buggy.json
PYTHONPATH=python python/.venv/bin/python -m cir_workflow goals tests/fixtures/unmet_goal.json
```

Generate ConcIR with DeepSeek. The default model is `deepseek-v4-pro`, the default key is `DEEPSEEK_API_KEY`, and the default request enables high reasoning effort and thinking:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow generate \
  --provider deepseek \
  --requirements "Model a producer and consumer sharing a bounded channel."
```

Generate or repair ConcIR with Qwen. Qwen uses `responses.create(model=..., input=...)`. Supply the workspace endpoint when it is required by the DashScope account:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow generate \
  --provider qwen \
  --base-url "https://{WorkspaceId}.cn-beijing.maas.aliyuncs.com/compatible-mode/v1" \
  --requirements "Model two workers that update a protected shared variable."

PYTHONPATH=python python/.venv/bin/python -m cir_workflow repair \
  --provider qwen \
  --base-url "https://{WorkspaceId}.cn-beijing.maas.aliyuncs.com/compatible-mode/v1" \
  tests/e2e/mutex_deadlock/buggy.json
```

## Modular generation (plan + merge)

Large projects can be generated as several independently produced ConcIR
fragments and merged into one Program before translation. Modularity is a
generation strategy only — the CVN is always built from the single merged
Program. The LLM planner decides when modular generation is warranted:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow plan \
  --provider deepseek \
  --requirements "Model a server with a request queue, worker pool, and shared registry."
```

A `merge` bundle lists the entry module and one `{module, concir}` entry per
fragment. Merge enforces global invariants (unique function names and goal ids,
consistently declared shared resources, exactly one entry owner) and tags each
function with its source `module`:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow merge modules/bundle.json \
  | python/.venv/bin/python -m cir_workflow analyze -
```

Cross-module `call`/`spawn` targets resolve during merge; shared resources are
deduplicated by name and consistency-checked.

Provider defaults can be overridden with `--model-id`, `--api-key-env`, `--base-url`, `--reasoning-effort`, `--thinking`, and `--no-thinking`. `--binary` selects a specific `cir2cvn` executable.

## Project Structure

```
src/
├── lib.rs                   # Public API
├── error.rs                 # TranslateError (T0xx–T3xx)
├── validate.rs              # Post-translation checks
└── translator/
    ├── mod.rs               # Three-phase orchestration
    ├── context.rs           # Translation context
    ├── expr_parser.rs       # ConcIR string → CVN expression
    ├── resource.rs          # Phase 1: resource scanning
    ├── control_flow.rs      # Transfer planning
    ├── operation.rs         # Phase 2: operation translation (incl. call expansion)
    ├── condvar.rs           # Condvar specialization
python/
├── cir_workflow/            # LLM orchestration and Rust subprocess client
│   ├── plan.py              # LLM modularity planning (plan command)
│   ├── merge.py             # Modular fragment → single Program assembly
│   ├── prompt_assets/       # Generation and repair prompts for the LLM
├── tests/                   # Offline Python workflow tests
└── pyproject.toml
```

## Experiment report

A Vite + React report lives under [`canvases/`](canvases/) and can be committed to git:

```bash
cd canvases
npm install
npm run dev
```

See [`canvases/README.md`](canvases/README.md).

## Rust Building and Tests

```bash
cargo build
cargo test
```

Run the Python offline tests with:

```bash
PYTHONPATH=python python/.venv/bin/python -m unittest discover -s python/tests
```

## Dependencies

- **cir** (`cir`) — ConcIR library (vendored in-repo under `cir/`)
- **cvn** — CVN library with `cir-anchor` feature (vendored in-repo under `cvn/`)
- **openai** — official Python SDK used by the DeepSeek and Qwen clients
