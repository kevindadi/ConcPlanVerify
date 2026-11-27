# ConcPlanVerify (v5)

End-to-end, traceable, bi-level verification framework for LLM-driven
construction and repair of concurrent Rust programs—from raw natural language
to formally verified, MIR-checked Rust code.

ConcPlanVerify cleanly separates **specification formalization**, **concurrency
architecture**, and **pure sequential/domain logic**:

1. **Stage 0 Autoformalization (`concplan.formalize`):** Dual-entry pipeline
   accepting either free-form natural language (`formalize_from_nl`) or
   pre-authored `R1..Rn` specs (`formalize_from_structured`). Extracts grounded
   entities (`roles`, `resources`, `domain_types`), classifies clauses into
   verifiable concurrency properties vs. `[U]` `sequential_hole` logic, synthesizes
   CTL/LTL + classic contracts in an **isolated** prompt (preventing
   specification-model collusion), and runs deterministic NL back-translation +
   sanity/non-vacuity validation.
2. **Model-level verification (`backends/concir` + `backends/unipn`):**
   Structured ConcIR (`SCIR`) models (`lock_guard`, `while_wait`, `seq_hole`,
   `TypeHole`, `RwLock`, multi-module `provides`/`requires`, `req` tags) are
   deterministically lowered to flat CIR with a `TraceMap` and exhaustively
   model-checked against CTL/LTL + classic contracts with **non-vacuity
   enforcement** (Loop A).
3. **Code-level static & modular MIR refinement verification (`backends/spn` +
   `concir-backend verify-refinement`):** Verified CIR emits a MIR-clean
   multi-file Rust skeleton (`src/main.rs`, `src/domain.rs` with `TypeHole`s,
   `src/<mod>.rs`). An LLM fills only `SeqHole`s and `TypeHole`s, and pluggable
   static verifiers check **multi-file purity (`O1`)** and **branch-pruning
   stuttering refinement (`O2`)**—whole-program or per-module via
   `--modular-refinement` (`spn --focus-module <m>` +
   `verify-refinement --module <m>`).
4. **Bi-level iterative repair & traceability:** Hole- or TypeHole-attributable
   failures repair *only* the offending `h*`/`th*` IDs (Loop B); architectural
   impossibilities escalate back to Stage 2 CIR revision (Loop C). Every run
   emits `traceability_manifest.json` linking
   `Clause ID -> Contract Property (with NL explanation & vacuity status) -> Module::Function::SID -> SeqHole/TypeHole -> Rust File:LineRange -> Verdict`.

## End-to-End Pipeline (Stage 0 + Loops A / B / C)

