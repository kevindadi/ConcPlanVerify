---
name: CIR-to-CVN Translator
overview: 在 cpn-guide-llm 仓库根目录实现 cir2cvn 翻译器，将 CIR 中间表示翻译为 CVN（Coloured Verification Net）用于并发程序验证。翻译器分三个阶段执行：资源扫描、函数体翻译、FnSummary 翻译。
todos:
  - id: scaffold
    content: 项目脚手架：Cargo.toml、模块声明文件 (lib.rs, translator/mod.rs)、目录结构
    status: completed
  - id: error
    content: "error.rs: TranslateError 枚举 (T0xx-T3xx)"
    status: completed
  - id: expr-parser
    content: "translator/expr_parser.rs: CIR 条件/值字符串解析 → CVN BoolExpr / Expr"
    status: completed
  - id: context
    content: "translator/context.rs: TranslateContext 结构体 + builder 封装方法 + 命名工具函数"
    status: completed
  - id: resource
    content: "translator/resource.rs: 阶段 1 资源扫描（Mutex/RwLock/Semaphore/Channel/Condvar → P_r + marking；Var/Atomic → V）"
    status: completed
  - id: control-flow
    content: "translator/control_flow.rs: Transfer 翻译 (next/branch/switch/return) + guard 互补生成"
    status: completed
  - id: operation
    content: "translator/operation.rs: Op 分派 (lock/drop/read/write/send/recv/cas/load/store/spawn/join/call/return) + lock_tracker + post_wait_locks 检查"
    status: completed
  - id: condvar
    content: "translator/condvar.rs: wait (t_cv_wait + wait place + reacquire) / notify (per-wait-site 变迁) / notify_all (链式或子集展开)"
    status: completed
  - id: fn-summary
    content: "translator/fn_summary.rs: 阶段 3 FnSummary → Call 变迁 + writes 置 Unknown"
    status: completed
  - id: orchestrate
    content: "translator/mod.rs: 三阶段编排 + 输入校验 (T0xx) + 最终 builder.build_with_anchor_check()"
    status: completed
  - id: validate
    content: "validate.rs: 可选的翻译后语义保持检查（控制流连通性、资源守恒等）"
    status: completed
  - id: tests
    content: "tests/: 三类测试用例 + fixture JSON 文件 + 集成测试（使用 cir/examples/ 已有 JSON）"
    status: completed
  - id: docs
    content: "doc/: architecture.md, translation_rules.md, examples.md, error_codes.md"
    status: completed
isProject: false
---

# CIR-to-CVN 翻译器实现计划

## 关键发现与设计决策

### 实际 API 与 spec 差异

- CIR crate 名为 `ceir`（非 `cir`），无 features。Cargo.toml 中使用 `cir = { package = "ceir", path = "./cir" }`
- `PlaceId` 是 `String` newtype（非枚举），语义由 `PlaceKind::Control / Resource / Wait` 表示
- CIR `Branch.cond` 是字符串（如 `"count > 0"`），`Op::ResOp.args` 是 `Vec<String>`（如 `"count + 1"`）。需实现**表达式解析器**将字符串转为 CVN `BoolExpr` / `Expr`
- CVN `CvnNetBuilder` 使用 consuming-self builder pattern，在 `TranslateContext` 中通过 `std::mem::take` 操作

### Condvar wait-after-lock 处理（方案 B）

`t_cv_reacquire` 输出到 wait 的 transfer 目标（如 s4）。若 s4 是对同一 mutex 的 lock，则将其翻译为 `Sequential`（不消耗 `rp(mtx)`），因为锁已由 reacquire 获取。翻译器在处理 wait 时将 `(fn_name, resume_sid, mutex_name)` 记录到 context 的 `post_wait_locks` 集合中，翻译 lock 操作时检查该集合。

---

## 模块结构

```
src/
├── lib.rs                        # pub fn translate(), re-export 错误类型
├── error.rs                      # TranslateError enum (T0xx-T3xx)
├── validate.rs                   # 可选的翻译后语义校验
└── translator/
    ├── mod.rs                    # 三阶段编排 + translate() 内部实现
    ├── context.rs                # TranslateContext: builder封装、命名、计数、索引
    ├── expr_parser.rs            # CIR 条件/值字符串 → CVN BoolExpr/Expr
    ├── control_flow.rs           # Transfer 翻译 (next/branch/switch/return)
    ├── resource.rs               # 阶段1: 资源扫描 → P_r + marking + V
    ├── operation.rs              # CIR Op → 变迁 + 弧 (lock/drop/send/recv/write/read/cas...)
    ├── condvar.rs                # Condvar wait/notify/notify_all 专用翻译
    └── fn_summary.rs             # 阶段3: FnSummary → 原子变迁
```

---

