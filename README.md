# ConcPlanVerify (v4)

End-to-end, traceable, bi-level verification framework for LLM-driven
construction and repair of concurrent Rust programs.

ConcPlanVerify separates **concurrency architecture** from **sequential
business logic**:

1. **Model-level verification (`backends/concir`, ConcIR v4):** Requirement
   clauses are turned into a Structured ConcIR (`SCIR`) model (`lock_guard`,
   `while_wait`, `seq_hole`, `req` tags), deterministically lowered to flat CIR
   with a `TraceMap`, and exhaustively model-checked against a frozen
   verification contract (Loop A).
2. **Code-level static & MIR refinement verification (`backends/spn` +
   `concir-backend verify-refinement`):** Verified CIR emits a MIR-clean Rust
   skeleton with typed `/* HOLE(id) expected: T */` placeholders. An LLM fills
   only the sequential holes, and pluggable static verifiers check **hole
   purity (`O1`)** and **branch-pruning stuttering refinement (`O2`)** over the
   compiler MIR Petri net exported by `spn`.
3. **Bi-level iterative repair & traceability:** Hole-attributable failures
   repair *only* the offending hole(s) (Loop B); architectural impossibilities
   or non-hole refinement failures escalate back to Stage 2 CIR revision
   (Loop C). Every run emits `traceability_manifest.json` linking
   `Clause ID -> Property ID -> Module::Function::SID -> Hole ID -> Rust File:LineRange -> Verdict`.

## End-to-End Bi-Level Pipeline

```
Requirements + Frozen Contract
          │
          ▼
┌────────────────────────────── Stage 1/2: Model Verification (Loop A) ──────────────────────────────┐
│  LLM generates SCIR v4 ──▶ concir-backend lower (TraceMap) ──▶ check / support / summarize         │
│         ▲                                                               │                          │
│         └─────────────── structured model-checking feedback ────────────┴──▶ explore (Petri net)   │
└─────────────────────────────────────────────────────────────────────────┬──────────────────────────┘
          ▲                                                               │ complete PASS
          │ Loop C: cross-layer escalation                                ▼
          │ (explicit escalate or exhausted Loop B)       concir-backend codegen (MIR-clean skeleton)
          │                                                               │
          │                                                               ▼
┌─────────┴──────────────────── Stage 3/4: Code Verification (Loop B) ───────────────────────────────┐
│  LLM fills seq_hole(s) ──▶ AstHolePurityPlugin (O1 purity + exact multi-line hole_ranges)          │
│         ▲                        │                                                                 │
│         │                        ▼                                                                 │
│         │                 SpnMirRefinementPlugin (spn MIR Petri net -> verify-refinement O2)       │
│         │                        │                                                                 │
│         └── hole-local repair ───┴──▶ [optional] dynamic conform + traceability_manifest.json      │
└────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Repository Layout

```
ConcPlanVerify/
├── pyproject.toml                  # PEP 621 package metadata (`concplan`, `src/` layout)
├── backends/
│   ├── concir/                     # git submodule: ConcIR v4 AST/SCIR, Petri verifier, codegen, summarize, verify-refinement
│   ├── spn/                        # Rust rustc_driver MIR Petri-net extractor (`spn --export-concir-net`)
│   └── third_party/                # Optional local static analyzers (e.g. `lockbud/`, `rudra/`, gitignored)
├── src/
│   └── concplan/
│       ├── __init__.py             # Top-level re-exports
│       ├── __main__.py             # Unified v4 CLI (`concplan` / `python -m concplan`)
│       ├── core/                   # toolchain, concir_client, models, llm, providers, prompts, env, json_utils
│       ├── model_verify/           # normalize, offline_workflow, revision_workflow, patch_repair
│       ├── code_verify/            # conformance, static_verify (plugins), bounded_monitor
│       ├── pipeline/               # e2e_pipeline (Loops A/B/C + traceability), benchmarks loader
│       └── prompts/                # Bundled v4 prompt assets loaded via `importlib.resources`
│           ├── scir_generation.md  # SCIR v4.0 generation prompt
│           ├── hole_repair.md      # Hole-filling, Loop B repair & Loop C escalation prompt
│           ├── cir_patch.md        # Constrained single-patch repair prompt
│           └── cir_local_revision.md # Function-scoped local revision prompt
├── tests/                          # Consolidated unittest suite (unit + integration + live SPN/ConcIR E2E)
├── benchmarks/
│   ├── families/                   # 6 concurrency families (lock-order, condvar, channel, semaphore, atomic-data, structure)
│   ├── real-cases/                 # Upstream real-world concurrency cases (`rmw-zenoh-998`, `dashmap-369`)
│   ├── FAMILIES.md
│   ├── MANIFEST.json
│   └── GENERATION_MANIFEST.json
└── docs/
    ├── ARCHITECTURE.md             # Layer-by-layer architecture & plugin extension guide
    └── REFINEMENT_SPEC.md          # Authoritative wire specification across ConcIR, SPN, and Python
