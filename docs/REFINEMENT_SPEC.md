# ConcIR v4 + PetriNet (SPN) + ConcPlanVerify Authoritative Wire Specification

This document is the single source of truth for interfaces across:
1. **`backends/concir/`** (Rust submodule — `concir` library & `concir-backend` CLI)
2. **`backends/spn/`** (Rust `rustc_driver` MIR analyzer `spn`)
3. **`src/concplan/`** (Python orchestration, static post-verification, and bi-level iteration)

All three components MUST adhere strictly to the schemas, CLI flags, and exit codes defined below.

---

## 1. `ConcIR` v4.0 Structured Surface IR (`SCIR`), `SeqHole`, Modular Summary & Refinement CLI

### 1.1 Backward Compatibility
All existing v3.5 JSON programs, contracts, CLI subcommands, exit codes (`0` pass, `1` fail, `2` usage, `3` unknown, `4` invalid, `5` unsupported), and integration tests MUST remain 100% passing.

### 1.2 Traceability Tags (`req`) & `SeqHole` Semantics
1. **Optional `req` tags** (`#[serde(default, skip_serializing_if = "Vec::is_empty")] pub req: Vec<String>`) are permitted on:
   - `Module`
   - `Resource`
   - `Function`
   - `Stmt`
   - `StructuredStmt`
2. **First-class `Op::SeqHole`** (in `ast.rs`, `sem/program.rs`, `interp`, `petri`, `validate`, `codegen`):
   ```json
   {
     "sid": "s2",
     "kind": "seq_hole",
     "id": "h_inc",
     "desc": "increment bounded counter",
     "reads": ["c"],
     "writes": ["c"],
     "updates_modeled": [
       { "target": "c", "expr": "c + 1" }
     ],
     "expected_type": "()",
     "req": ["R2"]
   }
   ```
   - `updates_modeled` defaults to `[]`, `expected_type` defaults to `"()"`, `req` defaults to `[]`.
   - **Validation & Lowering**: `SeqHole` is NO LONGER `Unsupported`! Every `updates_modeled` entry validates `target` as a writable local or shared `Var` slot (respecting `protection` lock checks when writing a protected `Var`!) and lowers `expr` via `BodyCtx::expr`.
   - **Execution (`interp` & `petri`)**: Executes as one atomic silent step (`tau`) in the current thread/frame, evaluating all `updates_modeled` RHS expressions in the pre-state and applying them simultaneously, then advancing to `next`.

### 1.3 Structured Function Body (`structured_body` in `Function`)
A `Function` in `ast.rs` may specify `structured_body: Vec<StructuredStmt>` (with `body` omitted or empty). Whenever `Program` is loaded (via a helper `Program::ensure_lowered(&mut self) -> Result<TraceMap, String>` called automatically by `check`, `support`, `explore`, `repair`, `codegen`, `summarize`, `verify-refinement`, and exposed via `concir-backend lower <input.json> [--out <lowered.json>] [--trace-map <map.json>]`), `structured_body` is deterministically lowered into flat `body: Vec<Stmt>` with generated `s1, s2, ...` `sid`s:

Supported `StructuredStmt` variants (`#[serde(tag = "kind")]`, each carrying `id: String` and `#[serde(default)] req: Vec<String>`):
- `stmt`: embeds any single flat `Op` (e.g., `assign_local`, `write_shared`, `read_shared`, `condvar_notify`, `condvar_notify_all`, `channel_send`, `channel_recv`, `semaphore_acquire`, `semaphore_release`, `atomic_load`, `atomic_store`, `atomic_cas`, `call`, `spawn`, `join`, `scope`, `return`, `seq_hole`, `nop`).
- `lock_guard`: `{ "id": "b1", "kind": "lock_guard", "resource": "m", "body": [ ... ] }`
  - Emits `mutex_lock(resource)`, lowers `body`, and emits `mutex_unlock(resource)` on normal fallthrough AND before any `return` inside `body` (tracking active lock guards on a lexical stack in LIFO order!).
