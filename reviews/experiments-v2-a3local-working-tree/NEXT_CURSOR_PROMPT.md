本轮目标：**跑出主批次，并把 Rust 臂 oracle 做成"自动抽取 + 专家标注"双轨**。任务量大，但优先级固定：§1 主批次 `flash-repair-smoke-v3` 排第一且不被任何其他节门控；§2 抽取 harness 修复与逐格归因；§3 `concir-instrument` 自动标注；§4 专家标注；§5 Track D 与成本表刷新；§6 `A3_free` 抽取；§7 面向论文的 `RESULTS.md`。各节独立提交；做不完的节写清停点，但 §1 必须完成。不做多模型、不写论文正文、不调 Lockbud、不放宽 codegen 模式的 conform 规则。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-a3local-working-tree/REVIEW.md（H-1..H-5）
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-contract-strength-working-tree/NEXT_CURSOR_PROMPT.md §四（主批次定义，本轮照做）
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{extract,normalize,arms,flash_smoke,conformance,detection,scale}.py
/Users/kevin/local-repos/ConcIR/src/{conform.rs,codegen.rs,schema.rs}、Cargo.toml

仓库边界不变。每个实验 JSON 带 `binary_sha256`/`git_rev`/`protocol_sha256`/`contract_sha256`。

一、主批次 `flash-repair-smoke-v3`（必须完成，不受 §2–§4 门控）

1. 按上一轮 §四 执行：8 个 hard 任务 × {A0, A1, A2-ml, A3_local, A3_whole}，K=4，Miri 16 种子 ×8 s，`repair_input/` + 终态 `DONE` 行 + 冻结契约。任一任务缺 `repair_input/` 或冻结契约未过 buggy FAIL/fixed PASS，先补齐（补齐本身不算偏差，记进 HANDOFF）。
2. 预算：修复臂 ≤200 请求，3 h；超限即停记 `stop_reason`，已完成格入表。Rust 臂 `oracle.model` 本轮**允许**全为 `inconclusive`，事后由 §2/§4 回填。
3. `SUMMARY.md`：主表（任务 × 臂）、按臂汇总（accept 率、false-accept 率、inconclusive 率、平均轮次、平均 tokens、平均 LLM/工具墙钟）、`A3_local` vs `A3_whole` 每轮 decision 分布（`check_invalid`/`sid_invalid`/`explore_fail`/`accepted` 占比）。数字全部由 `SUMMARY.json` 渲染。
4. PROTOCOL 增补任务集与理由、臂定义、预算、oracle 规则；偏差 D-19：主批次不受 Rust 臂 oracle 完善度门控。

二、抽取 harness 修复与逐格归因（H-1）

1. `extract.py`：覆盖 `src/main.rs` 时若缺 `mod cir_trace;` 自动前置；捕获全部 Python 异常并记为 `stage=harness`（不得再出现 AttributeError 一类原因）。
2. `normalize.py` 补：函数缺 `kind` → `normal`；资源缺 `kind` → 由 `type` 推断（Mutex/Condvar/Semaphore/Channel→`sync`，Var/Atomic→`var`）；`params` 字符串数组 → `[{name, type:"Int"}]`（记录并在反馈中提示确认类型）；`spawn.func`/`scope.funcs`/`call.func` 缺模块前缀 → 补当前模块 FQN；字符串字段收到单元素数组 → 取元素并记录；`expr` 数组 → 拒绝并回送（不猜）。
3. 每格落盘 `extraction_result.json`：`stage ∈ {parse, normalize, codegen, build, trace, conform, explore}`、JSON pointer（解析错误从 serde 消息的 line/col 反算到 pointer）、原始 stderr 路径、`normalizations[]`。
4. 对 v2 批次 12 格与 v3 主批次全部 Rust 接受格补跑抽取（v2 ≤24 请求，v3 ≤64 请求）。HANDOFF 报告按 `stage` 的失败分布；`harness_error` 必须为 0；给出 `extract_validated` 数。

三、自动标注工具 `concir-instrument`（H-2）

1. ConcIR 新增 bin `concir-instrument <input.rs> --out <dir>`（依赖 `syn`/`quote`/`proc-macro2`，仅此 bin 使用，主库不引入）：在每个并发调用点后（阻塞型：`.lock()`, `.wait(`, `.wait_while(`, `.recv(`, `.send(` 到 `sync_channel`, 用户类型的 `.acquire(`）或前（`.notify_one/all(`, `.release(`, `thread::spawn`, `thread::scope`, `.join()`, 显式 `drop(<guard>)`）插入 `cir_trace::ev(<tag>, "L<n>")`。`L<n>` 按源码顺序编号；输出 `labels.json`：`{label, line, op, receiver_expr, position: before|after}`。
2. 线程 tag：`cir_trace` 运行时加 `thread_local! TAG`；`thread::spawn(closure)` 改写为在闭包首行设置 `TAG = "<parent>.<L>"`；`scope.spawn` 同理。`ev(tag, ...)` 改为读取 thread_local，签名 `ev(label)` 保留旧签名兼容。
3. `conform --lenient-unlock`：模型的 `mutex_unlock`/`semaphore_release`/`condvar_notify*` 步允许在没有匹配事件时静默推进；只用于抽取模式；文档写明。
4. 抽取 prompt v4：输入 = 原始 Rust + `labels.json`；要求 CIR 中每条并发语句的 `sid` 直接使用对应标签（`L<n>` 合法化为 sid 格式由 normalizer 做，映射入记录）；不再要求 LLM 输出 Rust。`extract.py` 加 `--mode instrument|llm-annotate`，默认 instrument，syn 解析失败时回退 llm-annotate 并记录。
5. 测试：对 `benchmarks/families/*/rust/{fixed,correct}.rs`（≥10 个文件）跑 instrument → 全部可编译且 20 次原生运行产生轨迹；对 4 个 codegen 一致性用例的 `fixed.rs` 手写对应 CIR 做 conform（lenient）→ 100%。
6. 用 instrument 模式对 v2 的 12 格与 v3 的 Rust 接受格重跑抽取（与 §2 的 llm-annotate 并列报告，预算合计不超过 §2 给的额度）。

