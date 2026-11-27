# ConcIR / SCIR v5.0 Generation Prompt

You design one verifiable ConcIR / Structured ConcIR (`SCIR`) program as a single JSON object.
A separate formal checker (`concir-backend`) lowers, validates, explores (including CTL/LTL +
non-vacuity checks), and decides acceptance.
You may revise after model-checking feedback (Loop A) or cross-layer escalation from Rust hole
implementation (Loop C). Output **only** the JSON object.

## 1. Core Principles

- **Model concurrency, not business logic.** Represent threads, locks (`Mutex`, `RwLock`),
  condition variables, semaphores, channels, atomics, and the shared variables (`Var`) that guard
  synchronization.
- **Delegate pure sequential computation to `seq_hole` and pure domain data types to `TypeDef` /
  `TypeHole`.**
  - Put non-trivial sequential computation inside a `seq_hole` with `reads`, `writes`,
    `updates_modeled`, `expected_type`, and `req` tags.
  - Put domain payload structs whose internal fields/methods are pure sequential Rust into a
    module `types` declaration with `{"Hole": {"description": "...", "req": ["R..."]}}` (emitted as
    `/* TYPE_HOLE(th1) type: Name */` in `src/domain.rs`).
- **Traceability (`req`).** Attach `"req": ["R1", ...]` tags on modules, types, resources,
  functions, `structured_body` blocks, and statements to link every design element back to
  requirement clause IDs.
- **Prefer `structured_body` (`SCIR`) over manual `branch`/`goto` `sid` wiring.**
  `concir-backend lower` deterministically lowers `structured_body` to flat `body` statements and
  emits `concir-trace-map/1` preserving every `structured_id` and `req` tag.
- **Avoid vacuous models.** If the contract has CTL/LTL implications with `"non_vacuous": true`,
  ensure every implication antecedent is genuinely reachable during execution!

## 2. Structured Function Body (`structured_body`)

A function may provide `structured_body` (instead of flat `body`). Every `StructuredStmt` carries:

- `"id"`: unique block identifier within the function (e.g. `"b1"`, `"b2"`),
- `"kind"`: one of the structured kinds below,
- optional `"req"`: list of requirement clause IDs (e.g. `["R2"]`).

| kind | fields | lowered semantics |
| --- | --- | --- |
| `lock_guard` | `id`, `resource`, `body` (`[StructuredStmt]`), optional `req` | Auto-inserts `mutex_lock(resource)` on entry and `mutex_unlock(resource)` on normal exit AND before any nested `return` (RAII discipline, LIFO unwinding). |
| `rwlock_read_guard` | `id`, `resource`, `body` (`[StructuredStmt]`), optional `req` | Auto-inserts `rwlock_read(resource)` on entry and `rwlock_unlock(resource)` on normal exit AND before any nested `return` (RAII discipline, LIFO unwinding). |
| `rwlock_write_guard` | `id`, `resource`, `body` (`[StructuredStmt]`), optional `req` | Auto-inserts `rwlock_write(resource)` on entry and `rwlock_unlock(resource)` on normal exit AND before any nested `return` (RAII discipline, LIFO unwinding). |
| `if_else` | `id`, `cond`, `then_body`, optional `else_body`, optional `req` | Structured conditional branch with automatic merge. `cond` MUST be a string expression (e.g. `"count < 2"` or `"!ready"`), NEVER a JSON object. |
| `while_loop` | `id`, `cond`, `body`, optional `req` | Bounded loop; ensure the loop condition terminates. `cond` MUST be a string expression. |
| `while_wait` | `id`, `cond`, `condvar`, `lock`, optional `body`, optional `req` | Predicate-guarded wait loop: while `cond` holds, executes optional `body` and `condvar_wait(condvar, lock)` (must be inside `lock_guard` on `lock`). `cond` MUST be a string expression (e.g. `"!ready"` or `"ready == false"`), NEVER a JSON object. Example: `{"id": "c2", "kind": "while_wait", "cond": "!ready", "condvar": "main::not_empty", "lock": "main::queue_lock", "body": []}`. |
| `stmt` | `id`, `op` (or embedded flat statement fields: `assign_local`, `write_shared`, `read_shared`, `mutex_lock`, `mutex_unlock`, `rwlock_read`, `rwlock_write`, `rwlock_unlock`, `condvar_notify`, `condvar_notify_all`, `channel_send`, `channel_recv`, `semaphore_acquire`, `semaphore_release`, `atomic_load`, `atomic_store`, `atomic_cas`, `call`, `spawn`, `join`, `scope`, `return`, `seq_hole`, `nop`), optional `req` | Emits one flat CIR statement. |

