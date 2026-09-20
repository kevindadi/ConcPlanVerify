你在 `/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）和 `/Users/kevin/local-repos/ConcIR`（Rust 工具）两个仓库中工作。先读 `reviews/experiments-v2-main-batch-working-tree/REVIEW.md`（I-1..I-9），再读 `experiments/EXPERIMENTS_V2_HANDOFF.md` 最后一轮和 `experiments/RESULTS.md`。

本轮目标：**让主表站得住**。上轮 `flash-repair-smoke-v3` 已经给出方向正确的第一张表，但 A0/A1 的"模型宣称无缺陷"被记成 `no_build`、`false_accept` 没并入专家标注、A1 接受了未构建的候选、RESULTS 是手拼的且与 SUMMARY 不一致。修完这些后以 3 次重复、统一 binary 重跑主批次，抽取与专家标注覆盖全部接受格，`partial_deadlock_bystander` 做单独案例，RESULTS 全部由命令生成。各节独立提交；§1–§3 必须完成，§4–§7 做不完写清停点。不做多模型（§8 可选）、不写论文正文、不调 Lockbud、不放宽 codegen 模式的 conform 规则、不改 benchmark 的 buggy 输入与冻结契约。

---

## §1 A0/A1 回复语义修复（I-1、I-3）——先于一切

### 1.1 三分法解析

`arms.py`/相关模块中 A0、A1 的回复解析改为三类，并把原文与分类写进 `round-N/reply.json`：

- `program`：含且仅含一个完整 Rust 程序（fence 内或整段）。
- `claims_no_issue`：sentinel `NO_ISSUES`，或**无代码块**且命中判定词表的散文（词表写入 `PROTOCOL.md`，初版：`no issue|no issues|no defect|no bug|is correct|already correct|does not (have|contain) (a )?(bug|deadlock)|same order`；大小写不敏感；命中即归此类并保留原文供复核）。
- `other`：既非程序也非宣称。**一次**格式重试（计入预算），仍失败记 `format_error`。

### 1.2 宣称无缺陷的裁决

- A0（单轮）：`claims_no_issue` ⇒ `decision=claims_no_issue`，`accepted=True`，`accepted_artifact=input`（buggy 输入本身），`bug_present=True`（`reason=by_construction`），`false_accept=True`。SUMMARY 的 `oracle.*` 列对该格填 `input`。
- A1（自审迭代）：`claims_no_issue` ⇒ 接受**最近一次 `build_ok=True` 的候选**；若没有 ⇒ `decision=claims_no_issue_unbuilt`，`accepted=False`。任何 `accepted=True` 的 A1 格必须有 `build_ok=True`（在汇总里加断言）。
- A2 不变（工具绿灯才接受），但同样记录 `reply.kind`。

### 1.3 离线回归

对 `flash-repair-smoke-v3/run-…958361` 的全部 A0/A1 回复离线重分类（不发请求），产出 `experiments/flash-repair-smoke-v3/REPLY_RECLASS.md`：每格 `old_decision → new_kind/new_decision`。预期：`notify_one/A0` 变 `claims_no_issue/false_accept`；`cross_module/A1` r2 变 `claims_no_issue`（接受 r1 候选，r1 `build_ok` 为何要写明）；`send_while/A1` r3 变 `claims_no_issue_unbuilt`。

### 1.4 验收

- 三分法单元测试（sentinel / 散文宣称 / 程序 / 空回复 / 程序+散文混合）。
- `REPLY_RECLASS.md` 提交。
- 提交：`arms: three-way reply classification; claims_no_issue semantics for A0/A1`。

---

## §2 汇总规则与 RESULTS 生成器（I-2、I-4、I-5）

### 2.1 `false_accept` 规则落地

`summarize` 中：`bug_present = behavior∈{hang} ∨ (model=FAIL ∧ extract_validated) ∨ expert.bug_present=yes ∨ reason=by_construction`；`false_accept = accepted ∧ bug_present`。SUMMARY 主表加两列 `oracle.expert`（`yes/no/unsure/—`）和 `oracle.extract`（`PASS/FAIL/—`，仅 validated 才填）。按臂汇总加 `false_accept_by_source`（behavior / extract / expert / by_construction 计数，可重叠）。专家与自动 oracle 分歧格单列一张 `## Oracle disagreements` 表。

### 2.2 `python -m cir_workflow results`

实现生成器，输入一个或多个批次目录 + `expert-labels/CANDIDATES.json` + 抽取结果目录 + Track D/scale 目录，输出 `experiments/RESULTS.md`，**文件头写生成命令、各输入目录、各 sha**。禁止手改 RESULTS.md（在 `PROTOCOL.md` 和 RESULTS 头部写明）。RESULTS 至少包含：

