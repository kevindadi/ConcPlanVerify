# Review — Round p（freeze-6，2026-09-23）

范围：ConcPV HEAD `50f1667a`（tag `experiments-v2-freeze-6`），ConcIR `24fc982`（`concir-freeze-5`）。数据：`flash-gen-main-v3-code/run-20260923T182939`（59 格 / 112 请求）、`gen-code-replay-v1`、HANDOFF Round p。

## 结论（先说）

O-1/O-2 的修复方向正确，G3 v2 接受率 0.431 → 0.556、awp 31 → 40/72，`RF_all` 口径已统一。**但 HANDOFF 的两个结论不成立**：

1. "剩余代码阶段失败全是 `extra_op` 真实偏离" —— 错。活跑 19 个失败格里 **13 个是 `build_failed`**（semaphore 家族 9/9、`scope_bound_k_workers` 3/3、channel 1），全部因为 §1.2 引入的 `concir_sync` 注入方式与 prompt 冲突（P-1）。
2. "重放显示 harness 缺口占比 < 50%，触发止损" —— 重放对象是 freeze-5 的旧程序（它们自实现了 semaphore、早于 `concir_sync`），根本无法检验 semaphore 修复；用它来判定止损是逻辑错误。

真实情况：**19 个失败中至少 16 个仍是 harness 侧问题**（13 build + 3 main 收尾读），只有 `channel/bounded_backpressure_lock_held` 是有意义的偏离（且它暴露的是 CIR 阶段的一个漏洞，见 P-4）。修完后 G3 v2 接受率预计 ≥ 0.75，awp ≥ 55/72。止损条件不应视为已触发。

## P-1（P0）`concir_sync` 双重声明 → 12/12 semaphore/scope 格编不过

`rust_oracle.prepare_project` 往 `src/main.rs` 写 `mod concir_sync;` 并落 `concir_sync.rs`；四个 Rust prompt 又让 LLM "declare `mod concir_sync;`"。结果两种失败都出现了：LLM 写 `mod concir_sync;` → `E0428 defined multiple times`；LLM 自己内联 `mod concir_sync { … }`（它不知道运行时长什么样）→ 同样 E0428，且 instrument 重写 `use std::sync::{Mutex, Condvar}` 时漏掉了嵌套模块内的 use → `E0425`。三轮反馈里 LLM 在 duplicate 与 inline 之间来回，无法自愈——因为无论怎么写都和 harness 撞。

修法：`concir_sync` 做成真正的 path 依赖 crate（`Cargo.toml [dependencies] concir_sync = { path = … }`），prompt 改为 "外部 crate `concir_sync` 已链接，写 `use concir_sync::Semaphore;`，**不要声明 `mod concir_sync`**"；harness 另外把 LLM 代码里任何 `mod concir_sync` 声明/内联定义剥掉并记 `harness_note`。四个 prompt 同步改，措辞一致。

## P-2（P0）main 在 join 之后读共享状态被判违规（3 格）

`atomic-data/counter_overflow_safety` × 3：唯一违规是 `t0`（main）在两个 worker join 之后 `m.lock()` 读最终计数并打印。需求要求打印终态，CIR 的 `main` 只有 `scope + return`（生成 prompt 没让它建模收尾读）。这不是并发偏离：所有其他线程已结束，单线程读不可能引入死锁或竞态。

修法（conform `--op-resource`）：主线程在 `scope` 完成（所有 spawned 线程 join）之后的事件不参与匹配，单独计 `post_join_main_ops`，仍写进输出；prompt 的 "main 只做 CIR main 所做的事" 补一句 "join 之后允许读共享状态打印终态"。同样要跑 `conform-mutation-v2` 回归。

## P-3（P1）channel 家族接受格 RF 被 monitor `unmapped` 压低

v3-code 40 个接受格的覆盖：`unmapped` 15，全部在 channel 家族（6 格、36 条需求里 15 条 unmapped；`rendezvous_both_send` 一格 5/7 unmapped，RF 0.14 虽然 conform 32/32）。原因是 LLM 用 `Mutex<Receiver<T>>` 惯用法（Receiver 不是 Sync），instrument 看到 `ch_mutex0` 而不是 channel 资源，monitor 的 `--mapping` 对不上契约的 channel FQN。这直接把 G3 的 `RF_acc` 从 0.670 拉到 0.619，Complex 档失去优势。

修法：instrument v2 识别 `Mutex<Receiver<_>>`/`Mutex<Sender<_>>` 包装，把其中的 recv/send 归到 channel 资源，外层 mutex 不发事件（或发但标 `wrapper`，conform/monitor 忽略）；或 prompt 建议 "把 Receiver 移进接收线程，不要用 Mutex 包 Receiver"。两者都做。基线臂 channel 任务是否同样 unmapped 要一并统计。

## P-4（P1）`bounded_backpressure_lock_held`：CIR 声明了资源却不用，仍 PASS

该任务的 CIR 声明了 `m`（sync）但 `sender/receiver` 都没碰它，契约仍 PASS；LLM 写代码时按需求加了锁 → 代码同步结构比 CIR 多 → conform 正确拒绝。这是唯一"conform 抓到真实偏离"的格，但它说明 CIR 阶段应有 `resource declared but never used` 的检查（作为 INVALID 或修订反馈）。本轮不重跑 CIR 阶段；记为 CIR check 的待加项与论文限制，freeze-7 之后若有预算再评估。

## P-5（P1）重放实验设计

`gen-code-replay-v1` 只能检验 wait 单步编码与线程对齐（它们不依赖 LLM 改代码）；semaphore/`var`-锁/`main` 收尾读都需要新代码或新 conform 规则。下一轮重放应对 **v3-code 的 19 个失败格**做，且分开报三类：build 修复（P-1）、conform 规则修复（P-2）、仍失败。

## P-6（P2）其他

- `HUMAN_REVIEW_QUEUE_GEN.md`：`bare_wait_no_predicate` 与 `lost_wakeup_notify_before_wait` rep0 的 Rust sha 相同（`3d431c4362126415`）——两任务收敛到同一程序，按 sha 去重后队列应合并行并注明。
- 生成格 agent-proxy 专家标注仍未做。
- kimi G3 代码阶段未重跑（探针 v2 仍是 freeze-5 前的 harness）。
- semaphore 家族基线 G0/G1/G2 未重跑（它们的 prompt 也含错误的 `mod concir_sync;` 指令，重跑前必须先修 P-1）。
- 工作树里 2308 个被忽略的 `target/` 目录共 11 GB；跟踪文件 158k，pack 只有 21 MB——体积问题在忽略文件，不在仓库。
- 论文相关事项按用户要求本轮不再列。

## 对下一轮的判断

工具侧还有 P-1/P-2/P-3 三处，都是几十行的修复，且能用零请求重放先验证；然后只重跑 59 格代码阶段（≤ 177 Flash）+ kimi（≤ 72 Go）+ semaphore 基线（≤ 80 Flash）。同时做实验目录清理和 supplement 打包脚本。