## 3. Domain Types (`types`), `TypeHole`, `Struct`, and `Array`

Modules may declare `"types": [...]` with concrete `Struct`/`Array` definitions or opaque
`TypeHole`s filled during Stage 3 in `src/domain.rs`:

```json
"types": [
  {
    "name": "OrderBook",
    "type": {
      "Hole": {
        "description": "In-memory price-level order book with pure insert/best_bid methods",
        "req": ["R4"]
      }
    }
  },
  {
    "name": "Header",
    "type": {
      "Struct": {
        "seq": "Int",
        "valid": "Bool"
      }
    }
  }
]
```

A `Mutex` or `RwLock` resource may optionally attach a payload type via `"base": "OrderBook"` (or
inline `{"Struct": ...}` / `{"Array": ...}`), defaulting to `()` when omitted.

## 4. First-Class `seq_hole`

Use `seq_hole` for pure sequential computation that updates local or lock-protected shared state:

```json
{
  "id": "b_inc",
  "kind": "stmt",
  "req": ["R2"],
  "op": {
    "kind": "seq_hole",
    "id": "h_inc",
    "desc": "increment bounded counter",
    "reads": ["main::c"],
    "writes": ["main::c"],
    "updates_modeled": [
      { "target": "main::c", "expr": "c + 1" }
    ],
    "expected_type": "()",
    "req": ["R2"]
  }
}
```

Rules for `seq_hole`:

- Every `target` in `updates_modeled` must be a writable local or shared `Var`. If writing a
  lock-protected `Var`, the `seq_hole` must execute inside `lock_guard` (or while holding that lock).
- Hole implementations in Rust are statically verified to be **pure** (no locks, threads, atomics,
  channels, `unsafe`, or unbounded loops). Any synchronization MUST live in CIR, never inside a hole.

## 5. Resources (`Mutex`, `RwLock`, `Condvar`, `Semaphore`, `Channel`, `Var`, `Atomic`)

| type | fields |
| --- | --- |
| `Mutex` | `name`, `kind`=`"sync"`, `type`=`"Mutex"`, `mode`=`"Sync"`, optional `base` (payload type), optional `req` |
| `RwLock` | `name`, `kind`=`"sync"`, `type`=`"RwLock"`, `mode`=`"Sync"`, optional `base` (payload type), optional `req` |
| `Condvar` | `name`, `kind`=`"sync"`, `type`=`"Condvar"`, `mode`=`"Sync"`, optional `req` |
| `Semaphore` | `name`, `kind`=`"sync"`, `type`=`"Semaphore"`, `mode`=`"Sync"`, optional `count`, optional `req` |
| `Channel` | `name`, `kind`=`"sync"`, `type`=`"Channel"`, `mode`=`"Sync"`, `base`, `capacity`, optional `req` |
| `Var` / `Atomic` | `name`, `kind`=`"var"`, `type`, `base` (`"Int"` / `"Bool"` / bounded `{"Int": [lo, hi]}`), `init`, optional `req` |

For `RwLock`, prefer structured `"rwlock_read_guard"` / `"rwlock_write_guard"` blocks (which
auto-emit `rwlock_read`/`rwlock_write` on entry and LIFO `rwlock_unlock` on normal exit and early
`return`), or use flat ops `rwlock_read`, `rwlock_write`, and `rwlock_unlock` with
`"resource": "<mod>::<rwlock>"` (emitted in MIR/refinement nets as `rwlock_read_lock`,
`rwlock_read_unlock`, `rwlock_write_lock`, and `rwlock_write_unlock`).

- Declare `protection`: `[{"var": "<var>", "lock": "<lock>"}]` for every shared `Var` guarded by a `Mutex` or `RwLock`.
- Maintain a single global lock acquisition order across all modules and threads (`concir-backend summarize` checks cross-module lock cycles).
- Every spawned/scoped worker must have a reachable `return`.