1. 主表（多次重复时：每格 `accepted k/n`，`round`/`tokens` 为均值±极差，`false_accept k/n`）。
2. 按臂汇总（同上格式）+ `false_accept_by_source`。
3. A3 决策分布（`explore_fail/check_invalid/check_schema_error/stalled…`）。
4. 专家标注表 + 一致率 + 分歧表。
5. 抽取表（stage 分布、validated、validated 格的模型 verdict）。
6. Track D 表（每任务：ConcIR petri / interp / Miri / Lockbud 判定；buggy 与 fixed 各一行）。
7. Scale 表。
8. 偏差清单（D-xx）与 threats。

### 2.3 HANDOFF Round i 补写

按既有格式补 `# Round 2026-09-19i`：对照表、commit、6 个新 Rust 参考程序的去泄漏 lint 结果、停点。之后本轮再加 Round j。

### 2.4 验收

- `python -m cir_workflow results …` 对现有 v3 批次跑通，输出与 SUMMARY 数字一致（A0 `false_accept` 应 ≥3/7 含 `cycle_3lock` 专家格与 `notify_one` by_construction）。
- 提交：`summary: expert/extract columns, false_accept_by_source; results generator; HANDOFF round i`。

---

## §3 主批次重跑：`flash-repair-main-v1`（3 次重复，统一 binary）

### 3.1 前置

- ConcIR：在 §5 的 `holds_all` hint（如已做）之后 `cargo build --release`，记 sha 为 `BIN_MAIN`。**本节及以后所有离线重算都用 `BIN_MAIN`**。
- 用 `BIN_MAIN` 重算：`CONTRACT_STRENGTH.md`、`conformance-v4` 离线部分、v3 批次全部 accepted CIR 的 explore（结果写 `experiments/REBASE_<BIN_MAIN前8位>.md`，列出任何判定变化；预期无变化）。

### 3.2 协议

`experiments/flash-repair-main-v1/PROTOCOL.md`：8 任务 × 5 臂（`A0_direct / A1_self_iter / A2_tools_iter_ml / A3_local / A3_whole`）× **3 次重复**（`rep=0,1,2`，rep 写入每个请求的 `metadata` 并作为目录层级 `rep-N/`）；K=4；deepseek-flash、temperature 0、thinking off、max_tokens 4096、timeout 90 s；Miri 16 种子；预算上限 **300 请求**（上轮单次 78，三次约 240）；预算耗尽记 `stop_reason=budget`，未跑的格记 `not_run` 而非缺失。任务顺序：按 rep 外层、任务内层，保证 rep=0 先完整。

### 3.3 运行与汇总

- 用 §1 的新语义。
- SUMMARY 按 §2 格式；每 rep 一张子表 + 合并表。
- 对所有 `accepted=True` 的 A0/A1/A2 格（预期 ~50）：进入 §4 抽取与专家标注队列。

### 3.4 验收

- `rep=0` 全部 40 格有结果；`rep=1,2` 至少 A0/A2/A3_local 三臂完整（若预算不足，优先保这三臂，PROTOCOL 里写明优先序）。
- SUMMARY + `python -m cir_workflow results` 生成的 RESULTS 提交。
- 提交：`flash-repair-main-v1: 8x5x3 main batch on BIN_MAIN`。

---

## §4 双轨 oracle 覆盖全部接受格（I-6）

### 4.1 抽取 v5

- `labels.json` 视为必填清单：模型返回的 CIR 中每个标签必须恰好出现一次作为 sid。**codegen 之前**做该检查，缺失/重复 → 回送 `missing_labels/duplicate_labels` 列表重试一次（计预算）。
- normalizer：`Arc<Mutex<T>>`/`Mutex<T>`→`mutex`，`Condvar`→`condvar`，`Semaphore`→`semaphore`，`mpsc::*`→`channel`；出现 `Vec<`、`Arc::new`、`vec!` 等 Rust 表达式作为类型/值 → 回送 `E9xx non-cir-type` 指针（不静默改写）。
- prompt `rust_to_cir_extract_v5.md`：显式列出允许的 `kind` 值与类型名；给一个 3 行的正反例（`"mutex"` ✓，`"Arc<Mutex<i32>>"` ✗）。
- 对 §3 全部接受格跑，预算 **120 请求**。产出 `experiments/extraction-v5/{SUMMARY.md,CELLS.json}`：stage 分布、validated 数、每个 validated 格的模型 verdict 与其 behavior/miri/expert 是否一致。

### 4.2 专家标注 v1 扩展

