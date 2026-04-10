---
name: CIR DOT Export
overview: 为 CIR 库 (cir/ submodule) 添加 DOT 可视化导出功能,支持函数级和程序级控制流图生成,包含资源面板、跨函数边、回边高亮等特性.
todos:
  - id: types
    content: DotOptions/DotDirection 类型 + export/mod.rs API 骨架 + lib.rs 注册 + Cargo.toml 添加 insta
    status: completed
  - id: nodes
    content: "write_node: 按 Op/Transfer 类型生成节点属性 + format_label_compact + format_label_verbose"
    status: completed
  - id: edges
    content: "write_edges: 按 Transfer 类型生成边属性 + is_back_edge 回边检测"
    status: completed
  - id: function-subgraph
    content: write_function_subgraph + Function::to_dot 单函数导出
    status: completed
  - id: program-dot
    content: write_resource_panel + write_cross_function_edges + Program::to_dot_with_options 全局导出
    status: completed
  - id: tests
    content: 单元测试 + insta 快照测试(线性/分支/switch/回边/跨函数/资源面板/options)
    status: completed
isProject: false
---

# CIR DOT 可视化导出实现计划

## 现状

- CIR 库位于 `cir/`,crate 名 `ceir`,edition 2021
- 核心类型在 `[cir/src/ast.rs](cir/src/ast.rs)`:`Program`、`Function`、`Statement`、`Op`、`Transfer`、`Resource`、`Protection`
- 当前模块:`ast`、`diagnostic`、`validate`(见 `[cir/src/lib.rs](cir/src/lib.rs)`)
- 无额外重依赖,仅 `serde`/`serde_json`/`thiserror`
- CVN 已有 DOT 导出 `[cvn/src/export.rs](cvn/src/export.rs)` 可作参考模式(纯 `std::fmt::Write` 字符串拼接)

## 文件变更

### 新增文件

```
cir/src/export/
├── mod.rs          # DotOptions, DotDirection, re-export
└── dot.rs          # 所有 DOT 生成逻辑
```

### 修改文件

- `[cir/src/lib.rs](cir/src/lib.rs)` — 添加 `pub mod export;`
- `[cir/Cargo.toml](cir/Cargo.toml)` — `[dev-dependencies]` 添加 `insta = "1"`

---

## 实现细节

### 1. `cir/src/export/mod.rs` — 公开类型与 API

```rust
mod dot;

pub use dot::{DotDirection, DotOptions};

use crate::ast::{Function, Program};

impl Program {
    pub fn to_dot(&self) -> String { ... }
    pub fn to_dot_with_options(&self, options: &DotOptions) -> String { ... }
}

impl Function {
    pub fn to_dot(&self) -> String { ... }
}
```

`DotOptions` 和 `DotDirection` 按 spec 定义(5 个布尔/枚举字段 + `Default` impl).

### 2. `cir/src/export/dot.rs` — 核心生成逻辑

内部组织为若干私有函数,由 `Program::to_dot_with_options` 驱动:

```rust
// 顶层入口
pub(crate) fn program_to_dot(program: &Program, opts: &DotOptions) -> String

// 子函数
fn write_preamble(out, program_name, direction)
fn write_resource_panel(out, resources, protections)
fn write_function_subgraph(out, func, fn_prefix, opts)
fn write_cross_function_edges(out, functions)
fn write_node(out, fn_prefix, stmt, opts)        // 按 Op 类型选择形状/颜色
fn write_edges(out, fn_prefix, stmt, body, opts)  // 按 Transfer 类型选择边样式
fn format_label_compact(stmt) -> String            // 简洁模式标签
fn format_label_verbose(stmt) -> String            // 详细模式标签
fn is_back_edge(current_sid, target_sid) -> bool   // 回边检测
fn escape_dot(s: &str) -> String                   // 转义 DOT 特殊字符
```

### 3. 节点样式映射(关键逻辑)

节点样式由 `Op` 变体决定.核心 `match` 分支:

