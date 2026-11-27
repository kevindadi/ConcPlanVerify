# ConcPlanVerify v5 Architecture

ConcPlanVerify is organized into clearly separated formal backends (`backends/`,
Rust) unified onto **`UniPN`** (`backends/unipn/`) and a PEP 621 `src/`-layout
Python orchestration package (`src/concplan/`). All formal semantics—lowering,
validation, Petri-net exploration, CTL/LTL temporal model checking with
non-vacuity detection, compositional lock-order summaries, deterministic repair,
MIR-clean multi-file codegen (`SeqHole` + `TypeHole` + `RwLock`), and stuttering
refinement—live in Rust; Python owns Stage 0 autoformalization, LLM prompting,
workflow control loops (Loops A/B/C), static verifier composition, and
traceability reporting.

See [`docs/REFINEMENT_SPEC.md`](REFINEMENT_SPEC.md) for exact JSON schemas and
CLI exit codes.

---

## 1. Formal Backends (Rust — `backends/`)

### 1.1 `backends/unipn/` (Unified Petri Net & Formal Analysis Core)

Shared formal foundation used by both `concir` and `spn`:

- **`unipn::analysis::temporal`:** Generic finite-state CTL (15 operators) and
  LTL (12 operators + `Justice`/`Compassion` fairness) model checkers with
  implicit stutter self-loops on sink states, lasso counterexample extraction,
  deterministic natural-language back-translation (`explain_ctl`, `explain_ltl`),
  and **non-vacuity detection** (`vacuous`, `vacuous_antecedents`) for universal
  implications.
- **`unipn::analysis::refinement`:** Shared Branch-Pruning Stuttering Refinement
  and lock-divergence checker over labeled transition graphs.
- **`unipn-transfer`:** Bidirectional ConcIR v4/v5 $\leftrightarrow$ `UniPN` CVN
  converter supporting `seq_hole`, `req`, and `types` (`TypeHole`).

### 1.2 `backends/concir/` (git submodule — `concir-backend`)

The `concir-backend` CLI implements the ConcIR v5 formal engine:

- **Structured Surface IR (`SCIR` v4/v5, `concir-backend lower`):** Lowers
  structured control-flow constructs (`lock_guard`, `rwlock_read_guard`,
  `rwlock_write_guard`, `if_else`, `while_loop`, `while_wait`, `stmt`) into
  flat CIR statements with deterministic `s1, s2, ...` `sid`s and LIFO guard
  unwinding on early `return`, emitting `concir-trace-map/1` preserving
  `structured_id`, `hole_id`, and `req` traceability tags.
- **First-class `seq_hole` & Domain `TypeHole` (`types`):**
  - `seq_hole`: pure sequential computation with optional `updates_modeled` and
    `expected_type`, executed as an atomic silent step (`tau`).
  - `TypeDef` / `{"Hole": ...}`, `Struct`, `Array`, and `RwLock` (with optional
    payload `base` type on `Mutex` and `RwLock`).
- **Model Verification (`check`, `support`, `explore`, `summarize`, `explain-contract`, `export-cvn`):**
  - Exhaustive Petri-net / interpreter exploration evaluating classic properties
    AND **CTL/LTL properties with non-vacuity enforcement** (marking vacuous
    implications `FAIL` when `non_vacuous: true`).
  - `explain-contract`: deterministic JSON back-translation
    (`concir-contract-explanation/1`).
  - `export-cvn`: auto-lowers SCIR and exports a `UniPN` CVN net + degradation
    report.
  - `summarize`: compositional module summaries (`concir-module-summary/1`) with
    cross-module lock-cycle detection.
- **MIR-Clean Multi-File `codegen`:**
  - Feature-gated runtime tracing (`cir_trace`), real `AtomicI64`/`AtomicBool`,
    lock-free `SyncVar<T>` for `Var`, and payload-typed `Mutex<Payload>` /
    `Arc<std::sync::RwLock<Payload>>` (keeping payload-typed guard bindings
    `_g_*`, `_rg_*`, `_wg_*` under `not(feature = "cir_trace")` alongside OS
    sync guards so `SeqHole` payload method/field calls compile identically).
  - Emits `src/domain.rs` with `/* TYPE_HOLE(th1) type: Name */` placeholders
    and compile-time `Send + Sync + Clone + Default + 'static` assertions.
  - Multi-module programs emit non-entry modules to `src/<mod>.rs`, recording
    `file` on `holes` and `type_holes` in `codegen.json`.
