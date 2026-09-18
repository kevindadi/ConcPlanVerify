# 复核：experiments-v2 第 d 轮（一致性层 + 修复型冒烟）

复核对象：`ConcPlanVerify` 工作树（HEAD 仍是 `dac4792`，**2748 处改动未提交**）；`ConcIR` HEAD `e643ff5`（`codegen.rs`/`conform.rs`/`tests/conformance.rs` 只在暂存区，未进提交）。
复核方法：读 HANDOFF Round d、CONFORMANCE.json、repair-smoke 的 `result.json`/`revision-*.cir.json`/`llm/*.json`/`candidate.rs`，与基准输入文件逐一对照。

## 结论

- **接受**：C-1..C-5 修复及其回归；`.gitignore` 证据回收；A2 四列分列；`same_cv` 标签修正；能力族基准扩到 37 个就绪用例且每族 ≥2 buggy；**一致性层跑通**（4 个用例 51/51 conformant、篡改轨迹报 violation、Rust 234 / Python 88 全过）。这是本项目第一次有"代码的每次观测执行都是已验证模型的执行"的机器证据，方向正确。
- **修复型冒烟的臂间比较无效**（R-1、R-2）：Rust 臂拿到的输入把答案写在注释里；A3 的两次 reject 是 prompt schema 缺陷造成的"格式打架"，不是推理失败。这两个问题不修，任何多臂数字都不能进论文。
- 一致性层有两个口径问题（R-4）和一个覆盖面限制（R-6），需要在下一轮补齐后才能作为主证据。

## R-1 基准输入泄漏（P1）

`benchmarks/families/*/rust/buggy.rs` 的文档注释直接写明缺陷与修法。例：`condvar/bare_wait_no_predicate/rust/buggy.rs` 开头 `//! Expected defect: SignalLoss (lost wakeup). The waiter calls wait() unconditionally, so if the notifier runs first the notification is lost`，行内 `// s2: wait without checking ready first — the lost-wakeup bug.`。A0/A1/A2 三个臂一轮就"修好"，产出与注释描述逐字对应。
CIR 侧同样泄漏：`program` 字段为 `bare_wait`、`lost_wakeup`、`channel_deadlock`、`permit_leak`、`notify_choice_false_pass`、`partial_bystander`，会随 CIR 原文进入 A3 的 prompt。
影响：repair-smoke 中 Rust 臂的 accept 全部不可采信；A3 的输入虽无注释但有名字。

## R-2 A3 的 reject 是 schema 打架（P1）

| 任务 | 轮次 | 实际发生 |
| --- | --- | --- |
| bare_wait | v2 | LLM 已给出正确修法（谓词循环），但 `check` 报 E001：var 资源缺 `base`/`init` |
| bare_wait | v3、v4 | `write_shared` 用了 `resource` 字段，schema 要求 `var`；反馈里的 `detail` 却说"`expr` must be a string"（误导），并引用 LLM 看不到的临时文件行号。**v3 与 v4 字节相同**（sha `d81036fe…`），harness 未检测停滞，白白花掉最后一轮 |
| partial_deadlock | v2 | E005：sid 必须是 `s<number>`，LLM 用了 `b6`/`y1`；prompt 未说明 |
| partial_deadlock | v3、v4 | 语法合法但 AG EF 仍 FAIL；见 R-3 的诊断质量问题 |

结论：A3 两次 reject 中至少 bare_wait 一例本应在第 2 轮 PASS。CIR 生成 prompt 的 schema 摘要与 ConcIR 真实语法不一致（`write_shared.var`、sid 格式、var 资源必填字段），schema 错误的反馈文本不指向真实字段。上一轮 smoke-c 的 lost_wakeup（`expr` 为对象）是同一根因的另一表现。

## R-3 目标层失败的诊断可读性（P2，ConcIR）