```rust
fn node_attrs(op: &Op, is_entry: bool) -> NodeStyle {
    match op {
        Op::ResOp { action, .. } => match action.as_str() {
            "lock" | "acquire"         => rect, red border, penwidth=2
            "drop" | "release"         => rect, green border, penwidth=2
            "read" | "load"            => rect, blue border, penwidth=2
            "write" | "store"          => rect, orange border, penwidth=2
            "wait"                     => rect, purple border, penwidth=2
            "notify" | "notify_all"    => rect, purple border, dashed
            "send" | "recv"            => rect, cyan border, penwidth=2
            "cas"                      => rect, orange border, penwidth=2
            _                          => rect, default
        },
        Op::Spawn(_) | Op::SpawnAsync(_) => doubleoctagon
        Op::Join(_) | Op::Await(_)       => doubleoctagon, dashed fill
        Op::Call(_)                       => rect, rounded style
        Op::Return                        => ellipse, dark gray fill
    }
    // is_entry → 追加 penwidth=3 加粗边框
}
```

branch/switch 的菱形形状由 **Transfer** 类型(非 Op)决定 — 当 `stmt.transfer` 是 `Branch` 时用菱形浅黄,`Switch` 时用菱形浅橙.这需要在 `write_node` 中综合 `Op` 和 `Transfer` 两者.

### 4. 边样式映射

```rust
fn edge_attrs(transfer, current_sid) -> Vec<EdgeDef> {
    match transfer {
        Next(target) => [{
            target, style=solid, label=None,
            // 回边检测
            if is_back_edge(current_sid, target) { color=blue, penwidth=2 }
        }]
        Branch { cond, true_target, false_target } => [
            { true_target, style=solid, color=green, label="T" },
            { false_target, style=dashed, color=red, label="F" }
        ]
        Switch { cases, .. } => cases.map(|(label, target)| {
            { target, style=solid, label=label }
        })
        Return => [{ target="{fn}_ret", style=solid }]
    }
}
```

### 5. 回边检测

spec 定义为"目标 sid 数值 < 当前 sid 数值".从 sid 字符串提取数值后缀比较:

```rust
fn sid_number(sid: &str) -> Option<u32> {
    sid.strip_prefix('s').and_then(|n| n.parse().ok())
}

fn is_back_edge(current: &str, target: &str) -> bool {
    match (sid_number(current), sid_number(target)) {
        (Some(c), Some(t)) => t < c,
        _ => false,
    }
}
```

### 6. 资源面板

`subgraph cluster_resources` 内每个资源按类型映射形状:

- Mutex/RwLock → hexagon, `#ffe0e0` 填充
- Condvar → triangle
- Semaphore → pentagon (house)
- Channel → parallelogram
- Var/Atomic → rect

Protection 关系:`var_node -> lock_node [style=dotted, dir=both]`

### 7. 跨函数边

遍历所有函数的所有语句,对 `Op::Spawn`/`SpawnAsync`/`Join`/`Await`/`Call` 生成跨 subgraph 的虚线/点线边.需要用 `{fn_name}_{sid}` 作为全局唯一节点 ID,`{fn_name}_ret` 作为 return 虚拟节点 ID.

### 8. `Function::to_dot`

生成独立的 `digraph`,仅包含该函数的子图(无资源面板、无跨函数边),使用默认 `DotOptions`.

---

## 测试策略

在 `cir/tests/dot_export.rs` 中(需先在 `cir/Cargo.toml` 添加 `insta` dev-dependency):

- **线性控制流**:加载 `examples/with_summary.json` 中 worker 函数,验证节点数 = 语句数 + 1(ret),边数 = 语句数
- **分支**:构造带 branch 的函数,验证菱形节点 + T/F 边标签
- **switch**:加载 `examples/state_machine.json`,验证 switch 节点 + 枚举标签边
- **循环回边**:构造含回边的函数,验证回边带 `color=blue, penwidth=2`
- **跨函数**:加载 `examples/producer_consumer.json`,验证 spawn/join 虚线边
- **资源面板**:验证 `cluster_resources` 子图生成
- **insta 快照**:对 `examples/producer_consumer.json` 的完整 DOT 输出做快照测试
- **options 控制**:设 `show_resources=false` 验证无资源面板;设 `direction=LR` 验证 `rankdir=LR`

---

## 实施顺序

1. 基础框架:`DotOptions`/`DotDirection` 类型 + `mod.rs` API 骨架 + `lib.rs` 注册
2. 节点生成:`write_node` + `format_label_compact` + `format_label_verbose`
3. 边生成:`write_edges` + 回边检测
4. 函数子图:`write_function_subgraph` + `Function::to_dot`
5. 全局图:资源面板 + 跨函数边 + `Program::to_dot_with_options`
6. 测试:单元测试 + insta 快照