- `rwlock_read_guard`: `{ "id": "b2", "kind": "rwlock_read_guard", "resource": "rw", "body": [ ... ] }`
  - Emits `rwlock_read(resource)`, lowers `body`, and emits `rwlock_unlock(resource)` on normal fallthrough AND before any `return` inside `body` in LIFO order alongside `lock_guard` / `rwlock_write_guard`.
- `rwlock_write_guard`: `{ "id": "b3", "kind": "rwlock_write_guard", "resource": "rw", "body": [ ... ] }`
  - Emits `rwlock_write(resource)`, lowers `body`, and emits `rwlock_unlock(resource)` on normal fallthrough AND before any `return` inside `body` in LIFO order alongside `lock_guard` / `rwlock_read_guard`.
- `if_else`: `{ "id": "b2", "kind": "if_else", "cond": "c < 2", "then_body": [ ... ], "else_body": [ ... ] }` (`else_body` optional, defaults to `[]`).
- `while_loop`: `{ "id": "b3", "kind": "while_loop", "cond": "i < 4", "body": [ ... ] }`
- `while_wait`: `{ "id": "b4", "kind": "while_wait", "cond": "count == 0", "condvar": "cv", "lock": "m", "body": [ ... ] }` (`body` optional before `condvar_wait`).
  - Emits loop header `branch(cond, wait_entry, exit_sid)`, optional `body`, `condvar_wait(condvar, lock)`, and `goto(header_sid)`.

`TraceMap` JSON schema (`concir-trace-map/1`):
```json
{
  "schema": "concir-trace-map/1",
  "program": "demo",
  "entries": {
    "main::w1::s1": {
      "module": "main",
      "function": "w1",
      "sid": "s1",
      "structured_id": "b1",
      "hole_id": null,
      "req": ["R2"]
    }
  }
}
```

### 1.4 MIR-Clean `codegen` (`concir-backend codegen <program.json> --out <dir>`)
`backends/concir/src/codegen.rs` emits a generated crate that compiles cleanly under both dynamic trace execution AND `spn` static MIR Petri-net extraction:
1. **Cargo Features in generated `Cargo.toml`**:
   ```toml
   [package]
   name = "cir_generated"
   version = "0.1.0"
   edition = "2021"

   [features]
   default = ["cir_trace"]
   cir_trace = []
   ```
   Inside generated `src/cir_trace.rs`, guard `EVENTS` (`OnceLock<Mutex<...>>`) and the body of `record(...)` / `finish()` with `#[cfg(feature = "cir_trace")]` (when disabled, `record` and `finish` are no-ops `{ let _ = (tag, op, resource, sid); }`). Thus:
   - Default `cargo run` / Miri keeps 100% identical runtime trace behavior!
   - `spn` analyzing with `--no-default-features` sees ZERO synthetic `EVENTS` mutexes in MIR!
2. **Authentic Sync Primitives in `Shared`**:
   - `ResKind::Atomic`: emit `Arc<std::sync::atomic::AtomicI64>` (for `Int`/bounded `Int`) or `Arc<std::sync::atomic::AtomicBool>` (for `Bool`), using `Ordering::SeqCst` (`load`, `store`, `compare_exchange`).
   - `ResKind::Var`: emit `Arc<crate::cir_trace::SyncVar<T>>` backed by `std::cell::UnsafeCell<T>` (`unsafe impl<T: Send> Sync for SyncVar<T> {}`) with `.read()` and `.write(v)` methods instead of wrapping `Var` in a fake `Arc<Mutex<T>>`! (Safe for valid CIR programs where `protection` guarantees lock discipline, and prevents `spn` from seeing fake nested `Mutex::lock` calls on `Var` accesses!)
3. **`SeqHole` Emission & Enhanced `codegen.json`**:
   - For `SemOp::SeqHole`, emit:
     ```rust
     let _hole_val: <expected_ty> = /* HOLE(<id>) expected: <expected_ty> */ Default::default();
     ```
     followed by any `updates_modeled` assignments.
   - Extend `CodegenMap` (`codegen.json`) with backward-compatible fields:
     - `resources`: `[{"fqn": "main::m", "field": "r0_main_m", "kind": "Mutex"}, ...]`
     - `holes`: each `Hole` includes `id`, `function` (FQN `module::fn` or short name; include `module` field too!), `sid`, `expected`, `line` (1-based line in `src/main.rs`), `reads`, `writes`, `req`.

