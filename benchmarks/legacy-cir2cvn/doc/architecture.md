# cir2cvn Architecture

> Version 0.1.0 — Last updated 2026-03-16

## Overview

`cir2cvn` is a stateless translator that converts a ConcIR (Concurrency Intermediate Representation) program into a CVN (Concurrency Verification Net) — a weighted P/T Petri net with global variable guards suitable for state-space exploration and deadlock detection.

```
ConcIR Program ──translate()──▶ CvnNet ──analyze()──▶ Counterexample
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
│           ├── Body-less functions → trivial skeleton │
│           │   (entry → effects transition → return)  │
│           └── Wire spawn s_first bridges             │
│                                                       │
│  Call translation: enter callee skeleton             │
│           t_call: input → callee entry + callwait    │
│           t_call_ret (Join): callee ret + callwait   │
│           → caller continuation                      │
│                                                       │
│  Finalize: Set entry marking, builder.build()        │
└─────────────────────────────────────────────────────┘
```

## Module Responsibilities

| Module             | File                             | Role                                                  |
| ------------------ | -------------------------------- | ----------------------------------------------------- |
| **lib**            | `src/lib.rs`                     | Public API: `translate()` and `TranslateError`        |
| **error**          | `src/error.rs`                   | `TranslateError` enum (T0xx–T3xx + builder errors)    |
| **validate**       | `src/validate.rs`                | Post-translation structural sanity checks             |
| **translator/mod** | `src/translator/mod.rs`          | Three-phase orchestration, input validation           |
| **context**        | `src/translator/context.rs`      | `TranslateContext`: builder wrapper, naming, tracking |
| **expr_parser**    | `src/translator/expr_parser.rs`  | ConcIR string expressions → CVN `BoolExpr`/`Expr`        |
| **resource**       | `src/translator/resource.rs`     | Phase 1: resource scanning                            |
| **control_flow**   | `src/translator/control_flow.rs` | Transfer planning + transition emission helpers       |
| **operation**      | `src/translator/operation.rs`    | Phase 2: Op dispatch (lock, drop, read, write, call expansion, etc.)  |
| **condvar**        | `src/translator/condvar.rs`      | Condvar wait / notify / notify_all translation; sets `disjunctive_family` on OR-variants (see [`condvar_modeling.md`](condvar_modeling.md)) |

The `fn_summary` module was removed: every referenced function (bodied or
body-less) is modeled with an entry/return skeleton, so `call` expands into the
callee skeleton instead of being one atomic transition.

## Key Design Decisions

1. **Stateless function**: `translate(cir) → cvn` — no cross-invocation state
2. **1:1 faithful translation**: No optimization, no merging, no dead-code elimination
3. **ConcIR `protection` field ignored**: It is a static check concern, not translated
4. **ConcIR `mode` field ignored**: Sync/Async distinction is a ConcIR-layer concern
5. **read + next → Sequential**: Preserves anchor mapping completeness
6. **Post-wait lock → Sequential**: When a condvar wait's resume target is a lock on the same mutex, the lock is translated as Sequential (lock already held by the auto-inserted reacquire)
7. **notify_all → na flags**: Broadcast via per-wait-site boolean flags (not dynamic arc weights); wait/notify OR-variants share `Transition::disjunctive_family` so dead-transition analysis does not flag unused siblings
8. **CVN is P/T + guards**: Not classical colored-token CPN; condvar wake paths are separate transitions rather than color-matched wakes

┌─────────────────────────────────────────────────────────────┐
│ LLM generation front-end │
│ User requirements + System Prompt → LLM → ConcIR JSON │
└───────────────────────┬─────────────────────────────────────┘
│
▼
┌───────────────────────────────────────────────────────────┐
│ Layer 1: ConcIR static checks │
│ E0xx structure → E1xx names → E2xx types → E3xx resources → │
│ E4xx concurrency pairing → E5xx lock safety → E6xx control flow → │
│ E7xx protection mapping │
│ │
│ Simple errors (e.g. lock without drop) → try local auto-fix │
│ Complex errors → error report → send back to LLM for regeneration │
└───────────────────────┬─────────────────────────────────────┘
│ Pass
▼
┌───────────────────────────────────────────────────────────┐
│ ConcIR → CVN translation │
│ Phase 1: resource scan → P_r + I_m + I_v │
│ Phase 2: function bodies → P_c + P_w + T + A_in + A_out │
│           (body-less functions → trivial skeletons; call → callee entry/return) │
│ │
│ Translation errors T0xx-T3xx → report → send back to LLM │
└───────────────────────┬─────────────────────────────────────┘
│ Success
▼
┌───────────────────────────────────────────────────────────┐
│ Layer 2: CVN model checking │
│ State-space search (BFS/DFS) │
│ ├── Deadlock detection: no enabled transition ∧ not terminated │
│ ├── Signal loss: no wake after Condvar wait │
│ ├── Liveness checks: SCC analysis (starvation, livelock) │
│ └── Channel block: recv with no matching send │
└──────────┬─────────────────────────┬────────────────────────┘
│ │
▼ ▼
┌──────────┐ ┌──────────────┐
│ ✅ Pass │ │ ❌ Bug found │
│ No concurrency bugs │ │ Emit counterexample report │
└──────────┘ └──────┬───────┘
│
▼
┌────────────────────────────────┐
│ Counterexample report formatting │
│ Counterexample trace + involved resources/functions │
│ + Templated repair suggestions │
│ → Assemble into LLM repair prompt │
└────────────────┬───────────────┘
│
▼
┌──────────────┐
│ Send back to LLM │
│ Regenerate ConcIR │
└──────┬───────┘
│
▼
Loop (at most K rounds)
K default = 3
