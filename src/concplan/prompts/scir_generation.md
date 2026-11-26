# ConcIR / SCIR v4.0 Generation Prompt (v5)

You design one verifiable ConcIR / Structured ConcIR (`SCIR`) program as a single JSON object.
A separate formal checker (`concir-backend`) lowers, validates, explores, and decides acceptance.
You may revise after model-checking feedback (Loop A) or cross-layer escalation from Rust hole
implementation (Loop C). Output **only** the JSON object.

## 1. Core Principles

- **Model concurrency, not business logic.** Represent threads, locks, condition variables,
  semaphores, channels, atomics, and the shared variables (`Var`) that guard synchronization.
- **Delegate pure sequential computation to `seq_hole`.** Put non-trivial sequential computation
  inside a `seq_hole` with `reads`, `writes`, `updates_modeled`, `expected_type`, and `req` tags.
  The backend verifies the concurrency skeleton exhaustively, emits a Rust skeleton with typed
  `/* HOLE(id) expected: T */` placeholders, and a later stage fills and statically verifies the holes.
- **Traceability (`req`).** Attach `"req": ["R1", ...]` tags on modules, resources, functions,
  `structured_body` blocks, and statements to link every design element back to requirement clause IDs.
- **Prefer `structured_body` (`SCIR`) over manual `branch`/`goto` `sid` wiring.**
  `concir-backend lower` deterministically lowers `structured_body` to flat `body` statements and
  emits `concir-trace-map/1` preserving every `structured_id` and `req` tag.

## 2. Structured Function Body (`structured_body`)

A function may provide `structured_body` (instead of flat `body`). Every `StructuredStmt` carries:
- `"id"`: unique block identifier within the function (e.g. `"b1"`, `"b2"`),
- `"kind"`: one of the five structured kinds below,
- optional `"req"`: list of requirement clause IDs (e.g. `["R2"]`).

| kind | fields | lowered semantics |
| --- | --- | --- |
| `lock_guard` | `id`, `resource`, `body` (`[StructuredStmt]`), optional `req` | Auto-inserts `mutex_lock(resource)` on entry and `mutex_unlock(resource)` on normal exit AND before any nested `return` (RAII discipline). |
| `if_else` | `id`, `cond`, `then_body`, optional `else_body`, optional `req` | Structured conditional branch with automatic merge. |
| `while_loop` | `id`, `cond`, `body`, optional `req` | Bounded loop; ensure the loop condition terminates. |
| `while_wait` | `id`, `cond`, `condvar`, `lock`, optional `body`, optional `req` | Predicate-guarded wait loop: while `cond` holds, executes optional `body` and `condvar_wait(condvar, lock)` (must be inside `lock_guard` on `lock`). |
| `stmt` | `id`, `op` (or embedded flat statement fields: `assign_local`, `write_shared`, `read_shared`, `condvar_notify`, `condvar_notify_all`, `channel_send`, `channel_recv`, `semaphore_acquire`, `semaphore_release`, `atomic_load`, `atomic_store`, `atomic_cas`, `call`, `spawn`, `join`, `scope`, `return`, `seq_hole`, `nop`), optional `req` | Emits one flat CIR statement. |

## 3. First-Class `seq_hole`

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

## 4. Resources & Protection

| type | fields |
| --- | --- |
| `Mutex` | `name`, `kind`=`"sync"`, `type`=`"Mutex"`, `mode`=`"Sync"`, optional `req` |
| `Condvar` | `name`, `kind`=`"sync"`, `type`=`"Condvar"`, `mode`=`"Sync"`, optional `req` |
| `Semaphore` | `name`, `kind`=`"sync"`, `type`=`"Semaphore"`, `mode`=`"Sync"`, optional `count`, optional `req` |
| `Channel` | `name`, `kind`=`"sync"`, `type`=`"Channel"`, `mode`=`"Sync"`, `base`, `capacity`, optional `req` |
| `Var` / `Atomic` | `name`, `kind`=`"var"`, `type`, `base` (`"Int"` / `"Bool"`), `init`, optional `req` |

- Declare `protection`: `[{"var": "<var>", "lock": "<lock>"}]` for every shared `Var` guarded by a `Mutex`.
- Maintain a single global lock acquisition order across all modules and threads (`concir-backend summarize` checks cross-module lock cycles).
- Every spawned/scoped worker must have a reachable `return`.

## 5. Example: SCIR with `lock_guard`, `seq_hole`, and `req` Traceability

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
      "resources": [
        { "name": "m", "kind": "sync", "type": "Mutex", "mode": "Sync", "req": ["R1"] },
        { "name": "c", "kind": "var", "type": "Var", "base": "Int", "init": 0, "req": ["R2"] }
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

## 6. Example: Predicate-Guarded Wait with `while_wait`

```json
{
  "id": "b_guard",
  "kind": "lock_guard",
  "resource": "main::m",
  "req": ["R3"],
  "body": [
    {
      "id": "b_wait",
      "kind": "while_wait",
      "cond": "ready == false",
      "condvar": "main::cv",
      "lock": "main::m",
      "req": ["R3"]
    }
  ]
}
```

## 7. Responding to Cross-Layer Escalation (Loop C)

If `<verification_feedback>` contains `"stage": "cross_layer_escalation"`, the Rust hole-filling
stage discovered that the current CIR/SCIR architecture cannot express a requirement cleanly without
violating hole purity or stuttering refinement (e.g., an extra shared resource, lock, condvar, or
channel is needed). Revise the CIR/SCIR program to incorporate the needed synchronization primitive
while keeping the contract satisfied, and output the complete updated JSON object.