partial_deadlock v3/v4 收到的 `always_reachable` FAIL 诊断：`related_functions: ["0::0","0::1","0::2"]`（数字索引而非 FQN）、`counterexample[].origin` 为 `{module:0,function:1,sid:0}`（数字）、`blocked` 只列出 main 等 scope。前缀（a 拿 A、b 拿 B）作为"不可再达目标"的最短见证是正确的，但没有告诉 LLM"a 持 A 等 B、b 持 B 等 A"。需要在诊断中加 doom 状态摘要：每线程 `function / at_sid / holds[] / waiting_on`，并把所有索引渲染成名字。

## R-4 一致性层口径（P2）

- `coverage.sids_total = 4`（abba_fixed），但 abba 的两个 worker 各有 s1..s5 且 sid 按函数复用：覆盖率按**sid 字符串去重**计数，应按 `(function, sid)` 计数。
- `traces_total = 51 = 50 native + 1 miri`：`-Zmiri-many-seeds` 跑 64 个种子却只留下 1 条轨迹（输出文件被覆盖或只写一次）。要么每个种子单独跑并各留一条，要么 `CIR_TRACE_OUT` 追加写并带 run id。
- `hang` 列恒 0，但 CONFORMANCE 与 repair-smoke 对"超时"的命名不一致（`hang` vs `timeout`），统一。

## R-5 Miri 参数与异常（P2）

repair-smoke 的 Miri 是单种子、8 s 上限、无 many-seeds（协议偏差，HANDOFF 已声明）。`partial_deadlock` 的 A2-m 三轮 `miri_statuses` 全 `timeout`——程序确实死锁而旁观者线程死循环，超时是正确观测；但 A2-ml 第 3 轮出现 `tool_error ×5`，未查原因（怀疑 `RUSTC_WRAPPER=lockbud` 残留在 miri 调用环境）。对"需求要求所有线程终止"的任务，miri `timeout` 应单列为 `hang_suspect`（仍不算 detection）。

## R-6 codegen 覆盖面（P2）

`codegen` 仅支持单模块、无 channel/RwLock/复合值 → channel 族与 real-cases 无法做一致性；而"模块化"是 ConcIR 的头条能力，real-cases 是唯一的生产实践证据。下一轮至少补 `sync_channel(cap)`、多模块（`mod` + `pub(crate)`）。

## R-7 HOLE 填充与 `A3_free` 消融未跑（P2）

上一轮要求对 `scope_bound_k_workers` 跑一次 Flash 填洞 + lint + conform，并跑 `A3_free` 对照。实际该用例 `holes: []`（`assign_local` 全部直译），于是 LLM 填洞与自由生成对照都没有发生。需要含 nobody 函数的用例才能有 HOLE。

## R-8 仓库卫生（P1，阻塞复现）

- ConcPlanVerify 本轮**零提交**，包括 conformance-v1、repair-smoke、7 个新用例、HANDOFF。
- ConcIR `e643ff5` 的 `lib.rs` 声明 `pub mod codegen; pub mod conform;`，但两个源文件只在暂存区：**按该提交 checkout 无法编译**。
- `rust-toolchain.toml` 仍是浮动 `nightly`。本机 `rustc +nightly -V` = `a69a63265 2026-09-03`，对应的日期工具链名是 `nightly-2026-09-04`（rustup 用发布日）；用 `rustup toolchain install nightly-2026-09-04 --component miri` 后比对 hash 再钉死。

## 对实验设计的判断

一致性层已经能支撑"模型到代码"这条主张；修复型主实验的方向对，但当前数据是脏的。下一轮不要扩大规模，只做三件事：把输入洗干净、让 A3 不因格式输掉、把 Rust 臂的 oracle 补出来；然后在同样 3 个任务上重跑一次，看数字是否发生实质变化。若洗净后 Rust 臂在 bare_wait/abba 上依然一轮修好，那就是事实——这类教科书缺陷不是我们的主战场，主表应偏向 partial_deadlock、cross_module、nested_scope、notify_one_wrong_pick、bounded_backpressure 这类工具漏报且 LLM 会错的用例。