### 1.5 Compositional Module Summary CLI (`concir-backend summarize`)
```bash
concir-backend summarize <program.json> [--module <mod_name>] [--out <summary.json>]
```
Outputs JSON (`schema: "concir-module-summary/1"`, stdout + optional `--out`):
- Per module: `name`, `provides`, `requires`, `lock_order_edges: [{"first": "m1", "second": "m2", "function": "f"}]`, `quotient_states`, `quotient_transitions` (obtained by hiding internal `Nop`/`AssignLocal`/`ReadShared`/`WriteShared`/`SeqHole` transitions via $\tau$-closure and collapsing stuttering-equivalent states), and `cross_module_lock_cycle: bool`.
- Exit code `0` if valid & acyclic cross-module lock graph, `1` if cross-module lock cycle detected, `4` if invalid CIR.

### 1.6 Code-to-Model Stuttering Refinement CLI (`concir-backend verify-refinement`)
```bash
concir-backend verify-refinement \
  --spec <program.json> \
  --impl-net <impl_sync_net.json> \
  [--contract <contract.json>] \
  [--codegen-map <codegen.json>] \
  [--out <refinement_report.json>]
```
Checks **Branch-Pruning Stuttering Refinement ($N_{\text{impl}} \preceq_{\text{ref}} N_{\text{spec}}$)** — allowing the implementation to determinize/prune non-deterministic branches of the spec while enforcing soundness:
1. **Hole Purity Check (`O1_purity`)**: Zero `purity_violations` reported in `impl_sync_net.json`.
2. **Weak / Stuttering Simulation (`O2_weak_simulation`)**:
   - Canonicalize observable sync labels `(op, resource_fqn)` using `codegen_map.resources` (if provided) or suffix/name matching (`r0_main_m` / `main::m` / `m` $\to$ `main::m`). Note: `condvar_notify` and `condvar_notify_all` match `condvar_notify*` if `impl_net` emits `condvar_notify` or exact match when distinguished; `scope` in spec matches `spawn` + `join` of its child functions in impl!
   - Compute $\tau^*$-closure on both $N_{\text{spec}}$ (explored via `ConcIR` Petri engine) and $N_{\text{impl}}$ (`states` + `edges` from `impl_sync_net.json`, or explored from `places` + `transitions` if `states` is empty).
   - Check that from `(s_impl_0, s_spec_0)`, every observable step $s_{\text{impl}} \xrightarrow{\tau^* \cdot a} s'_{\text{impl}}$ can be matched by some $s_{\text{spec}} \xrightarrow{\tau^* \cdot a} s'_{\text{spec}}$ (greatest fixed-point weak simulation relation $R$, or trace-refinement BFS with Shortest Distinguishing Witness Trace on failure!).
   - *Crucial practical detail for `scope` / thread tags*: Compare observable sync actions `(op, resource_fqn)` (and for `spawn`/`join`, target function name if mapped; treat `scope` transitions in spec as $\tau$ or matchable with `spawn`/`join` if `SemOp::Scope` doesn't emit separate `spawn`/`join` labels in spec's Petri net—check how `backends/concir/src/petri/exec.rs` steps `Scope`!). Specifically: in `ConcIR`, `Scope` starts child threads and waits at `ScopeWait` until children return; in Rust/`spn`, `Scope` lowers to `spawn` transitions for each child and `join` transitions! So in `verify-refinement`, treat `spawn`/`join`/`scope` lifecycle transitions consistently (either map spec `Scope` entry/exit to `spawn`/`join` OR project the observable alphabet $\Sigma_{\text{sync}}$ onto synchronization resource primitives `{mutex_lock, mutex_unlock, rwlock_read, rwlock_write, rwlock_unlock, condvar_wait, condvar_notify, condvar_notify_all, sem_acquire, sem_release, channel_send, channel_recv, atomic_load, atomic_store, atomic_cas}` while checking thread completion via `preserved_goals`!). Projecting $\Sigma_{\text{sync}}$ onto resource synchronization operations + checking thread completion in `preserved_goals` makes `std::thread::scope` vs `Vec<JoinHandle>` 100% representation-independent!
