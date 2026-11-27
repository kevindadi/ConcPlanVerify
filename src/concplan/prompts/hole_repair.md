# Hole Filling and Hole-Local Repair Prompt (v5: SeqHole + TypeHole)

You fill or repair **only** the designated holes in a verified Rust concurrency skeleton generated
by `concir-backend codegen`:

- **Sequential Holes (`SeqHole`)**: `/* HOLE(id) expected: T */` inside `src/main.rs` or `src/<mod>.rs`.
- **Domain Type Holes (`TypeHole`)**: `/* TYPE_HOLE(id) type: Name */` inside `src/domain.rs`.

Output **only** a single JSON object mapping each hole ID (`h*` and/or `th*`) to its Rust code.

## 1. Strict Purity Rules (Obligation O1)

### 1.1 Sequential Holes (`SeqHole`, e.g. `h1`, `h_inc`)

Every `SeqHole` represents pure sequential local computation. Your Rust expression or block MUST obey:

1. **No synchronization primitives:** never use `std::sync`, `Mutex`, `Condvar`, `RwLock`,
   `Semaphore`, `Barrier`, `OnceLock`, `mpsc`, `channel`, `.lock()`, `.try_lock()`, `.read()`,
   `.write()`, `.wait()`, `.wait_while()`, `.notify_one()`, `.notify_all()`, `.send()`, or `.recv()`.
2. **No thread or blocking primitives:** never use `std::thread`, `thread::spawn`, `thread::scope`,
   `thread::sleep`, or `scope(...)`.
3. **No atomics:** never use `Atomic*`, `std::sync::atomic`, or `Ordering::*`.
4. **No unsafe, tracing, or new imports:** never use `unsafe`, `extern`, `cir_trace`, `use`, or
   `// @cir` annotations.
5. **No potentially diverging loops:** never use `loop { ... }`, `while ... { ... }`, or `spin_loop()`.
   (Holes may run while holding locks; unbounded loops violate lock-divergence freedom.)
6. **Respect the expected Rust type:** the expression/block must type-check against `expected: T`.
7. **Domain types & guards:** access domain types defined in `src/domain.rs` via
   `crate::domain::<TypeName>` (never add `use` statements inside a `SeqHole`). Also note that
   `Mutex<()>` / `RwLock<()>` guards dereference to `()`, unless the CIR resource explicitly
   declared `"base": "<TypeName>"`.

### 1.2 Domain Type Holes (`TypeHole`, e.g. `th1`, in `src/domain.rs`)

A `TypeHole` defines the pure Rust data structure (`struct` / `enum` + pure `impl` methods) for a
domain type in `src/domain.rs`:

1. **Preserve type identity and trait bounds:** the defined `pub struct <Name>` MUST implement
   `Send + Sync + Clone + Default + 'static` (derive `#[derive(Clone, Debug, Default, PartialEq)]`
   unless custom pure impls are needed).
2. **Pure data & pure methods ONLY:** you may define fields, helper pure structs/enums, and pure
   `impl <Name>` methods (bounded `for` loops are allowed).
3. **Strictly forbidden inside `TypeHole`:** NEVER introduce `std::sync`, `Mutex`, `RwLock`,
   `Condvar`, `Semaphore`, `Barrier`, `OnceLock`, `mpsc`, `channel`, `Atomic*`, `Ordering::*`,
   `std::thread`, `thread::spawn`, `thread::scope`, `thread::sleep`, `unsafe`, `extern`,
   `cir_trace`, `// @cir`, or unbounded `loop`/`while` without an explicit bound!

## 2. Reading Repair Diagnostics (Loop B)

When repairing after a static verification failure, `<static_verification_violations>` provides
structured diagnostics from `AstHolePurityPlugin` and/or `SpnMirRefinementPlugin`:

- `obligation`: `"O1_purity"` (forbidden construct in a `SeqHole` or `TypeHole`) or
  `"O2_refinement"` (MIR Petri net diverged from the verified CIR model).
- `kind`: e.g. `sync_in_hole`, `sync_in_type_hole`, `thread_in_hole`, `atomic_in_hole`,
  `unsafe_in_hole`, `loop_in_hole`, `unexpected_sync_step`, `deadlock_introduced`,
  `lock_divergence`, `missing_preserved_goal`.
- `hole_id`, `sid`, `req_clauses`, `rust_span`: exact file (`src/main.rs`, `src/domain.rs`, or
  `src/<mod>.rs`), line/column, and requirement clause linked to the violation.
- `witness_trace` & `expected_spec_ops`: shortest distinguishing execution trace showing how the
  implementation violated the verified CIR specification.

Repair **ONLY** the hole IDs listed in `<offending_holes>`. Do not include or alter passing holes.

Example hole-fill / repair response (combining a `TypeHole` and a `SeqHole`):

```json
{
  "th1": "#[derive(Clone, Debug, Default, PartialEq)]\npub struct OrderBook {\n    pub best_bid: i64,\n}\n\nimpl OrderBook {\n    pub fn record_bid(&mut self, price: i64) {\n        if price > self.best_bid { self.best_bid = price; }\n    }\n}",
  "h_inc": "{ let next = val + 1; next }"
}
```

## 3. Cross-Layer Escalation Protocol (Loop C)

If fulfilling the requirements inside a `SeqHole` or `TypeHole` is **impossible** without adding
or changing synchronization, threads, shared state, or locking order in the CIR model itself, do
**not** smuggle synchronization into the hole. Instead, return an explicit escalation JSON object:

```json
{
  "escalate": {
    "reason": "Computing the aggregate requires a shared Mutex-protected accumulator resource not declared in the current CIR.",
    "module": "main"
  }
}
```

The orchestrator will route your escalation back to Stage 2 CIR revision, re-verify the revised
model, and regenerate the Rust skeleton.
