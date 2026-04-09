---
name: CVN TransitionKind Refactor
overview: 细化 CVN 的 TransitionKind 枚举，为 Semaphore（Acquire/Release）、RwLock 读锁（ReadLock/ReadUnlock）、Condvar 重获取（CondvarReacquire）、Var/Atomic 读写（VarRead/AtomicStore）添加独立变体，并同步更新翻译器、DOT 导出和测试。
todos:
  - id: cvn-enum
    content: 在 cvn/src/model/transition.rs 添加 7 个新 TransitionKind 变体
    status: completed
  - id: cvn-dot
    content: 重写 cvn/src/export.rs 的变迁样式映射为按 kind 着色
    status: completed
  - id: translator-op
    content: 更新 src/translator/operation.rs：Semaphore→Acquire/Release, RwLock read→ReadLock/ReadUnlock, read+next→VarRead, store→AtomicStore
    status: completed
  - id: translator-condvar
    content: 更新 src/translator/condvar.rs：reacquire 用 CondvarReacquire
    status: completed
  - id: tests
    content: 更新测试断言（semaphore tid/kind），重新生成 CVN DOT 文件
    status: completed
isProject: false
---

# CVN TransitionKind 命名细化

## 变更概述

当前 `TransitionKind` 存在复用：Semaphore acquire/release 用 `Lock/Unlock`，RwLock 读锁用 `Lock`，Condvar reacquire 用 `Lock`，Var/Atomic read+next 用 `Sequential`。需添加 6 个新变体使每种 CIR 操作有独立标签。

## 新增变体（6 个）

在 [cvn/src/model/transition.rs](cvn/src/model/transition.rs) 的 `TransitionKind` 中添加：

- `ReadLock` — RwLock.read（读锁获取）
- `ReadUnlock` — RwLock.drop（读锁释放）
- `Acquire` — Semaphore.acquire
- `Release` — Semaphore.release
- `VarRead` — Var.read/Atomic.load + next（替代 Sequential）
- `AtomicStore` — Atomic.store（替代 VarWrite）
- `CondvarReacquire` — wait 被唤醒后重新获取锁（替代 Lock）

保留现有变体不变（不删除），仅将复用的场景改为新变体。

## 文件变更清单

### CVN 库（cvn/）

**[cvn/src/model/transition.rs](cvn/src/model/transition.rs)** — 枚举定义

- 在 `Unlock` 之后添加 `ReadLock`、`ReadUnlock`
- 在 `Recv` 之后添加 `Acquire`、`Release`
- 在 `VarWrite` 之后添加 `VarRead`、`AtomicStore`
- 在 `CondvarNotifyAll` 之后添加 `CondvarReacquire`

**[cvn/src/validate.rs](cvn/src/validate.rs)** — 良构性检查

- `check_control_input_arcs`（第 145-149 行）：`CondvarReacquire` 允许 2 个 control input（和 Lock 一样需要 control + resource arc，但 resource arc 不是 control place 所以无需改动）— 实际上无需修改，CondvarReacquire 仍只有 1 个 control input arc + 1 个 resource input arc。

**[cvn/src/export.rs](cvn/src/export.rs)** — DOT 导出

- 替换第 86-93 行的通用 `Debug` 格式为专用样式映射（按 `TransitionKind` 变体设置不同颜色/边框）

### cir2cvn 翻译器（src/）

**[src/translator/operation.rs](src/translator/operation.rs)**

- `translate_lock`（第 204 行）：Semaphore 时用 `Acquire`（需按 `res_kind` 区分），Mutex/RwLock 写锁保持 `Lock`
- `translate_rw_read_lock`（第 227 行）：`Lock` → `ReadLock`
- `translate_drop`（第 266 行）：Semaphore 时用 `Release`，RwLock 读锁时用 `ReadUnlock`，Mutex/RwLock 写锁保持 `Unlock`
- `translate_read`（第 293-340 行）：read+next 时 `Sequential` → `VarRead`
- `translate_store`（第 434-436 行）：不再委托 `translate_write`，改用 `AtomicStore`

**[src/translator/condvar.rs](src/translator/condvar.rs)**

- 第 75 行：`t_cv_reacquire` 的 `TransitionKind::Lock` → `TransitionKind::CondvarReacquire`

### 测试

**[tests/category2_resource/test_semaphore.rs](tests/category2_resource/test_semaphore.rs)**

- 第 14 行：transition ID `"main_s1_lock"` → `"main_s1_acquire"`
- 第 15 行：transition ID `"main_s2_unlock"` → `"main_s2_release"`
- 添加 `TransitionKind::Acquire` / `Release` 的 kind 断言

**DOT 快照和生成文件** — 重新生成

- `cir/tests/snapshots/` 下的 4 个快照不受影响（CIR DOT 不涉及 TransitionKind）
- `dots/cvn/` 下的生成文件运行 `generate_dots` 重新生成

## TransitionKind → DOT 样式映射

`cvn/src/export.rs` 中替换通用 Debug 格式为：

```
kind         | label 示例             | fillcolor   | color    | style
─────────────┼───────────────────────┼─────────────┼──────────┼───────
Sequential   | "sequential"          | gray90      | black    | filled
Lock         | "lock"                | gray90      | red      | filled
Unlock       | "unlock"              | gray90      | green    | filled
ReadLock     | "read_lock"           | gray90      | blue     | filled
ReadUnlock   | "read_unlock"         | gray90      | teal     | filled
Acquire      | "acquire"             | gray90      | red      | "filled,dashed"
Release      | "release"             | gray90      | green    | "filled,dashed"
Send         | "send"                | gray90      | cyan4    | filled
Recv         | "recv"                | gray90      | cyan4    | "filled,bold"
VarRead      | "var_read"            | gray90      | black    | filled
VarWrite     | "var_write"           | gray90      | orange   | filled
AtomicStore  | "atomic_store"        | gray90      | orange   | "filled,bold"
BranchTrue   | "branch_T"           | gray90      | green    | filled
BranchFalse  | "branch_F"           | gray90      | red      | filled
Switch       | "switch(Init)"       | gray90      | orange   | filled
CasSuccess   | "cas_succ"            | gray90      | green    | filled
CasFailure   | "cas_fail"            | gray90      | red      | filled
Spawn        | "spawn"               | gray90      | blue     | filled
Join         | "join"                | gray90      | blue     | "filled,dashed"
Call         | "call"                | gray90      | black    | "filled,rounded"
CondvarWait  | "cv_wait"             | gray90      | purple   | filled
CondvarNotify| "cv_notify"           | gray90      | purple   | "filled,dashed"
CondvarNotifyAll | "cv_notify_all"   | gray90      | purple   | "filled,dashed"
CondvarReacquire | "cv_reacquire"    | gray90      | purple   | "filled,dotted"
Return       | "return"              | gray90      | black    | filled
```

## 不变的部分

- CVN 使能/发火语义不变（TransitionKind 不影响语义）
- 弧的 weight / guard / update 不变
- 良构性条件 W1-W9 不变
- `Transition::is_return()` 不变
- CIR DOT 导出不受影响
