你在两个仓库中工作：`/Users/kevin/local-repos/ConcPlanVerify`（Python 编排、prompt、benchmark、实验数据）、`/Users/kevin/local-repos/ConcIR`（Rust 验证后端）。**本轮不做任何论文写作**（不改 `paper-review/papers/ConcPlanVerify/` 下任何文件，不生成论文用 notes）；只做实验、清理与打包。先读 `ConcPlanVerify/reviews/experiments-v2-round-p-working-tree/REVIEW.md`（P-1..P-6）、HANDOFF Round p、`python/cir_workflow/rust_oracle.py`（`prepare_project`/`instrument_wrappers`）、`python/cir_workflow/generation.py`（`run_llmcode_from_cir`/`mapping_to_cir`）、`prompts/rust_*.md`、ConcIR `src/conform.rs`（`--op-resource`）与 instrument v2。

## 本轮要解决的事实（必须先理解）

freeze-6 活跑 `flash-gen-main-v3-code` 的 19 个 G3 代码阶段失败格里：

- **13 个 `build_failed`**：`prepare_project` 往 `main.rs` 写 `mod concir_sync;`，四个 Rust prompt 又叫 LLM "declare `mod concir_sync;`"，或 LLM 内联自己的 `mod concir_sync {…}` → `E0428`；instrument 重写 `use std::sync` 时漏掉嵌套模块 → `E0425`。semaphore 家族 9/9、`scope_bound_k_workers` 3/3、channel 1。
- **3 个 `counter_overflow_safety`**：唯一违规是 main 在所有 worker join 之后 `lock()` 读终态打印，CIR main 只有 `scope+return`。不是并发偏离。
- **1 个 `bounded_backpressure_lock_held` rep0**：CIR 声明 `m` 却不用而 PASS，代码按需求加了锁 → conform 正确拒绝（真实偏离，暴露 CIR check 缺口）。
- 40 个接受格里 channel 家族 `unmapped` 15/36：LLM 用 `Mutex<Receiver<T>>` 包装，instrument 报 `ch_mutex0`，monitor 对不上契约的 channel 资源，RF 被压低。

HANDOFF 里"剩余失败全是真实偏离、harness 缺口 < 50%、止损触发"的结论作废：重放用的是 freeze-5 旧程序，检验不了 semaphore 修复。止损未触发。

预算：DeepSeek Flash ≤ 400 请求，OpenCode Go ≤ 80 请求。各节独立提交；停点写 HANDOFF Round q。

---

## §1 harness 修复第二轮（D1 上午）

### 1.1 `concir_sync` 作为 crate 依赖（ConcPV，必要时 ConcIR）

- `concir_sync` 变成独立 crate（放 ConcIR 仓 `crates/concir_sync/` 或 ConcPV `runtime/concir_sync/`，选一处并写 README），`prepare_project` 在 `Cargo.toml` 加 path 依赖，**不再**往 `main.rs` 写 `mod concir_sync;`。
- 四个 prompt（`rust_generation_v1`、`rust_self_review_v1`、`rust_tool_feedback_v1`、`rust_from_cir_v1`）同一句话："An external crate `concir_sync` is already linked; write `use concir_sync::Semaphore;`. Do not declare `mod concir_sync` and do not implement a semaphore yourself."
- harness 容错：LLM 代码中出现 `mod concir_sync;` 或内联 `mod concir_sync {…}` 时剥除、记 `harness_note`，不算 build 失败。
- instrument v2 对 `concir_sync::Semaphore` 的 `acquire/try_acquire/release/Permit drop` 发 `sem_acquire/sem_release`；确认嵌套模块内的 `use std::sync::…` 也被正确重写（加一个含嵌套 mod 的单测）。

### 1.2 conform：join 后的主线程操作（ConcIR）

`--op-resource` 模式下，主线程在 CIR `scope` 对应的所有 spawned 线程结束之后的事件不参与匹配，计入输出字段 `post_join_main_ops`（数量与 op 列表），status 不因此变 violation。`rust_from_cir_v1` 的 main 约定补一句："after joining all workers, main may read shared state to print the terminal line."
**回归**：`conform-mutation-v2` 全套重跑，任何突变从"抓到"变"漏掉"→ 停并写 HANDOFF。

### 1.3 `Mutex<Receiver<_>>` / `Mutex<Sender<_>>` 包装（ConcIR instrument v2 + ConcPV mapping）

