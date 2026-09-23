# 复核：experiments-v2 第 n 轮（生成为主：benchmark v3、有界 oracle、`flash-gen-main-v1`、kimi-k3 探针、freeze-4）

复核对象：`ConcPlanVerify` HEAD `419f0df8`，tag `experiments-v2-freeze-4`；`ConcIR` HEAD `6e2a3de`，tag `concir-freeze-3`。
复核方法：读 HANDOFF Round n、`flash-gen-main-v1/{PROTOCOL,SUMMARY}.md`、`gen-model-probe-v1/SUMMARY.md`、`GENERATION_MANIFEST.json`；用脚本合并两批 `SUMMARY.json` 的 288 格，逐格核对 G3 的 `model_outcome / conform / codegen_error / name_alignment`；读 `same_cv_different_locks` 的需求文档。

## 结论

- §0–§3、§6 按计划交付：24 任务三档、`req` 标签契约、instrument v2 + `monitor` 有界 oracle（10/10 buggy hang、8/10 fixed 全 `PASS_bounded`）、288 格主批次、kimi-k3 72 格探针、freeze-4。方向转换在两天内落地，这是本轮的主要成绩。
- 头条数字方向正确但口径不稳：G3 的 RF 0.504（Flash）/ 0.684（kimi）为四臂最高，RC 最高；但 G3 accept 只有 45/72，且**只有 20/45 拿到 conform 证明**。逐格核对后，这两个缺口都不是模型能力问题，而是**工具 codegen 这一环**和**名字对齐**造成的（N-1、N-2）。
- 用户提出的架构判断——验证后端只负责验证 CIR，代码应由 LLM 依据已验证的 CIR 生成，工具生成代码必然受限——**被本轮数据直接证实**（N-1）。这是下一轮唯一必须做的方法变更。

## N-1 工具 codegen 是 G3 的瓶颈，且带来一处语义不一致（P1，方法）

45 个模型 PASS 的 G3 格里：
- **23 格 `codegen_error: filled project failed to build`**（`holes = []`，即工具自己生成的完整程序编不过），集中在 atomic-data、semaphore、lock-order、structure 四族。这些 CIR 已经通过穷尽验证，却因为 codegen 的表达/翻译能力止步，`accepted-with-proof` 被压到 20。
- **2 格 `same_cv_different_locks` conform `timeout 32/32`**：模型 PASS 但 codegen 的 Rust 挂起。原因是 ConcIR 的 condvar 语义允许一个 cv 配多把锁，`std::sync::Condvar` 不允许——CIR 与目标语言在这一构造上不一致，codegen 无法弥合。
- 早前 a3-to-rust 的 "0 holes / 正确即构造" 平凡性问题同源：codegen 只能覆盖纯同步骨架，一旦 CIR 承载了需求里的功能逻辑就失效。

结论：**把代码生成交给 LLM**——输入为需求文档 + 已验证 CIR，输出完整 Rust；工具侧只做 instrument v2（自动挂事件）→ conform v2（代码是否忠实于 CIR 的同步结构）→ monitor（契约的有界检查）→ behavior/Miri；conform/monitor 的违规反馈给 LLM 修 Rust（不改 CIR）。`accepted-with-proof` 重新定义为：模型 PASS（穷尽）∧ conform PASS ∧ monitor 无 FAIL。工具 codegen 保留为消融臂（`G3_codegen`），本轮的 23 个 build 失败与 2 个挂起正是它的对照数据。

## N-2 名字对齐掩盖了 27 个 `generation_failed` 的真实原因（P1，协议）

G3 未接受的 27 格里 `model_outcome` 是 **INVALID 20、FAIL 7**：INVALID 意味着契约里的资源/函数名在模型的 CIR 里找不到、无法求值。已接受的 45 格里 30 格依赖了启发式 `name_alignment`（`counter→c`、`supervisor→w1` 这类改名）。契约对模型不可见是对的，但**实体名字属于需求而非契约**：需求文档应当命名角色与资源（"锁 `account_a`"、"通知者 `notifier`"），契约沿用这些名字，启发式对齐取消。这样 INVALID 会大幅减少，Rust 臂 monitor 的 `resources.json` 映射也同样受益。需求文档一改，四臂都要重跑（Flash 便宜，一批约 630 请求）。

## N-3 RF 口径（P1，报告）

SUMMARY 的 RF 是"全部格，未接受记 0"：G3 0.504。只看接受格 G3 是 **0.774**。两者都要报（`RF_all` 惩罚不接受，`RF_acc` 衡量接受产物的质量），并给 accept rate；否则 Medium 档 G3 0.305 < 其他臂的现象无法解读（它主要来自 INVALID）。Event-B Agent 报的是最终模型的指标，相当于 `RF_acc`。

## N-4 `same_cv_different_locks` 的需求本身要求了一个反模式（P2，benchmark）

R2/R3 要求"两把不同锁 + 同一个条件变量"，这是修复 benchmark 里的 bug 模式被直接抄成了生成需求。生成设定下需求应写意图（两个 waiter 各守各的数据、notifier 必须唤醒两者且不依赖竞态），让设计自由。同时 ConcIR 应对"一个 cv 绑定多把锁"给出 `UNSUPPORTED`/警告而非 PASS（目标语言不支持）。

## N-5 未做与风险

- 生成格的专家标注未跑；loom 未做（可放弃）。
- **Part B 论文一行未动**，距截稿 9 天。
- 主批次 SUMMARY 由两批合并（"later overrides earlier"），RESULTS 头部要写清合并规则；`--check-generation` 通过但 `[U]` 比例未报。

## 对实验的判断

本轮把评估结构对齐到了 FSE 同类论文；数据同时暴露了架构里"工具 codegen"这一环是错误的分工。下一轮把 G3 改为"LLM 按已验证 CIR 写代码 + 工具后验证"，把实体名放进需求、重跑主批次与探针，RF 双口径。之后必须立刻转入写作。
