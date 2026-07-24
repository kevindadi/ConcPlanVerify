# cir2cvn

Translator from **CIR** (Concurrency Intermediate Representation) to **CVN** (Concurrency Verification Net).

## Overview

This crate implements a faithful 1:1 translation from CIR programs into CVN Petri nets
suitable for state-space exploration and deadlock/livelock detection. It is the bridge
between LLM-generated CIR and the CVN analysis back-end (which performs model checking).

```
User Requirements ──(LLM)──▶ CIR ──(cir2cvn)──▶ CVN ──(analysis)──▶ Counterexample
```

## Usage

```rust
use cir2cvn::translate;

let program: cir::ast::Program = serde_json::from_str(&json)?;
let net = translate(&program)?;

// Use the CVN for analysis
let state = net.initial_state();
let enabled = net.enabled_transitions(&state);
```

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
```

## Building

```bash
cargo build
cargo test
```

With LLM helpers (NL→CIR generation, repair loop):

```bash
cargo build --features llm
cargo test -p cir2cvn --features llm
```

## Dependencies

- **cir** (`ceir`) — CIR library (vendored in-repo under `cir/`)
- **cvn** — CVN library with `cir-anchor` feature (vendored in-repo under `cvn/`)
- **uni-llm** — optional LLM client for the `llm` feature (vendored in-repo under `uni-llm/`)
