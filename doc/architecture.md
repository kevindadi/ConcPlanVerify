# cir2cvn Architecture

> Version 0.1.0 — Last updated 2026-03-16

## Overview

`cir2cvn` is a stateless translator that converts a CIR (Concurrency Intermediate Representation) program into a CVN (Concurrency Verification Net) — a weighted P/T Petri net with global variable guards suitable for state-space exploration and deadlock detection.

```
CIR Program ──translate()──▶ CvnNet ──analyze()──▶ Counterexample
```

## Three-Phase Translation Pipeline

```
┌─────────────────────────────────────────────────────┐
│                  translate(program)                   │
│                                                       │
│  Phase 0: Input validation (T0xx errors)             │
│           Index FnSummaries into context             │
│                                                       │
│  Phase 1: Resource scanning                          │
│           ├── Mutex / RwLock / Semaphore / Channel   │
│           │   → resource places + initial marking    │
│           ├── Condvar → register (places on demand)  │
│           └── Var / Atomic → variable store (V)      │
│                                                       │
│  Phase 2: Function body translation                  │
│           ├── Pre-scan condvar wait-sites            │
│           ├── For each function, for each statement: │
│           │   Op + Transfer → transitions + arcs     │
│           └── Wire spawn s_first bridges             │
│                                                       │
│  Phase 3: (Integrated into Phase 2)                  │
│           FnSummary calls → Call transitions with    │
│           writes set to Unknown                      │
│                                                       │
│  Finalize: Set entry marking, builder.build()        │
└─────────────────────────────────────────────────────┘
```

## Module Responsibilities

| Module | File | Role |
|--------|------|------|
| **lib** | `src/lib.rs` | Public API: `translate()` and `TranslateError` |
| **error** | `src/error.rs` | `TranslateError` enum (T0xx–T3xx + builder errors) |
| **validate** | `src/validate.rs` | Post-translation structural sanity checks |
| **translator/mod** | `src/translator/mod.rs` | Three-phase orchestration, input validation |
| **context** | `src/translator/context.rs` | `TranslateContext`: builder wrapper, naming, tracking |
| **expr_parser** | `src/translator/expr_parser.rs` | CIR string expressions → CVN `BoolExpr`/`Expr` |
| **resource** | `src/translator/resource.rs` | Phase 1: resource scanning |
| **control_flow** | `src/translator/control_flow.rs` | Transfer planning + transition emission helpers |
| **operation** | `src/translator/operation.rs` | Phase 2: Op dispatch (lock, drop, read, write, etc.) |
| **condvar** | `src/translator/condvar.rs` | Condvar wait / notify / notify_all translation |
| **fn_summary** | `src/translator/fn_summary.rs` | FnSummary indexing for Phase 2 call translation |

## Key Design Decisions

1. **Stateless function**: `translate(cir) → cvn` — no cross-invocation state
2. **1:1 faithful translation**: No optimization, no merging, no dead-code elimination
3. **CIR `protection` field ignored**: It is a static check concern, not translated
4. **CIR `mode` field ignored**: Sync/Async distinction is a CIR-layer concern
5. **read + next → Sequential**: Preserves anchor mapping completeness
6. **Post-wait lock → Sequential**: When a condvar wait's resume target is a lock on the same mutex, the lock is translated as Sequential (lock already held by the auto-inserted reacquire)
7. **notify_all → chain expansion**: Produces 2K transitions for K wait-sites (manageable for typical K ≤ 3)
