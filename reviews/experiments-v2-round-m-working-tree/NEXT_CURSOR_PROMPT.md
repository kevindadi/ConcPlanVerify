你在三个仓库中工作：`/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）、`/Users/kevin/local-repos/ConcIR`（Rust 工具）、`/Users/kevin/paper-review/papers/ConcPlanVerify/`（论文 LaTeX，只放 `.tex/.bib/.sty/figures/tables/notes`，遵守该工作区 `AGENTS.md`）。先读 `ConcPlanVerify/reviews/experiments-v2-round-m-working-tree/REVIEW.md`、`experiments/EXPERIMENTS_V2_HANDOFF.md` Round m、`docs/PAPER_EVIDENCE_MAP.md`、`experiments/RESULTS.md`、`benchmarks/FAMILIES.md`、`benchmarks/build_families.py`、`prompts/concir_generation_v2.md`、`prompts/rust_generation_v1.md`，以及论文 `paper.tex`。

## 本轮的重新定位（必须先理解）

目标会议 FSE 2027，截稿 **2026-10-02 AoE**。论文的核心主张是"**从用户需求出发，基于 ConcIR 的模型先行生成，比纯 LLM 直接生成更能满足需求且不引入并发缺陷**"。此前 m 轮的实验全部是**修复**设定（输入带缺陷程序），那是因为缺陷可以由构造保证、oracle 清楚。现在冻结契约（`reachability/always_reachable/safety/preserved/holds_all…`）已经足以充当**需求满足**的 oracle，所以主 benchmark 改为**需求 → 程序的生成任务**，修复研究降为第二研究（"难以让 LLM 自然生成带缺陷程序，故手工构造缺陷 benchmark 评估检测与修复能力"）。

对标 FSE 2026 的 Event-B Agent（27 个系统、按需求条数分三档、指标 RC/RF、基线为 LLM 自动形式化路线）：我们要有**需求文档分条 → 契约属性逐条对应 → 覆盖率/满足率**这一套，并且比它多出"落到可运行 Rust + 操作绑定后验证"。

时间只有 11 天，任务按天排优先级；**D1–D8 出数据，D9–D11 写作**。每节独立提交；做不完的写停点。预算：DeepSeek Flash ≤ 1400 请求，OpenCode Go ≤ 150 请求。

---

# Part A — 生成 benchmark 与主实验（D1–D6）

## §0 Benchmark v3：需求文档与契约对应（D1–D2）

对每个已有冻结契约的任务（目标 24–28 个，覆盖 lock-order / condvar / channel / semaphore / structure / atomic-data 六族），新建 `benchmarks/<family>/<case>/generation_input/`：

- `REQUIREMENTS.md`：**分条编号**的需求文档（`R1..Rn`，n 在 4–16 之间），三类条目：功能（线程/角色做什么、数据如何流动）、并发（互斥/顺序/握手/背压/容量/通知语义，用自然语言描述**意图**而非实现）、终止与可观察输出（`DONE …` 终态行）。不得出现 ConcIR/CIR/sid/契约术语；不得给出代码。现有一两句的 `requirements.txt` 作为起点扩写。
- `contract.json` 在冻结契约基础上为**每条属性加 `req: ["R3","R5"]`**；每条 `Ri` 至少被一条属性覆盖，否则该条标 `unverifiable`（如纯功能语句）并在 `REQUIREMENTS.md` 里标 `[U]`。`build_families.py` 增加校验：所有 `req` 引用存在、每条非 `[U]` 需求至少一条属性。
- **复杂度分档**：按 `(需求条数, CIR 参考的线程数+资源数, 参考模型状态数)` 三个量定义 Simple/Medium/Complex（阈值写死在 `benchmarks/TIERS.md`，每档任务数尽量接近）。参考 CIR 用现有 `fixed.cir.json`；没有的任务先补一个通过契约的参考 CIR（可用 A3 流程生成后人工确认，`provenance` 记录）。
- 契约与需求打 tag `contracts-v3`，`benchmarks/GENERATION_MANIFEST.json` 列出任务、档位、条数、属性数、`[U]` 条数。

验收：`python benchmarks/build_families.py --check-generation` 全绿；`GENERATION_MANIFEST.json` 提交；提交 `benchmarks v3: requirement documents, req-tagged contracts, complexity tiers`。

## §1 Rust 臂的需求 oracle：instrument v2 + 轨迹监控（D1–D3，与 §0 并行）

纯 LLM 生成的 Rust 没有模型，需求满足只能在运行轨迹上做**有界**检查。实现：

- ConcIR `concir-instrument` v2：自由 Rust → `cir_trace::sync` 包装类型（`Mutex/Condvar/Semaphore/mpsc/spawn/scope`），sid 按 (资源, 种类, 顺序) 生成，资源名按变量名生成并输出 `resources.json`；失败原因分类（非 std 原语、锁在辅助函数/泛型中、宏展开等）。
- ConcIR 新子命令 `concir-backend monitor --contract contract.json --resources resources.json --traces <dir>`：对一组观测轨迹检查属性——`safety/never_holds_all/unreachable` 在**每条**轨迹上必须成立（违反即 FAIL）；`reachability/always_reachable/holds_all` 至少在一条轨迹上观察到（否则 `not_observed`）；`deadlock_free` 由 behavior（hang/timeout）与 Miri 决定。资源名对齐：`resources.json` 与契约资源名的映射先自动（同名/包含），失败时记 `resource_unmapped` 并让 agent-proxy 手工给映射写进 `mapping.json`（记录 provenance）。输出每条属性 `PASS_bounded / FAIL / not_observed / unmapped`。
- 轨迹来源：native 运行 N=32 + Miri 16 种子，每次运行写一条 `cir_trace` 轨迹。
- 明确写进 `PROTOCOL.md`：Rust 臂的 oracle 是**有界**的（`PASS_bounded`），A3 的模型判定是**穷尽**的；论文里两者不混用一个词。

验收：对 10 个任务的 `fixed.rs` 参考跑 instrument v2 + monitor，所有非 `[U]` 属性应为 `PASS_bounded`（不通过的逐条解释：是 instrument 限制、映射问题还是参考程序问题）；对 `buggy.rs` 参考至少一条属性 FAIL 或 behavior hang。提交（ConcIR）`instrument v2 + monitor`，（ConcPV）`rust oracle: bounded trace monitor harness`。

## §2 生成臂与主批次 `flash-gen-main-v1`（D3–D5）

四臂，K=4，3 rep，DeepSeek Flash，temperature 0：

| 臂 | 输入 | 流程 |
| --- | --- | --- |
| `G0_direct` | REQUIREMENTS.md | 一次生成 Rust |
| `G1_self_iter` | 同上 | 生成 → 自审（`rust_self_review_v1`）迭代，`claims_no_issue` 语义沿用 |
| `G2_tools_iter` | 同上 | 生成 → build + Miri 16 + Lockbud 反馈迭代，工具全绿接受 |
| `G3_concir` | 同上 | 生成 CIR（`concir_generation_v2` + 契约**不给**模型，只给需求）→ normalize/check → explore vs `contract.json` → 失败则 `A3_local` 局部修订（触发 T3 升级到 whole）→ 接受后 codegen v2 → build → conform v2 → behavior |

- **契约对模型不可见**（四臂一致，只见需求文档）；G3 的验证器看契约。
- 每格记录：accepted、轮数、tokens、llm_ms、tool_ms、verify_ms；Rust 产物（G0/G1/G2 的接受候选与 G3 的 codegen 输出）统一过 §1 的 oracle：build → behavior（`DONE` 终态）→ Miri 16 → instrument v2 + monitor → agent-proxy 专家标注（rubric v3：`bug_present`、逐条 `Ri` 满足 yes/no/unsure 与 evidence）。
- G3 额外记录：模型判定（穷尽）、conform、`check_invalid/explore_fail/stalled/escalated` 分布。
- 预算：≤ 28 任务 × 4 臂 × 3 rep，请求上限 1100；`rep=0` 全档优先，然后 `rep=1,2`；预算耗尽记 `not_run`。
- 指标（写进 `results` 生成器）：
  - **RC**（覆盖率）= 可判定的非 `[U]` 需求条数 / 总条数（Rust 臂受 `unmapped`/instrument 失败影响，如实反映）；
  - **RF**（满足率）= 判定为 PASS（G3 穷尽 / Rust 臂 `PASS_bounded`）的条数 / 总条数；
  - **defect rate** = accepted ∧ (behavior hang ∨ Miri detected ∨ monitor FAIL ∨ expert bug_present=yes)；
  - **accepted-with-proof** = G3 模型 PASS ∧ conform PASS；
  - 成本：tokens/RF-satisfied-clause、轮数。
  - 全部按 Simple/Medium/Complex 分档 + Overall，k/n 与均值±极差。

验收：`experiments/flash-gen-main-v1/{PROTOCOL.md,run-*/,SUMMARY.md}`；RESULTS 新增 §"Generation (main)"；`tables/gen_main.tex`、`gen_arms.tex`、`gen_tiers.tex`。提交 `flash-gen-main-v1: requirement-to-program, 4 arms x 3 reps, tiered`。

## §3 前沿模型（D6）——OpenCode Go ≤150

`kimi-k3`（chat/completions；不可用则 `kimi-k2.7-code`）：`G0_direct`、`G2_tools_iter`、`G3_concir` 三臂 × 全部任务 × 1 rep，K=4。同 oracle。回答"强基座下 G3 是否仍有 RF/defect 增量"。全格必有结果或 `not_run`+原因；tokens 记录；温度限制作表注。产出 `experiments/gen-model-probe-v1/`，RESULTS §"Generation with a frontier model"，`tables/gen_probe.tex`。提交。

## §4 修复研究收口（D6–D7，第二研究）

沿用 m 轮结果（`flash-repair-main-v1` 等）不重跑；只做：
- 后编辑 `no_edit` 分母与 E2/E3 强制编辑重试（原 §A2，DeepSeek ≤60）。
- 两格人工—自动分歧证据页（原 §A3，留 `owner_verdict` 给用户）。
- 探针 kimi/glm 补齐到 10×3 或显式 `not_run`（原 §A1，与 §3 共用额度，优先 §3）。
- 文案修正（原 §A4）。
提交按原编号。

## §5（可选，D7）loom 进 Track D

对 10 个有 Rust 参考的任务，把 buggy/fixed 参考改写为 `loom::sync` 版本，跑 `loom::model` 检测；结果加入 Track D 表（`loom` 列）。不做 loom 修复臂。提交 `track D: loom column`。

## §6 结果冻结（D8）

- `python -m cir_workflow results …` 加入生成主批次、探针、修复研究；`tables/*.tex` 全部重生成；`FREEZE_MANIFEST.md` 追加；tag `experiments-v2-freeze-4` / `concir-freeze-3`。
- `docs/PAPER_EVIDENCE_MAP.md` 重排：(1) 生成 RC/RF/defect 分档；(2) 强基座增量；(3) 落地与后验证（a3-to-rust、conform 召回、后编辑）；(4) 修复研究（原 (a)(b)(c)(e)）；(5) 检测能力 Track D（含 loom 若有）；(6) 规模；(7) 专家轨与人工；(8) 抽取 limitation；(9) threats。
- HANDOFF `# Round 2026-09-2xn`。
提交 `freeze-4; evidence map v3; handoff round n`。

---

# Part B — 论文（D9–D11，`papers/ConcPlanVerify/`）

## §B1 差距清单（D9 上午）

`notes/GAP_AUDIT.md`：按 m 轮 prompt §B1 的范围，加上"生成为主、修复为辅"的叙事重排：Introduction 的问题陈述改为需求→程序；Overview 例子换为一个 Medium 档生成任务（从需求到 CIR 到 Rust 的三段）；方法节需新增 codegen、`cir_trace` v2、conform v2、monitor（有界 oracle）的定义；评估节全部重写。

## §B2 评估节重写（D9–D10）

复制 `tables/*.tex` 到 `papers/ConcPlanVerify/tables/`（`notes/TABLES_SYNC.md` 记 tag）。结构：
1. Setup：benchmark v3（任务数、三档、需求条数分布、契约属性数、`[U]` 比例）、四臂、oracle 两级（穷尽 vs 有界）、指标定义、模型与预算、binary sha。
2. RQ1 需求满足与缺陷（生成主批次）：分档 RC/RF/defect 表；G3 的 accepted-with-proof；一段解读"纯 LLM 的 RF 在 Complex 档如何下降"。
3. RQ2 强基座增量（§3）。
4. RQ3 成本（tokens/轮数；G3 的验证开销）。
5. RQ4 落地与后验证（a3-to-rust、conform 召回 v1→v2、后编辑真实编辑格）。
6. RQ5 缺陷检测与修复（第二研究：主修复表、`cycle_3lock` 工具全绿、契约挡空洞修复案例、local/whole/tiered、专家与人工）。
7. RQ6 检测能力与规模（Track D 含 loom 若有、scale）。
8. Threats：作者构造的需求与契约、agent-proxy 标注、单主模型、Rust 臂 oracle 有界、instrument 覆盖、抽取轨失败、M5 盲区。
数字全部 `\input`；`latexmk -pdf` 通过；图占位并列 `notes/FIGURES_TODO.md`。提交 `evaluation: generation-first rewrite`。

## §B3 方法节与引言（D10–D11）

按 `GAP_AUDIT.md` 重写 Introduction 贡献列表、Overview 例子、方法节新增小节（Code generation and operation-bound conformance；Bounded requirement monitoring for unmodeled programs）、Related Work 增补（Event-B Agent 等 LLM+形式化循环、Rust 并发工具 Miri/Lockbud/loom/shuttle、Petri 网并发验证）。提交 `paper: intro/overview/method aligned with experiments-v2-freeze-4`。

---

## 止损点

- **D5 结束**时 `flash-gen-main-v1` 的 `rep=0` 若未跑完全部任务，缩到 Simple+Medium 两档继续，Complex 记 `not_run`。
- **D6 结束**时 §1 的 monitor 若对 Rust 臂 `unmapped` 超过 30% 属性，RC 表如实给出并在 threats 中说明，不再投入修 instrument。
- **D8 结束**若主批次不完整或 RESULTS 未能生成，停止并向用户报告，改按 ISSTA 2027（2027-01-11）节奏。

## 全局规则

- 契约对生成模型不可见；需求文档不得含 ConcIR 术语与代码。
- RESULTS.md 与 `tables/*.tex` 只能由命令生成；论文数字全部 `\input`。
- 修复研究的既有数据与判定不动。
- 每次 LLM 请求写 `requests.jsonl`；每次工具调用落 `calls/<seq>-<tool>/`。
- 人工复核与 `owner_verdict` 不得由代理填写。
- 每节独立提交；停点写进 HANDOFF `Not done` 与 `notes/GAP_AUDIT.md`。
