本轮目标：不扩规模，把修复型主实验的数据洗干净并重跑同样 3 个任务。具体：(1) 先把上一轮全部工作提交进两个仓库并修复 ConcIR 提交不可编译、toolchain 未钉死的问题；(2) 消除基准输入泄漏（Rust 注释、CIR 程序名、需求措辞）；(3) 让 A3 不再因 schema 格式输掉：prompt 内嵌由 ConcIR 生成的权威语法表、确定性 Tier-1 schema 规范化、停滞检测与局部修补回退；(4) ConcIR 诊断把索引渲染成名字并给出 doom 状态摘要；(5) 一致性层修口径（按 `(function,sid)` 计覆盖率、每个 miri 种子一条轨迹）、补 channel 与多模块 codegen、真正跑一次 HOLE 填充与 `A3_free` 对照；(6) 为 6 个 repair 任务写 `rust/tests/behavior.rs` 与 `bug_present` 规则，接通 LLM 抽取 + 一致性验证的模型 oracle；(7) 重跑 repair-smoke（同 3 任务）与 conformance 冒烟并对比上一轮。不做全量 batch、不做多模型、不写论文、不调 Lockbud。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-conformance-working-tree/REVIEW.md（R-1..R-8）
/Users/kevin/local-repos/ConcPlanVerify/experiments/EXPERIMENTS_V2_HANDOFF.md（Round d）
/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v1/run-20260918T161254-8755-bf0e2a/{SUMMARY.md, condvar__bare_wait_no_predicate/A3_ours_revision/*/result.json, llm/llm-29.json, llm/llm-30.json, llm/llm-21.json, llm/llm-22.json}
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{revision_workflow,arms,conformance,rust_arm,prompts}.py，prompts/concir_generation_v1.md
/Users/kevin/local-repos/ConcIR/src/{codegen.rs,conform.rs,validate/,explore/}，doc/{ebnf.md,error_codes.md,backend-usage.md}

仓库边界不变。

一、仓库卫生（R-8，先做）

1. ConcIR：`git add src/codegen.rs src/conform.rs tests/conformance.rs` 后 `git commit --amend` 或追加 `fix: include codegen/conform sources`（写明 `e643ff5` 单独 checkout 不可编译）。
2. toolchain：`rustup toolchain install nightly-2026-09-04 --component miri rust-src`，`rustc +nightly-2026-09-04 -V` 与当前 `a69a63265 2026-09-03` 比对；一致则 `rust-toolchain.toml` 钉为 `nightly-2026-09-04`，不一致则钉到实际匹配的日期。`cargo test --offline --all-targets` 全过后记录 hash 与 binary sha256。
3. ConcPlanVerify：把上一轮全部改动分组提交（benchmarks / conformance-v1 / repair-smoke / harness / docs），每组一个 commit；`git status` 干净后再开始本轮。

二、基准去泄漏（R-1）

1. 新建 `benchmarks/build_families.py --sanitize`：对每个用例生成 `repair_input/` 目录：
   - `input.rs`：由 `rust/buggy.rs` 去掉全部 `//`、`//!`、`/* */` 注释与空行压缩，用 `rustfmt` 重排；标识符不改。
   - `input.cir.json`：由 `buggy.cir.json` 复制，`program` 改为 `case_<家族序号><用例序号>`（如 `case_c2`），删除任何 `description`/`note` 字段。
   - `requirements.txt`：人工重写，不得出现 deadlock/lost/leak/bug/fix/wrong/order 等定性词，只描述期望行为与终止性；MANIFEST 记录每个 requirements 的 sha。
2. `repair_task.json` 指向 `repair_input/`，不再指向原文件。原 `rust/buggy.rs`、`buggy.cir.json` 保留作 ground truth。
3. 加一个 lint 测试：`repair_input/*` 中 grep 上述词表必须为空；`input.cir.json` 的 `program` 必须匹配 `case_[a-z][0-9]+`。

三、A3 不再因格式输掉（R-2）

1. ConcIR 新子命令 `schema`：从 `ast.rs` 的 serde 定义机器生成 JSON：每种 statement kind 的必填/可选字段与类型、每种 resource 的必填字段（含 var 的 `base`/`init`）、sid 正则、expr 为字符串、contract 形状。输出即权威，禁止手写维护。
2. `prompts/concir_generation_v2.md`：schema 段由 `concir-backend schema` 渲染的紧凑表替换（每 kind 一行），另附 3 个最小合法片段（`write_shared`、`branch`+`goto`、var 资源声明）。旧 v1 保留，PROTOCOL 登记偏差。
3. `python/cir_workflow/normalize.py`：确定性 Tier-1 规范化，**只做**有唯一无歧义映射的改写并逐条记录到 `normalizations[]`：`write_shared/read_shared.resource → var`；`mutex_lock.var → resource` 之类的字段别名（别名表从 `schema` 输出反推，写死在文件里并有测试）；expr 对象 `{kind:bool,value:true}` → `"true"`、`{kind:int,value:n}` → `"n"`；缺 `base` 时若 `init` 是 bool/int 则推断；sid 不合规**不改**（改 sid 会破坏可追溯性），只回送。规范化后的程序另存 `revision-<n>.normalized.cir.json`，check 用规范化版本，反馈里告诉 LLM 做了哪些改写。
4. schema 反馈重写：去掉文件路径与行号，改为 `path`（JSON pointer）+ 该节点原文 + 正确形状示例；`detail` 不得含与本错误无关的提示。
5. 停滞检测：候选 sha 与上一轮相同 → 不再重发整份反馈，切换到**局部修补 prompt**：只给出错节点（含上下各 2 条语句）、错误、正确形状，要求只回该函数的 `body` 数组；harness 合并回整份程序。连续两次停滞则 `stalled` 结束。
6. 回归：(a) `resource`→`var` 规范化后 check valid；(b) 停滞两次触发局部修补；(c) sid 不合规不被改写；(d) 把 smoke-d 的 bare_wait v3 原文喂给新链路，期望 check valid 且 explore PASS（离线，不用 LLM）。

四、ConcIR 诊断可读性（R-3）

1. `explore` 诊断里所有 `related_functions`、`counterexample[].origin`、`blocked[].thread` 渲染为 FQN / `function.sid` / 线程的入口函数名（保留数字字段作 `_idx` 后缀）。
2. 目标层（`reachability`/`always_reachable`/`preserved`）FAIL 时新增 `doom_state`：`threads: [{thread, function, at_sid, holds: [resource...], waiting_on: {kind, resource} | null}]` 与 `free_resources`。deadlock 类 FAIL 复用同一结构。
3. `repair_hints` 对目标层至少给出一条与 doom_state 一致的具体提示（例如"线程 a 在 s2 持有 main::mtx_a 等待 main::mtx_b；线程 b 相反——统一获取顺序或在 s2 前释放"）。模板化生成，不引入 LLM。
4. 更新 `doc/backend-usage.md` 诊断示例；insta 快照按需刷新并在 commit 说明。

五、一致性层修口径与扩覆盖（R-4、R-6、R-7）

1. 覆盖率按 `(function, sid)` 去重；`sids_total` 取自程序中带 `ev` 的语句集合；报告 `uncovered[]`。
2. miri：每个种子单独 `cargo miri run` 并设 `CIR_TRACE_OUT=traces/miri-<seed>.jsonl`；`traces_total = native + seeds`。超时统一叫 `timeout`，并在结果中标 `hang_suspect: bool`（需求要求终止且超时）。
3. codegen 补：`channel_send/recv` → `std::sync::mpsc::sync_channel(cap)`（cap=0 也用 `sync_channel(0)`）；多模块 → `mod <name> { pub(crate) ... }` 与跨模块 FQN 引用；nobody 函数 → 保留 HOLE。`rwlock`/`select`/`async` 仍报 `unsupported_for_codegen`。至少让 `channel/*` 与 `real-cases/rmw-zenoh-998` 能 codegen + build。
4. HOLE 与对照：新增用例 `structure/worker_with_payload`（correct）：`scope` 下 k 个 worker，每个调用 nobody 函数 `compute()`，随后在锁内 `write_shared`。codegen 后必有 ≥1 HOLE。跑 Flash 填洞 + lint + 50 native + miri 0..15 → conform；同时跑 `A3_free`（同一 CIR 让 LLM 自由写 Rust 并自插 `ev`）→ 只回放。两者的 conformant/violation/coverage 并列进 `experiments/conformance-v2/CONFORMANCE.md`。预算 ≤6 请求。
5. 重跑 conformance-v1 的 4 个用例进 conformance-v2（新口径、新 miri 方式）。

六、Rust 臂 oracle（三列，分列不合成）

1. `oracle.behavior`：为 6 个 repair-smoke 相关用例（abba、partial_deadlock、bare_wait、lost_wakeup、permit_leak、notify_one_wrong_pick）各写 `rust/tests/behavior.rs`：把候选 `main.rs` 编译为 lib 后调用入口，`std::thread::spawn` 包一层 10 s 超时，断言终止 + 期望终态（如 `ready==true`、permits 回到初值）。超时 → `hang`。测试文件由 harness 注入候选项目，不交给 LLM。
2. `oracle.miri`：many-seeds 0..64 + 超时 8 s；状态 `detected/clean/timeout/tool_error`；查清 R-5 的 `tool_error ×5`（对比 A2-ml 第 3 轮 `calls/*-miri/env.json` 是否残留 `RUSTC_WRAPPER`），修后补回归。
3. `oracle.model`（抽取）：`prompts/rust_to_cir_extract_v1.md`——给候选 Rust，要求输出 CIR **和**一份在每个并发操作前插入 `cir_trace::ev(tag,"fn.sid")` 的 Rust 副本；harness 用 conformance 回放验证抽取（native 20 + miri 0..15）；全部 conformant 才对抽取 CIR 跑 explore，结果记 `model_verdict` 并附 `extract_validated=true`；否则 `extract_unverified`。每个候选 ≤2 次请求。
4. `bug_present`（Rust 臂）= `behavior=hang` 或 `model_verdict=FAIL(validated)`；`false_accept` = 臂 accept 且 `bug_present=true`；`inconclusive` = 三列都拿不到结论。SUMMARY 每格显示三列原始值，不只显示合成布尔。

七、重跑 repair-smoke（同 3 任务，`experiments/flash-repair-smoke-v2/`）

- 任务：abba_2lock、partial_deadlock_bystander、bare_wait_no_predicate（全部用 `repair_input/`）。臂：A0、A1、A2-m、A2-ml、A3（v2 prompt + normalize + 停滞处理）。K=4。预算：修复臂 ≤48 请求，oracle 抽取 ≤24 请求，总 ≤72。
- `SUMMARY.md` 增加与 smoke-v1 的对照表：每任务×臂 accepted/round/tokens 的前后变化，并标注变化原因（去泄漏 / schema / 诊断）。
- A3 若仍 reject，把每轮 `decision` 与反馈摘要列进 HANDOFF，不得只写"未达 PASS"。

八、交付物与验收

- ConcIR：`schema` 子命令；诊断 FQN + doom_state；codegen channel/多模块；conform 覆盖率口径；toolchain 钉死；`cargo test` ≥234 全过；提交完整可 checkout 编译。
- ConcPlanVerify：`normalize.py`、v2 prompt、停滞/局部修补、抽取 oracle、behavior 测试 ×6、`--sanitize` 与 lint 测试、conformance-v2、repair-smoke-v2、PROTOCOL 偏差登记（D-7 去泄漏、D-8 规范化、D-9 抽取 oracle）、HANDOFF Round e（含 R-1..R-8 对照表、conformance-v2 表、repair-smoke v1→v2 对照表、A3 每轮决策、未完成项）。全部 commit。
- 验收线：去泄漏 lint 通过；smoke-d bare_wait v3 原文离线走新链路 = PASS；conformance-v2 四个旧用例仍 100% 且覆盖率按 `(function,sid)` 为 100%；`worker_with_payload` 的骨架臂与 `A3_free` 都有数字（哪个高都如实记）；repair-smoke-v2 中每个 Rust 臂的 accept 都有三列 oracle 原始值，`inconclusive` 比例写进 HANDOFF；至少一个 Rust 臂 false_accept 或 A3 accept 是可测的。
- 不做：多模型、全量 batch、论文正文、Lockbud 调参、放宽 conform 规则。偏离本 prompt 的地方在 HANDOFF 列为偏差并说明。