- 对 §3 全部接受格按 `docs/EXPERT_LABEL_RUBRIC.md` 标注，`labeler: agent-proxy`。
- 在 `CANDIDATES.json` 增加字段 `human_reviewed: bool`、`human_label`；把需要用户人工复核的 ≥6 格（全部分歧格 + 随机 4 格，记录随机种子）列到 `expert-labels/HUMAN_REVIEW_QUEUE.md`，留空格供用户填写。**不要代填。**

### 4.3 验收

- `extraction-v5` validated ≥ 3 且 `harness_error = 0`；每个 validated 格的 verdict 与 behavior/expert 至少一项交叉印证并写进 SUMMARY。
- 全部接受格有 `agent-proxy` 标签；`HUMAN_REVIEW_QUEUE.md` 提交。
- 提交两个：`extract v5: mandatory-label check, type normalizer, all accepted cells`、`expert labels: main-v1 accepted cells + human review queue`。

---

## §5 `holds_all` 反馈 hint 与 `partial_deadlock_bystander` 案例（I-7）

### 5.1 ConcIR

`preserved` 中 `holds_all`/`never_holds_all` 失败时，诊断附模板化 `hint` 字段（机器生成、不含任务名）：
> The design requires a reachable state in which `<function>` holds all of `<resources>` simultaneously; no such state exists in this revision. Keep the nested acquisition; fix the defect by acquisition order, scope, or handshake instead of releasing early.

`never_holds_all` 对偶。`diagnostics_regression` 加对应用例。提交：`explore: hint for holds_all/never_holds_all preservation failures`。**此提交须在 §3.1 编译 `BIN_MAIN` 之前完成**；若来不及，`BIN_MAIN` 不含它，本节案例用单独 sha 并在 RESULTS 偏差里写明。

### 5.2 案例

`experiments/case-partial-deadlock-v1/`：
- 用 `BIN_MAIN`，A3_local 与 A3_whole 各跑 rep=0..2、K=6（预算 60 请求）；
- 每轮记录：诊断（含 hint）、候选 diff、`preserved` 各项 PASS/FAIL；
- `CASE.md`：这个任务为什么难（握手 + 嵌套持锁 + 旁观者）、契约挡掉了哪些"空洞修复"（引用具体候选）、模型最终是否找到设计保持的修复、A2-ml 的修复（上轮 r2 接受、专家 `design_preserved=yes`）与 A3 的差别。
- 这份案例是论文里的 running example 候选，写清楚但不写论文语言。

### 5.3 验收

- `CASE.md` + 原始轮次目录提交；提交：`case: partial_deadlock_bystander under holds_all contract`。

---

## §6 Track D v3 表（RESULTS §6 的数据源）

- Track D 目录里补/整理成 `TRACKD.json`：每任务 × {buggy, fixed} × {concir_petri, concir_interp, miri(16), lockbud} 判定 + 耗时；确认 `cycle_3lock` buggy 行的 Lockbud 判定与 `flash-repair-main-v1` 中 A2-ml 的 Lockbud 输出一致（同为 clean 则在 RESULTS 里并列指出：静态与动态工具都放过了三锁环，专家/ConcIR 抓到）。
- `python -m cir_workflow results` 读取它。
- 提交：`track D: TRACKD.json + results table`。

---

## §7 HANDOFF Round j 与 RESULTS 最终生成

- `# Round 2026-09-20j`：I-1..I-9 对照表；各节 commit；`BIN_MAIN` sha；预算使用（主批次 / 抽取 / 案例分别）；停点；`## Not done`。
- 最后一次运行 `python -m cir_workflow results …` 并提交生成的 `RESULTS.md`（头部含命令）。
- 提交：`handoff: round j; RESULTS regenerated`。

---

## §8（可选，仅在 §1–§7 完成且预算有余时）第二模型探针

若 DeepSeek 端点还有 `deepseek-chat`（V3）或等价非 Flash 模型：只跑 `A2_tools_iter_ml` 与 `A3_local` 两臂、8 任务、rep=0、K=4，预算 60 请求，目录 `experiments/model-probe-v1/`。目的只是回答"结论是否 Flash 特有"，不进主表；RESULTS 单列一节。

---

## 全局规则

- 同一 `BIN_MAIN` 贯穿 §3–§6；任何用了别的 sha 的结果在 RESULTS 偏差里逐条列出。
- 所有 LLM 请求写 `requests.jsonl`（含 rep、arm、task、round、tokens、latency、`reply.kind`）；每次工具调用继续落 `calls/<seq>-<tool>/`。
- RESULTS.md 只能由命令生成；SUMMARY 数字与 RESULTS 不一致视为 bug。
- 不改 benchmark 的 buggy 输入与冻结契约；如发现契约问题，记 `CONTRACT_ISSUES.md`，本轮不改。
- 每节独立提交，提交信息写清节号；做不完的节在 HANDOFF `Not done` 写停点与原因。
