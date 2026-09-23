你在三个仓库中工作：`/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）、`/Users/kevin/local-repos/ConcIR`（Rust 验证后端）、`/Users/kevin/paper-review/papers/ConcPlanVerify/`（论文 LaTeX，只放 `.tex/.bib/.sty/figures/tables/notes`，遵守该工作区 `AGENTS.md`，不提交编译产物）。先读 `ConcPlanVerify/reviews/experiments-v2-round-o-working-tree/REVIEW.md`（O-1..O-6）、HANDOFF Round o、`python/cir_workflow/generation.py` 的 `run_llmcode_from_cir / mapping_to_cir / _rewrite_traces / _conform_all_op_resource`、`python/cir_workflow/gen_results.py`、ConcIR `src/conform.rs`（`--op-resource`）、`cir_trace` 运行时（instrument v2 的 `sync` 包装类型）。

## 本轮要解决的事实（必须先理解）

freeze-5 中 G3 v2 代码阶段 28/59 格失败，全部是 `post_verify_fail` × 3 轮、conform `violation 32/32`、behavior/monitor 都 OK。REVIEW O-1 用 `concir-backend conform <cir> <trace> --op-resource` 重放存档 trace 证明这些失败是 **harness 编码缺口**，不是 LLM 偏离设计：

1. `Condvar::wait(guard)` 的包装先发 `mutex_unlock`（guard Drop）、唤醒后发 `mutex_lock`、最后才发 `condvar_wait`；CIR 的 `wait` 是单步。→ 标准 `while !*ready { ready = cv.wait(ready) }` 必违规。
2. std 无 Semaphore，LLM 用 `Mutex<usize>+Condvar` 自实现；instrument 看到 mutex/condvar，CIR 资源是 `Semaphore`，映射不到。基线臂在 semaphore 任务上同样 `unmapped`。
3. `mapping_to_cir` 把 spawn 顺序 zip 到 CIR 模块顺序的 worker；spawn 顺序不同或同 worker 多实例即线程错位（`expected []`）。
4. LLM 给 CIR 的 `var c` 单独包 `Mutex` 是真实偏离，但反馈只有 `no model statement matches mutex_lock on c_mutex0 (event 1)`，prompt 也没约定"var → 保护锁内字段，不加锁"，LLM 三轮不改。

另外 O-2：`gen_results.aggregate` 的 `RF_all` 用 `_mean` 丢 `None`，G3 未接受格 `coverage=None` → G3 的 `RF_all == RF_acc`，被高估约 2.3 倍；G0/G1/G2 未接受格有 monitor 值，口径不一致。

截稿 **2026-10-02 AoE**，今天 09-23。**D1 修工具与统计，D2 离线重放 + 活跑代码阶段 + freeze-6，D3–D8 只写论文**。预算：DeepSeek Flash ≤ 260 请求（59×3 + 余量），OpenCode Go ≤ 80 请求。各节独立提交；停点写 HANDOFF。

---

# Part A（D1–D2）

## §1 harness 修复（D1 上午，ConcIR + ConcPV）

### 1.1 condvar wait 编码（ConcIR `cir_trace` / instrument v2）

`cir_trace::sync::Condvar::wait / wait_while / wait_timeout` 在整个调用期间抑制被消费 guard 的 `mutex_unlock` 与重获的 `mutex_lock` 事件，只发一个 `condvar_wait`（r = cv；如 conform 需要，附 `m` 字段指明配对 mutex）。`notify_one/notify_all` 不变。**回归**：`experiments/conform-mutation-v2` 里所有"unlock 提前 / wait 前无锁 / 锁序"突变仍须被抓到（重跑该套件，写进 `CONFORM_GAPS.md` 新小节"after wait-encoding fix"）。不要在 conform 里做"容忍 unlock/lock 包裹 wait"的放宽——那会掩盖真正的 unlock-before-wait 突变。

### 1.2 semaphore 运行时（对所有臂一致）

std 没有 Semaphore。提供一个极小 crate `concir_sync`（或 `cir_trace::sync::Semaphore`，对外名字不带 `cir_trace`）：`Semaphore::new(n)`、`acquire() -> Permit`（Permit Drop = release）、`try_acquire`、`release`。instrument v2 把它识别为 `Semaphore` 资源，发 `sem_acquire/sem_release`。**所有生成臂**（G0/G1/G2/G3、探针）的 system prompt 加同一句话："可以使用 `std::sync`、`std::sync::mpsc` 与提供的 `concir_sync::Semaphore`（计数信号量：`new/acquire/release`）；不要自实现信号量。"这不是设计泄漏，是库；四臂措辞完全相同，写进 PROTOCOL。若时间不够对 G0/G1/G2 重跑，则在表注写明基线 semaphore 任务的 `unmapped` 来自这一限制（12/54），G3 重跑后按同一规则计。

### 1.3 线程对齐（ConcPV `mapping_to_cir`，必要时 ConcIR conform）

优先按名字：instrument 的 spawn 标签记录闭包内首个被调用的函数名（或 LLM 遵守实体命名时的 worker 名），与 CIR `module::function` 末段匹配；名字匹配不上再回退到顺序。同名多实例 → 一对多允许（conform `--op-resource` 对同一 CIR 函数允许多个线程各自独立推进；确认现有实现是否支持，不支持则加）。把最终线程映射写进 `mapping.json` 并在反馈里给 LLM 看（"线程 `t2` 被当作 `notifier`"）。

### 1.4 反馈与 prompt 约定（ConcPV）

- `_llmcode_system`/`rust_from_cir_v1.md` 加约定："每个 CIR mutex/condvar/channel/semaphore 对应且仅对应一个同步原语；CIR `var` 是普通共享数据，放进保护它的 mutex 的 `Mutex<...>` 里（或作为局部变量），**不要为 var 单独加锁**；`main` 只做 CIR 的 `main` 所做的事（spawn/join/打印终态），不额外访问共享状态。"
- 反馈 = conform `first_violation` 的 `detail + expected + got + thread + event_index`，再加一条按 `kind` 生成的自然语言解释（extra_lock / missing_wait / order / unmapped_resource / thread_mismatch）。`CELL.json` 每轮记录 `first_violation` 全文与 `kind`。
- `_defect` 统一为 `accepted ∧ (hang ∨ monitor FAIL ∨ Miri detected)`，所有臂一致（O-4）。

### 1.5 RF 口径（ConcPV `gen_results.py`）

- `RF_all`：**未接受格记 0**，所有臂一致（不看它有没有 monitor 值）；`RF_acc`：仅接受格。表头与 PROTOCOL 写清两者语义（`RF_all` = 交付质量含失败，`RF_acc` = 交付程序的需求满足率）。
- 追加一列 `RF_run`：对有 monitor 覆盖值的格取均值（旧口径），只进 RESULTS.md 不进主表，方便对照 freeze-5。
- 单测：三种口径对 G3 v2 与 G0 各算一格。

提交：（ConcIR）`cir_trace: single-step condvar wait; concir_sync::Semaphore; conform: name-based thread alignment`；（ConcPV）`G3 v2 feedback with violation kind; var-in-lock convention; RF_all counts non-accepted as 0`。tag `concir-freeze-5`。

## §2 离线重放（D1 下午，零 LLM 请求）

对 `flash-gen-main-v2/run-20260923T005232` 中 28 个代码阶段失败的 G3 格，取其 `code/round-3.rs`（或最后一轮）在修复后的工具链上重跑 instrument → build → conform → monitor，产出 `experiments/gen-code-replay-v1/SUMMARY.md`：

- 每格：修复前 `violation 32/32` → 修复后 conform 状态；仍违规者的 `kind` 与一句人读判断（真实偏离 / 仍是 harness）。
- 汇总："harness 缺口占失败格比例"（目标：写进论文 Threats 与 RQ2 的解释），按家族。
- 同样重放 31 个接受格，确认无回归（修复后应仍 conformant；出现新违规必须解释）。
- 重放 `gen-llmcode-smoke-v1` 的 45 个 Rust，得到修复后 conform PASS 率对比 15/45。

这份重放只用于诊断与论文的 Threats 段；**主表数字来自 §3 活跑**。

## §3 活跑代码阶段 `flash-gen-main-v3-code`（D2）

- 复用 freeze-5 的 59 个模型 PASS CIR（路径见各格 `cir_path`），只跑 G3 v2 的代码阶段（K_code = 3，修复后的 prompt/反馈/工具），3 rep 对应各自 rep 的 CIR。≤ 177 Flash 请求。**不重跑 CIR 阶段**，PROTOCOL 写明并给出 CIR 来源的 run id 与 sha。
- 13 个 CIR 阶段未 PASS 的格沿用 freeze-5 结果（含 3 个 UNSUPPORTED）。
- `G3_codegen` 消融不重跑（其 conform 用的也是旧编码；表注说明其数字是 freeze-5 的，或若 §2 重放显示消融也受 1.1 影响，则只对 rep0 的 24 个 CIR 重放 codegen 产物——零请求）。
- 若 §1.2 已就位且预算允许（Flash 剩余 ≥ 60）：只对 semaphore 家族 3 任务 × 3 rep 重跑 G0/G1/G2（≤ 27 + 迭代），其余任务沿用 freeze-5；PROTOCOL 写清哪些格是新跑。否则不跑，走表注。
- kimi-k3：只重跑 G3 代码阶段 24 格（≤ 72 Go 请求），CIR 复用 `gen-model-probe-v2`。
- 指标同 freeze-5 + `RF_all` 新口径 + conform violation kind 分布 + "conform 抓到、monitor/behavior 没抓到"的格数（这次这一数字才有意义，要区分 kind 是真实偏离的格）。
- 接受的 G3 Rust：agent-proxy 专家标注 rubric v3（`bug_present`、逐条 `Ri`），按 sha 去重；≥ 8 格人工抽样队列 `HUMAN_REVIEW_QUEUE_GEN.md` 留空给用户（agent 不填）。

## §4 freeze-6（D2 晚）

`python -m cir_workflow results …` 合并 v3-code、重放、探针；重生成 `tables/*.tex`（`gen_main.tex` 双列 RF 新口径；`gen_g3_stages.tex` 加"代码阶段 conform 拒绝 → 其中真实偏离"两列；新增 `gen_replay.tex`）；`FREEZE_MANIFEST.md` 追加；tag `experiments-v2-freeze-6`。HANDOFF Round p 写：freeze-5 → freeze-6 每个主指标的变化与原因（O-1 修复 / O-2 口径），一段话。

## 止损

- §1.1 修复后 `conform-mutation-v2` 若有突变从"抓到"变"漏掉"→ 停，写 HANDOFF，不进 §3。
- §2 重放若"harness 缺口占比" < 50%（即多数失败确是 LLM 偏离）→ §3 仍跑，但论文 RQ2 主张改为"conform 拒绝真实偏离 N/59，反馈 K_code=3 内修复 M 格"，并把 accept 与 codegen 消融并列如实报。
- §3 若 G3 v2 接受率仍 < 0.6 → 不再迭代工具，进入 Part B；主张落在 `RF_acc`、awp、Complex 档与 `accepted_with_proof`，`RF_all` 新口径照实报。

---

# Part B（D3–D8）论文 —— 只用 freeze-6 数字

## §B1 差距重审（D3 上午）

更新 `notes/GAP_AUDIT.md` 为 v4：逐条对照 `docs/PAPER_EVIDENCE_MAP.md`（freeze-6）与 `paper.tex`，每条 keep/rewrite/delete/add。重点：
- §2 Overview 运行示例是否已是 Medium 档生成任务（GAP_AUDIT v3 要求，核实）。
- §3.9 Code construction 已按 LLM-写代码写好；补一句 harness 提供 `concir_sync::Semaphore` 与 wait 单步事件的说明。
- §5 RQ2：conform 拒绝数字全部替换为 freeze-6 的"真实偏离"口径；harness 缺口进 Threats。
- RF 措辞（O-3）：设计层契约穷尽 PASS（`cir_pass`），代码层 RF 对所有臂由 monitor 有界测得，conform 连接两者；表注写明 `RF_all` 未接受记 0。

## §B2 评估节（D3–D5）

- RQ1 主表 `gen_main.tex`（四臂 + codegen 消融行）+ 分档 `gen_tiers.tex`：G3 在 Complex 档的优势、Simple 档的劣势都写；`RF_all` 新口径下若 G3 < G0，正文直说"含失败格时 G3 交付更少但交付的更正确、且带证明"，不回避。
- RQ2 `gen_g3_stages.tex` + `gen_replay.tex`：CIR 阶段（PASS/INVALID/FAIL/UNSUPPORTED、轮数）→ 代码阶段（build、conform 真实偏离、反馈修复、monitor）。
- RQ3 成本：tokens/correct、轮数、verify_ms，分阶段。
- RQ4 conform 价值：`gen_conform_value.tex`（conform 抓到而 monitor/behavior 没抓到）+ `conform_recall_v1v2.tex` + 突变回归。
- RQ5/RQ6 修复第二研究：沿用 freeze-5 表（`main.tex`、`main_detail.tex`、`mutation.tex`、`postedit.tex`、`scale.tex`、`trackd.tex`），只校对数字与措辞。
- Expert/human review 小节：生成格 rubric v3 结果 + 人工队列状态（"由作者复核 N 格，其中 …"——数字由用户填，留 `\todo`）。
- Threats：harness 编码缺口（重放占比）、`[U]` 21%、bounded oracle、单一主模型 + kimi 探针、W1xx 限制、benchmark 自建。

## §B3 引言/概览/方法/相关工作/结论（D5–D7）

- 引言贡献列表与摘要数字对齐 freeze-6；"验证后端只验 CIR、LLM 写代码、工具后验证"作为方法定位一句话。
- Related Work 加 Event-B Agent（arXiv 2605.17475）对比段：同为需求→形式模型→代码，差异在并发 IR、Petri 网穷尽验证、代码侧 conform。
- 方法节里所有 `codegen` 作为主路径的残留措辞清掉（grep `translator|codegen`）。
- D7 全文编译、页数对 FSE 模板、参考文献完整；`notes/TABLES_SYNC.md` 记录每张表来源命令。
- D8 留给用户通读与 `owner_verdict`。

## 全局规则

- API key 只从 `.env` 读，不写进任何提交文件与 `requests.jsonl`。
- `HUMAN_REVIEW_QUEUE*.md`、`owner_verdict` 只由用户填。
- 契约对生成模型不可见；实体命名与 `concir_sync` 库说明可以给。
- 不改 benchmark 需求与冻结契约（v3.1 冻结）；不改 freeze-5 的原始数据目录，新数据放新目录。
- `RESULTS.md`、`tables/*.tex` 只由命令生成；论文目录只放 LaTeX 与 `notes/*.md`。
- 每节结束提交；任何止损触发先写 HANDOFF 再停。
