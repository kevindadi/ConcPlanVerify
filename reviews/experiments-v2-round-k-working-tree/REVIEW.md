# 复核：experiments-v2 第 k 轮（A3 落 Rust、逐候选 oracle、参考审计、A3_tiered、抽取冻结、论据地图）

复核对象：`ConcPlanVerify` HEAD `dc0e6341`（干净）；`ConcIR` HEAD `f42764f`（干净）。
复核方法：读 HANDOFF Round k、`PAPER_EVIDENCE_MAP.md`、`REFERENCE_AUDIT.md`、`a3-to-rust-v1/SUMMARY.*`、RESULTS 按臂汇总与 Track D；核对人工队列填写状态。

## 结论

- §1–§7 全部交付，42/260 请求。报告层的 J-1/J-4 修完；参考审计给出了两个可信的结论（Lockbud 不把 `drop(guard)` 当释放 → 假阳；Miri "detected" 是路径里的任务名匹配到 `deadlock` → 分类 bug，已修并加回归）。专家轨 v2 按候选 sha 去重，`unsure` 0/49，一致率 95/104，`design_loss` 6。抽取轨按终止判据冻结为 limitation，这是正确决定。A3_tiered 21/24、0 false-accept、684 tok/correct，与 A3_local 持平。
- 主表现在五（六）臂在 Rust 侧列对齐：A3 全部 19 个去重 CIR 经 codegen → build → strict conform PASS → `terminated_ok` → Miri clean 16。
- 但 §3 的结果**弱于表面**（K-1），这是本轮最需要处理的问题；其余是收尾与泛化。

## K-1 conform 100% 是近似平凡的（P1，论证）

`a3-to-rust-v1` 全部 19 个 CIR `holes = 0`：骨架即完整程序，LLM 没碰过一行 Rust，conform 检查的是"我们自己的 codegen 是否忠实于我们自己的 CIR"。这是 codegen 的正确性回归，不是"LLM 生成的具体代码符合 ConcIR 要求"的后验证证据。审稿人会指出：(d) 项在当前 benchmark 上是 correct-by-construction，conform 无从失败。

需要两个补充实验让 conform 有"能失败"的机会，并测出它的召回：

1. **突变敏感性（不发请求）**：对 19 个生成程序注入受控同步突变（交换相邻两次 lock 的顺序、删除一次 unlock/drop、把一个 `ev` 移到调用前、把 `notify_all` 改 `notify_one`、把 `send` 移出临界区），每个程序 ≥5 个突变体；跑 conform / behavior / Miri 16。报告 conform 对每类突变的检出率，与 Miri、behavior 对照。这给 conform 一个测得的召回，而不是 100% 的空数字。
2. **后编辑一致性（发请求）**：模拟真实开发流程——把生成的 Rust 交给 LLM 做一个与并发无关的"业务"改动（加日志、把循环体抽成辅助函数、给消息加 payload 计算、"顺手优化一下锁"）；然后 conform。度量：编辑后 conform PASS 率、被 conform 抓到而 Miri/behavior 没抓到的漂移数、抓到的漂移类型。这才是"落地到具体代码 + 后验证"的直接证据，也正对 2605.17475 那篇的缺口。

另外 19 行 `traces` 全部是 36——写明这是固定调度数还是去重后的不同 trace 数；若是前者，补一列 `distinct_traces`。

## K-2 A3_tiered 未能解决 `partial_deadlock`（P2）

K=4 下 local 用掉 2 轮（`explore_fail` → `stalled`），whole 只剩 2 轮，而 A3_whole 独立运行需要 3–4 轮。这是预算切分问题，不是策略问题。两个廉价的补充：升级触发提前到 "1 轮 `explore_fail` 且候选与输入的 diff 仅为释放/重排"（即诊断带 `holds_all` hint 时立即升级）；或对该任务单独跑 tiered K=6（≤ 20 请求）作为案例附录。两者都做则可以回答"tiered 是否只是预算问题"。

## K-3 A2-ml 未按新分类器重算（P2，偏差可离线消除）

分类器修复只会让检测更严格；但 A2 的 `tools_green` 判定依赖旧分类器对 Miri 输出的解读。所有 Miri 输出都在 `calls/` 里，用新分类器**离线重分类**全部 A2 轮次（0 请求），确认没有任何 `tools_green` 决策改变；写进 HANDOFF 即可关闭这条偏差。

## K-4 泛化面窄（P2，审稿风险）

8 任务 × 1 模型（Flash）× 3 rep。两条低成本扩展：
- Track D 里另有两个带 Rust 参考的任务（`abba_2lock`、`bare_wait_no_predicate`）没进主批次；加进去是 2 × 6 臂 × 3 rep ≈ 60 请求，主表变 10 任务。
- 第二模型探针（上上轮 §8 一直没跑）：`deepseek-chat` 或端点上任一非 Flash 模型，只跑 A2-ml 与 A3_local、10 任务 × 1 rep，≈ 60 请求。回答"结论是否 Flash 特有"。

## K-5 人工复核仍空白

`HUMAN_REVIEW_QUEUE.md` 13 行全空。(h) 项停在 partial，论文里"人工复核"不能写。这是用户本人的事，代理不能代填。

## K-6 数据冻结与论文表格

论据地图 9 项里 7 项 ready。下一步应当把实验数据**冻结**（tag + 全部 sha 清单），并把 RESULTS 里的表由生成器直接产出 LaTeX（`tables/*.tex`），论文只 `\input`。这样写作期间任何数字变化都可追溯到重新生成，不会出现手抄错数。

## 对实验的判断

主张 (a)(a′)(b)(c)(f)(g)(i) 已经站住；(d) 需要 K-1 的两个实验把 conform 从平凡变为有召回的度量；(e) 需要 K-2 的小补充；(h) 等你填表。做完 K-1..K-4 并冻结，实验部分就够写了。