- instrument 识别只包着 channel 端点的 Mutex，把其中 `recv/try_recv/send` 归到 channel 资源，外层 mutex 标 `wrapper: true`；conform 与 monitor 忽略 `wrapper` 事件（conform 输出里计 `wrapper_ops`）。
- `mapping_to_cir` 与 monitor `--mapping` 对 channel 资源按名字优先对齐（LLM 已被要求用需求实体名）。
- 同一 prompt 补建议："move the `Receiver` into the receiving thread instead of wrapping it in a Mutex."
- 统计基线臂（G0/G1/G2）channel 任务的 `unmapped`，修复是否同样使其下降，写进 PROTOCOL 备注。

### 1.4 CIR check：声明未用资源（ConcIR，仅加检查，不重跑 CIR 阶段）

`check` 新增 `W2xx resource_declared_unused`（warning 级，进 revision 反馈文本），不改变 PASS/INVALID 判定。在 HANDOFF 记录：freeze-5 的 59 个 PASS CIR 里有多少会触发该 warning（零请求，直接跑 check）。

提交：（ConcIR）`concir_sync crate; conform: post-join main ops, channel wrapper mutex; check: W2xx unused resource`，tag `concir-freeze-6`；（ConcPV）`harness: concir_sync as dependency, strip duplicate mod; prompts unified`。

## §2 离线重放 `gen-code-replay-v2`（D1 下午，零请求）

对象：`flash-gen-main-v3-code/run-20260923T182939` 全部 59 格的最后一轮 Rust。

- 19 个失败格：修复后各自到哪一步（build / conform / monitor / accepted），分三类报：P-1 修复、P-2 修复、仍失败（附 `first_violation` 全文与一句人读判断）。
- 40 个接受格：无回归（conform 仍 32/32；`unmapped` 应下降，逐格前后对比）。
- 另外重放 `gen-llmcode-smoke-v1` 45 个 Rust，给修复后的 conform PASS 率。

`SUMMARY.md` 顶部一句话：harness 修复后离线可解释的失败占比。

## §3 活跑 `flash-gen-main-v4-code`（D2）

- G3 v2 代码阶段：复用 freeze-5 的 59 个 PASS CIR（`cir_path` 不变），K_code = 3，修复后的 prompt/harness，3 rep。≤ 177 Flash。**单一 run id**，不做只补失败格的部分重跑。
- semaphore 家族基线 G0/G1/G2：3 任务 × 3 rep × 3 臂，用修好的 `concir_sync` 说明；≤ 90 Flash。channel 家族基线是否重跑取决于 §1.3 对基线 `unmapped` 的统计（若基线 channel `unmapped` ≥ 1/3，也重跑：3 任务 × 3 rep × 3 臂，≤ 90 Flash）。其余格沿用 freeze-5，PROTOCOL 逐格写清来源 run id。
- kimi-k3：G3 代码阶段 24 格（CIR 复用 `gen-model-probe-v2`），≤ 72 Go。
- 指标：freeze-6 全部 + `post_join_main_ops`/`wrapper_ops` 计数 + conform violation kind 分布（此轮才有意义）+ "conform 抓到、monitor/behavior 没抓到"的格数。
- 接受的 G3 Rust：agent-proxy 专家标注 rubric v3（`bug_present`、逐条 `Ri` 满足 + 证据行号），按 sha 去重；`HUMAN_REVIEW_QUEUE_GEN.md` 重生成（≥ 8 格、去重后合并同 sha 行并注明），verdict 列留空。

## §4 freeze-7（D2 晚）

`python -m cir_workflow results …` 合并 v4-code、重放 v2、探针；重生成 `RESULTS.md` 与 `tables/*.tex`（命令生成，不手改）；`FREEZE_MANIFEST.md` 追加；tag `experiments-v2-freeze-7`。HANDOFF Round q 写 freeze-6 → freeze-7 主指标变化与归因（P-1/P-2/P-3 各贡献多少格）。

止损：§3 之后 G3 v2 接受率仍 < 0.7 → 停止工具迭代，把剩余失败格逐格写成人读的失败原因表，进 HANDOFF。

## §5 实验目录清理（D3 上午，与 §3 跑批并行可做）

原则：**不删数据，只归档；不动任何 freeze tag 引用的内容**。

