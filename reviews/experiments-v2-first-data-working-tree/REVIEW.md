# 复核：experiments-v2 第 f 轮（第一批修复型对照数据）

复核对象：`ConcPlanVerify` HEAD `0038f08`（干净）；`ConcIR` HEAD `79c9440`（干净）。
复核方法：读 HANDOFF Round f、`flash-repair-smoke-v2/run-…47223d/SUMMARY.{md,json}`、`conformance-v3/CONFORMANCE.md`；打开 A3 三个任务的接受版 CIR 与人工 `fixed.cir.json` 逐语句对照；读 `extract.py` 与抽取 prompt；核对 `oracle` 原始字段。

## 结论

- **接受**：N-1 隐式 return 语义统一 + `E114` + `engine_agreement`（38 用例零分歧）；N-2 `thread_leak` 分类；R-3 诊断（partial_deadlock 的对称 `doom_state` 已出现）；N-4 版本绑定；R-6 多模块 codegen（`cross_module_cycle` fixed 5/5 conformant）；conformance-v3 四个旧用例 100%；HOLE 填充臂 66/66；**repair-smoke-v2 是第一批可采信的数据**：A0 在 partial_deadlock 上出现可测的 false-accept（accept r1，`behavior=hang`，miri 16 种子全 `timeout`），A3 三个任务全部 accept 且模型 PASS，A2 去泄漏后从 1 轮变 4 轮。
- **但对这批数据的两处解读要收回**（F-1、F-3），并且发现一个会在扩大规模时放大的**契约欠规范**问题（F-1），这比继续加任务更紧要。

## F-1 契约欠规范：A3 在 partial_deadlock 上的"修复"是设计变更（P1）

对照：
- 人工 `fixed.cir.json`：a、b 都按全局顺序 `lock a → lock b → unlock b → unlock a`，**仍然同时持有两把锁**，只是去掉握手。
- A3 接受版（v3）：a 变成 `lock a → release → unlock a → lock b → unlock b`；b 变成 `acquire → lock b → unlock b → lock a → unlock a`；旁观者 c 也被改成依次拿 a、b。**没有任何线程再同时持有两把锁**——原设计中"在同时持有 a、b 的临界区里做事"这一意图被删掉了。
- 对照 A2-m 第 4 轮的 Rust：先握手再 `lock a; lock b`，保留了嵌套持有。

当前 contract 的 `preserved` 只有 `function_completed`，`PredicateSpec` 只有 `VarEq/VarCmp/FunctionCompleted(AtLeast)/ScopeCompleted/StatementReached`，**没有任何谓词能表达"某线程同时持有若干资源"**，所以"删掉临界区"这类空洞修复在模型层是 PASS。这正是论文里"goal/preserved 防止静默删行为"要防的事，而现有契约防不住。
要求：ConcIR 新增谓词 `holds_all { function, resources[] }`（执行 `function` 的某线程同时持有全部 `resources`）与 `safety` 里的互斥类不变式（如 `mutex_exclusive`）；为全部 buggy 用例重写 contract，把设计意图写进 `preserved`；然后**离线**用新 contract 重跑 v2 里 A3 的三个接受版 CIR，报告哪些会被新契约拒绝。这一步不需要 LLM，是本轮最有价值的实验：它给出"契约强度 vs 空洞修复率"的第一组数字。

## F-2 抽取 oracle 0/12 的原因是琐碎的（P1，修起来便宜）

- 12 格中 10 格 `codegen failed: JSON parse error … unknown field 'contract'`：LLM 把 contract 塞进 CIR 顶层，strict serde 直接拒绝。`normalize.py` 应把**顶层**未知字段（`contract`、`description`…）剥离并记录；抽取 prompt 明说不要 contract。
- 2 格 `reply not a {cir,rust} object`：需要解析回退（从 ```json 块中取）与一次格式反馈重试。
- **更根本的错配**：抽取 prompt 要求 `ev` 插在并发操作**之前**，而 `conform` 自 round d 起的规则是 lock/acquire/wait/channel 的事件 = **完成**步（调用返回后）。即使构建成功，"之前"的事件会在阻塞型操作上系统性地违规。prompt 必须改成与 `doc/backend-usage.md` 的规则一致，并要求 `cir_trace::finish()`。
- 修好后**不需要重跑臂**：12 个候选 Rust 都在盘上，抽取 ≤24 请求即可补齐 `oracle.model`。

## F-3 "terminated" 被写成 "bug fixed"（P1，措辞）

HANDOFF："A2-m/A2-ml on partial … all end `terminated` (bug fixed)"。`behavior` 列是 10 s 看门狗跑一遍，只证明"这次没挂"，不证明缺陷修好，更不证明行为保持（把 wait 整个删掉也 `terminated`）。在 `oracle.model` 为 `unverified` 的前提下，10 个 `terminated` 应记为 **`terminated, unverified`**。HANDOFF 与 SUMMARY 里所有 "bug fixed" 措辞改掉。
补强：`repair_input/requirements.txt` 追加一条可观测终态要求（例如"完成时精确打印一行 `DONE a=1 b=1 c=1`"或"`ready=true` 且 waiter 完成"），看门狗检查该行；这是需求层可观测，不泄漏缺陷。原 `behavior.rs` ×6 的意图由此替代，登记偏差。

## F-4 SUMMARY 展示掩盖了证据（P2）

- `oracle.miri` 列显示 `False`，而原始字段是 `miri_statuses = timeout ×16`（A0 partial）。这恰好是"Miri 对带旁观者的部分死锁无能为力"的直接证据，却被布尔列吞掉。列改为状态计数（`clean 16` / `timeout 16` / `thread_leak 3,clean 13`）。
- `oracle.model` 文本被截断到 "codegen failed: J"。

## F-5 channel 一致性仍是 violation（P1，已记录未修）

`rendezvous_both_send(fixed)` 0/58、`bounded_backpressure(fixed)` 49/58。HANDOFF 的分析是运行时"接收方先完成"与模型"归到后到者"的归因不一致。这是 `conform` 的匹配规则问题，不是程序问题：cap=0 的 rendezvous 在模型里是**一个**步，在轨迹里是**两个**完成事件、顺序任意。`conform` 应把该步当作需要消费两个事件的复合步，中间允许其他线程的事件插入。三个 channel buggy 用例 + real case 都卡在这里，不能再"记录"。

## F-6 `A3_free` 仍无数据（P2）

自由生成的程序没调 `cir_trace::finish()`，66 条轨迹全 missing。prompt 加硬性要求，1 次请求重跑。

## 其他

- HANDOFF §6 的 `cross_module_cycle` 结果未写 binary sha / git_rev（应为 `79c9440` 的构建）。
- `SUMMARY.md` v1→v2 对照表的 `reason` 列是人工标注，应说明依据（如 A3 三个 changed 的依据是 v2 每轮 decision）。
- Rust 238 / Python 100 全过。

## 对实验的判断

有了第一组可采信数字后，下一步**不是加任务**，而是先做 F-1：如果不把设计意图写进契约，扩大规模只会批量制造"模型 PASS 的空洞修复"，到审稿时被一个反例打穿。F-1 离线可做且直接产出论文可用的数字（契约强度对空洞修复的拦截率）。F-2/F-3/F-4 让现有 12 格 Rust 数据变得完整可信，成本 ≤24 请求。F-5 修完 channel 族才能进主表。做完这些再把任务扩到全部 hard 用例。
