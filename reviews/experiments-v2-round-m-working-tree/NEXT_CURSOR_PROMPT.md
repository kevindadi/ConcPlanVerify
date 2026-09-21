你在三个仓库中工作：`/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）、`/Users/kevin/local-repos/ConcIR`（Rust 工具）、`/Users/kevin/paper-review/papers/ConcPlanVerify/`（论文 LaTeX，只放 `.tex/.bib/.sty/figures/tables`，遵守该工作区 `AGENTS.md`）。先读 `ConcPlanVerify/reviews/experiments-v2-round-m-working-tree/REVIEW.md`（M-1..M-5）、`experiments/EXPERIMENTS_V2_HANDOFF.md` Round m、`docs/PAPER_EVIDENCE_MAP.md`、`experiments/RESULTS.md`，以及论文 `paper.tex` 全文。

本轮目标：**实验收口，转入写作。** Part A 在 ConcPlanVerify/ConcIR 补齐探针、修后编辑分母、整理人工分歧证据、可选补 `concir-instrument` v2，然后 `freeze-3`。Part B 在论文仓库做"论文—证据"对齐审计，并重写评估节为可编译的初稿（表格全部 `\input` 自生成的 `.tex`）。各节独立提交；**§A1–§A4、§B1–§B2 必须完成**，其余做不完写停点。不改 benchmark、契约、主批次 LLM 数据、主表判定；论文方法节本轮只做差距清单不重写正文。

预算：OpenCode Go ≤ 120 请求（§A1），DeepSeek ≤ 60（§A2）。

---

# Part A — 实验收口（ConcPlanVerify / ConcIR）

## §A1 模型探针补齐（M-1）——OpenCode Go ≤120

- 任务集改为 10 任务（`MAIN_V1_TASKS`，含 `abba_2lock`、`bare_wait_no_predicate`）；`kimi-k2.7-code` 与 `glm-5.3-flash` 各补齐到 10 × 3 臂全部有结果。已有结果的格**不重跑**（按 task/arm 复用，记 `reused_from`）。
- 任何未能完成的格记 `decision=not_run` + `reason`（额度/429/超时/崩溃），汇总表的 `cells` 必须恒为 10；解释 5 个 kimi run 目录（哪个是有效 run，其余为何中断）写进 `PROTOCOL.md` 的 `Run history`。
- 记录 A0/A2 的 tokens（从 `usage` 取）；`kimi` 强制 temperature 1 作为表注。
- 接受的 A0/A2 Rust：behavior + Miri 16 + agent-proxy 专家标注（rubric v2，按 sha 去重）；接受的 A3 CIR：`a3-to-rust-v2`（`BIN_V2`，0 请求）。`false_accept` 按主协议规则。
- `SUMMARY.md` 移到 `experiments/model-probe-v2/` 根目录（run 子目录的保留为原始）。对照表加 DeepSeek Flash rep 0 同格；每模型一句结论：(a)(b)(c) 是否复现。
- RESULTS §Model probe、`tables/model_probe.tex`、评估地图 (j) 更新。
- 提交 `model-probe-v2: complete 10x3 grid for kimi/glm; not_run explicit; tokens`。

## §A2 后编辑分母与强制编辑重试（M-2）——DeepSeek ≤60

- 汇总：`no_edit`（0 行改动）单列并从 conform/drift 分母剔除；RESULTS §Post-edit 与 `tables/postedit.tex` 加 `no_edit` 列，文字说明只对真实编辑格陈述。
- 对 E2 的 15 个与 E3 的 11 个 no-op 格各做**一次**强制编辑重试（`prompts/post_edit_v2.md`，不提 CIR/sid/事件）：
  - E2：必须新增至少一个 `fn` 并在线程体中调用它；返回前自检"与原文件相比是否有改动"。
  - E3：必须至少改动一处与锁、条件变量、通道或信号量相关的代码（缩短持有区、合并/拆分临界区、调整通知方式等），并保持程序输出不变。
- 每格：build → conform v2 → behavior → Miri 16；记录 `changed_lines`、`sync_calls_moved`（基于事件流 diff）。仍为 no-op 的记 `no_edit_after_retry`。
- `SUMMARY.md` 新节 "Forced-edit retry"：E3 真实改动了什么，conform 抓住了哪些（kind 分布），哪些等价；`drift_caught_only_by_conform` 重算。
- 提交 `post-edit-conform-v2: no_edit denominator; forced-edit retry (E2/E3)`。

## §A3 人工—自动分歧证据页（M-3）——不发请求

`experiments/flash-repair-main-v1/expert-labels/HUMAN_DISAGREEMENT_EVIDENCE.md`，两格各一节：`acquire_twice/A0 f42afd77e7a9`、`partial_deadlock/A1 a08bd1a020fa`。每节：完整 `candidate.rs`（带行号）、`requirements.txt` 的终态要求、behavior 的 argv/stdout/stderr/超时阈值与实际耗时、Miri 16 种子逐个状态、agent-proxy 的 `evidence` 字段、auto 判 `bug_present` 的具体规则来源。末尾留 `owner_verdict:` 与 `owner_reason:` 空行**给用户填**。若整理证据时发现 auto 侧是假警报（如超时阈值过短、`thread_leak` 误计），只在页中指出，不改判、不改代码。

提交 `expert labels: human/auto disagreement evidence page (2 candidates)`。

## §A4 文案修正与 freeze-3

- RESULTS 偏差节删除 "left blank for the owner"，改为人工复核已合并（17 行 / 11 候选，2 处待裁决见 §A3）。
- HANDOFF 注明 `4971f650` 提交信息与内容不符（用户本地自动信息）。
- `python -m cir_workflow results …` 重生成；`tables/*.tex` 重生成。
- tag `experiments-v2-freeze-3`（ConcIR 若无提交则不打新 tag，manifest 写明沿用 `concir-freeze-2`）；`FREEZE_MANIFEST.md` 追加。
- HANDOFF `# Round 2026-09-2xn`：M-1..M-5 对照表、预算分列、停点。
- 提交 `freeze-3; handoff round n`。

## §A5（可选）`concir-instrument` v2（M-5）——不发请求

自由 Rust → 包装类型重写（`std::sync::{Mutex,Condvar}`、`mpsc`、`thread::spawn`/`scope` → `cir_trace::sync::*`，sid 按 (资源, 种类, 顺序) 生成，写 `labels.json`）。验收：对 `flash-repair-main-v1` 中 A0/A2 接受的 Rust 候选（按 sha 去重）跑 instrument v2 → build → 与该任务**冻结契约对应的 fixed CIR** 做 conform v2；报告 build 率、conform PASS/violation kind 分布、instrument 失败原因分布（如锁调用在辅助函数内、非 `std` 原语）。目录 `experiments/instrument-v2-free-rust/`。若做，RESULTS 加 §"Conform on free-form Rust"，评估地图加 (k)。提交（ConcIR + ConcPV 各一）。

---

# Part B — 论文对齐（`/Users/kevin/paper-review/papers/ConcPlanVerify/`）

## §B1 论文—证据差距清单

新建 `papers/ConcPlanVerify/notes/GAP_AUDIT.md`（`notes/` 只放 `.md`，不入编译）。逐节对照 `paper.tex` 与 `docs/PAPER_EVIDENCE_MAP.md` / `RESULTS.md` / ConcIR 当前实现，每条给 `paper 现状 → 证据/实现现状 → 动作(keep/rewrite/delete/add)`。至少覆盖：

- Overview §2.1 signal-loss 例子与 §2.2 pipeline/repair hierarchy：是否换成 `partial_deadlock_bystander` running example（CASE.md）；三层 repair tiers 与现在的 `A3_local → A3_whole` 升级、`normalize` Tier-1 的对应关系。
- Method §3.1 CIR：FQN、`scope`、`bound`、channel `capacity`、condvar wait-set 语义、契约语言（`deadlock_free/safety/reachability/always_reachable/unreachable/preserved`，含 `holds_all/mutex_exclusive/never_holds_all`）——paper 写了哪些、缺哪些。
- Method §3.4–3.6 bug detection / goal reachability / repair loop：与 `explore` 诊断格式、`holds_all` hint、`A3_local` prompt、停滞检测的对应。
- **缺失的方法内容**：codegen（CIR → Rust）、`cir_trace` v2 操作绑定事件、conform v2（violation kinds）、`concir-instrument`。
- Evaluation §4 全部：旧的 9 模式、旧的指标 → 新的 10 任务 × 6 臂 × 3 rep、五类 oracle（build/behavior/Miri/expert/conform）、false-accept 规则、Track D、scale、突变召回、后编辑、模型探针、抽取 limitation。
- 附录 A–F：哪些与实现已不一致（如 CVN 翻译规则里 condvar/channel 的处理是否与 `petri/exec.rs` 一致，需对照源码抽查 3 条规则）。
- Threats/limitations 需要新增的条目：单 provider 家族为主、8–10 任务、agent-proxy 标注、抽取轨失败、M5 盲区、codegen 产物 0 洞。

提交（论文仓库）`notes: paper-evidence gap audit`。

## §B2 评估节重写初稿

- 把 `ConcPlanVerify/experiments/tables/*.tex` 复制到 `papers/ConcPlanVerify/tables/`（复制而非软链；每个文件头部保留生成命令注释；`notes/TABLES_SYNC.md` 记录来源 commit/tag `experiments-v2-freeze-3`）。
- 重写 `\section{Evaluation}`（替换 §4.1–§4.4）为以下结构，全部数字来自 `\input{tables/…}` 或 RESULTS 中可追溯的值（在 `.tex` 注释里标 RESULTS 的节名）：
  1. Setup：benchmark（10 任务、去泄漏、冻结契约、参考程序审计）、臂（A0/A1/A2-ml/A3_local/A3_whole/A3_tiered）、oracle 与 `false_accept`/`design_loss` 定义、模型与预算、binary sha。
  2. RQ1 修复正确性：主表 + 按臂汇总；`cycle_3lock` 工具全绿的段落；专家轨与人工复核。
  3. RQ2 成本：tokens/correct、轮数、local vs whole、tiered 与轮数需求（`partial_deadlock`、`bare_wait` 两个附录案例各一句）。
  4. RQ3 契约与设计保持：running example（CASE.md 的三段：为何难、契约挡了什么、whole 的修复）、`design_loss`。
  5. RQ4 落地与后验证：a3-to-rust-v2、conform 召回 v1→v2 表、后编辑（只对真实编辑陈述，含 §A2 强制重试结果若已出）、盲区。
  6. RQ5 检测能力与规模：Track D、scale。
  7. RQ6 泛化：10 任务扩展的结论修正、模型探针（若 §A1 完成）。
  8. Threats to validity（按 §B1 清单）。
- 每个 RQ 先写一句"主张"，再表，再两到三句解读；不写方法节内容。图暂用占位 `\fbox{}` 并在 `notes/FIGURES_TODO.md` 列出（主流程图、running example 时序、召回 v1/v2 柱状图），图用 drawio MCP 在下一轮做。
- `latexmk -pdf paper.tex` 必须通过（编译产物不提交）。
- 提交（论文仓库）`evaluation: rewrite against experiments-v2-freeze-3 tables`。

## §B3（可选）方法节改写要点

不改正文，只在 `notes/METHOD_REWRITE_PLAN.md` 给每个小节的改写要点与需要新增的定义（契约谓词形式化、局部再生成的语义、操作绑定事件与 conform 关系的定义草稿），供下一轮写作。

---

## 全局规则

- 论文目录只放 LaTeX 相关文件与 `notes/*.md`；编译产物不提交；表格来自复制的 `tables/*.tex`，不得手抄数字。
- RESULTS.md 与 `tables/*.tex` 只能由命令生成。
- 主批次数据、`BIN_MAIN` 判定、契约、buggy 输入不动；conform/a3-to-rust 用 `BIN_V2`。
- 人工复核内容与 `owner_verdict` 不得由代理填写。
- 每节独立提交；做不完的节在 HANDOFF `Not done` 与 `notes/GAP_AUDIT.md` 末尾写停点。
