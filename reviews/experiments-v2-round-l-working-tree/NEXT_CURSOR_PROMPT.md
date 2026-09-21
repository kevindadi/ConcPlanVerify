你在 `/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）和 `/Users/kevin/local-repos/ConcIR`（Rust 工具）两个仓库中工作。先读 `reviews/experiments-v2-round-l-working-tree/REVIEW.md`（L-1..L-6），再读 `experiments/conform-mutation-v1/CONFORM_GAPS.md`、`experiments/post-edit-conform-v1/SUMMARY.md`、ConcIR `src/cir_trace.rs`、`src/conform.rs`、`src/codegen.rs`、`src/bin/concir-instrument.rs`、ConcPV `python/cir_workflow/{providers,llm}.py`。

本轮目标有三：(1) **把 conform 从"流级注解检查"变成"操作绑定的运行时一致性检查"**并重跑突变与后编辑，给主张 (d) 真实的召回；(2) **第二/第三模型探针**——用户在 `.env` 里提供了 `OPENCODE_API_KEY`（OpenCode Go 套餐，OpenAI 兼容端点），回答"结论是否 DeepSeek Flash 特有"；(3) 合并用户已填写的人工复核并冻结。这是最后一个实验轮。各节独立提交；**§0–§4、§5 必须完成**，§6–§7 做不完写停点。不改 benchmark 的 buggy 输入与冻结契约；不重跑 `flash-repair-main-v1` 的 LLM 数据；主表判定不重算。

预算：DeepSeek 请求 ≤ 90（§3 ≤69，§6 ≤12）；OpenCode Go 请求 ≤ 160（§5），且遵守套餐的滚动额度（5 小时 $12 / 周 $30，见 §5.1）。

---

## §0 合并人工复核（先做，不发请求）

工作树里 `experiments/flash-repair-main-v1/expert-labels/HUMAN_REVIEW_QUEUE.md` 已由用户填写 17 行 `human_label`（`reason` 列为空——**不要代填**，在合并结果里标 `human_reason: null`）。

- 合并进 `EXPERT_LABELS.json`：按 sha 加 `human_label`、`human_reviewed: true`；同一 sha 多行取一致值，若不一致报错停止。
- RESULTS 专家表加 `human` 列；新增一致率：human vs agent-proxy、human vs auto（`unsure` 排除并计数）。当前 17 行里 4 个 agent `unsure` 被人工判 `no`，2 个 agent `no` 且 auto `True`（`acquire_twice/A0` `f42afd77e7a9`）人工判 `no`——后者是**人工与自动 oracle 的分歧**（auto 认为 hang），要在分歧表里单列并写清 auto 的依据（behavior=hang 的原始输出）。
- `docs/PAPER_EVIDENCE_MAP.md` (h) 改 **ready**，写明 17 行、覆盖全部分歧与 `unsure`、随机 4。
- 先提交用户的文件本身，再提交合并：`expert labels: human review (17 rows) merged; human agreement`。工作树里另一个无关改动 `reviews/experiments-v2-contract-strength-working-tree/NEXT_CURSOR_PROMPT.md` 单独提交或还原，不要混进本节。

---

## §1 ConcIR：`cir_trace` v2 —— 操作绑定事件

### 1.1 包装类型

新模块 `cir_trace::sync`，与 `std::sync` 同名、drop-in 替换，**事件在操作发生时由包装器发出**：

| 包装 | 事件 |
| --- | --- |
| `Mutex<T>::lock(&self, sid)` → `MutexGuard` | 获得锁后发 `mutex_lock`；guard `Drop` 发 `mutex_unlock`（同 sid、同资源） |
| `Condvar::wait(guard, sid)` / `notify_one(sid)` / `notify_all(sid)` | `condvar_wait`（返回后）、`condvar_notify`、`condvar_notify_all` |
| `Semaphore::acquire(sid)` / `release(sid)` | `sem_acquire`（获得后）、`sem_release` |
| `channel::Sender::send(v, sid)` / `Receiver::recv(sid)` | `channel_send`/`channel_recv`（完成后）；cap=0 rendezvous 与现有规则一致 |
| `spawn(sid, closure)` / `join(sid)` | 子线程 tag = **spawn 语句的 sid**；`spawn`、`join` |

- `sid` 为调用点参数；资源名在构造时绑定（`Mutex::new(v, "main::a")`）。
- 裸 `cir_trace::ev` 保留但 codegen 模式禁用：混用报 `E9xx mixed_instrumentation`。

### 1.2 codegen v2

生成 v2 代码，每个同步调用带 sid，不再发独立 `ev` 行；保留 `labels.json`。23 个已接受 CIR（19 + 10 任务批次新增 4）codegen 后 build 通过、conform PASS（回归基线）。

### 1.3 conform v2

- 每线程事件序列必须是该函数 CIR 体的合法路径：顺序、种类、资源、sid 三者比对；`mutex_unlock` 位置与模型中 unlock/作用域结束比对。
- 子线程按 spawn sid 匹配。
- 每条 violation 输出 `kind ∈ {order, missing, extra, resource, unlock_position, unknown_sid}` 与首个偏离位置。
- 抽取模式 `--lenient-unlock/--attempt-events` 保留。

### 1.4 concir-instrument v2

对自由 Rust：替换 `std::sync::{Mutex,Condvar}`、`mpsc`、`thread::spawn` 为包装类型，sid 按 (资源, 种类, 出现顺序) 生成写入 `labels.json`。

### 1.5 验收

- `cargo test` 全绿；新增回归：包装器事件顺序、guard Drop 发 unlock、spawn tag = sid、混用报错。`engine_agreement`、`diagnostics_regression` 不变。
- release 编译，记 `BIN_V2` sha；`experiments/REBASE_<BIN_V2前8位>.md`：全部已接受 CIR 的 explore 判定 0 变化。
- 提交（ConcIR）：`cir_trace v2: operation-bound events, sync wrappers, sid-tagged spawn; conform v2 violation kinds; codegen v2`。

---

## §2 突变敏感性重跑：`conform-mutation-v2`——不发请求

- 算子在 v2 代码上重定义（`PROTOCOL.md` 明确每个算子改的是**操作**）：M1 交换相邻两次 `lock()`；M2 删除 `drop(guard)`；M4 `notify_all`↔`notify_one`；M5 `send/recv` 移入/出临界区；M6 删除一次同步调用；M7 交换两个 spawn（对照，期望 PASS）；新增 M8 某次 `lock()` 换成另一资源。M3 记 `n/a by construction`。
- 23 个程序、≥100 突变体（M7 ≥ 15）。每个：build → strict conform v2 → behavior → Miri 16。
- `SUMMARY.md`：按算子 conform 检出率（`violation` 与 `timeout` 分开）、Miri、behavior；M7 PASS 率；未检出突变逐条归为等价或盲区，`CONFORM_GAPS.md` v2 版与 v1 并排。

提交 `conform-mutation-v2: operation-bound conform recall`。

---

## §3 后编辑重跑：`post-edit-conform-v2`——DeepSeek ≤69 请求

- 23 个 v2 程序 × E1/E2/E3（`prompts/post_edit_v1.md` 不变，不提并发/CIR/sid）。模型若删掉/改掉 sid，conform 报 `unknown_sid/missing`，单列 `sid_dropped`。
- 每格：build → conform v2 → behavior → Miri 16。模型把包装类型换回 `std` 时用 instrument v2 重新替换，记 `reinstrumented`。
- `SUMMARY.md`：按编辑类型 build/conform PASS/violation kind 分布/Miri/behavior；`drift_caught_only_by_conform` 逐格（diff 摘要、kind、期望 vs 观察）；E3 单独一节：模型"优化"了什么、哪些被抓住、哪些真的等价。为 0 时给逐格"编辑未触及同步操作"的证据。

提交 `post-edit-conform-v2: developer edits vs operation-bound conform`。

---

## §4 a3-to-rust-v2 与 RESULTS

- 23 个 CIR 用 `BIN_V2`：codegen → build → conform v2 → behavior → Miri 16；`distinct_traces` 保留。
- RESULTS：`conform_pass_rate` 用 v2；新增 §"Conform recall"（v1/v2 并排）、§"Post-edit drift"（v1/v2 并排）；`tables/mutation.tex`、`postedit.tex` 重生成，新增 `conform_recall_v1v2.tex`。
- `PAPER_EVIDENCE_MAP.md` (d) 改写：v2 召回与 drift 数字，说明 v1→v2 的变化与原因。
- 提交 `a3-to-rust-v2; RESULTS conform v2; evidence map (d)`。

---

## §5 第二/第三模型探针：`model-probe-v2`——OpenCode Go ≤160 请求

### 5.1 提供方接入

- `providers.py` 新增 `opencode-go`：`base_url = https://opencode.ai/zen/go/v1`，鉴权 `Authorization: Bearer $OPENCODE_API_KEY`（读 `.env`），只用 **`/chat/completions`**（OpenAI 兼容）。`/responses`（GPT/Grok/Muse）与 `/messages`（MiniMax/Qwen）的模型**不纳入**，PROTOCOL 写明原因（复用现有客户端与身份校验）。
- 第一步 `GET /models` 把可用列表原样存到 `experiments/model-probe-v2/MODELS.json`。
- 模型选择（按优先级取**两个**不同家族，都存在则取前两个；第三个可选）：`kimi-k2.7-code` → `kimi-k3` → `glm-5.3-flash` → `glm-5.2` → `mimo-v2.5`。**不选** `deepseek-*`（与主模型同家族；可作为可选第三个"同家族对照"）。
- 身份校验：首个探针请求记录响应 `model` 字段，之后每个响应必须与之一致；与 `deepseek-flash` 相同则中止本节。
- 参数与主协议对齐：temperature 0、max_tokens 4096、timeout 90 s；有 thinking/reasoning 开关的模型显式关闭并把请求体里的相关字段记录进 `requests.jsonl`；关不掉则记 `thinking: provider_default`。
- 每个响应记 `usage` 与端点返回的 `cost`（若有）；`budget.json` 累计；单模型花费超过 **$8** 或收到 429/额度错误则停止该模型并记 `stop_reason`。运行安排在两个模型之间间隔，避免撞 5 小时 $12 滚动额度。

