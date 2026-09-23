你在三个仓库中工作：`/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）、`/Users/kevin/local-repos/ConcIR`（Rust 验证后端）、`/Users/kevin/paper-review/papers/ConcPlanVerify/`（论文 LaTeX，只放 `.tex/.bib/.sty/figures/tables/notes`，遵守该工作区 `AGENTS.md`）。先读 `ConcPlanVerify/reviews/experiments-v2-round-n-working-tree/REVIEW.md`（N-1..N-5）、HANDOFF Round n、`flash-gen-main-v1/{PROTOCOL,SUMMARY}.md`、`prompts/concir_generation_v2.md`、`prompts/rust_generation_v1.md`、ConcIR `src/bin/concir-instrument.rs`、`src/conform.rs`、`src/monitor.rs`（或对应模块）。

## 架构修正（本轮的核心，必须先理解）

ConcIR 是**验证后端**：接收 LLM 从需求生成的 CIR，通过迭代把 CIR 修到满足契约；之后**由 LLM 依据这份已验证的 CIR 生成可执行 Rust**——不再由工具 codegen 生成代码。工具在代码侧只做三件事：`concir-instrument` v2 自动挂事件、`conform` v2 检查代码的同步结构是否忠实于 CIR、`monitor` 对契约做有界检查。conform/monitor 的违规作为反馈让 LLM 修 Rust（不改 CIR）。理由见 REVIEW N-1：上一轮 45 个模型 PASS 的 CIR 中 23 个被工具 codegen 编不过、2 个因 CIR 与 `std::sync::Condvar` 语义不一致而挂起——工具生成代码是瓶颈，也与"CIR 承载用户需求逻辑、LLM 负责实现"的定位相悖。工具 codegen 降为消融臂。

截稿 **2026-10-02 AoE**，今天 09-22。**D1–D4 出数据，D5 起写作**（Part B 与实验并行，先用 freeze-4 数字占位，最后换表）。预算：DeepSeek Flash ≤ 1500 请求，OpenCode Go ≤ 150 请求。各节独立提交；停点写 HANDOFF。

---

# Part A（D1–D4）

## §1 G3 v2：LLM 按已验证 CIR 生成代码 + 工具后验证（D1）

### 1.1 流程（`arms.py` 新臂 `G3_concir_llmcode`，替代主表中的 G3；旧流程改名 `G3_codegen` 作消融）

1. 需求 → LLM 生成 CIR（`concir_generation_v2`，契约不可见）→ normalize/check → explore vs 契约 → 失败则 `A3_local` 修订（T3 升级 whole），K_cir = 4。
2. 模型 PASS 后 → **LLM 生成 Rust**：新 prompt `prompts/rust_from_cir_v1.md`，输入 = 已验证 CIR（JSON）+ 需求文档 + 一段固定说明："CIR 是权威设计，支配线程、共享资源、同步操作顺序、分支与状态变量：每个 CIR 函数对应一个线程体/函数，每个资源对应一个同步原语或共享变量，同步与状态更新的顺序与 CIR 语句顺序一致。需求文档只用于补足 CIR 未建模的功能细节（标 `[U]` 的条目：载荷计算、输出格式等）与实体命名；两者冲突以 CIR 为准"。不给 sid、不提 `cir_trace`、不给契约。PROTOCOL 写明：契约覆盖的需求条目在 CIR 上穷尽成立，conform 证明代码同步结构忠实于 CIR，数据相关属性在代码上由 monitor 有界确认——三段各用各的词。
3. 工具侧：`concir-instrument` v2 → build → `conform` v2（vs 该 CIR）→ `monitor`（vs 契约，native 32 + Miri 16）→ behavior（`DONE` 终态）。
4. 违规反馈：conform 的 violation（kind、线程、期望 vs 观察）与 monitor FAIL 的属性（用需求条号 `Ri` 转述，不暴露契约原文）→ LLM 修 Rust，K_code = 3。build 失败也反馈。
5. 接受条件：build ∧ conform PASS ∧ monitor 无 FAIL ∧ behavior `terminated_ok`。记录 `accepted_with_proof = 模型 PASS ∧ conform PASS ∧ monitor 无 FAIL`。

### 1.2 instrument v2 的适用性

对 LLM 自由 Rust 挂事件依赖 instrument v2 的覆盖；失败原因（非 std 原语、锁在泛型/辅助函数、`parking_lot` 等）记 `instrument_limit` 并作为反馈让 LLM 改用 `std::sync`（一次）。`PROTOCOL.md` 写明这一限制。

### 1.3 冒烟

用 `flash-gen-main-v1` 中已接受的 45 个 CIR 直接跑 1.1 的第 2–5 步（跳过 CIR 阶段，≤ 45×4 = 180 Flash 请求），产出 `experiments/gen-llmcode-smoke-v1/SUMMARY.md`：build 率、instrument 失败率、conform PASS 率、monitor FAIL 分布、每格轮数、与 `G3_codegen` 同 CIR 的对照（23 个 codegen 编不过的 CIR 现在多少能落地）。**若 conform PASS 率 < 40%，先修 prompt/反馈再进 §3**。

提交（ConcPV）`G3 v2: LLM code from verified CIR + conform/monitor feedback; G3_codegen ablation`。

## §2 Benchmark v3.1：实体命名进需求、反模式需求改写（D1，与 §1 并行）

- 每个任务的 `REQUIREMENTS.md` 增加一段 "Entities"：列出角色（线程/函数）与共享资源的**名字**（与冻结契约中的 FQN 末段一致，如 `account_a`、`notifier`），并在 R 条目中使用这些名字。名字是需求的一部分，不是契约泄漏；契约原文仍不可见。
- 取消启发式 `name_alignment`；模型 CIR 里未使用需求命名的实体 → 契约求值 INVALID，作为反馈（"需求中的实体 `X` 未在设计中出现"）进入修订轮，不再静默改名。
- `same_cv_different_locks` 的 R2/R3 改为意图级描述（两个 waiter 各守各的数据；notifier 必须唤醒两者且不依赖竞态），契约对应属性同步调整；ConcIR 对"一个 condvar 绑定多把 mutex"新增 `W1xx unsupported_in_target`，模型判定为 `UNSUPPORTED`（不算 PASS）。
- monitor 的 `resources.json` 映射优先按需求命名匹配；仍失败才走 agent 手工映射并记录。
- `GENERATION_MANIFEST.json` 升 v3.1，写 `[U]` 比例；tag `contracts-v3.1`。

提交（ConcPV）`benchmarks v3.1: entity names in requirements; same_cv intent rewrite; no heuristic alignment`；（ConcIR）`sem: condvar bound to multiple mutexes -> UNSUPPORTED`。

## §3 主批次重跑 `flash-gen-main-v2`（D2–D3）

- 四臂：`G0_direct`、`G1_self_iter`、`G2_tools_iter`、`G3_concir`（= §1 v2 流程）；`G3_codegen` 作为第五臂只跑 rep 0（消融，≤ 24 任务）。K=4（G3 为 K_cir 4 + K_code 3），3 rep，Flash，temperature 0，`contracts-v3.1`。
- 预算 ≤ 1100；`rep=0` 全档优先。
- 指标：`accept_rate`；**`RF_all`**（未接受记 0）与 **`RF_acc`**（仅接受格）两口径都报；RC；`defect`（accepted ∧ (hang ∨ monitor FAIL ∨ Miri detected ∨ conform violation)）；`accepted_with_proof`；tokens、轮数、`verify_ms`；按 Simple/Medium/Complex 与 Overall。
- G3 额外：CIR 阶段与代码阶段分开的轮数/tokens；INVALID/FAIL/UNSUPPORTED 分布（INVALID 应显著下降，写进 HANDOFF 与 v1 对比）；instrument 失败率；conform violation kind 分布；"conform 抓到而 monitor/behavior 没抓到"的格数（这是后验证的直接价值）。
- 接受的 Rust 产物（四臂）：agent-proxy 专家标注（rubric v3：`bug_present`、逐条 `Ri` 满足），按 sha 去重；人工抽样队列 ≥ 8 格留给用户。

提交 `flash-gen-main-v2: requirements->CIR->LLM code, 4 arms x 3 reps + codegen ablation`。

## §4 前沿模型探针 `gen-model-probe-v2`（D4）——OpenCode Go ≤150

`kimi-k3`：`G0_direct` 与 `G3_concir` 两臂 × 24 任务 × 1 rep（约 24 + 24×(≤4+≤3) ≤ 190，超出则 Complex 档 G0 记 `not_run`，G3 优先）。同 oracle、同指标。提交 `gen-model-probe-v2: kimi-k3 G0/G3 under v3.1`。

## §5 结果冻结 freeze-5（D4）

- `python -m cir_workflow results …` 加入 v2 主批次、探针 v2、`gen-llmcode-smoke-v1`、`G3_codegen` 消融；`tables/*.tex` 重生成（`gen_main.tex` 含 `RF_all/RF_acc` 双列；新增 `gen_g3_stages.tex`、`gen_codegen_ablation.tex`、`gen_conform_value.tex`）。
- `FREEZE_MANIFEST.md` 追加；tag `experiments-v2-freeze-5` / `concir-freeze-4`。
- `docs/PAPER_EVIDENCE_MAP.md` v4：(1) 需求满足与缺陷（分档、双口径）；(2) 模型 PASS → 代码落地率与 conform 证明率，对照 codegen 消融；(3) conform 独立抓到的偏离；(4) 强基座；(5) 成本；(6) 修复研究（第二研究）；(7) Track D 与规模；(8) 专家轨与人工；(9) 抽取 limitation；(10) threats。
- HANDOFF `# Round 2026-09-2xo`：N-1..N-5 对照、预算、v1→v2 的 INVALID/落地率/证明率变化一句话结论、停点。

提交 `freeze-5; evidence map v4; handoff round o`。

---

# Part B（D2 起并行，D5–D9 主力，`papers/ConcPlanVerify/`）

## §B1 差距清单（D2）

`notes/GAP_AUDIT.md`：按 m 轮 §B1 范围，叙事改为"需求 → 已验证 CIR → LLM 实现 → 工具后验证"；Overview 例子选一个 Medium 档生成任务，给需求片段、CIR 片段、LLM Rust 片段、conform 反馈一轮的四段；方法节需新增：契约与需求条目的对应（RC/RF 定义）、代码生成阶段的 LLM 输入约定、instrument v2/conform v2/monitor 的定义与"穷尽 vs 有界"的区分；删除工具 codegen 作为主路径的描述（保留为消融）。

## §B2 评估节（D5–D7）

复制 `tables/*.tex`（`notes/TABLES_SYNC.md` 记 tag）。结构：
1. Setup：benchmark v3.1（24 任务、三档、需求条数分布、`[U]` 比例、实体命名规则）、四臂 + codegen 消融、两级 oracle、指标（含 `RF_all/RF_acc`）、模型与预算、binary sha。
2. RQ1 需求满足与缺陷（分档双口径；G3 的 `accepted_with_proof`）。
3. RQ2 从已验证 CIR 到代码：LLM 实现 vs 工具 codegen（落地率、证明率、`same_cv` 语义缺口）；conform 独立抓到的偏离；后验证召回（v1→v2 突变表）。
4. RQ3 强基座增量（kimi-k3）。
5. RQ4 成本（CIR 阶段/代码阶段分开）。
6. RQ5 缺陷检测与修复（第二研究，沿用 freeze-4 的修复结果）。
7. RQ6 检测能力与规模（Track D、scale）。
8. Threats：作者构造的需求/契约、agent-proxy 标注、单主模型、Rust 臂 oracle 有界、instrument 覆盖、抽取轨失败、M5 盲区、benchmark 规模。
数字全部 `\input`；`latexmk -pdf` 通过；图占位并列 `notes/FIGURES_TODO.md`。

## §B3 引言/概览/方法/相关工作（D7–D9）

按 `GAP_AUDIT.md` 重写；贡献列表与 RQ 一一对应；相关工作增补 Event-B Agent 等 LLM+形式化循环、Rust 并发工具（Miri/Lockbud/loom/shuttle）、Petri 网并发验证。D9 结束前全文可编译并交用户通读。

---

## 止损点

- **D1 结束**：`gen-llmcode-smoke-v1` conform PASS 率 < 40% 且原因在 instrument 覆盖 → 限定 LLM 只用 `std::sync` 并重跑一次；仍 < 40% 则 G3 v2 与 `G3_codegen` 并列报告，不替换。
- **D3 结束**：`flash-gen-main-v2` rep 0 未跑完 → 缩到 Simple+Medium，Complex `not_run`。
- **D4 结束**：RESULTS 未能生成 → 用 freeze-4 数据写作，v2 作为补充实验；并向用户报告是否转 ISSTA 2027。

## 全局规则

- 契约对生成模型不可见；实体名可见（属于需求）。工具不生成代码（消融臂除外）。
- RESULTS.md 与 `tables/*.tex` 只能由命令生成；论文数字全部 `\input`。
- 修复研究既有数据与判定不动。
- 每次 LLM 请求写 `requests.jsonl`；每次工具调用落 `calls/<seq>-<tool>/`。
- 人工复核与 `owner_verdict` 不得由代理填写。
- 每节独立提交；停点写 HANDOFF `Not done` 与 `notes/GAP_AUDIT.md`。