- **Whole-Program & Modular Stuttering Refinement (`verify-refinement [--module <M>]`):**
  Delegates core weak simulation & lock-divergence checking to
  `unipn::analysis::refinement`, supports `RwLock` ops (`rwlock_read_lock`,
  `rwlock_read_unlock`, `rwlock_write_lock`, `rwlock_write_unlock`), and
  supports `--module <M>` symmetric module-ownership + module-origin projection
  combined with lock-order cycle freedom.

### 1.3 `backends/spn/` (`spn`)

A `rustc_driver`-based MIR analyzer built on `UniPN` `main` and invoked via
`RUSTC_WRAPPER=spn` with `--no-default-features`:

- `SPN_FLAGS="--export-concir-net <impl_net.json> --codegen-map <codegen_filled.json> [--focus-module <M>]"`
  exports `concir-impl-net/1` including `RwLock` transitions.
- Attributes synchronization/thread/atomic/unsafe/loop MIR violations inside
  BOTH `hole_ranges` (`src/main.rs`, `src/<mod>.rs`) AND `type_hole_ranges`
  (`src/domain.rs`) directly to the offending `hole_id` / `th*` ID.
- `--focus-module <M>` projects external transitions to `tau` for fast
  per-module MIR refinement.

### 1.4 `backends/third_party/` (Optional Local Analyzers)

Optional gitignored external Rust static analyzers (such as `lockbud` and
`rudra`). When `backends/third_party/lockbud/target/{release,debug}/lockbud` is
built (or `CONCIR_LOCKBUD` is set), `LockbudPlugin` can be composed into
`CompositeStaticVerifier`.

---

## 2. Python Orchestration Layer (`src/concplan/`)

### 2.1 `concplan.formalize` (Stage 0 Autoformalization)

Dual-entry formalization layer preventing specification-model collusion:

- **Entry A (`formalize_from_nl`):**
  - Step 0.1 (`nl_to_clauses`): raw natural language $\to$ grounded `entities`
    (`roles`, `resources`, `domain_types`) + atomic clauses `R1..Rn` classified
    as `safety | liveness | reachability | sequential_hole` (`verifiable: true`
    vs `[U]` `verifiable: false`) + canonical `REQUIREMENTS.md`.
  - Step 0.2 (`clauses_to_contract`): **strictly isolated** prompt (never seeing
    any candidate SCIR/CIR program!) synthesizing `contract.json` with classic +
    CTL/LTL formulas (`non_vacuous: true` enforced on implications).
- **Entry B (`formalize_from_structured`):** Accepts existing `REQUIREMENTS.md`
  or `requirements.json` (plus optional `contract.json`), synthesizing a
  contract via Step 0.2 when omitted.
- **Step 0.3 (`explain_contract_data`, `validate_contract_sanity`):**
  Pure-Python recursive deterministic NL renderer (plus optional
  `ConcirClient.explain_contract` pass-through) and sanity checks for unique IDs,
  clause existence & verifiability, entity grounding, and `non_vacuous: true`.

### 2.2 `concplan.core`

Low-level infrastructure shared across all workflows:

- `toolchain.py` — Backend-first, Windows `.exe`-safe binary discovery.
- `concir_client.py` — Typed subprocess wrapper (`ConcirClient`, `ConcirResult`,
  `ConcirIdentity`) for `check`, `support`, `explore`, `lower`, `summarize`,
  `verify_refinement(module=...)`, `explain_contract`, `export_cvn`, `repair`,
  `replay`, `repair_context`, and `evaluate_patch`.
- `models.py`, `llm.py`, `providers.py`, `env.py`, `json_utils.py`.
- `prompts.py` — `importlib.resources`-backed loader and SHA-256 registry for
  the bundled prompt assets (including Stage 0 `nl_to_clauses.md` and
  `clauses_to_contract.md`), plus vacuity-aware `build_explore_feedback`.

### 2.3 `concplan.model_verify`