1. 生成 `experiments/EVIDENCE_TIERS.md`：对 `experiments/*` 每个目录标 Tier A（`RESULTS.md`/`tables/*.tex`/`FREEZE_MANIFEST.md` 直接引用）、Tier B（被 A 依赖或作为版本对照：`flash-gen-main-v1`、`conform-mutation-v1`、`post-edit-conform-v1`、`model-probe-v1` 等）、Tier C（已被后续版本完全取代的冒烟/试跑：`pilot-*`、`conformance-v1..v3`、`extraction-v4`、`detection-v1/v2`、`flash-repair-smoke-v1..v3`、`flash-arms-smoke-v1`、`gen-model-probe-v1`、`rebase-*`、`a3-to-rust-v1` 等，以实际引用为准）。
2. Tier C 用 `git mv` 到 `experiments/_archive/<name>/`，每个附 `WHY_ARCHIVED.md`（被哪个目录取代、原 tag）。`docs/PAPER_EVIDENCE_MAP.md` 中的路径同步更新。
3. 工作树中被忽略的 `experiments/**/target/`（2308 个，约 11 GB）用 `git clean -fdX -- experiments` 之前先 `-n` 预览，确认只列出 `target/` 后执行；HANDOFF 记录释放体积。
4. `scripts/` 里已被 `python -m cir_workflow …` 子命令或新版脚本取代的一次性脚本 `git mv` 到 `scripts/legacy/`，加 `scripts/README.md` 列出每个活跃脚本对应的实验目录与 freeze。
5. 各实验目录内重复落盘的 `cir_trace.rs`/`concir_sync.rs` 副本、空 `stderr.txt`：只统计数量写进 EVIDENCE_TIERS，不删（打包时排除）。

提交 `experiments: tier evidence, archive superseded runs, legacy scripts`。

## §6 Supplement 打包脚本（D3 下午）

`scripts/make_supplement.py`（纯标准库）+ `make supplement`，输出 `dist/concplanverify-supplement-<freeze-tag>.zip` 与同名 `.sha256`。

内容（`SUPPLEMENT_MANIFEST.md` 列出每项来源与 sha）：
- `README.md`：目录说明、环境（Rust/Python 版本、`concir-backend` 构建命令）、每张表的复现命令、预算与 provider 说明（不含 key）。
- `benchmarks/`：需求文档、冻结契约 v3.1、`GENERATION_MANIFEST.json`、修复研究的 buggy/fixed 参考与 `REFERENCE_AUDIT.md`。
- `prompts/`：全部 prompt 文件。
- `concir/`：ConcIR 在 `concir-freeze-6` 的 `git archive`（源码 + `concir_sync` crate，无 target）。
- `python/`、`scripts/`（活跃脚本）、`Makefile`/`pyproject`。
- `experiments/`：Tier A 目录的 `PROTOCOL.md`、`SUMMARY.md`、`SUMMARY.json`、`CELLS.json`、各格 `CELL.json`、最终 Rust/CIR 产物、`mapping.json`、专家标注与人工队列；**排除** `proj/`、`target/`、`traces/`（可选 `--with-traces` 打包为单独 zip）、`llm/` 原始请求日志（除非 `--with-llm-log`，且先过 secret 扫描）。
- `RESULTS.md`、`tables/*.tex`、`FREEZE_MANIFEST.md`、`docs/PAPER_EVIDENCE_MAP.md`、`EVIDENCE_TIERS.md`。

硬性检查（任一失败则不产出 zip，退出非零）：
- 匿名化：包内不得出现 `/Users/kevin`、作者姓名、机器名、GitHub 用户名；路径统一改写为包内相对路径（`CELL.json` 等里的绝对路径重写，重写规则写进 README）。
- 秘密扫描：`sk-`、`OPENCODE_API_KEY`、`DEEPSEEK`、`Bearer `、`.env` 内容，命中即失败。
- 体积：默认包 ≤ 200 MB；超出则报告最大的 20 个文件并失败。
- 自检：在临时目录解压，`python -m cir_workflow results --from <unzipped>/experiments` 能重生成与包内一致的 `RESULTS.md` 与 `tables/*.tex`（diff 为空）。

跑一次生成 freeze-7 的包，把 `SUPPLEMENT_MANIFEST.md` 与体积、自检结果写进 HANDOFF。`dist/` 加入 `.gitignore`。

提交 `supplement: make_supplement.py with anonymization/secret/size/self-check gates`。

## 全局规则

- API key 只从 `.env` 读，不写进任何提交文件、`requests.jsonl` 或 supplement。
- `HUMAN_REVIEW_QUEUE*.md`、`owner_verdict` 只由用户填。
- 契约对生成模型不可见；实体命名与 `concir_sync` 库说明可以给，四臂措辞一致。
- 不改 benchmark 需求与冻结契约（v3.1）；不改已有 run 目录的原始数据；新数据放新目录。
- `RESULTS.md`、`tables/*.tex` 只由命令生成。
- 每节结束提交；任何止损或回归触发先写 HANDOFF 再停。
- 本轮不碰论文仓库。