## 阶段详解

### 阶段 1：资源扫描 (`[resource.rs](src/translator/resource.rs)`)

扫描 `Program.resources`，对每种资源类型：

| 资源类型  | CVN 生成                                                                             | 初始 marking | V                       |
| --------- | ------------------------------------------------------------------------------------ | ------------ | ----------------------- |
| Mutex     | `add_resource_place("rp_mtx", "mtx", ResourceType::Mutex)`, token=1                  | rp_mtx: 1    | -                       |
| RwLock    | `add_resource_place("rp_rw", "rw", ResourceType::RwLock{max_readers: N})`, token=N   | rp_rw: N     | -                       |
| Semaphore | `add_resource_place("rp_sem", "sem", ResourceType::Semaphore{count: c})`, token=c    | rp_sem: c    | -                       |
| Channel   | `add_resource_place("rp_ch", "ch", ResourceType::Channel)`, token=0                  | rp_ch: 0     | -                       |
| Condvar   | `add_resource_place("rp_cv", "cv", ResourceType::Condvar)` -- 仅注册，不产生 marking | -            | -                       |
| Var       | 无库所                                                                               | -            | `name: Val::from(init)` |
| Atomic    | 无库所                                                                               | -            | `name: Val::from(init)` |

**RwLock N 计算**：扫描所有函数体中 `Op::Spawn / Op::SpawnAsync` 的目标函数名，去重计数 + 1（entry）。

### 阶段 2：函数体翻译 (`[operation.rs](src/translator/operation.rs)` + `[control_flow.rs](src/translator/control_flow.rs)`)

逐函数逐语句处理。对每个 `Statement { sid, op, transfer }`：

1. 生成控制库所 `cp_{fn_name}_{sid}`（首次遇到时）
2. 根据 `op` + `transfer` 组合生成变迁和弧

**核心翻译逻辑** (在 `operation.rs` 中)：

```rust
fn translate_statement(ctx: &mut TranslateContext, fn_name: &str, stmt: &Statement) {
    ctx.ensure_control_place(fn_name, &stmt.sid);
    match &stmt.op {
        Op::ResOp { resource, action, args } => translate_res_op(ctx, fn_name, stmt, resource, action, args),
        Op::Spawn(f) => translate_spawn(ctx, fn_name, stmt, f),
        Op::Join(f) => translate_join(ctx, fn_name, stmt, f),
        Op::Call(f) => translate_call(ctx, fn_name, stmt, f),
        Op::Return => translate_return(ctx, fn_name, stmt),
        // SpawnAsync / Await 同 Spawn / Join
    }
}
```

`**res_op` 分派 (action 字段决定)：

- `lock` → 检查 `post_wait_locks` 集合，若匹配则生成 Sequential；否则正常 Lock 变迁
- `drop` → Unlock 变迁（RwLock 需查 `lock_tracker` 确定 weight=N 或 1）
- `read` → 与 transfer 合并：next→Sequential, branch→BranchTrue/False, switch→Switch 组
- `write` → VarWrite 变迁 + output arc update
- `send` / `recv` → Send/Recv 变迁
- `acquire` / `release` → 同 lock/drop
- `load` → 同 read
- `store` → 同 write（但操作 Atomic）
- `cas` → CasSuccess + CasFailure 双变迁
- `wait` / `notify` / `notify_all` → 委托给 `condvar.rs`

### 阶段 3：FnSummary 翻译 (`[fn_summary.rs](src/translator/fn_summary.rs)`)

对每个 `FnSummary`，在调用点生成一个 `Call` 变迁：

- `A_in`: `(cp(caller, s), w=1, True)`
- `A_out`: `(cp(caller, s_next), w=1, { writes中每个变量: Lit(Unknown) })`

---

## 核心数据结构

### TranslateContext (`[context.rs](src/translator/context.rs)`)

```rust
pub(crate) struct TranslateContext {
    builder: CvnNetBuilder,
    // 已注册的控制库所 (fn_name, sid) → place_id_string
    control_places: HashSet<(String, String)>,
    // 资源类型索引: resource_name → ResourceInfo
    resource_map: HashMap<String, ResourceInfo>,
    // RwLock N 值
    rwlock_n: u32,
    // 每个函数的锁持有状态追踪 (fn_name, resource_name) → LockKind(Read/Write)
    lock_tracker: HashMap<(String, String), LockKind>,
    // Condvar wait-site 集合: cv_name → Vec<WaitSite>
    wait_sites: HashMap<String, Vec<WaitSite>>,
    // post-wait lock 标记: (fn_name, sid) → mutex_name
    post_wait_locks: HashMap<(String, String), String>,
    // FnSummary 索引: fn_name → &FnSummary
    fn_summary_map: HashMap<String, FnSummary>,
    // 错误收集
    errors: Vec<TranslateError>,
}
```