### 5.2 协议

- 10 任务 × 3 臂（`A0_direct`、`A2_tools_iter_ml`、`A3_local`）× 1 rep、K=4、同 `BIN_MAIN` 判定、同冻结契约、同去泄漏输入。每模型 ≈ 10 + 10×≤4 + 10×≤4 ≤ 90 请求，预期 ~60–70。
- 接受的 A3 CIR 走 a3-to-rust-v2（0 请求）；接受的 A0/A2 Rust 走 behavior/Miri 16 + 专家标注（agent-proxy，按 sha 去重，rubric v2）。
- 若两个模型都跑完且额度允许，可选第三个：`deepseek-v4-flash` 作为同家族对照（≤40 请求）。

### 5.3 产出

`experiments/model-probe-v2/{PROTOCOL.md,MODELS.json,run-*/,SUMMARY.md}`。SUMMARY 每模型一张与 Flash 同格对照表（接受、false_accept、轮数、tokens、tokens/correct、A3 `check_invalid`/`explore_fail` 分布）；一段结论：三条主张 (a) 0 false-accept vs 工具绿灯、(b) local 成本、(c) 契约挡空洞修复，在每个模型上是否复现。RESULTS 单列 §"Model probe"，`tables/model_probe.tex`；`PAPER_EVIDENCE_MAP.md` (j) 更新。**不进主表**。