```
Free-Form NL (Entry A) or Pre-Authored R1..Rn (Entry B)
          │
          ▼
┌────────────────────────────── Stage 0: Autoformalization ──────────────────────────────────────────┐
│  Step 0.1 nl_to_clauses (entities + R1..Rn [V]/[U]) ──▶ Step 0.2 isolated clauses_to_contract       │
│  ──▶ Step 0.3 deterministic NL back-translation + sanity & non-vacuity check ──▶ Frozen Contract   │
└──────────────────────────────────────────────────────────┬─────────────────────────────────────────┘
          │                                                ▼
┌─────────┼──────────────────── Stage 1/2: Model Verification (Loop A) ──────────────────────────────┐
│  LLM generates SCIR v5 (lock_guard, RwLock, TypeHole, seq_hole, multi-module)                      │
│  ──▶ concir-backend lower (TraceMap) ──▶ check / support / summarize ──▶ explore (UniPN CTL/LTL)   │
│         ▲                                                                     │                    │
│         └──────────── counterexample + vacuity feedback ──────────────────────┘                    │
└───────────────────────────────────────────────────────────────────────────────┬────────────────────┘
          ▲                                                                     │ complete PASS
          │ Loop C: cross-layer escalation                                      ▼
          │ (explicit escalate or exhausted Loop B)        concir-backend codegen (src/main.rs,
          │                                                src/domain.rs TypeHoles, src/<mod>.rs)
          │                                                                     │
          │                                                                     ▼
┌─────────┴──────────────────── Stage 3/4: Code Verification (Loop B) ───────────────────────────────┐
│  LLM fills SeqHole(s) & TypeHole(s) ──▶ AstHolePurityPlugin (O1 purity across all src/*.rs)        │
│         ▲                                        │                                                 │
│         │                                        ▼                                                 │
│         │            SpnMirRefinementPlugin (whole-program or --modular-refinement per module)     │
│         │                                        │                                                 │
│         └── offending h*/th*-only repair ────────┴──▶ [optional] dynamic conform + manifest.json   │
└────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Repository Layout

```
ConcPlanVerify/
├── AGENTS.md                       # Developer & agent workflow guide, invariants, and MCP/CLI quickstart
├── pyproject.toml                  # PEP 621 package metadata (`concplan`, `concplan-mcp`, `src/` layout)
├── backends/
│   ├── unipn/                      # Unified Petri Net core (`temporal` CTL/LTL, `refinement`, `unipn-transfer`)
│   ├── concir/                     # git submodule: ConcIR v5 AST/SCIR, UniPN CTL/LTL verifier, TypeHole/RwLock codegen, modular verify-refinement
│   ├── spn/                        # Rust rustc_driver MIR Petri-net extractor (`spn --export-concir-net --focus-module`)
│   └── third_party/                # Optional local static analyzers (e.g. `lockbud/`, `rudra/`, gitignored)
├── src/
│   └── concplan/
│       ├── __init__.py             # Top-level re-exports
│       ├── __main__.py             # Unified v5 CLI (`concplan` / `python -m concplan`)
│       ├── formalize/              # Stage 0 dual-entry autoformalization, NL back-translation, sanity checks
│       ├── core/                   # toolchain, concir_client, models, llm, providers, prompts, env, json_utils
│       ├── model_verify/           # normalize, offline_workflow, revision_workflow, patch_repair
│       ├── code_verify/            # conformance, static_verify (SeqHole/TypeHole purity + modular SPN), bounded_monitor
│       ├── pipeline/               # e2e_pipeline (Stage 0 + Loops A/B/C + traceability), examples loader
│       ├── mcp/                    # Zero-dependency stdio MCP server (`concplan-mcp` / `python -m concplan.mcp`)
│       └── prompts/                # Bundled v5 prompt assets loaded via `importlib.resources`
│           ├── nl_to_clauses.md        # Stage 0.1 NL -> grounded entities + R1..Rn clauses
│           ├── clauses_to_contract.md  # Stage 0.2 isolated clauses -> CTL/LTL contract
│           ├── scir_generation.md      # SCIR v5.0 generation (TypeHole, RwLock, multi-module)
│           ├── hole_repair.md          # SeqHole & TypeHole filling, Loop B repair & Loop C escalation
│           ├── cir_patch.md            # Constrained single-patch repair prompt
│           └── cir_local_revision.md   # Function-scoped local revision prompt
├── tests/                          # Consolidated unittest suite (Stage 0 + unit + integration + live SPN/ConcIR E2E + MCP)
├── examples/                       # Stage 0-driven natural-language examples (`spec.md` -> Stage 0 -> SCIR -> MIR refinement)
└── docs/
    ├── ARCHITECTURE.md             # Layer-by-layer v5 architecture & extension guide
    └── REFINEMENT_SPEC.md          # Authoritative wire specification across UniPN, ConcIR, SPN, and Python
```

## Build Prerequisites

1. **Python 3.10+** (standard library is sufficient for all offline/scripted
   workflows and tests; install in editable mode for `concplan` and live LLM calls):
   ```bash
   python -m pip install -e .
   ```
2. **Build `concir-backend` (submodule):**
   ```bash
   cargo build --release --manifest-path backends/concir/Cargo.toml --bin concir-backend
   ```
3. **Build `spn` (MIR Petri-net analyzer, for `SpnMirRefinementPlugin`):**
   ```bash
   cargo build --release --manifest-path backends/spn/Cargo.toml
   ```
4. **Optional `lockbud` (for `LockbudPlugin`):**
   Discovered automatically in `backends/third_party/lockbud/target/{release,debug}/`
   (or override via `CONCIR_LOCKBUD`).

## CLI Usage

When installed via `pip install -e .`, invoke `concplan ...` directly; when
running from source, prefix with `PYTHONPATH=src python -m concplan ...`.

### Stage 0 Autoformalization & Contract Inspection

```bash
# List Stage 0 examples:
PYTHONPATH=src python -m concplan list-examples

