# AGENTS.md — ConcPlanVerify Project & Agent Development Guide

This guide helps human developers and AI coding agents understand **ConcPlanVerify (v5)**, navigate the codebase, and develop verified concurrent Rust programs using either the **MCP server (`concplan-mcp`)** or the **CLI (`concplan`)**.

---

## 1. Core Paradigm (v5)

ConcPlanVerify strictly separates **specification formalization**, **concurrency architecture**, and **pure sequential / domain logic**:

1. **Stage 0 — Dual-Entry Autoformalization (`concplan.formalize`)**
   - **Entry A (Raw Natural Language):** Decomposes free-form text into grounded entities (`roles`, `resources`, `domain_types`) and atomic clauses `R1..Rn` (`safety | liveness | reachability | sequential_hole` `[U]`), then synthesizes `contract.json` in an **isolated prompt** (never seeing any candidate SCIR/CIR model to prevent specification-model collusion).
   - **Entry B (Structured Requirements):** Validates and explains pre-authored `REQUIREMENTS.md` / `requirements.json` and optional `contract.json`.
   - **Anti-Hallucination Guardrails:** Entity grounding checks, automatic `"non_vacuous": true` on CTL/LTL implications (`concir-backend validate-contract` Rust SSoT), and deterministic NL back-translation (`explain_contract_data` / `concir-backend explain-contract`).
2. **Stage 1 / 2 — Concurrency Model Verification (`SCIR` + `UniPN`, Loop A)**
   - You author concurrency control flow in **Structured ConcIR (`SCIR`)** (`lock_guard`, `rwlock_read_guard`, `rwlock_write_guard`, `if_else`, `while_loop`, `while_wait`, `seq_hole`, `TypeHole`, `Mutex`, `RwLock`, `Condvar`, `Semaphore`, `Channel`, `Atomic`, `Var`, multi-module `provides`/`requires`, and `req` traceability tags).
   - `concir-backend lower` lowers `structured_body` to flat CIR + `concir-trace-map/1`.
   - `concir-backend check`, `support`, `summarize` (cross-module lock-order cycles), and `explore` (backed by `unipn::analysis::temporal` CTL/LTL + Justice/Compassion fairness + non-vacuity) exhaustively verify the model.
3. **Stage 3 / 4 — Skeleton Codegen, Pure Hole Filling & MIR Refinement (Loops B & C)**
   - `concir-backend codegen` emits a MIR-clean multi-file Rust Cargo project:
     - `src/main.rs` (entry module + `Shared` + `__res` + `main`)
     - `src/<mod>.rs` (each non-entry module)
     - `src/domain.rs` (concrete `Struct`/`Array` defs + `/* TYPE_HOLE(th*) type: Name */` placeholders with `Send + Sync + Clone + Default + 'static` bounds)
     - `src/cir_trace.rs` (feature-gated `cir_trace` runtime + `SyncVar`, `Semaphore`, `Channel`)
   - You fill **only** `SeqHole` (`h*`) and `TypeHole` (`th*`) slots with pure Rust.
   - `AstHolePurityPlugin` checks skeleton integrity + comment/string-safe `O1` purity across all `src/*.rs`.
   - `SpnMirRefinementPlugin` runs `cargo check --offline --no-default-features` under `RUSTC_WRAPPER=spn` (whole-program or per-module via `--focus-module <m>`) and checks **branch-pruning stuttering refinement (`O2`)** via `concir-backend verify-refinement`.
   - Hole-attributable failures repair only the offending `h*`/`th*` IDs (**Loop B**); architectural impossibilities escalate back to Stage 2 SCIR revision (**Loop C**).

---

## 2. Repository Map

