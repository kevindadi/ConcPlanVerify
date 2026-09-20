你在 `/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）和 `/Users/kevin/local-repos/ConcIR`（Rust 工具）两个仓库中工作。先读 `reviews/experiments-v2-round-k-working-tree/REVIEW.md`（K-1..K-6），再读 `experiments/EXPERIMENTS_V2_HANDOFF.md` 的 Round k、`docs/PAPER_EVIDENCE_MAP.md`、`experiments/a3-to-rust-v1/SUMMARY.md`。

本轮目标：**把 conform 从"100% 但无从失败"变成有测得召回的后验证度量，补齐泛化面，关闭遗留偏差，然后冻结数据并产出论文可直接 `\input` 的表格**。这是实验阶段的收尾轮；做完即进入写作。各节独立提交；**§1–§3、§6 必须完成**，§4–§5 做不完写停点。不改 benchmark 的 buggy 输入与冻结契约；不放宽 strict codegen 模式的 conform 规则；不重启抽取轨。

预算：LLM 请求总上限 **220**（§2 ≤60，§3 ≤20，§4 ≤70，§5 ≤60，其余 0）。

---

## §1 conform 突变敏感性：`conform-mutation-v1`（K-1.1）——不发请求

对 `a3-to-rust-v1` 的 19 个生成程序（以及 8 个任务的 `fixed.rs` 参考若已有 `cir_trace` 标注，否则只用前者）注入受控同步突变。突变算子（写进 `PROTOCOL.md`，每个算子给出 AST 级定义，用 `syn` 或与 `concir-instrument` 同一套遍历实现，放在 ConcIR 新 bin `concir-mutate` 或 ConcPV 脚本中，任选其一但要写明）：

| id | 算子 | 期望被谁抓到 |
| --- | --- | --- |
| M1 | 交换同一函数内相邻两次 `lock()` 的顺序 | conform（顺序/ev 不匹配）或 explore（若引入环） |
| M2 | 删除一次 `drop(guard)`/提前 unlock，使持锁区间延长 | conform（unlock 事件缺失） |
| M3 | 把一个 `cir_trace::ev` 移到对应调用之前 | conform（事件顺序） |
| M4 | `notify_all` ↔ `notify_one` | conform（事件种类） |
| M5 | 把 `send`/`recv` 移入或移出临界区 | conform |
| M6 | 删除一个 `ev` 但保留调用（代码"未标注"） | conform（事件缺失） |
| M7 | 交换两个线程 spawn 的顺序（语义等价，**期望 PASS**，作为假阳对照） | 无 |

每个程序对每个适用算子生成 ≥1 个突变体（目标总量 ≥80，M7 ≥10）。每个突变体跑：build → strict conform → behavior → Miri 16。

产出 `experiments/conform-mutation-v1/{PROTOCOL.md,MUTANTS.json,SUMMARY.md}`：
- 按算子：突变体数、conform FAIL 数（检出率）、Miri detected 数、behavior hang/wrong_state 数；M7 的 conform PASS 率（假阳率）。
- 每个 conform FAIL 的 `reason` 类别分布。
- 若某类突变 conform 未检出，逐条分析是**算子等价**（记为等价突变并排除）还是 **conform 盲区**（记 `CONFORM_GAPS.md`，本轮不改 conform 规则）。

验收：M1–M6 合计 conform 检出率有数字；M7 假阳率有数字；`CONFORM_GAPS.md`（可为空）提交。提交 `conform-mutation-v1: mutation sensitivity of strict conform`。

---

## §2 后编辑一致性：`post-edit-conform-v1`（K-1.2）——≤60 请求

模拟"开发者拿到生成代码后继续改"的流程。对 19 个生成程序，各做 3 种编辑任务（prompt 写进 `prompts/post_edit_v1.md`，**不提并发、不提 CIR、不提 ev**，只给完整 Rust 与自然语言任务）：

- E1 加日志：在每个线程的关键步骤打印进度（`eprintln!`）。
- E2 重构：把每个线程闭包体抽成一个独立 `fn`，保持行为。
- E3 "顺手优化"：让模型"在不改变功能的前提下减少锁持有时间/提高吞吐"。

每个 (程序, 编辑) 一次请求，57 请求。每个编辑结果跑：build → strict conform → behavior → Miri 16 → explore（对编辑后代码用 `concir-instrument` 重新标注后能否 conform；**不允许**让模型修 ev）。

产出 `experiments/post-edit-conform-v1/{PROTOCOL.md,CELLS.json,SUMMARY.md}`：
- 按编辑类型：build 率、conform PASS 率、behavior、Miri。
- **关键指标** `drift_caught_only_by_conform`：conform FAIL 且 behavior=terminated_ok 且 Miri clean 的格数，逐格给 diff 摘要与 conform reason（这些是"工具全绿但已偏离模型"的直接证据）。
- E3 单独分析：模型"优化"了什么，是否破坏 `holds_all`/顺序。
- 若 conform FAIL 是因为 `concir-instrument` 无法标注重构后的代码（例如锁调用进入了辅助函数），记 `instrument_limit`，不算 drift，单列。

验收：三种编辑各 19 格有结果；`drift_caught_only_by_conform` 有数字与逐格清单。提交 `post-edit-conform-v1: developer edits vs strict conform`。

---

## §3 A3_tiered 补充（K-2）——≤20 请求

1. `PROTOCOL.md` 增加升级触发 T2：**第 1 轮**诊断含 `holds_all`/`never_holds_all` hint 且候选相对输入的 diff 只包含释放/重排（用现有 `stalled_local_patch` 判定）→ 立即升级。
2. 仅对 `partial_deadlock_bystander` 跑：tiered(T2, K=4) × 3 rep 与 tiered(原触发, K=6) × 3 rep，≤ 20 请求。
3. 写进 `case-partial-deadlock-v1/CASE.md` 新节 "Tiered escalation"：两种设置各接受几次、轮数、tokens；回答"tiered 失败是预算切分还是策略问题"。

验收：CASE.md 更新；RESULTS 中 A3_tiered 主行不变（主表仍是 K=4 原触发），补充结果单列。提交 `A3_tiered: early-escalation trigger + K=6 on partial_deadlock (case addendum)`。

---

## §4 主表扩到 10 任务（K-4a）——≤70 请求

把 `lock-order/abba_2lock` 与 `condvar/bare_wait_no_predicate` 加进 `flash-repair-main-v1`（同协议、同 `BIN_MAIN`，6 臂 × 3 rep）。前置：两任务的 `repair_input/` 去泄漏 lint 通过、冻结契约含设计保持属性（若契约缺 `preserved`，按既有流程补并记入 `CONTRACT_STRENGTH.md`，**不改 buggy 输入**）。接受的 A3 CIR 走 `a3-to-rust` 与专家标注（按 sha 去重）。

验收：RESULTS 主表 10 任务；按臂汇总重算；HANDOFF 写明新任务是否改变任何按臂结论。提交 `flash-repair-main-v1: +abba_2lock, +bare_wait_no_predicate (10 tasks)`。

---

## §5 第二模型探针：`model-probe-v1`（K-4b）——≤60 请求

端点上任一非 Flash 模型（优先 `deepseek-chat`；不可用则记录并跳过本节）。只跑 `A2_tools_iter_ml` 与 `A3_local`，10 任务 × 1 rep、K=4、同协议。产出 `experiments/model-probe-v1/SUMMARY.md`：与 Flash 同格对照（接受、false-accept、轮数、tokens）。**不进主表**，RESULTS 单列一节。

提交 `model-probe-v1: second model on A2-ml and A3_local`。

---

## §6 收尾：偏差关闭、数据冻结、LaTeX 表格

### 6.1 A2-ml 离线重分类（K-3）——不发请求
用修复后的 `classify_detection` 重跑 `flash-repair-main-v1` 全部 A2 轮次的已存 Miri/Lockbud 输出，产出 `flash-repair-main-v1/A2_RECLASS.md`：每轮 `old → new` 分类，`tools_green` 决策是否有变化。若无变化，RESULTS 偏差里该条改为"已离线验证无影响"。

### 6.2 a3-to-rust 补列
`traces` 列旁加 `distinct_traces`；PROTOCOL 写明 36 的含义。

### 6.3 LaTeX 表格生成
`python -m cir_workflow results --latex experiments/tables/`：至少产出 `main.tex`（主表，按任务×臂，k/n 与均值±极差）、`arms.tex`（按臂汇总，含 conform_pass_rate、tokens/correct）、`trackd.tex`、`mutation.tex`（§1）、`postedit.tex`（§2）、`expert.tex`、`scale.tex`。每个文件头部注释写生成命令与 sha。用 `booktabs`，不含 `\begin{table}` 外壳（论文侧包）。

### 6.4 冻结
- 全部 ConcPV 提交后打 tag `experiments-v2-freeze-1`；ConcIR 对应 commit 打 `concir-freeze-1`。
- `experiments/FREEZE_MANIFEST.md`：两仓 tag/commit、`BIN_MAIN` 与任何其他 binary sha、每个实验目录的 sha256（目录级，用 `find … | sort | sha256sum` 写明命令）、请求总数与分节预算。
- `docs/PAPER_EVIDENCE_MAP.md` 更新：(d) 指向 §1/§2 的召回与 drift 数字；(e) 指向 §3；新增 (j) 泛化（10 任务 + 第二模型）；(h) 若你已填人工队列则改 ready。
- HANDOFF `# Round 2026-09-2xl`：K-1..K-6 对照表、预算、停点、`Not done`。

提交 `freeze: experiments-v2-freeze-1, LaTeX tables, evidence map`。

---

## 全局规则

- 同一 `BIN_MAIN` 贯穿全轮；ConcIR 若有提交（`concir-mutate` 等），新 binary 只用于新增实验，主表判定不重算，除非变更触及 explore/conform 语义（若触及则重跑 `REBASE_<sha8>.md`）。
- RESULTS.md 与 `tables/*.tex` 只能由命令生成。
- 每次 LLM 请求写 `requests.jsonl`；每次工具调用落 `calls/<seq>-<tool>/`。
- `HUMAN_REVIEW_QUEUE.md` 不得由代理填写；若用户已填写，合并进 `EXPERT_LABELS.json` 的 `human_label` 字段并在 RESULTS 专家表加 `human` 列。
- 每节独立提交；做不完的节在 HANDOFF `Not done` 写停点与原因。
