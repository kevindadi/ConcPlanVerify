---
name: Condvar Translation Rewrite
overview: 将 CIR CondVar 翻译规则从当前的"链式展开 + 结构性冲突"方案重写为基于全局变量 (`nw_cv`, `na_w`) 和资源库所 `rp(cv)` 的新方案。涉及 CVN 模型、翻译器、分析/修复、文档和示例的全面修改。
todos:
  - id: model-transition-kind
    content: 修改 TransitionKind 枚举：删除旧 variant，添加 CondvarWaitEnter/WakeByNotify/WakeByNotifyAll/NotifyLost/NotifyAllLost
    status: completed
  - id: model-export
    content: 更新 cvn/src/export.rs 的 transition_style 匹配新 TransitionKind
    status: completed
  - id: model-validate
    content: 更新 cvn/src/validate.rs 的 max_allowed 逻辑
    status: completed
  - id: context-naming
    content: 在 context.rs 中添加 ra_id/nw_var_name/na_var_name 命名辅助函数
    status: completed
  - id: resource-scan
    content: 在 resource.rs 中为 Condvar 创建 rp 库所和 nw 全局变量
    status: completed
  - id: condvar-rewrite
    content: 完全重写 condvar.rs：translate_wait (4变迁)、translate_notify (2变迁)、translate_notify_all (2变迁)
    status: completed
  - id: prescan-na-vars
    content: 在 prescan_condvar_waits 中为每个 wait-site 注册 na 变量
    status: completed
  - id: repair-signal-loss
    content: 更新 repair/mod.rs 中的信号丢失检测逻辑和 format_step_description
    status: completed
  - id: doc-rules
    content: 重写 doc/translation_rules.md 的 Condvar 章节
    status: completed
  - id: doc-paper
    content: 更新 paper/main.tex 的 TransitionKind 表和翻译规则
    status: completed
  - id: dot-examples
    content: 重写 dots/cvn/ 下的 producer_consumer 和 complex_rwlock CVN 示例
    status: completed
  - id: test-signal-loss
    content: 重写 cvn/tests/condvar_signal_loss.rs 使用新结构
    status: completed
  - id: test-integration-e2e
    content: 运行 integration 和 e2e 测试，确保通过
    status: completed
isProject: false
---

# CondVar 翻译规则重写

## 核心变化对比

**旧方案**:

- `wait` → 2 变迁 (cv_wait + cv_reacquire)
- `notify_one` → 对每个 wait-site 1 变迁（直接消费 wp token，nondeterministic）
- `notify_all` → 2K 变迁链式展开 (wake + skip per wait-site)
- 无全局变量

**新方案**:

- `wait` → 4 变迁 (enter + wake1 + wakeA + reacq)
- `notify_one` → 2 变迁 (notify + lost)
- `notify_all` → 2 变迁 (notifyAll + allLost)
- 引入全局变量 `nw_cv` (Int) 和 `na_{sid}` (Bool)
- 引入资源库所 `rp(cv)` (initial 0)

## A. CVN 模型层修改

### 1. TransitionKind 枚举 — `[cvn/src/model/transition.rs](cvn/src/model/transition.rs)`

替换旧的 4 个 variant 为 8 个新 variant：

```rust
// 旧：CondvarWait, CondvarNotify { target_wait_place }, CondvarNotifyAll, CondvarReacquire
// 新：
CondvarWaitEnter,
CondvarWakeByNotify,
CondvarWakeByNotifyAll,
CondvarReacquire,       // 保留
CondvarNotify,          // 无字段（不再引用 target_wait_place）
CondvarNotifyLost,
CondvarNotifyAll,       // 保留名称，语义改变
CondvarNotifyAllLost,
```

### 2. DOT 导出 — `[cvn/src/export.rs](cvn/src/export.rs)`

更新 `transition_style()` 中的 match arms，为新 variant 分配标签和颜色。

### 3. CVN 验证器 — `[cvn/src/validate.rs](cvn/src/validate.rs)`

`check_control_input_arcs` 中的 `max_allowed` 逻辑：

- 移除 `CondvarNotify { .. }` 和 `CondvarNotifyAll` 的 `max_allowed = 2` 特殊处理（新方案中这些变迁只有 1 个控制流输入弧）

## B. 翻译器修改

### 4. 命名与上下文 — `[src/translator/context.rs](src/translator/context.rs)`

- 新增 `ra_id(fn_name, sid)` → `"ra_{fn_name}_{sid}"` （重获库所命名）
- 新增 `nw_var_name(cv_name)` → `"nw_{cv_name}"`
- 新增 `na_var_name(fn_name, sid)` → `"na_{fn_name}_{sid}"`
- `wp_id` 可简化（不再需要 cv_name 参数，但保留也行）

### 5. 资源扫描 — `[src/translator/resource.rs](src/translator/resource.rs)`

在 `("sync", "Condvar")` 分支中新增：

```rust
ctx.add_resource_place(&res.name, ResourceType::Condvar);
// rp(cv) 初始 0 token，无需 set_initial_tokens
ctx.add_variable(&nw_var_name(&res.name), Val::int(0));
```

