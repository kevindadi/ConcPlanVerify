你在 `/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）和 `/Users/kevin/local-repos/ConcIR`（Rust 工具）两个仓库中工作。先读 `reviews/experiments-v2-main-v1-working-tree/REVIEW.md`（J-1..J-7），再读 `experiments/EXPERIMENTS_V2_HANDOFF.md` 的 Round j、`experiments/RESULTS.md`、`experiments/case-partial-deadlock-v1/CASE.md`。

本轮目标：**让"我们的方法"在主表里落到 Rust，并把 oracle 与报告层做实**。主表方向已稳定；缺的是 (1) A3 两臂的 Rust 侧 oracle（codegen → 填洞 → build → conform → behavior → miri → expert），(2) 逐候选的专家标注与更少的 `unsure`，(3) 两个参考程序的可疑判定，(4) 报告层 bug，(5) 数据支持的 `A3_tiered` 臂，(6) 抽取轨的终止判据。各节独立提交；**§1–§4 必须完成**，§5–§7 做不完写停点。不做多模型、不写论文正文、不改 benchmark 的 buggy 输入与冻结契约（参考 fixed 程序若确有缺陷可改，见 §2）、不放宽 codegen 模式的 conform 规则。

预算：本轮 LLM 请求总上限 **260**（§3 ≤110，§5 ≤90，§6 ≤40，其余 ≤20）。全部请求写 `requests.jsonl`。

---

## §1 报告层修复（J-1、J-4）——不发请求

### 1.1 抽取计数与原因

- `extraction-v5`：以 `CELLS.json` 为唯一真源；`SUMMARY.md` 与 RESULTS 均由它生成。`validated` 的定义写进 `PROTOCOL.md`：`stage=explore ∧ 契约求值成功 ∧ verdict∈{PASS,FAIL}`；`INVALID` 不算 validated，单列 `contract_invalid`。
- `stage=codegen/labels/conform` 的每格必须有非空 `reason`（取 ConcIR stderr 首个 `E\d{3}`/panic 首行，或 harness 自己的分类）。补跑离线部分把 29 个空 `reason` 填上（不发请求：从已存的 `calls/` 目录重新解析）。
- RESULTS 抽取表按 (task, arm, rep) 展开，另给按 stage×reason 的计数表。

### 1.2 主表聚合列

- `build/behavior/miri/expert/extract/conform` 在多 rep 下显示计数：`hang 3/3`、`detected 1/3 · clean 2/3`、`yes 2/3 · unsure 1/3`。
- 恢复 `llm_ms`、`tool_ms`、`verify_ms`（ConcIR explore/conform 时间）列；按臂汇总加 `tokens_per_correct_accept = Σtokens / (accepted − false_accept)`，分母为 0 记 `∞`。
- 修 `A3 decision distribution` 空表。
- RESULTS 说明里写明：A0 的 `accepted` 只表示产出了可解析程序。

### 1.3 Track D 表

- 只保留有 Rust 参考的 benchmark 任务；其余任务放到附表 `Track D (CIR-only)`，不出现 `None` 行。
- `miri` 列改为 status 计数（`clean 6 · timeout 0 …`），种子数与主批次一致（16），需要重跑的离线重跑。
- `lockbud.available` 必须是 bool + commit。

验收：`python -m cir_workflow results …` 重新生成；`extraction-v5` 三处数字一致；提交 `results: per-rep oracle counts, cost columns, extraction truth source, track D tables`。

---

## §2 参考程序审计（J-3）——不发请求

对 `cycle_3lock/fixed.rs`（Lockbud `DoubleLock`）与 `partial_deadlock_bystander/fixed.rs`（Miri `detected`）：

1. 手工读代码；跑 `cargo miri` 16 种子并保留 `calls/`；跑 Lockbud 保留原始输出；把 Miri 输出按 `deadlock / timeout / thread_leak / clean` 分类。
2. 结论只能是三者之一并写进 `benchmarks/REFERENCE_AUDIT.md`：(a) 工具假阳（给出理由与原始输出行号）；(b) 参考程序确有缺陷 → 修正 `fixed.rs`，记 `provenance` 与 diff，标记所有依赖它的实验结果（A2 的工具反馈用了它吗？Track D 用了它）为"需重算"，并在 RESULTS 偏差里列出；(c) 分类错误（如 `thread_leak` 被计入 detected）→ 修 `rust_arm` 分类并回归。
3. 顺带对全部 8 个任务的 `fixed.rs` 跑一遍 Miri 16 种子 + Lockbud，结果表进 `REFERENCE_AUDIT.md`。

验收：`REFERENCE_AUDIT.md` 提交；若走 (b)/(c)，受影响的表重算并在 HANDOFF 写清。提交 `benchmarks: reference audit (cycle_3lock, partial_deadlock_bystander)`。

---

## §3 A3 落到 Rust：`a3-to-rust-v1`（J-7）——主任务，≤110 请求

对 `flash-repair-main-v1` 中全部 `accepted=True` 的 A3_local / A3_whole 格（41 格；按接受 CIR 的 sha 去重后再跑，重复 sha 复用结果并在表中标 `dedup_of`）：

1. `concir-backend codegen`（`BIN_MAIN`）生成骨架 + `labels.json`；
2. LLM 填洞（现有 conformance harness 的 prompt；填洞失败允许 1 次重试，计预算）；
3. `cargo build` → `conform`（**strict codegen 模式**，不用 `--lenient-unlock/--attempt-events`）→ behavior（`DONE …` 终态检查）→ Miri 16 种子；
4. 结果写回主批次该格的 `oracle.*`（新增 `oracle.conform = PASS/FAIL/no_build/…`，含 trace 条数），并进 §4 的专家标注队列。

目录 `experiments/a3-to-rust-v1/`：`PROTOCOL.md`、`run-*/`、`SUMMARY.md`（每格：CIR sha、填洞轮数、build、conform、behavior、miri、tokens、ms）。

验收：
- 全部去重后的 CIR 都有结果（不允许 `not_run`）；`conform` 失败的格逐条给 `reason`（`ev` 缺失/顺序/未知 sid/…），并区分"填洞代码错"与"骨架/工具问题"（后者是 ConcIR bug，修并记录）。
- RESULTS 主表 A3 行的 `build/behavior/miri/expert` 不再是 `None`，并多出 `conform` 列；按臂汇总加 `conform_pass_rate`。
- 提交 `a3-to-rust-v1: codegen+holes+conform for all accepted A3 CIRs`。

---

## §4 逐候选专家标注 v2（J-2）——不发 LLM 请求（代理标注由 Cursor 代理完成）

- `docs/EXPERT_LABEL_RUBRIC.md` 升级 v2：`bug_present` 必须给出**判定过程**字段 `evidence`：列出每个线程的获取序列（锁/信号量/通道/条件变量），指出是否存在环或丢失唤醒；`unsure` 仅允许附 `unsure_reason∈{needs_execution, unfamiliar_api, ambiguous_spec}`，目标 `unsure ≤ 15%`。新增 `design_preserved` 的判定依据（对照 `requirements.txt` 与冻结契约里的 `preserved`）。
- 标注对象：`flash-repair-main-v1` 全部接受格 **+ §3 的 A3 Rust 产物**，按候选 sha 去重（`EXPERT_LABELS.json` 每条一个 sha，附 `cells: [(task,arm,rep)…]`）。
- 汇总新增 `design_loss = accepted ∧ design_preserved=no`，按臂计数进 RESULTS。
- 重建 `HUMAN_REVIEW_QUEUE.md`：全部分歧格 + 全部 `unsure` + 随机 4 格（种子记录），**留空**给用户。上一版队列若用户已填写，合并保留。

验收：`unsure` 比例写进 HANDOFF；RESULTS 专家表按 sha 展开、附 `cells`；提交 `expert labels v2: per-candidate, evidence field, design_loss`。

---

## §5 `A3_tiered` 臂（J-5）——≤90 请求

- 定义：K=4 总预算内，先 `A3_local`；触发升级条件 = `stalled`（连续两轮同一候选）或 `explore_fail` 连续 2 轮；升级后切 `A3_whole` prompt，携带 local 阶段的全部诊断与最后候选；升级最多一次。`PROTOCOL.md` 写死规则。
- 跑 8 任务 × 3 rep（`experiments/flash-repair-main-v1/` 追加臂，`rep-N/<task>/A3_tiered/`，与主批次同协议、同 `BIN_MAIN`）。
- 记录 `escalated: bool`、升级前后轮数与 tokens。接受的 CIR 追加进 §3 流程（预算内）。

验收：主表新增 `A3_tiered` 行；按臂汇总给 `escalation_rate`；HANDOFF 写明它是否在接受率、false-accept、tokens 三项上都不劣于 `A3_local`/`A3_whole`。提交 `A3_tiered: local->whole escalation arm, 8x3`。

---

## §6 抽取 v6 与终止判据（J-6）——≤40 请求

- ConcIR：修 HANDOFF 提到的 backend panics（codegen/conform 对畸形 CIR 必须返回 `E\d{3}` 而非 panic），加回归用例。
- `labels` 阶段失败 10 格：分析原因（是否 `labels.json` 中有 `spawn`/`scope` 以外模型确实无法对应的标签），修 prompt/normalizer。
- 只对 v5 中 `stage∈{labels, codegen}` 且原因属于 harness/normalizer 可修的格重跑（≤40 请求）。
- **终止判据**：修完后 `validated`（按 §1.1 定义）若 `< 5/63`，写 `experiments/extraction-v6/EXTRACTION_LIMITS.md`（失败模式分布、为何 LLM 从代码抽 CIR 不可靠、对 model-first 主张的含义），抽取轨冻结，不再投预算；若 `≥ 5`，把 validated 格的 verdict 与 behavior/expert 的一致性写进 RESULTS。

提交 `extract v6: backend panics -> errors; label-stage fixes; termination criterion`。

---

## §7 HANDOFF Round k、RESULTS 再生成、论据地图

- `# Round 2026-09-2xk`：J-1..J-7 对照表、各节 commit、预算使用（§3/§5/§6 分列）、`BIN_MAIN` 是否变化（ConcIR 若有提交则新 sha 并重跑 `REBASE_*`）、停点、`Not done`。
- 最后一次 `python -m cir_workflow results …` 并提交 RESULTS。
- 新建 `docs/PAPER_EVIDENCE_MAP.md`：每条论文主张 → 支撑表/数字 → 生成命令 → 原始 artefact 路径。至少覆盖：(a) 0 false-accept vs 工具绿灯接受带环程序；(b) 局部再生成的 token/轮数/接受率；(c) 设计保持契约挡住空洞修复（案例）；(d) A3 产物的 conform 通过率（§3）；(e) `A3_tiered`（§5）；(f) 检测能力 Track D；(g) 规模表；(h) 专家轨与人工复核；(i) 抽取轨的结果或 limitation。每项标 `ready / partial / missing`。

提交 `handoff: round k; RESULTS regenerated; paper evidence map`。

---

## 全局规则

- 同一 `BIN_MAIN` 贯穿全轮；ConcIR 有提交则重新编译、记新 sha、重跑 `REBASE_<sha8>.md`，所有离线判定用新 binary。
- RESULTS.md 只能由命令生成；任何表与其 JSON 真源不一致视为 bug。
- 每次 LLM 请求写 `requests.jsonl`（rep、arm、task、round、tokens、latency、`reply.kind`）；每次工具调用落 `calls/<seq>-<tool>/`。
- 不改 buggy 输入与冻结契约；契约问题记 `CONTRACT_ISSUES.md`。
- `HUMAN_REVIEW_QUEUE.md` 不得由代理填写。
- 每节独立提交，提交信息写节号；做不完的节在 HANDOFF `Not done` 写停点与原因。