```

## Build Prerequisites

1. **Python 3.10+** (standard library is sufficient for all offline/scripted
   workflows and tests; install the package in editable mode for the `concplan`
   console script and live LLM calls via `openai>=1.68,<3`):
   ```bash
   python -m pip install -e .
   ```
   Or run directly from source without installing by setting `PYTHONPATH=src`.
2. **Build `concir-backend` (submodule):**
   ```bash
   cargo build --release --manifest-path backends/concir/Cargo.toml --bin concir-backend
   ```
   Binary discovery checks `backends/concir/target/release/` and
   `backends/concir/target/debug/` automatically (or override via
   `CONCIR_BACKEND` / `--binary`).
3. **Build `spn` (MIR Petri-net analyzer, for `SpnMirRefinementPlugin`):**
   ```bash
   cargo build --release --manifest-path backends/spn/Cargo.toml
   ```
   Discovered automatically in `backends/spn/target/{release,debug}/` (or
   override via `CONCIR_SPN`).
4. **Optional `lockbud` (for `LockbudPlugin`):**
   Discovered automatically in `backends/third_party/lockbud/target/{release,debug}/`
   (or override via `CONCIR_LOCKBUD`).

## CLI Usage

When installed via `pip install -e .`, invoke `concplan ...` directly; when
running from source, prefix with `PYTHONPATH=src python -m concplan ...`.

### Discover Active Benchmarks

```bash
PYTHONPATH=src python -m concplan list-benchmarks
PYTHONPATH=src python -m concplan list-benchmarks --suite families
```

### Direct `concir-backend` Pass-Through Commands

```bash
PYTHONPATH=src python -m concplan check program.json
PYTHONPATH=src python -m concplan lower scir_program.json --output lowered.json --trace-map trace_map.json
PYTHONPATH=src python -m concplan summarize program.json --output summary.json
PYTHONPATH=src python -m concplan explore program.json contract.json --engine petri
PYTHONPATH=src python -m concplan verify-refinement \
  --spec program.json --impl-net impl_net.json \
  --contract contract.json --codegen-map codegen_filled.json
```

### End-to-End Bi-Level Pipeline (`e2e`)

Scripted offline run against a benchmark task:

```bash
PYTHONPATH=src python -m concplan --out out/e2e-demo e2e \
  --task lock-order/abba_2lock \
  --provider scripted --script responses.json \
  --run-summarize --report out/e2e_report.json
```

Live LLM run (reads API key from environment or local `.env`, e.g.
`DEEPSEEK_API_KEY` or `DASHSCOPE_API_KEY`):

```bash
PYTHONPATH=src python -m concplan --out out/e2e-live e2e \
  --task lock-order/abba_2lock \
  --provider deepseek --model deepseek-chat \
  --run-summarize --run-dynamic-conform --report out/e2e_live.json
```

Other orchestration commands: `offline`, `revision`, `patch-repair`, and
`conformance` (run `concplan <cmd> --help` or `python -m concplan <cmd> --help`
for flags).

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

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — Modular architecture and
  how to add custom `StaticVerifierPlugin`s or LLM providers.
- [`docs/REFINEMENT_SPEC.md`](docs/REFINEMENT_SPEC.md) — Wire schemas
  (`concir-trace-map/1`, `concir-module-summary/1`, `concir-impl-net/1`,
  `concir-refinement-report/1`) and exit-code contracts.