```text
ConcPlanVerify/
├── AGENTS.md                        # This developer & agent guide
├── README.md                        # Overview, prerequisites, CLI & MCP quickstart
├── pyproject.toml                   # PEP 621 package (`concplan` + `concplan-mcp` scripts)
├── docs/
│   ├── ARCHITECTURE.md              # Layer-by-layer v5 architecture & extension points
│   └── REFINEMENT_SPEC.md           # Authoritative JSON wire schemas & CLI exit codes
├── backends/
│   ├── unipn/                       # Unified Petri-net core (`analysis/temporal/{ast,graph,vacuity,explain,ctl,ltl,gba}`, `refinement`, `obligations`, `unipn-transfer`)
│   ├── concir/                      # git submodule: 7-crate Cargo workspace (`crates/concir-{ir,sem,codegen,engine,verify,repair,cli}`) + root `concir` facade & `concir-backend`
│   ├── spn/                         # git submodule: rustc_driver MIR Petri-net extractor (`spn`)
│   └── third_party/                 # Optional gitignored analyzers (e.g. `lockbud`)
├── src/concplan/
│   ├── __main__.py                  # Thin CLI entrypoint -> `concplan.cli`
│   ├── cli/                         # Modular CLI parser & command dispatch (`parser.py`, `commands.py`)
│   ├── formalize/                   # Stage 0 autoformalization (`models.py`, `markdown.py`, `backend_contract.py` Rust SSoT, `workflow.py`)
│   ├── core/                        # Toolchain discovery, `client/{protocol,runner,client}.py`, LLM providers, bundled prompts
│   ├── model_verify/                # CIR normalization (`normalize.py`), shared `runner.py`, and revision loop (`revision_workflow.py`)
│   ├── code_verify/                 # Modular verification (`rust_lexer`, `holes`, `verifier`, `ast_purity`, `spn_mir`, `lockbud`, `dynamic`) + thin facades
│   ├── pipeline/                    # Bi-level E2E pipeline (`manifest.py`, `fill_prompts.py`, `e2e.py`, `examples.py`)
│   ├── prompts/                     # Bundled v5 system prompts (`nl_to_clauses.md`, `clauses_to_contract.md`, `scir_generation.md`, `hole_repair.md`, ...)
│   └── mcp/                         # Zero-dependency stdio MCP server (`workspace.py`, `handlers_model.py`, `handlers_code.py`, `tools.py`, `protocol.py`, `server.py`)
├── examples/                        # Stage 0-driven natural language concurrency examples
└── tests/                           # Consolidated unittest suite (offline + live SPN/ConcIR E2E + MCP tests)
```

---

## 3. Concurrent Rust Development Workflow (MCP & CLI)

When helping a user design, verify, and implement concurrent Rust code:

1. **Check toolchain readiness:** Call `concplan_toolchain_status` (MCP) to confirm `concir-backend` and `spn` binaries are built.
2. **Ground & validate the contract (Stage 0):**
   - Decompose natural-language requirements into grounded `roles`, `resources`, `domain_types`, and `R1..Rn` clauses (`[V]` concurrency vs. `[U]` `sequential_hole`).
   - Synthesize `contract.json` using CTL/LTL (`non_vacuous: true` on implications) and `preserved` completion goals, then call `concplan_validate_contract` to verify entity grounding, vacuity rules, and NL back-translation.
3. **Design & verify the SCIR concurrency model (Stage 1/2, Loop A):**
   - Model only threads, locks (`Mutex`/`RwLock`), condvars, semaphores, channels, atomics, and control-branching `Var`s; push business data into `types` (`TypeHole` / `Struct` / `Array`) and pure logic into `seq_hole`.
   - Validate via `concplan_lower_scir` -> `concplan_check_model` -> `concplan_summarize_modules` (multi-module) -> `concplan_explore_model` (`engine="petri"`).
   - If `explore` fails or flags `vacuous: true`, revise the SCIR model (or run `concplan_repair_model` for lock-order fixes) until `outcome == "PASS"` and `complete == true`.
4. **Generate the Rust skeleton & fill holes (Stage 3):**
   - Call `concplan_codegen_skeleton` to generate `Cargo.toml`, `src/main.rs`, `src/<mod>.rs`, `src/domain.rs`, and `codegen.json`.
   - Write pure Rust for each `TypeHole` (`th*` in `src/domain.rs`) and `SeqHole` (`h*` in `src/main.rs` / `src/<mod>.rs`), then apply via `concplan_fill_holes`.
5. **Verify code purity & MIR refinement (Stage 4, Loops B & C):**
   - Call `concplan_verify_code` (runs `AstHolePurityPlugin` + `SpnMirRefinementPlugin`, optionally with `modular=True` or `run_dynamic_conform=True`).
   - On `fail_hole` / `fail_refinement` attributed to `offending_holes`, fix **only** the offending `h*`/`th*` fills via `concplan_fill_holes` (Loop B).
   - If a requirement genuinely needs new synchronization, shared state, or different lock ordering, **never** sneak sync primitives into a hole—escalate back to Step 3 and update the SCIR model (Loop C).

