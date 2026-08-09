You are a concurrency bug repair expert. You receive a CIR (Concurrency Intermediate Representation) JSON artifact that contains a concurrency bug, along with a bug report from a model checker. Fix the CIR according to the repair suggestion and output the complete fixed CIR JSON. Output only JSON, no explanatory text.

# CIR Language Reference

CIR is a statement-level, verification-oriented concurrency model. Each CIR artifact is a JSON object with the following schema.

## Top-level Structure

```json
{
  "program": "<name>",
  "resources": ["..."],
  "protection": ["..."],
  "functions": ["..."],
  "entry": "<entry function name>",
  "goals": ["..."]
}
```

## Resources

Each resource is a named synchronization primitive or data variable.

| Field | Required                   | Description                                                                   |
| ----- | -------------------------- | ----------------------------------------------------------------------------- |
| name  | yes                        | Globally unique resource name                                                 |
| kind  | yes                        | `"sync"` (synchronization primitive) or `"var"` (data variable)               |
| type  | yes                        | One of: `Mutex`, `RwLock`, `Semaphore`, `Channel`, `Condvar`, `Var`, `Atomic` |
| mode  | sync resources             | `"Sync"` or `"Async"`; required for every sync resource                       |
| count | `Semaphore`                | Initial permit count; required for `Semaphore`                                |
| base  | `Channel`, `Var`, `Atomic` | Channel payload/base type, or variable base type                              |
| init  | `Var`, `Atomic`            | Initial value; required for `Var` and `Atomic`                                |

Examples:

```json
{"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}
{"name": "rw",  "kind": "sync", "type": "RwLock", "mode": "Sync"}
{"name": "sem", "kind": "sync", "type": "Semaphore", "mode": "Sync", "count": 3}
{"name": "ch",  "kind": "sync", "type": "Channel", "mode": "Sync", "base": "Int"}
{"name": "cv",  "kind": "sync", "type": "Condvar", "mode": "Sync"}
{"name": "ready", "kind": "var", "type": "Var", "base": "Bool", "init": false}
{"name": "counter", "kind": "var", "type": "Atomic", "base": "Int", "init": 0}
```

## Protection Mapping

Declares which lock protects which variable. Only `Var` resources (not `Atomic`) need protection.

```json
{ "var": "ready", "lock": "mtx" }
```

## Functions

Each function has a `name`, `kind` (`"normal"`, `"async"`, or `"closure"`), and a `body` array of statements.

A function with `"body": []` is a body-less ("nobody") function: no control flow,
no callsites. It may carry an optional `"effects": { "reads": [...], "writes": [...] }`
naming declared resources a computation touches (writes are modeled as unknown in
the CVN).

### Statement

Each statement is `{ "sid": "...", "op": ..., "transfer": ... }`.

- `sid`: Unique identifier within the function (format: `"s1"`, `"s2"`, ...).
- `op`: The operation (see below).
- `transfer`: The successor logic (see below).

### Operations (`op`)

Resource operations use the array format `["res_op", "<resource>", "<action>", ...args]`:

All operation arrays have exact tuple shapes. The argument count is the number of elements after the action name:

| Action       | Args         | Resource Types | Description                                                 |
| ------------ | ------------ | -------------- | ----------------------------------------------------------- |
| `lock`       | 0            | Mutex, RwLock  | Acquire exclusive lock                                      |
| `drop`       | 0            | Mutex, RwLock  | Release lock                                                |
| `read`       | 0            | RwLock, Var    | Acquire a read lock or read a variable                      |
| `wait`       | 1 mutex name | Condvar        | Wait on condvar and associate it with a Mutex/RwLock        |
| `notify`     | 0            | Condvar        | Wake one waiter                                             |
| `notify_all` | 0            | Condvar        | Wake all waiters                                            |
| `acquire`    | 0            | Semaphore      | Acquire semaphore permit                                    |
| `release`    | 0            | Semaphore      | Release semaphore permit                                    |
| `send`       | 1 value      | Channel        | Send to channel                                             |
| `recv`       | 0            | Channel        | Receive from channel                                        |
| `write`      | 1 value      | Var            | Write variable (for example `"true"` or `"42"`)             |
| `load`       | 0            | Atomic         | Atomic load                                                 |
| `store`      | 1 value      | Atomic         | Atomic store                                                |
| `cas`        | 2 values     | Atomic         | Compare-and-swap (expected, desired); use `branch` transfer |

For example, a valid condvar wait is `["res_op", "cv", "wait", "mtx"]`; the fourth element must name a declared Mutex or
RwLock. Unknown actions and missing or extra arguments are validation errors.

Control operations:

| Format                    | Description                                   |
| ------------------------- | --------------------------------------------- |
| `["spawn", "<fn>"]`       | Spawn a new OS thread running function `<fn>` |
| `["spawn_async", "<fn>"]` | Spawn an async task                           |
| `["join", "<fn>"]`        | Wait for spawned thread to complete           |
| `["await", "<fn>"]`       | Await an async task                           |
| `["call", "<fn>"]`        | Synchronous function call                     |
| `"return"`                | Return from function (string, not array)      |
| `"nop"`                   | No operation                                  |

### Transfer (Successor Logic)

| Format                                                       | Description                         |
| ------------------------------------------------------------ | ----------------------------------- |
| `["next", "<sid>"]`                                          | Go to next statement                |
| `["branch", "<cond>", "<true_sid>", "<false_sid>"]`          | Conditional branch                  |
| `["switch", "<var>", {"val1": "sid1", "val2": "sid2", ...}]` | Multi-way branch                    |
| `"return"`                                                   | Function return (string, not array) |

### Business Goals

Optional post-conditions for semantic regression prevention:

```json
{
  "id": "G1",
  "desc": "Both threads complete, lock released",
  "marking": { "worker.ret": 1, "mtx": 1 },
  "variables": { "ready": true }
}
```

`marking` keys must be a declared resource name, a control-place reference in the
form `function.sid`, or a raw CVN place id beginning with `cp_`, `rp_`, `wp_`, or
`ra_`. The display forms `cp(worker, ret)` and `rp(mtx)` are not valid CIR keys.
Resource and control-place keys express a minimum token count; a zero count on a
Channel or Condvar checks that its pending-token place is empty. `variables` contains
CVN variable names and JSON scalar values.

## Key Rules

1. Every `lock` must have a matching `drop` in the same function.
2. Every `spawn` must have a matching `join` (or `spawn_async`/`await`).
3. `sid` values must be unique within each function and use the `"s" + number` format.
4. Every `next` transfer must reference a valid `sid` in the same function.
5. The last reachable statement should have `"op": "return"` and `"transfer": "return"`.
6. Do not add or remove resources unless the fix requires it.
7. Condvar `wait` requires the associated mutex name as an extra argument: `["res_op", "cv", "wait", "mtx"]`. Do not add a `paired_with` resource field.
8. For `cas`, use `branch` transfer: the true branch is "CAS succeeded", false is "CAS failed".
9. `Var` resources accessed without holding their protecting lock will be flagged as errors.
10. The output must be a complete, valid CIR JSON — do not omit any function or resource.
11. `res_op` action names and argument counts are strict; never emit unknown actions,
    extra arguments, or omit required arguments.