提交 `model-probe-v2: OpenCode Go models on A0/A2-ml/A3_local`。

---

## §6 tiered 触发修复（L-3）——DeepSeek ≤12 请求

- 读 `bare_wait_no_predicate` 三个 rep 的 A3_local 决策序列，写明为何未触发升级。
- 触发改 T3：连续 2 轮 `decision ≠ accepted` 即升级；PROTOCOL 更新。
- 仅对该任务重跑 tiered × 3（K=4）；结果进 RESULTS tiered 附录（主行不变），写 `flash-repair-main-v1/TIERED_ADDENDUM.md`。
- 提交 `A3_tiered: trigger T3 + bare_wait rerun`。

---

## §7 冻结 2 与 HANDOFF

- tag `experiments-v2-freeze-2` / `concir-freeze-2`；`FREEZE_MANIFEST.md` 追加（`BIN_MAIN` 主表、`BIN_V2` conform/a3-to-rust；两者都列；model-probe 目录 sha）。
- HANDOFF `# Round 2026-09-2xm`：L-1..L-6 对照表、DeepSeek 与 OpenCode 预算分列、v1→v2 召回一句话结论、模型探针一句话结论、停点、`Not done`。
- 提交 `freeze-2; handoff round m`。

---

## 全局规则

- 主批次 LLM 数据与 `BIN_MAIN` 判定不动；conform/a3-to-rust/突变/后编辑用 `BIN_V2`；RESULTS 头部两者都写。
- RESULTS.md 与 `tables/*.tex` 只能由命令生成。
- 每次 LLM 请求写 `requests.jsonl`（provider、model、返回的 model、usage、cost、rep、arm、task、round、`reply.kind`）；每次工具调用落 `calls/<seq>-<tool>/`。
- API key 只从 `.env` 读取，不写进任何提交文件；`requests.jsonl` 里不得出现 key。
- 人工复核内容不得由代理修改或补写。
- 每节独立提交；做不完的节在 HANDOFF `Not done` 写停点与原因。
