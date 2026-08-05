# Condvar 建模与死转移族检测

本文说明 CIR 条件变量如何落到 CVN、为何会出现「假死转移」,以及 `disjunctive_family` 如何把或变体做成一等公民。

## 1. CVN 不是彩色 token CPN

当前 CVN（见 [`cvn/README.md`](../cvn/README.md)、[`cvn_spec.md`](cvn_spec.md)）是**带全局变量守卫的加权 P/T 网**：

- 库所承载无色 token；
- 变迁行为由弧权、`BoolExpr` 守卫与 `VarUpdate` 决定；
- **没有**按颜色匹配 waiter 的 token。

因此 `wait` 的「被 notify 唤醒」与「被 notify_all 唤醒」必须拆成两条互斥变迁（或等价的守卫分叉），而不能用一条带颜色匹配的 wake 变迁表达。真 CPN（waiter 着色）可以把二者合并，见文末「后续方向」——本仓库本轮不落地。

## 2. 翻译一览（不变）

细则见 [`translation_rules.md`](translation_rules.md) §3；实现见 [`src/translator/condvar.rs`](../src/translator/condvar.rs)。

### 辅助结构

| 结构 | 含义 |
|------|------|
| `rp(cv)` | 条件变量资源库所；`notify` 成功时投入 1 token |
| `nw_cv` | 当前 waiter 计数（Int） |
| `wp(fn,sid)` | 该 wait 调用点的等待库所 |
| `ra(fn,sid)` | 唤醒后重获锁之前的中间库所 |
| `na_fn_sid` | `notify_all` 针对该 wait 点的布尔旗标 |

### `wait(cv, mtx)` → 4 变迁

```
t_enter     : cp → wp + rp(mtx)     nw++, na←false
t_wake1     : wp + rp(cv) → ra      nw--          (notify_one 路径)
t_wakeA     : wp → ra               guard na      (notify_all 路径)
t_reacquire : ra + rp(mtx) → cp'
```

`t_enter` **不入族**（单独永死才有意义）。`t_wake1` / `t_wakeA` / `t_reacquire` 共享 family `{fn}_{sid}:wait_wake`。

### `notify` / `notify_all` → 各 2 变迁

```
t_notify[_all]      : guard nw > 0   …成功投递 / 置 na
t_notify[_all]_lost : guard nw == 0  …通知方继续前进（lost wakeup 的入口）
```

分别共享 `{fn}_{sid}:notify` 与 `{fn}_{sid}:notify_all`。

## 3. SignalLoss 如何检出

`notify_*_lost` **本身不是缺陷**：Rust 在无人等待时 `notify` 是合法的。缺陷是随后 **waiter 卡在 `wp` 且再也无法被唤醒**，在状态空间里表现为含 wait 库所的死锁 / 阻塞。`repair::analyze` 将这类反例分类为 `BugKind::SignalLoss`。

## 4. `disjunctive_family`：为何需要

同一 CIR 语句编译出的多条变迁是**析取族**：一次执行至多走一支。若按「单变迁从未开火」报 `DeadTransition`，则：

- 正确使用 `notify_all` 的程序会因 `cv_wake1` 未开火被误报；
- 只用 `notify` 时 `cv_wakeA` 会被误报。

字段语义（[`cvn::model::Transition::disjunctive_family`](../cvn/src/model/transition.rs)）：

- 同一 `Some(id)` 的变迁构成一族；
- **族内任一成员在可达图中开火 ⇒ 整族存活**；
- 仅当族内全未开火时，`find_dead_transitions` 报**一条**反例（代表元为字典序最小的 transition id）。

翻译器在创建变迁后调用 `set_disjunctive_family`；检测在 [`cvn::analysis::find_dead_transitions`](../cvn/src/analysis/search.rs) 内完成。**不再**用 transition id 后缀启发式。

`BranchTrue` / `BranchFalse`、`CasSuccess` / `CasFailure` **不自动入族**：本项目里「一支永假」可以是真缺陷（如 `dead_transition` case）。

## 5. 后续方向（真 CPN，未做）

若将 wait 库所改为携带 waiter 颜色（或等价身份），可用单条 wake 变迁按颜色匹配，并简化 `notify_all` 的广播模型。那需要扩展 CVN 核心，超出「P/T + 守卫」现状，故单独规划。