3. **No New Deadlocks & Lock-Divergence Freedom (`O2_deadlock_and_divergence`)**:
   - No reachable state in $N_{\text{impl}}$ has `deadlock == true`.
   - No reachable $\tau$-cycle in $N_{\text{impl}}$ occurs while `held_locks` is non-empty (unless also present in spec).
4. **Preserved Non-Triviality (`O2_preserved_goals`)**:
   - Pruning non-determinism cannot produce a vacuous no-op implementation: every function named in `contract.preserved` (`function_completed`) MUST appear in `impl_sync_net.completed_functions` (matched by suffix `cf_<fn>` or `<fn>`), AND if $N_{\text{spec}}$ has $\ge 1$ reachable observable sync transition on every execution, $N_{\text{impl}}$ must have $\ge 1$ reachable observable sync transition!

Output JSON (`concir-refinement-report/1`):
```json
{
  "schema": "concir-refinement-report/1",
  "outcome": "pass",
  "relation": "stuttering_refinement",
  "checks": {
    "hole_purity": true,
    "weak_simulation": true,
    "deadlock_free": true,
    "lock_divergence_free": true,
    "preserved_goals": true
  },
  "spec_states": 12,
  "impl_states": 8,
  "violations": []
}
```
Each item in `violations` on failure:
```json
{
  "obligation": "O1_purity | O2_refinement | O3_data_guard",
  "kind": "sync_in_hole | unexpected_sync_step | deadlock_introduced | lock_divergence | missing_preserved_goal",
  "message": "...",
  "hole_id": "h1",
  "sid": "s2",
  "req_clauses": ["R2"],
  "rust_span": "src/main.rs:42:5: 42:30",
  "witness_trace": [
    { "step": 0, "op": "mutex_lock", "resource": "main::m", "span": "src/main.rs:30:5" }
  ],
  "expected_spec_ops": ["mutex_unlock(main::m)"]
}
```
Exit codes: `0` (`pass`), `1` (`fail`), `2` (CLI usage error), `3` (`unknown`, e.g., `impl_sync_net.truncated == true`), `4` (`invalid` spec/json).

---

## 2. `backends/spn/` (`spn`) Export Mode (`concir-impl-net/1`)

Two CLI options in `backends/spn/src/options.rs` (parsed from `SPN_FLAGS` or CLI args):
1. `--export-concir-net <FILE>` (optional `PathBuf`): writes the `concir-impl-net/1` JSON file at the end of `pipeline::project`.
2. `--codegen-map <FILE>` (optional `PathBuf`): reads `codegen.json` (plus optional `hole_ranges` `{"h1": {"file": "src/main.rs", "start_line": 40, "end_line": 46}}`) to:
   - Map MIR resource place names (such as `shared.r0_main_m` or local/static names) to canonical `ConcIR` resource FQNs (`main::m`);
   - Detect `purity_violations` whenever any synchronization transition (`EventKind`) or cut back-edge loop summary has a source `span` whose line number falls inside a `HOLE` range `[start_line, end_line]`!