---

## 4. Hard Invariants & Common Pitfalls

- **Hole Purity (`O1`) — Zero Sync/Thread/Unsafe/Unbounded Loops in Holes:**
  - `SeqHole` (`h*`): no `std::sync`, `Mutex`, `RwLock`, `Condvar`, `Semaphore`, `mpsc`, `channel`, `Atomic*`, `Ordering::*`, `std::thread`, `scope(`, `unsafe`, `cir_trace`, `use`, `// @cir`, or `loop`/`while`/`spin_loop`.
  - `TypeHole` (`th*` in `src/domain.rs`): must define `pub struct <Name>` implementing `Send + Sync + Clone + Default + 'static`; pure `struct`/`enum`/`impl` + bounded `for` loops (or explicitly `@bound`-annotated loops) are allowed, while sync primitives, threads, atomics, `unsafe`, `cir_trace`, and unbounded loops are forbidden.
  - Never modify skeleton code or `// @cir <sid>` comments outside hole/type-hole markers.
- **`ComplexBaseType::Struct` Wire Format:**
  - A `Struct` in `types` or `base` is a JSON object mapping field names directly to types:
    `{"Struct": {"seq": "Int", "valid": "Bool"}}` — **not** `{"Struct": {"fields": [...]}}`.
- **Never Downgrade `unknown` to `pass`:**
  - Missing `spn` binary, missing `lockbud`, or `truncated: true` state graphs must always remain `status == "unknown"` and must never be reported as verified.
- **Submodule Build & Pinning Rules:**
  - When building or testing `backends/spn`, always change directory into `backends/spn` first (`Set-Location backends\spn` / `cd backends/spn`) so Cargo picks up `backends/spn/rust-toolchain.toml` (`rustc-dev` / `rustc_driver`).
  - `backends/concir/Cargo.toml` and `backends/spn/Cargo.toml` pin `unipn` to a Git revision on `github.com/kevindadi/UniPN`; never commit local `path = "../unipn"` overrides.
- **MCP Stdio Transport:**
  - `concplan-mcp` / `python -m concplan.mcp` speaks JSON-RPC 2.0 over `stdin`/`stdout`. Never print debug output to `stdout` in MCP server code; always use `sys.stderr`.

---

## 5. Build, Test, CLI & MCP Quick Reference

### Build Rust Backends

```powershell
cargo test --manifest-path backends/unipn/Cargo.toml --workspace
cargo test --manifest-path backends/concir/Cargo.toml
cargo build --release --manifest-path backends/concir/Cargo.toml --bin concir-backend
Push-Location backends/spn; cargo test --lib; cargo build --release; Pop-Location
```

### Run Python Tests

```powershell
# Windows PowerShell
.\.venv\Scripts\python.exe -m unittest discover -s tests -v

# Linux / macOS
PYTHONPATH=src:. python3 -m unittest discover -s tests -v
```

### Install & Run CLI / MCP Server

```powershell
.\.venv\Scripts\python.exe -m pip install -e .

# Inspect registered MCP tools
.\.venv\Scripts\python.exe -m concplan.mcp --list-tools

# Start MCP stdio server
concplan-mcp
# or: python -m concplan.mcp
# or: python -m concplan mcp
```

Example `mcp_config.json` entry for MCP clients (Claude Code / Cursor / Antigravity / IDE):

```json
{
  "mcpServers": {
    "concplan": {
      "command": "E:/local-repos/ConcPlanVerify/.venv/Scripts/python.exe",
      "args": ["-m", "concplan.mcp"],
      "cwd": "E:/local-repos/ConcPlanVerify"
    }
  }
}
```

---

## 6. Authoritative Documentation & Prompt Assets

- Architecture overview: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Wire schemas, CLI flags & exit codes: [`docs/REFINEMENT_SPEC.md`](docs/REFINEMENT_SPEC.md)
- Bundled prompts: [`src/concplan/prompts/nl_to_clauses.md`](src/concplan/prompts/nl_to_clauses.md), [`src/concplan/prompts/clauses_to_contract.md`](src/concplan/prompts/clauses_to_contract.md), [`src/concplan/prompts/scir_generation.md`](src/concplan/prompts/scir_generation.md), [`src/concplan/prompts/hole_repair.md`](src/concplan/prompts/hole_repair.md)
