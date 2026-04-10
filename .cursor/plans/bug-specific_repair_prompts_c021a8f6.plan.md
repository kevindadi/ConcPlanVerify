---
name: Bug-specific repair prompts
overview: 为每类 bug 设计专用的 repair prompt 模板,包含结构化反例证据和 buggy-to-fixed CIR 示例,替换当前混合在一起的通用 prompt.
todos:
  - id: create-templates
    content: 创建 5 个 per-bug-type repair template (.md),每个包含 bug 机理、修复策略、buggy/fixed CIR 示例对
    status: completed
  - id: refactor-render
    content: 重构 render.rs 按论文 Table 4 的 9 字段结构化输出,根据 BugKind 选择对应 template
    status: completed
  - id: cleanup-suggestion
    content: 将 suggestion.rs 的逻辑合并到 templates,精简冗余代码
    status: completed
  - id: update-schema-prompt
    content: 从 cir_schema_prompt.md 移除死锁完整示例,避免与 template 重复
    status: completed
isProject: false
---

# Per-Bug-Type Repair Prompt 方案

## 一、当前问题诊断

当前 [`src/repair/render.rs`](src/repair/render.rs) 的 `render_repair_prompt` 存在三个层次的问题:

### 1.1 prompt 结构与论文 Table 4 不对齐

论文定义了 9 个 prompt 字段(Bug kind, Witness trace, Bug-state summary, Held resources, Waiting relations, CIR slice, Preservation constraints, Repair hint, Output contract).当前代码实际产出的只有 bug kind + trace + 一段中文 suggestion + 通用约束,缺少:
- **CIR slice (Lambda)**: `report.cir_slice` 字段已有数据但未渲染
- **Preservation constraints (Gamma_ctx)**: `report.preservation_constraints` 已有但未渲染
- **Repair hint (H)**: `report.repair_hint` 已有但未渲染
- **Held resources / Waiting relations**: 死锁参与者信息中已有 `holding` 和 `waiting_for`,但没有按论文格式组织

### 1.2 没有 per-bug-type 的专用 prompt

不同 bug 需要完全不同的修复模式:
- **Deadlock** — 锁重排序模式 (before/after 对比)
- **SignalLoss** — while-loop 保护 wait 模式 (read -> branch -> wait -> loop-back)
- **ChannelBlock** — 把阻塞 channel 操作移到锁外
- **Livelock/Starvation** — 目前无 repair template

### 1.3 缺少 buggy->fixed CIR 对比示例

system prompt 有一个死锁示例但只有 buggy 版本.per-bug-type 应各提供一对 buggy/fixed CIR,让 LLM 理解修复的精确格式.

---

## 二、方案设计

### 2.1 架构:三层 prompt 组合

```
System Prompt (不变)         ← CIR schema 参考 (cir_schema_prompt.md)
  +
Per-Bug-Type Template        ← NEW: 每类 bug 一个修复策略模板 + buggy/fixed 示例
  +
Instance Evidence            ← 当前 BugReport 的结构化字段 (反例 trace, CIR slice, 约束等)
```

最终 user prompt 结构:

```
# 并发 Bug 修复请求

## Bug 诊断
Bug kind: Deadlock
Witness trace (sid): w1.s1 -> w2.s1 -> w1.s2(blocked) -> w2.s2(blocked)

## Bug-state summary
- w1: at s2, holding [mtx_a], waiting for mtx_b
- w2: at s2, holding [mtx_b], waiting for mtx_a
Final state: {w1.s2, w2.s2, R(mtx_b)×0, R(mtx_a)×0}

## Relevant CIR slice (仅涉及 bug 的语句)
- w1.s1: lock(mtx_a)
- w1.s2: lock(mtx_b)
- w2.s1: lock(mtx_b)
- w2.s2: lock(mtx_a)

## Preservation constraints
- Resource 'mtx_a' must remain
- Resource 'mtx_b' must remain

## Repair strategy: Deadlock → Lock ordering
<per-bug-type template with buggy/fixed CIR example>

## Current CIR
```json
{ ... full CIR ... }
```

## Output
Output the complete revised CIR JSON.
```

### 2.2 实现方式

每类 bug 的 repair template 作为独立的 `.md` 文件,通过 `include_str!` 编译期嵌入:

- `src/repair/templates/deadlock.md` — 锁重排序策略 + buggy/fixed 示例对
- `src/repair/templates/signal_loss.md` — while-loop 保护策略 + buggy/fixed 示例对
- `src/repair/templates/channel_block.md` — channel 操作移出锁 + buggy/fixed 示例对
- `src/repair/templates/livelock.md` — 引入退出条件 / 加延迟
- `src/repair/templates/starvation.md` — 公平调度建议

每个模板内容约 30-50 行,包含:
1. **Bug 机理解释**(一段话)
2. **通用修复策略**(3-5 条规则)
3. **Buggy CIR 片段** -> **Fixed CIR 片段** 的精确对比

### 2.3 render.rs 重构

重构 `render_repair_prompt` 函数,按论文 Table 4 的 9 字段结构化输出:

1. **Bug kind** — `report.kind.name()`
2. **Witness trace** — `report.trace` 中的 anchor_sids
3. **Bug-state summary** — 从 `report.kind` 提取参与者状态 + `report.final_marking_summary`
4. **Held resources + Waiting relations** — 从 `DeadlockParticipant.holding` / `.waiting_for` 提取
5. **CIR slice** — 渲染 `report.cir_slice`
6. **Preservation constraints** — 渲染 `report.preservation_constraints`
7. **Repair strategy** — 根据 `report.kind` 选择对应的 per-bug template
8. **Current CIR** — 完整 JSON
9. **Output contract** — 输出完整修复后的 CIR JSON

### 2.4 suggestion.rs 可废弃

当前 `suggestion_for()` 返回的中文字符串可以合并到 per-bug template 中,不再单独生成.

---

## 三、修改文件清单

- **新建** `src/repair/templates/deadlock.md` — 锁排序策略 + buggy/fixed CIR 示例
- **新建** `src/repair/templates/signal_loss.md` — while-loop 策略 + buggy/fixed CIR 示例
- **新建** `src/repair/templates/channel_block.md` — channel 移出锁策略 + buggy/fixed CIR 示例
- **新建** `src/repair/templates/livelock.md` — 退出条件策略
- **新建** `src/repair/templates/starvation.md` — 公平调度策略
- **重构** [`src/repair/render.rs`](src/repair/render.rs) — 按论文 Table 4 结构化输出,动态选择 template
- **简化** [`src/repair/suggestion.rs`](src/repair/suggestion.rs) — 逻辑合并到 templates,此文件可精简或删除
- **更新** [`src/repair/cir_schema_prompt.md`](src/repair/cir_schema_prompt.md) — 移除其中的死锁示例(避免与 template 重复)

buggy/fixed 示例来源:已有的 `tests/e2e/` 下的 JSON 文件可以直接复用.