### Canonical Op Mapping in `spn`
Map `unipn::pt::TransitionType` to `concir-impl-net/1` `op`:
- `Lock(_)` $\to$ `"mutex_lock"`
- `Unlock(_)` $\to$ `"mutex_unlock"`
- `RwLockRead(_)` $\to$ `"rwlock_read"`
- `RwLockWrite(_)` $\to$ `"rwlock_write"`
- `DropRead(_)` | `DropWrite(_)` $\to$ `"rwlock_unlock"`
- `Wait` $\to$ `"condvar_wait"`
- `Notify(_)` $\to$ `"condvar_notify"` (or `"condvar_notify_all"` if provenance/transition name indicates `notify_all`)
- `AtomicLoad(..)` $\to$ `"atomic_load"`
- `AtomicStore(..)` $\to$ `"atomic_store"`
- `AtomicCmpXchg(..)` $\to$ `"atomic_cas"`
- `Spawn(_)` $\to$ `"spawn"`
- `Join(_)` $\to$ `"join"`
- Custom `cir_trace::Semaphore::{acquire, release}` / `cir_trace::Channel::{send, recv}`: recognize `cir_trace::Semaphore::acquire*` as `"sem_acquire"`, `cir_trace::Semaphore::release*` as `"sem_release"`, `cir_trace::Channel::send*` as `"channel_send"`, `cir_trace::Channel::recv*` as `"channel_recv"` (skipping descent into `cir_trace::` internals so `Semaphore` and `Channel` aren't expanded into internal helper `Mutex`/`Condvar` transitions!).
- All other transitions: `"observable": false, "op": "tau"`.

### State Graph Export in `concir-impl-net/1`
From `explored.results[0].state_graph` and `constructed.net` (or `raw_net` when `reduce_net = false` — note: when `--export-concir-net` is active, force `reduce_net = false` or map transitions accurately so observable sync transitions are NEVER merged away by net reduction!):
- **IMPORTANT**: When `--export-concir-net` is passed, disable `reduce_net` (or preserve all observable sync transitions) so `state_graph` transitions correspond directly to `raw_net` transition IDs!
- Export `places`, `transitions`, `states` (`id`, `initial`, `terminal`, `deadlock`, `held_locks`), `edges` (`from`, `to`, `transition`), `truncated` (`state_graph.truncated`), `purity_violations`, and `completed_functions` (functions whose `end` place reaches $\ge 1$ token in some reachable state).

---

## 3. `ConcPlanVerify` Python Orchestration & Pluggable Static Verification (`src/concplan/`)

1. **Backend-First Toolchain Discovery (`src/concplan/core/toolchain.py`)**:
   - `concir-backend`, `concir-instrument`, `bind_check`: check `REPO / "backends/concir/target/release"` and `REPO / "backends/concir/target/debug"` FIRST (honoring `CONCIR_BACKEND`, `CONCIR_INSTRUMENT`, `CONCIR_BIND_CHECK`, `CONCIR_TOOLCHAIN`).
   - `spn`: check `CONCIR_SPN` env or `REPO / "backends/spn/target/{release,debug}"`.
   - `lockbud`: check `CONCIR_LOCKBUD` (or `LOCKBUD_BIN`) env or `REPO / "backends/third_party/lockbud/target/{release,debug}"`.
2. **Typed `ConcirClient` Extensions (`src/concplan/core/concir_client.py`)**:
   - `lower(program_path, *, out_path=None, trace_map_path=None)`
   - `summarize(program_path, *, module=None, out_path=None)`
   - `verify_refinement(spec_path, impl_net_path, *, contract_path=None, codegen_map_path=None, out_path=None)`
3. **Pluggable Post-Codegen Static Verification (`src/concplan/code_verify/static_verify.py`)**:
   - Defines `StaticViolation`, `StaticVerificationReport`, `StaticVerifierPlugin` protocol, and `CompositeStaticVerifier`.
   - Implements three plugins:
     1. `AstHolePurityPlugin`: structural skeleton integrity + hole line-range computation (`hole_ranges` written into an augmented `codegen_filled.json` after `fill_holes` so multi-line hole fills have exact `[start_line, end_line]` spans!) + AST-aware check for forbidden sync/unsafe/thread constructs (ignoring comments and string literals!).
     2. `SpnMirRefinementPlugin`: invokes `cargo` with `RUSTC_WRAPPER=<spn>` and `--no-default-features` + `SPN_FLAGS="--export-concir-net <net.json> --codegen-map <codegen_filled.json>"` on the generated crate, then calls `ConcirClient.verify_refinement(...)` to check `concir-impl-net/1` against the verified CIR & contract! Gracefully reports `"unknown"` (never a fake pass!) if `spn` binary is absent.
     3. `LockbudPlugin`: optional adapter wrapping `backends/third_party/lockbud` when configured.
4. **End-to-End Traceable & Bi-level Iterative Pipeline (`src/concplan/pipeline/e2e_pipeline.py`)**:
   - Connects Stage 1 (Requirements + Contract) $\to$ Stage 2 (SCIR/CIR generation + modular/exhaustive verification + Loop A revision) $\to$ Stage 3 (`codegen` + `fill_holes`) $\to$ Stage 4 (`CompositeStaticVerifier` + optional dynamic `conform`):
     - **Loop B (Hole-local repair)**: When `StaticVerificationReport` fails with `hole_id`, re-prompts the provider ONLY for the offending hole(s) with exact `StaticViolation` diagnostics (`rust_span`, `witness_trace`, `req_clauses`).
     - **Loop C (Cross-layer Escalation)**: When the hole filler emits `{"escalate": {"reason": "...", "module": "..."}}` or exceeds `max_hole_rounds`, routes structured escalation diagnostics back to Stage 2 CIR revision!
   - Emits `traceability_manifest.json` linking `Clause ID -> Property ID -> Module::Function::SID (Structured ID) -> Hole ID -> Rust File:LineRange -> Verification Status`.
5. **Bundled Prompts & Unit/Integration Tests**:
   - `src/concplan/prompts/` bundles `nl_to_clauses.md`, `clauses_to_contract.md`, `scir_generation.md`, `hole_repair.md`, `cir_patch.md`, and `cir_local_revision.md` (loaded via `importlib.resources.files("concplan.prompts")`).
   - Root `tests/` contains the consolidated unittest suite (`test_formalize_stage0.py`, `test_static_verify.py`, `test_e2e_pipeline.py`, `test_live_spn_concir_e2e.py`, etc.).

---

## 4. ConcIR v5 + UniPN Unification Extensions

### 4.1 Temporal Properties (CTL & LTL) and Non-Vacuity (`unipn::analysis::temporal`)
`PropertySpec` (`#[serde(tag = "kind", rename_all = "snake_case")]`) adds:
- **`ctl`**: `{ "kind": "ctl", "id": "...", "formula": <CtlFormula<PredicateSpec>>, "non_vacuous": true, "req": ["R3"] }`
  - `CtlFormula` tags (`#[serde(tag = "op", rename_all = "snake_case")]`): `true`, `false`, `atom` (`{atom}`), `not` (`{arg}`), `and` / `or` (`{args}`), `implies` / `eu` / `au` (`{lhs, rhs}`), `ex` / `ax` / `ef` / `af` / `eg` / `ag` (`{arg}`).
- **`ltl`**: `{ "kind": "ltl", "id": "...", "formula": <LtlFormula<PredicateSpec>>, "fairness": [<FairnessConstraint>], "non_vacuous": true, "req": ["R3"] }`
  - `LtlFormula` tags (`#[serde(tag = "op", rename_all = "snake_case")]`): `true`, `false`, `atom` (`{atom}`), `not` (`{arg}`), `and` / `or` (`{args}`), `implies` / `until` / `release` (`{lhs, rhs}`), `next` / `eventually` / `always` (`{arg}`).
  - `FairnessConstraint` tags (`#[serde(tag = "kind", rename_all = "snake_case")]`): `justice` (`{atom}` = $\mathrm{GF}\,p$), `compassion` (`{enabled, taken}` = $\mathrm{GF}\,\text{enabled} \to \mathrm{GF}\,\text{taken}$).
- **Non-Vacuity**: `PropertyResult` adds optional `vacuous: bool`, `vacuous_antecedents: Vec<String>`, and `explanation: Option<String>`. When `non_vacuous: true` and an implication antecedent holds in 0 reachable states, `explore` marks the property `FAIL` and surfaces `vacuous_antecedents` to Loop A.
- **Additional CLIs**:
  - `concir-backend explain-contract <contract.json> [--output <file>]` $\to$ `concir-contract-explanation/1`.
  - `concir-backend export-cvn <program.json> [--output <file>]` $\to$ `unipn::cvn::CvnNet` + initial state + degradation report.

### 4.2 Domain `TypeHole` (`src/domain.rs`), Composite Types, `RwLock`, and Multi-File Codegen
- **`TypeDef` / `TypeHole`**: Module `types` entries support `{"Hole": {"description": "...", "req": ["R..."]}}`, `{"Struct": ...}`, and `{"Array": ...}`. `Mutex` and `RwLock` resources accept an optional `"base"` payload type.
- **Codegen Emission**:
  - Emits `src/domain.rs` containing `/* TYPE_HOLE(th1) type: <Name> */ #[derive(Clone, Debug, Default, PartialEq)] pub struct <Name> {}` plus a compile-time `Send + Sync + Clone + Default + 'static` bound assertion. Under `#[cfg(not(feature = "cir_trace"))]` (used by `spn`), structured mutex/rwlock guards with non-`()` payloads bind `_g_*`, `_rg_*`, and `_wg_*` to payload-typed `&mut {payload}` / `&{payload}` references backed by `Default::default()` while keeping OS sync guards (`__spn_*`) intact so `SeqHole` method/field calls compile identically under both feature modes.
  - Emits multi-module programs into `src/<mod>.rs` and records `type_holes` (`id`, `type_name`, `module`, `file`, `line`, `description`, `req`) and `file` on `holes` in `codegen.json`.
  - `fill_type_holes_marked` / `fill_holes_marked` wrap filled `TypeHole`s in `/* TYPE_HOLE_BEGIN(th1) */ ... /* TYPE_HOLE_END(th1) */` and write `type_hole_ranges` into `codegen_filled.json`.
  - `AstHolePurityPlugin` scans all `src/*.rs` files (`src/main.rs`, `src/domain.rs`, `src/<mod>.rs`), allowing pure `struct`/`enum`/`impl` + bounded loops in `TypeHole`s while forbidding `Mutex`, `RwLock`, `Condvar`, `Semaphore`, `mpsc`, `Atomic*`, `thread::spawn`, `unsafe`, `cir_trace`, and unbounded `loop`/`while`.
- **`RwLock` Canonical Ops**: `rwlock_read_lock`, `rwlock_read_unlock`, `rwlock_write_lock`, `rwlock_write_unlock` are unified across `ConcIR`, `spn`, `conform`, and `verify-refinement`.

### 4.3 Compositional / Modular MIR Refinement
- `spn` accepts `--focus-module <MOD>` (recording `focus_module` in `concir-impl-net/1`), projecting external transitions to silent `tau` and filtering purity attribution to `<MOD>` + shared `src/domain.rs` (observable iff `fn_in_module(MOD) || span_in_module(MOD) || resource_in_module(MOD)` for resources *owned* by `MOD`).
- `concir-backend verify-refinement --module <MOD>` checks stuttering refinement projected symmetrically to transitions on resources *owned* by `<MOD>` or originating inside `<MOD>` (ignoring `requires.resources` ownership so shared imported resources modified by other modules project to `tau` symmetrically), combined with cross-module lock-order acyclicity.
- `SpnMirRefinementPlugin(modular=True)` and `concplan e2e --modular-refinement` orchestrate global `summarize` + per-module `spn --focus-module <m>` + `verify-refinement --module <m>`, emitting `modular_reports` in `traceability_manifest.json`.

### 4.4 Stage 0 Dual-Entry Autoformalization (`concplan.formalize`)
- **Entry A (`formalize_from_nl`)**: Raw NL $\to$ Step 0.1 (`nl_to_clauses`: grounded `roles`, `resources`, `domain_types` + `R1..Rn` classified into `safety | liveness | reachability | sequential_hole` with `verifiable` flags + `REQUIREMENTS.md`) $\to$ Step 0.2 (`clauses_to_contract`: strictly isolated from candidate SCIR models) $\to$ Step 0.3 (`explain_contract_data` + `validate_contract_sanity`).
- **Entry B (`formalize_from_structured`)**: Existing `REQUIREMENTS.md` / `requirements.json` (+ optional `contract.json`) $\to$ synthesizes missing contract via Step 0.2 or validates & explains existing contract via Step 0.3.
- **Upgraded `traceability_manifest.json`**: Preserves all v4 keys and adds `formalization`, `nl_explanations`, `vacuity_status`, `type_holes`, `modular_reports`, `req_net_check`, and `design_refinement`.

---

## 5. High-Level Colored Petri Net (`HlnNet`), Constraint Patterns, Health Suite & Two-Net Refinement (`N_req` $\sqsubseteq_{\text{ext}}$ `N_code`)

### 5.1 Requirement Net IR (`concir-req-ir/1`) & Self-Check (`concir-backend check-req`)
```json
{
  "schema": "concir-req-ir/1",
  "name": "pc_req",
  "roles": [
    { "name": "producer", "instances": 1, "init_state": "ready", "states": ["ready", "done"], "terminal_states": ["done"], "req": ["R1"] }
  ],
  "state_vars": [
    { "name": "items", "type": { "Int": [0, 1] }, "init": 0, "req": ["R1"] }
  ],
  "milestones": [
    { "name": "item_produced", "description": "Producer adds an item", "req": ["R1"] }
  ],
  "steps": [
    { "id": "p_produce", "role": "producer", "from_state": "ready", "to_state": "done", "guard": "items == 0", "updates": [{"target": "items", "expr": "items + 1"}], "milestone": "item_produced", "req": ["R1"] }
  ]
}
```
CLI: `concir-backend check-req <req_ir.json> [--contract <contract.json>] [--out <report.json>]` (also `concplan check-req` / MCP `concplan_check_req`).
Output schema: `concir-req-check/1` (`outcome`, `complete`, `deadlock_free`, `no_dead_milestones`, `declared_milestones`, `reachable_milestones`, `missing_milestones`, `deadlock_states`, `diagnostics`, `properties`).
Exit codes: `0` (`pass`), `1` (`fail`), `2` (`usage`), `3` (`unknown`), `4` (`invalid`), `5` (`unsupported`).

### 5.2 Milestone Anchors & `ConstraintPattern` / `Health` Extensions
- Optional `"milestone": "<name>"` field is supported on flat `Stmt`, `StructuredStmt::Stmt` (and inline ops in `structured_body`), and `TraceMapEntry`.
- `PredicateSpec::MilestoneReached`: `{"kind": "milestone_reached", "milestone": "<name>"}`.
- `PropertySpec::Pattern` (`kind: "pattern"`):
  - `always`: `{"pattern": "always", "cond": <PredicateSpec>}`
  - `exclude`: `{"pattern": "exclude", "preds": [<PredicateSpec>, ...]}`
  - `at_most`: `{"pattern": "at_most", "conds": [<PredicateSpec>, ...], "max": N}`
  - `precedence`: `{"pattern": "precedence", "effect": <PredicateSpec>, "cause": <PredicateSpec>, "non_vacuous": true}`
  - `triggers`: `{"pattern": "triggers", "cause": <PredicateSpec>, "effect": <PredicateSpec>, "fairness": [...], "non_vacuous": true}`
  - `can_reach`: `{"pattern": "can_reach", "goal": <PredicateSpec>}`
  - `always_can_reach`: `{"pattern": "always_can_reach", "goal": <PredicateSpec>}`
- `PropertySpec::Health` (`kind: "health"`): `{"kind": "health", "id": "...", "checks": ["deadlock", "partial_deadlock", "lost_wakeup", "wake_starvation", "acquire_starvation", "resource_leak", "double_lock", "data_race"]}`.

### 5.3 Stage 0–2 Design Extension Refinement (`concir-backend verify-design`)
CLI: `concir-backend verify-design --req <req_ir.json> --design <program.json> [--contract <contract.json>] [--out <report.json>]` (also `concplan verify-design` / MCP `concplan_verify_design`).
Output schema (`concir-design-refinement/1`):
```json
{
  "schema": "concir-design-refinement/1",
  "outcome": "pass | fail | unknown | invalid | unsupported",
  "relation": "extension_stuttering_refinement",
  "complete": true,
  "checks": {
    "req_valid": true,
    "design_valid": true,
    "weak_simulation": true,
    "deadlock_free": true,
    "lock_divergence_free": true,
    "milestone_preservation": true,
    "health_obligations": true,
    "contract_satisfied": true
  },
  "req_states": 3,
  "design_states": 18,
  "declared_milestones": ["item_produced", "item_consumed"],
  "realized_milestones": ["item_produced", "item_consumed"],
  "missing_milestones": [],
  "violations": []
}
```
Truncated exploration (`complete == false`) MUST report `unknown` (exit `3`) when no counterexample is found, never `pass` (`0`).
