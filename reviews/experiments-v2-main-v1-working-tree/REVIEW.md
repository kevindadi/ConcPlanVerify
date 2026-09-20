# 复核：experiments-v2 第 j 轮（主批次 8×5×3、双轨 oracle、案例、RESULTS 生成器）

复核对象：`ConcPlanVerify` HEAD `ea1b235`（干净）；`ConcIR` HEAD `d3d59ed`（干净）。
复核方法：读 `RESULTS.md`（生成版）、HANDOFF Round j、`case-partial-deadlock-v1/CASE.md`；用脚本核对 `EXPERT_LABELS.json`、`extraction-v5/CELLS.json`、`detection-v3/TRACKD.json`、`HUMAN_REVIEW_QUEUE.md`。

## 结论

- §1–§3 达到要求：三分法回复语义落地并有离线重分类；`false_accept` 并入专家/构造来源；RESULTS 由命令生成且头部带命令与 sha；主批次 120 格全部有结果，203/300 请求，单一 `BIN_MAIN 4bec943d`。主表方向稳定：A3_local 21/24、A3_whole 20/24、两者 0 false-accept；A0 24/24 接受但 12 false-accept；A2-ml 22/24、3 false-accept（全是 `cycle_3lock`，Miri 与 Lockbud 三次重复全绿）。
- 案例 `partial_deadlock_bystander` 有一个重要的方法发现：**A3_local 三次全部停滞，A3_whole 2/3 找到保持嵌套持锁的修复**（统一全局顺序 + 单一共享信号量替代交叉握手）。局部再生成在需要跨函数重构握手时不够。这直接支持"局部→整体"的分层升级策略，也正是论文原本的 repair tiers。
- 但报告层与 oracle 层仍有会影响论文数字的缺陷（J-1..J-6），以及一个叙事层面的缺口（J-7：A3 臂在主表里没有 Rust 侧 oracle）。

## J-1 抽取 validated 三个数不一致（P1）

`extraction-v5/CELLS.json` 63 格 `validated=0`；`extraction-v5/SUMMARY.md` 写 2/63；RESULTS 写 1/22 且唯一 validated 格的 verdict 是 `INVALID`。`INVALID`（契约无法求值）不能算 validated。`stage=codegen` 的 29 格 `reason` 全为空串——失败原因没被记录。RESULTS 把 63 格折叠成 22 行（按 task×arm）也丢了 rep 维度。

## J-2 专家标注按 (task, arm) 而非按候选（P1）

`EXPERT_LABELS.json` 只有 22 条，`reps: 3`——一个标签套用到三个不同候选。temperature 0 下不少候选跨 rep/跨臂完全相同（`cycle_3lock` A0/A2 三 rep 同 sha `c7d11e854196`），所以正确做法是**按候选 sha 去重后逐一标注**，成本不高。另外 22 条里 11 条 `unsure`，"一致率 11/11 = 1.0" 是空洞的：专家轨对半数接受格没有给出判断。`design_preserved=no`（`cross_module/A0`）没有进任何汇总——设计丢失应作为独立指标 `design_loss` 计入。

## J-3 Track D 暴露出参考程序可疑（P1）

- `cycle_3lock` **fixed** 参考：Lockbud 报 `DoubleLock`；buggy 反而 `clean`。要么 Lockbud 假阳，要么修好的 Rust 参考真的有双重加锁。
- `partial_deadlock_bystander` **fixed** 参考：Miri `detected`。要么参考程序在 Miri 下真死锁/挂起，要么 `thread_leak`（旁观者无限循环、主线程不 join）被记成 detected。
两者都必须查清；参考程序若有误会波及 A2 的工具反馈与 Track D 表。Track D 的 Miri 只跑 6 种子（主批次 16）；`miri` 列是 bool，丢了 status；`lockbud available=None`；表里混入 P1–P9、`throttle_n_permits` 等全 `None` 行。

## J-4 主表聚合列语义不清（P2）

`build/behavior/miri/expert/extract` 在 3 次重复下只显示一个值（`partial/A0` 显示 `miri clean`，上轮同格 `detected 16`）。应显示按 rep 的计数（`hang 3/3`、`detected 1/3 clean 2/3`）。`A3 decision distribution` 表为空（生成器 bug）。`llm_ms/tool_ms` 列从 RESULTS 消失了，成本论证需要它们。

## J-5 A3_local 的停滞机制（P2，方法）

案例表明停滞发生在"释放再获取"这个局部吸引子上：局部 prompt 只能改一个函数体，而设计保持修复需要同时改两个 worker 和一个资源。当前 `stalled` 后直接结束。应把停滞作为升级触发：`A3_tiered = local（≤2 轮或停滞）→ whole（剩余预算）`。这是数据支持的方法改进，且与论文 tiers 叙事一致。

## J-6 抽取轨的投入产出（P2）

v4→v5 从 1/8 到 0–2/63，`labels` 阶段失败 10、`codegen` 29（原因未记）、HANDOFF 提到"a few backend panics"（ConcIR bug，必须修）。抽取作为 Rust 臂第三 oracle 的价值取决于能否稳定 validated；需要设一个**终止判据**：修完 harness/panic 后若 validated 仍 < 5/63，抽取转为论文 limitation（"LLM 从代码抽 CIR 不可靠，这正是为什么要 model-first"），不再投预算。

## J-7 A3 在主表里没有 Rust 侧 oracle（P1，叙事）

主表 A3 两臂 `build/behavior/miri/expert` 全为 `None`——我们的方法在表里止于 CIR。论文的核心主张是"落实到具体代码 + 后验证"，conformance-v4 只在子集上做过。每个接受的 A3 CIR 都应走 `codegen → LLM 填洞 → build → conform(strict) → behavior → miri → expert`，让五臂在同一组 Rust 侧列上对齐，A3 额外多一列 `conform`。这是下一轮最有价值的一节。

## 其他

- `HUMAN_REVIEW_QUEUE.md` 11 格仍空白，需要**你本人**填写；论文中"人工复核"依赖它。
- `partial/A1` 2/3 接受、`terminated_ok` 但专家 `yes`：应在分歧表里出现并进入人工复核队列（已在）。
- A0 24/24 "接受"是平凡的（单轮无拒绝）；RESULTS 说明里要写明 A0 的 `accepted` 只表示产出了程序。

## 对实验的判断

主表已经能支撑三条主张：(a) 验证驱动修复 0 false-accept 而工具驱动基线以全绿接受带环程序；(b) 局部再生成把整体修订的 token 降到 1/4–1/5 且提高接受率；(c) 契约中的设计保持属性挡住空洞修复，且揭示局部/整体的能力边界。下一轮要补的是让 A3 落到 Rust 并通过 conform（J-7）、把专家轨做成逐候选且少 `unsure`（J-2）、查清两个参考程序（J-3）、修报告 bug（J-1/J-4）、增加 `A3_tiered`（J-5）、给抽取轨设终止判据（J-6）。做完这轮，就可以开始写实验节。
