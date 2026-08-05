# Translation and Validation Error Codes

> Version 0.1.0 — Last updated 2026-03-16

## T0xx — Invalid CIR Input

| Code | Error | Description |
|------|-------|-------------|
| T001 | `MissingEntry` | The program's `entry` field references a function that does not exist |
| T002 | `EmptyEntryBody` | The entry function has an empty body (no statements) |
| T003 | `UnknownFunction` | A spawn/join/call references a function that is neither defined nor summarized |

## T1xx — Resource Translation Errors

| Code | Error                 | Description                                                            |
|------|-----------------------|------------------------------------------------------------------------|
| T101 | `UnknownResourceType` | Unrecognized resource `kind`/`type` combination                        |
| T102 | `CondvarLockNotFound` | Condvar `wait` references a lock that does not exist in resources      |
| T103 | `CondvarLockNotMutex` | Condvar `wait` references a lock that is neither a Mutex nor an RwLock |

## T2xx — Control-Flow Translation Errors

| Code | Error | Description |
|------|-------|-------------|
| T201 | `InvalidTarget` | A transfer target sid does not exist in the function body |
| T202 | `InvalidBranchCondition` | Branch condition string cannot be parsed into a BoolExpr |
| T203 | `SwitchNotEnum` | Switch variable is not an Enum type |

## T3xx — Consistency Errors

| Code | Error | Description |
|------|-------|-------------|
| T301 | `AmbiguousRwLockDrop` | Cannot determine whether a RwLock drop releases a read-lock or write-lock |
| T302 | `NoWaitSites` | A condvar notify/notify_all has no corresponding wait-sites |

## E3xx — CIR Validation Errors

These errors are emitted by the CIR validator before translation. The complete resource compatibility matrix is
maintained in [`../cir/README.md`](../cir/README.md).

| Code | Error                   | Description                                                                |
|------|-------------------------|----------------------------------------------------------------------------|
| E310 | `UnknownResourceAction` | A `res_op` uses an action that is not part of the canonical CIR action set |
| E311 | `ResourceActionArity`   | A canonical `res_op` action has a missing or extra argument                |

## E4xx — Concurrency Pairing (additions)

| Code | Severity | Description |
|------|----------|-------------|
| E409 | error    | `call` targets a bodied function whose body contains synchronization operations (`res_op`/`spawn`/`join`/`call`/…). Calls are translated as one atomic transition, so the callee's locking behavior would be silently dropped from the model — a cross-function lock chain that deadlocks in real code would go unreported. Inline the callee or replace its body with a `fn_summary`. |
| E410 | warning  | `call` targets a bodied function (pure computation). The body is not executed by the model; declare a `fn_summary` to document its reads/writes. |

## Builder Errors

Errors from `CvnNetBuilder::build()` are wrapped as `BuilderError(message)`. These indicate well-formedness violations in the generated CVN (e.g. missing places, disconnected transitions).