### 表达式解析器 (`[expr_parser.rs](src/translator/expr_parser.rs)`)

解析 CIR 中的字符串表达式：

```
cond_string  = expr cmp_op expr          → BoolExpr::Cmp
value_string = literal | var_ref | expr binop expr  → Expr

cmp_op = "==" | "!=" | ">" | "<" | ">=" | "<="
binop  = "+" | "-" | "*" | "/" | "%"
literal = integer | "true" | "false" | quoted_string | enum_variant
var_ref = identifier
```

实现为简单的 token 分割 + 模式匹配（非完整递归下降，因为 CIR 表达式结构简单平坦）。

---

## 命名约定

| 元素               | PlaceId/TransitionId 格式      | 示例                                    |
| ------------------ | ------------------------------ | --------------------------------------- |
| 控制库所           | `cp_{fn_name}_{sid}`           | `cp_worker_s1`                          |
| 返回库所           | `cp_{fn_name}_ret`             | `cp_worker_ret`                         |
| 资源库所           | `rp_{res_name}`                | `rp_mtx`                                |
| 等待库所           | `wp_{cv_name}_{fn_name}_{sid}` | `wp_cv_waiter_s3`                       |
| Reacquire 中间库所 | `cp_{fn_name}_{sid}_reacquire` | `cp_waiter_s3_reacquire`                |
| 变迁               | `{fn_name}_{sid}_{kind}`       | `worker_s1_lock`, `main_s2_branch_true` |

---

## 错误类型 (`[error.rs](src/error.rs)`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("T001: program missing entry function '{0}'")] MissingEntry(String),
    #[error("T002: entry function '{0}' has empty body")] EmptyEntryBody(String),
    #[error("T003: spawn/join references unknown function '{0}'")] UnknownFunction(String),
    #[error("T101: unknown resource type '{0}'")] UnknownResourceType(String),
    #[error("T102: condvar wait references non-existent lock '{0}'")] CondvarLockNotFound(String),
    #[error("T103: condvar wait lock '{0}' is not a Mutex")] CondvarLockNotMutex(String),
    #[error("T201: transfer target sid '{0}' not found in function '{1}'")] InvalidTarget(String, String),
    #[error("T202: invalid branch condition '{0}'")] InvalidBranchCondition(String),
    #[error("T203: switch variable '{0}' is not Enum type")] SwitchNotEnum(String),
    #[error("T301: cannot determine lock kind for RwLock drop at {0}:{1}")] AmbiguousRwLockDrop(String, String),
    #[error("T302: condvar notify for '{0}' has no wait-sites")] NoWaitSites(String),
}
```

---

## 测试策略

### 目录结构

```
tests/
├── category1_control_flow/
│   ├── test_sequential.rs         # 示例 1.1
│   ├── test_branch.rs             # 示例 1.2
│   ├── test_switch.rs             # 示例 1.3
│   ├── test_spawn_join.rs         # 示例 1.4
│   └── test_loop.rs               # 示例 1.5
├── category2_resource/
│   ├── test_mutex.rs              # 示例 2.1
│   ├── test_rwlock.rs             # 示例 2.2
│   ├── test_semaphore.rs          # 示例 2.3
│   ├── test_channel.rs            # 示例 2.4
│   ├── test_condvar.rs            # 示例 2.5
│   └── test_var_atomic.rs         # 示例 2.6
├── category3_guard_update/
│   ├── test_write_update.rs       # 示例 3.1
│   ├── test_unknown_propagation.rs # 示例 3.2
│   ├── test_cas.rs                # 示例 3.3
│   ├── test_fn_summary.rs         # 示例 3.4
│   └── test_protection_ignored.rs  # 示例 3.5
└── fixtures/                       # CIR JSON 测试输入
```

每个测试用例：构造/加载 CirProgram → 调用 `translate()` → 断言库所数/变迁数/弧权重/guard/marking/V。

### 使用 cir/examples/ 已有 JSON

翻译 `producer_consumer.json`、`state_machine.json`、`complex_rwlock.json`、`with_summary.json` 作为集成测试。

---

## 文档

```
doc/
├── architecture.md          # 三阶段流程图 + 模块职责
├── translation_rules.md     # 完整翻译规则表（从 spec 第三节）
├── examples.md              # 翻译示例集（从 spec 第四节）
├── error_codes.md           # T0xx-T3xx 错误码列表
├── cir_spec.md              # symlink → ../cir/doc/ (若存在)
└── cvn_spec.md              # symlink → ../cvn/doc/ (若存在)
```

---

## 实施顺序

按依赖关系分步：先基础设施，再核心翻译逻辑（从简单到复杂），最后测试和文档。每步完成后编译检查。
