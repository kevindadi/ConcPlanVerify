# Translation and Validation Error Codes

> Version 0.1.0 — Last updated 2026-03-16

## T0xx — Invalid ConcIR Input

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

## ConcIR Validation Errors (E0xx–E7xx)

These errors are emitted by the ConcIR validator before translation. The complete
resource compatibility matrix and error code reference are maintained in the
ConcIR repository (`cir/doc/error_codes.md`, with the grammar in
`cir/doc/syntax.md`), which is the canonical schema owner. The
translator surfaces them unchanged from `concir::validate::validate`.


## Builder Errors

Errors from `CvnNetBuilder::build()` are wrapped as `BuilderError(message)`. These indicate well-formedness violations in the generated CVN (e.g. missing places, disconnected transitions).
