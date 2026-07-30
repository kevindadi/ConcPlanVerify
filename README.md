# cir2cvn

Translator from **CIR** (Concurrency Intermediate Representation) to **CVN** (Concurrency Verification Net), with a Python CLI for LLM-driven CIR generation and repair.

## Overview

This crate implements a faithful 1:1 translation from CIR programs into CVN Petri nets
suitable for state-space exploration and deadlock/livelock detection. It is the bridge
between LLM-generated CIR and the CVN analysis back-end (which performs model checking).

```text
User Requirements --(Python + DeepSeek/Qwen)--> CIR JSON
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

The default workflow is Python. Python owns LLM interaction, prompts, JSON extraction, and the generation/repair loop. Rust remains the source of truth for CIR schema validation, CIR-to-CVN translation, state-space analysis, and goal reachability. The boundary is JSON over stdin/stdout through the `cir2cvn` binary.

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

Validate and analyze CIR without an LLM:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow validate tests/fixtures/canonical_schema.json
PYTHONPATH=python python/.venv/bin/python -m cir_workflow analyze tests/e2e/mutex_deadlock/buggy.json
PYTHONPATH=python python/.venv/bin/python -m cir_workflow goals tests/fixtures/unmet_goal.json
```

Generate CIR with DeepSeek. The default model is `deepseek-v4-pro`, the default key is `DEEPSEEK_API_KEY`, and the default request enables high reasoning effort and thinking:

```bash
PYTHONPATH=python python/.venv/bin/python -m cir_workflow generate \
  --provider deepseek \
  --requirements "Model a producer and consumer sharing a bounded channel."
```

Generate or repair CIR with Qwen. Qwen uses `responses.create(model=..., input=...)`. Supply the workspace endpoint when it is required by the DashScope account:

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
    ├── expr_parser.rs       # CIR string → CVN expression
    ├── resource.rs          # Phase 1: resource scanning
    ├── control_flow.rs      # Transfer planning
    ├── operation.rs         # Phase 2: operation translation
    ├── condvar.rs           # Condvar specialization
    └── fn_summary.rs        # FnSummary indexing
python/
├── cir_workflow/            # LLM orchestration and Rust subprocess client
│   └── prompt_assets/       # Generation and repair prompts for the LLM
├── tests/                   # Offline Python workflow tests
└── pyproject.toml
```

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

- **cir** (`cir`) — CIR library (vendored in-repo under `cir/`)
- **cvn** — CVN library with `cir-anchor` feature (vendored in-repo under `cvn/`)
- **openai** — official Python SDK used by the DeepSeek and Qwen clients