# Entry A: raw natural language from an example -> REQUIREMENTS.md + contract.json + NL explanation
PYTHONPATH=src python -m concplan --out out/stage0 formalize \
  --example visit_counter --from-nl \
  --provider deepseek --model deepseek-chat

# Entry B: validate & explain an existing example's structured spec + contract
PYTHONPATH=src python -m concplan --out out/stage0 formalize \
  --example visit_counter

# Deterministic NL back-translation of any contract.json
PYTHONPATH=src python -m concplan explain-contract contract.json

# Export a CIR/SCIR program to UniPN CVN JSON
PYTHONPATH=src python -m concplan export-cvn program.json --output program.cvn.json
```

### Direct `concir-backend` Pass-Through Commands

```bash
PYTHONPATH=src python -m concplan check program.json
PYTHONPATH=src python -m concplan lower scir_program.json --output lowered.json --trace-map trace_map.json
PYTHONPATH=src python -m concplan summarize program.json --output summary.json
PYTHONPATH=src python -m concplan explore program.json contract.json --engine petri
PYTHONPATH=src python -m concplan verify-refinement \
  --spec program.json --impl-net impl_net.json \
  --contract contract.json --codegen-map codegen_filled.json --module worker
```

### End-to-End Bi-Level Pipeline (`e2e`)

```bash
# From free-form NL with Stage 0 + modular per-module MIR refinement:
PYTHONPATH=src python -m concplan --out out/e2e-v5 e2e \
  --nl-file spec.txt \
  --provider deepseek --model deepseek-chat \
  --run-summarize --modular-refinement --report out/e2e_v5.json

# Against an example (Entry A from spec.md):
PYTHONPATH=src python -m concplan --out out/e2e-demo e2e \
  --example config_cache --from-nl \
  --provider deepseek --model deepseek-chat \
  --run-summarize --report out/e2e_report.json
```

### MCP Server (`concplan-mcp`)

ConcPlanVerify ships a zero-dependency stdio MCP server exposing 12 formal concurrency development tools (`concplan_validate_contract`, `concplan_lower_scir`, `concplan_check_model`, `concplan_summarize_modules`, `concplan_explore_model`, `concplan_repair_model`, `concplan_export_cvn`, `concplan_codegen_skeleton`, `concplan_fill_holes`, `concplan_verify_code`, `concplan_list_examples`, `concplan_toolchain_status`), documentation/prompt resources, and workflow prompts:

```bash
# Inspect registered MCP tools, resources, and prompts
PYTHONPATH=src python -m concplan.mcp --list-tools

# Start the stdio MCP server (or use `concplan-mcp` after `pip install -e .`, or `python -m concplan mcp`)
PYTHONPATH=src python -m concplan.mcp
```

Example `mcp_config.json`:

```json
{
  "mcpServers": {
    "concplan": {
      "command": "python",
      "args": ["-m", "concplan.mcp"],
      "cwd": "/path/to/ConcPlanVerify"
    }
  }
}
```

## Running Tests

Linux / macOS:

```bash
PYTHONPATH=src:. python3 -m unittest discover -s tests -t . -v
```

Windows PowerShell:

```powershell
$env:PYTHONPATH="src;."
python -m unittest discover -s tests -t . -v
```

Unit tests run completely offline; integration and live end-to-end tests
(`test_concir_integration.py`, `test_bounded_monitor.py`,
`test_model_workflows.py`, `test_live_spn_concir_e2e.py`) automatically exercise
the built `concir-backend` and `spn` binaries when present.

## Further Reading

- [`AGENTS.md`](AGENTS.md) — ConcPlanVerify developer & agent workflow guide, invariants, and MCP/CLI quickstart.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — Modular v5 architecture and
  how to add custom `StaticVerifierPlugin`s or LLM providers.
- [`docs/REFINEMENT_SPEC.md`](docs/REFINEMENT_SPEC.md) — Wire schemas
  (`concir-requirements/2`, `concir-contract-explanation/1`,
  `concir-trace-map/1`, `concir-module-summary/1`, `concir-impl-net/1`,
  `concir-refinement-report/1`, `concir-traceability-manifest/1`) and exit codes.