### 6. 核心翻译 — `[src/translator/condvar.rs](src/translator/condvar.rs)` (完全重写)

`**translate_wait(ctx, fn, stmt, cv, args, input_cp)**` — 生成 4 变迁：

```
t_enter:  cp(f,sid) → wp(sid) + rp(mtx)      [nw_cv += 1, na_sid ← false]
t_wake1:  wp(sid) + rp(cv) → ra(sid)          [nw_cv -= 1]
t_wakeA:  wp(sid) → ra(sid)                   [guard: na_sid == true; nw_cv -= 1, na_sid ← false]
t_reacq:  ra(sid) + rp(mtx) → cp(f, sid')
```

新增库所：`wp(sid)`（Wait place）、`ra(sid)`（Control place）。

`**translate_notify(ctx, fn, stmt, cv, input_cp)**` — 生成 2 变迁：

```
t_notify: cp(f,sid) → cp(f,sid') + rp(cv)     [guard: nw_cv > 0]
t_lost:   cp(f,sid) → cp(f,sid')              [guard: nw_cv == 0]
```

`**translate_notify_all(ctx, fn, stmt, cv, input_cp)**` — 生成 2 变迁：

```
t_notifyAll: cp(f,sid) → cp(f,sid')           [guard: nw_cv > 0; na_w1..wk ← true]
t_allLost:   cp(f,sid) → cp(f,sid')           [guard: nw_cv == 0]
```

### 7. 预扫描 — `[src/translator/operation.rs](src/translator/operation.rs)`

`prescan_condvar_waits`: 新增 — 为每个 wait-site 注册 `na_{fn}_{sid}` 全局变量（Bool，初始 false）。

## C. 分析/修复修改

### 8. 信号丢失检测 — `[src/repair/mod.rs](src/repair/mod.rs)`

- `detect_stuck_condvar_notify`: 更新逻辑 — 不再检查 `CondvarNotify { target_wait_place }` 的 wait place 输入，改为检查 `CondvarNotifyLost`/`CondvarNotifyAllLost` 是否在 trace 中出现
- `format_step_description`: 更新 match arms 适配新 TransitionKind

## D. 文档修改

### 9. 翻译规则文档 — `[doc/translation_rules.md](doc/translation_rules.md)`

重写 "## 3. Condvar Operations" 章节，与用户提供的规则一致。

### 10. 论文 — `[paper/main.tex](paper/main.tex)`

更新第 671 行的 TransitionKind 表和第 759-772 行的翻译规则描述。

## E. 示例/测试修改

### 11. CVN 示例 DOT 文件

- `[dots/cvn/example_producer_consumer.dot](dots/cvn/example_producer_consumer.dot)`：1 wait + 1 notify_all → 6 变迁
- `[dots/cvn/example_complex_rwlock.dot](dots/cvn/example_complex_rwlock.dot)`：1 wait + 1 notify_all → 6 变迁

### 12. 测试

- `[cvn/tests/condvar_signal_loss.rs](cvn/tests/condvar_signal_loss.rs)`：重写手写 CVN 以使用新结构（rp_cv、nw 变量、na 变量、4 变迁 wait）
- `[tests/integration.rs](tests/integration.rs)`：验证通过（可能需要调整断言）
- `[tests/e2e.rs](tests/e2e.rs)`：signal_loss 测试需通过
- CIR snapshot 测试不受影响（CIR 侧不变）

## F. producer_consumer CVN 示例详解

以 consumer wait(cv, mtx) at s3 和 producer notify_all(cv) at s3 为例：

**新增库所**:

- `rp_cv` (Resource, Condvar, 0 tokens)
- `ra_consumer_s3` (Control place, 替代 `cp_consumer_s3_reacquire`)

**新增变量**:

- `nw_cv`: Int = 0
- `na_consumer_s3`: Bool = false

**consumer wait 变迁**:

- `consumer_s3_cv_enter` [CondvarWaitEnter]: cp_consumer_s3 → wp_cv_consumer_s3 + rp_mtx, update nw_cv := nw_cv+1, na_consumer_s3 := false
- `consumer_s3_cv_wake1` [CondvarWakeByNotify]: wp_cv_consumer_s3 + rp_cv → ra_consumer_s3, update nw_cv := nw_cv-1
- `consumer_s3_cv_wakeA` [CondvarWakeByNotifyAll]: wp_cv_consumer_s3 → ra_consumer_s3, guard na_consumer_s3==true, update nw_cv := nw_cv-1, na_consumer_s3 := false
- `consumer_s3_cv_reacquire` [CondvarReacquire]: ra_consumer_s3 + rp_mtx → cp_consumer_s4

**producer notify_all 变迁**:

- `producer_s3_cv_notify_all` [CondvarNotifyAll]: cp_producer_s3 → cp_producer_s4, guard nw_cv > 0, update na_consumer_s3 := true
- `producer_s3_cv_notify_all_lost` [CondvarNotifyAllLost]: cp_producer_s3 → cp_producer_s4, guard nw_cv == 0