四、专家标注 oracle（H-4）

1. `docs/EXPERT_LABEL_RUBRIC.md`（v1）：判定 `bug_present`（yes/no/unsure）的检查清单（锁序、等待谓词、通知丢失、permit 收支、通道满/空阻塞、线程终止），`design_preserved`（原需求要求的临界区/握手/终态是否保留），必须引用行号，每条一句理由。
2. 对 v2 与 v3 主批次中所有 Rust 臂**被接受**的候选，按 rubric 标注，写 `experiments/<batch>/labels/<task>__<arm>.json`：`candidate_sha256, rubric_version, labeler: "cursor-agent", bug_present, lines[], design_preserved, rationale`。标注者不得看 oracle 结果（先标后比）。
3. SUMMARY 加列 `oracle.expert`；`bug_present` 规则改为：`behavior=hang` 或 `model=FAIL(validated)` 或 `expert=yes`；三者不一致的格单列到 HANDOFF 的"分歧表"并给出理由。报告自动 oracle 与专家的一致率（在双方都有结论的格上）。

五、Track D 与成本表刷新（用当前 binary）

1. `experiments/detection-v3/`：全部 buggy（21）与 fixed/correct 用例：ConcIR explore（冻结契约，petri 与 interp）、Miri 16 种子 ×8 s（Rust 参考程序）、Lockbud；按族汇总检出/漏报/假阳；`thread_leak`/`timeout` 单列。表格与 JSON 均带 binary sha。
2. `experiments/scale-v2/`：B5 锁链规模表重跑；另加"全部 37 个就绪用例的 explore 状态数与墙钟"表（petri/interp 各一列），作为方法成本证据。
3. `benchmarks/FAMILIES.md` 与 MANIFEST 同步（含 `conformance_only` 状态变化）。

六、`A3_free` 抽取（G-5 遗留，≤4 请求）

对 conformance-v4 的 `A3_free` 程序用 §3 的 instrument 模式抽取并验证；若验证通过且模型 PASS，则其 66 条违规归因为"自标注错"；否则为"代码错"。写进 HANDOFF 与 conformance-v4 的 md。

七、面向论文的结果汇总 `docs/RESULTS.md`

按论文可能的表格组织，每张表下写来源目录、`protocol_sha256`、`binary_sha256`、生成命令：
1. 能力矩阵与基准（FAMILIES）；
2. 检出能力（detection-v3）；
3. 多臂修复主表与按臂汇总（smoke-v3）+ 教科书两例附表（smoke-v2）；
4. 契约强度（contract-strength-v1）；
5. 一致性层（conformance-v4，含 skeleton_fill vs A3_free）；
6. 规模与成本（scale-v2）；
7. 已知限制与偏差清单（从 PROTOCOL 的 D-1..D-19 汇总）。
不写解释性文字，只放表与来源；每次实验更新后重渲染（提供 `python -m cir_workflow results` 命令从各 JSON 生成，不手填）。

八、交付物与验收

- ConcIR：`concir-instrument` bin、`cir_trace` thread_local tag、`conform --lenient-unlock`、文档；测试全过；单独 commit。
- ConcPlanVerify：smoke-v3 完整目录；抽取修复 + `extraction_result.json` + 两种模式的结果；rubric 与 labels；detection-v3、scale-v2；`A3_free` 归因；`RESULTS.md` 与生成命令；PROTOCOL 偏差 D-19..D-22（主批次解绑、lenient-unlock、专家标注、instrument 模式）；HANDOFF Round i（H-1..H-5 对照、每节 commit、主表与按臂汇总、抽取 stage 分布、专家一致率、分歧表、未完成项）。
- 验收线：smoke-v3 ≥6 个任务跑完 5 臂且 `A3_local`/`A3_whole` decision 分布已出；抽取 `harness_error=0`，两种模式各有 stage 分布；instrument 对 ≥10 个参考程序可编译并产轨迹；所有 Rust 接受格有 `oracle.expert`；`RESULTS.md` 由命令生成且每表带来源；detection-v3 覆盖全部 21 buggy。
- 顺序：§1 → §2 → §4 → §3 → §5 → §6 → §7；§3 若超过预计工时可停在 §3.5 的测试处，其余节不受影响。
