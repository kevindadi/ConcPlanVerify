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

| Module             | File                             | Role                                                  |
| ------------------ | -------------------------------- | ----------------------------------------------------- |
| **lib**            | `src/lib.rs`                     | Public API: `translate()` and `TranslateError`        |
| **error**          | `src/error.rs`                   | `TranslateError` enum (T0xx–T3xx + builder errors)    |
| **validate**       | `src/validate.rs`                | Post-translation structural sanity checks             |
| **translator/mod** | `src/translator/mod.rs`          | Three-phase orchestration, input validation           |
| **context**        | `src/translator/context.rs`      | `TranslateContext`: builder wrapper, naming, tracking |
| **expr_parser**    | `src/translator/expr_parser.rs`  | CIR string expressions → CVN `BoolExpr`/`Expr`        |
| **resource**       | `src/translator/resource.rs`     | Phase 1: resource scanning                            |
| **control_flow**   | `src/translator/control_flow.rs` | Transfer planning + transition emission helpers       |
| **operation**      | `src/translator/operation.rs`    | Phase 2: Op dispatch (lock, drop, read, write, etc.)  |
| **condvar**        | `src/translator/condvar.rs`      | Condvar wait / notify / notify_all translation; sets `disjunctive_family` on OR-variants (see [`condvar_modeling.md`](condvar_modeling.md)) |
| **fn_summary**     | `src/translator/fn_summary.rs`   | FnSummary indexing for Phase 2 call translation       |

## Key Design Decisions

1. **Stateless function**: `translate(cir) → cvn` — no cross-invocation state
2. **1:1 faithful translation**: No optimization, no merging, no dead-code elimination
3. **CIR `protection` field ignored**: It is a static check concern, not translated
4. **CIR `mode` field ignored**: Sync/Async distinction is a CIR-layer concern
5. **read + next → Sequential**: Preserves anchor mapping completeness
6. **Post-wait lock → Sequential**: When a condvar wait's resume target is a lock on the same mutex, the lock is translated as Sequential (lock already held by the auto-inserted reacquire)
7. **notify_all → na flags**: Broadcast via per-wait-site boolean flags (not dynamic arc weights); wait/notify OR-variants share `Transition::disjunctive_family` so dead-transition analysis does not flag unused siblings
8. **CVN is P/T + guards**: Not classical colored-token CPN; condvar wake paths are separate transitions rather than color-matched wakes

┌─────────────────────────────────────────────────────────────┐
│ LLM 生成端 │
│ 用户需求 + System Prompt → LLM → CIR JSON │
└───────────────────────┬─────────────────────────────────────┘
│
▼
┌───────────────────────────────────────────────────────────┐
│ 第一层:CIR 静态检查 │
│ E0xx 结构 → E1xx 名称 → E2xx 类型 → E3xx 资源 → │
│ E4xx 并发配对 → E5xx 锁安全 → E6xx 控制流 → │
│ E7xx 保护映射 → E8xx FnSummary │
│ │
│ 简单错误(如 lock 缺 drop)→ 尝试本地自动修复 │
│ 复杂错误 → 错误报告 → 发回 LLM 重新生成 │
└───────────────────────┬─────────────────────────────────────┘
│ 通过
▼
┌───────────────────────────────────────────────────────────┐
│ CIR → CVN 翻译 │
│ 阶段 1:资源扫描 → P_r + I_m + I_v │
│ 阶段 2:函数体 → P_c + P_w + T + A_in + A_out │
│ 阶段 3:FnSummary → 原子变迁 │
│ │
│ 翻译错误 T0xx-T3xx → 报告 → 发回 LLM │
└───────────────────────┬─────────────────────────────────────┘
│ 成功
▼
┌───────────────────────────────────────────────────────────┐
│ 第二层:CVN 模型检验 │
│ 状态空间搜索(BFS/DFS) │
│ ├── 死锁检测:无使能变迁 ∧ 非终止 │
│ ├── 信号丢失:Condvar wait 后无人唤醒 │
│ ├── 活性检查:SCC 分析(饥饿、活锁) │
│ └── Channel 阻塞:recv 无对应 send │
└──────────┬─────────────────────────┬────────────────────────┘
│ │
▼ ▼
┌──────────┐ ┌──────────────┐
│ ✅ 通过 │ │ ❌ 发现 bug │
│ 无并发bug │ │ 生成反例报告 │
└──────────┘ └──────┬───────┘
│
▼
┌────────────────────────────────┐
│ 反例报告格式化 │
│ 反例 trace + 涉及资源/函数 │
│ + 模板化修复建议 │
│ → 组装为 LLM 修复 prompt │
└────────────────┬───────────────┘
│
▼
┌──────────────┐
│ 发回 LLM │
│ 重新生成 CIR │
└──────┬───────┘
│
▼
循环(最多 K 轮)
K 默认 = 3