Model-level verification, normalization, and repair loops (`normalize.py`,
`offline_workflow.py`, `revision_workflow.py`, `patch_repair.py`).

### 2.4 `concplan.code_verify`

Post-codegen static and dynamic Rust verification:

- `conformance.py` — `codegen`, `holes_of`, `type_holes_of`, `fill_holes`,
  `lint_filled`, `collect_traces` (native & Miri), and `conform_all`.
- `static_verify.py` — Pluggable static verification architecture:
  - `fill_holes_marked`, `fill_type_holes_marked`, and multi-file
    `augment_codegen_map_with_hole_ranges` (tracking both `hole_ranges` across
    `src/main.rs` / `src/<mod>.rs` and `type_hole_ranges` in `src/domain.rs`).
  - `AstHolePurityPlugin`: multi-file skeleton integrity (`src/*.rs`) and
    comment/string-safe `O1` purity for both `SeqHole`s and `TypeHole`s
    (allowing pure `struct`/`enum`/`impl` + bounded loops in `TypeHole`s while
    forbidding `Mutex`, `RwLock`, `Condvar`, `Semaphore`, `mpsc`, `Atomic*`,
    `thread::spawn`, `unsafe`, `cir_trace`, and unbounded `loop`/`while`).
  - `SpnMirRefinementPlugin`: supports both whole-program mode and modular mode
    (`modular=True` / `focus_modules=[...]`, combining `concir-backend summarize`
    with per-module `spn --focus-module <m>` + `verify-refinement --module <m>`).
  - `LockbudPlugin`: optional static deadlock detector.
- `bounded_monitor.py` — Wraps `concir-backend monitor`.

### 2.5 `concplan.pipeline`

- `e2e_pipeline.py` — `run_e2e_pipeline`: integrates Stage 0 (optional
  `nl_spec` or auto-formalization when `contract` is omitted) $\to$ Stage 1/2
  (Loop A with vacuity feedback) $\to$ Stage 3 (`codegen` + `SeqHole` &
  `TypeHole` filling) $\to$ Stage 4 (`CompositeStaticVerifier` with optional
  `modular_refinement=True` + Loop B offending-hole/type-hole repair + Loop C
  cross-layer escalation + optional dynamic `conform`). Emits upgraded
  `traceability_manifest.json` (`formalization`, `nl_explanations`,
  `vacuity_status`, `type_holes`, `modular_reports`, plus all v4 keys).
- `examples.py` — Stage 0-first example discovery and loaders.

---

## 3. Bundled Prompts (`src/concplan/prompts/`)

1. `nl_to_clauses.md` — Stage 0.1 atomic clause decomposition & entity/domain-type grounding.
2. `clauses_to_contract.md` — Stage 0.2 isolated CTL/LTL contract synthesis.
3. `scir_generation.md` — SCIR v5.0 generation (`lock_guard`, `while_wait`, `seq_hole`, `TypeHole`, `RwLock`, multi-module, `req`).
4. `hole_repair.md` — Pure `SeqHole` & `TypeHole` filling, Loop B repair, and Loop C escalation.
5. `cir_patch.md` — Constrained single-patch repair candidate format.
6. `cir_local_revision.md` — Function-scoped local CIR revision format.

---

## 4. Extending ConcPlanVerify

### 4.1 Plugging in a New `StaticVerifierPlugin`

Implement the `StaticVerifierPlugin` protocol from
`concplan.code_verify.static_verify` and compose it in `CompositeStaticVerifier`
(which threads `codegen_filled.json` across plugins and never downgrades
`unknown`/`fail_*` to `pass`). Any violation carrying a `hole_id` (`h*` or `th*`)
or `rust_span` inside `hole_ranges`/`type_hole_ranges` is routed to **Loop B**;
non-hole violations escalate to **Loop C**.

### 4.2 Plugging in a New LLM Provider

1. Implement the `LlmClient` protocol in `src/concplan/core/llm.py` and register
   its provider key in `create_llm_client` and `default_base_url`.
2. Wrap it with `LlmCandidateProvider(client, model_config, system_prompt=..., user_prompt_builder=...)`.
3. Pass distinct providers for `clause_provider`, `contract_provider`,
   `cir_provider`, and `hole_provider` to `run_e2e_pipeline(...)`.