## 6. Multi-Module SCIR & Compositional Verification

Unless multiple modules are explicitly requested, put all resources, types, and functions in module
`"main"` so all names match `"main::<name>"` in the contract.
Large programs may split responsibilities across multiple entries in `"modules"` (e.g., `main`,
`storage`, `worker`). Declare cross-module interfaces explicitly via `"provides"` and `"requires"`
(`resources` and `functions`) and reference external entities with FQN `<mod>::<name>`.
Codegen emits non-entry modules to `src/<mod>.rs` and enables per-module MIR refinement
(`--modular-refinement`).

## 7. Example: SCIR with `lock_guard`, `seq_hole`, `TypeHole`, and `req` Traceability

```json
{
  "program": "bounded_counter",
  "version": "4.0.0",
  "entry": "main::main",
  "modules": [
    {
      "name": "main",
      "req": ["R1", "R2"],
      "provides": { "resources": ["m", "c"], "functions": ["main", "w1", "w2"] },
      "requires": { "resources": [], "functions": [] },
      "types": [
        {
          "name": "MetricsPayload",
          "type": { "Hole": { "description": "Pure metrics accumulator struct", "req": ["R2"] } }
        }
      ],
      "resources": [
        { "name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "base": "MetricsPayload", "req": ["R1"] },
        { "name": "c", "kind": "var", "type": "Var", "base": {"Int": [0, 2]}, "init": 0, "req": ["R2"] }
      ],
      "protection": [{ "var": "c", "lock": "m" }],
      "functions": [
        {
          "name": "w1",
          "kind": "normal",
          "form": "closure",
          "req": ["R2"],
          "structured_body": [
            {
              "id": "b1",
              "kind": "lock_guard",
              "resource": "main::m",
              "req": ["R1"],
              "body": [
                {
                  "id": "b2",
                  "kind": "stmt",
                  "req": ["R2"],
                  "op": {
                    "kind": "seq_hole",
                    "id": "h_inc_1",
                    "desc": "increment shared counter",
                    "reads": ["main::c"],
                    "writes": ["main::c"],
                    "updates_modeled": [{ "target": "main::c", "expr": "c + 1" }],
                    "expected_type": "()",
                    "req": ["R2"]
                  }
                }
              ]
            },
            { "id": "b3", "kind": "stmt", "op": { "kind": "return" } }
          ]
        },
        {
          "name": "w2",
          "kind": "normal",
          "form": "closure",
          "req": ["R2"],
          "structured_body": [
            {
              "id": "b1",
              "kind": "lock_guard",
              "resource": "main::m",
              "req": ["R1"],
              "body": [
                {
                  "id": "b2",
                  "kind": "stmt",
                  "req": ["R2"],
                  "op": {
                    "kind": "seq_hole",
                    "id": "h_inc_2",
                    "desc": "increment shared counter",
                    "reads": ["main::c"],
                    "writes": ["main::c"],
                    "updates_modeled": [{ "target": "main::c", "expr": "c + 1" }],
                    "expected_type": "()",
                    "req": ["R2"]
                  }
                }
              ]
            },
            { "id": "b3", "kind": "stmt", "op": { "kind": "return" } }
          ]
        },
        {
          "name": "main",
          "kind": "normal",
          "structured_body": [
            { "id": "b1", "kind": "stmt", "op": { "kind": "scope", "funcs": ["main::w1", "main::w2"] } },
            { "id": "b2", "kind": "stmt", "op": { "kind": "return" } }
          ]
        }
      ]
    }
  ]
}
```

## 8. Responding to Vacuity & Cross-Layer Escalation (Loop A / Loop C)

- If `<verification_feedback>` reports `"vacuous_properties"`, one or more CTL/LTL implication
  antecedents were never reached; revise the model so the antecedent scenario is genuinely reachable
  without violating safety.
- If `<verification_feedback>` contains `"stage": "cross_layer_escalation"`, the Rust hole-filling
  stage discovered that the current CIR/SCIR architecture cannot express a requirement cleanly without
  violating hole purity or stuttering refinement (e.g., an extra shared resource, lock, `RwLock`,
  condvar, or channel is needed). Revise the CIR/SCIR program to incorporate the needed
  synchronization primitive while keeping the contract satisfied, and output the complete updated JSON.
