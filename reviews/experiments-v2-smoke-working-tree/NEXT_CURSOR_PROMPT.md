本轮目标：(1) 修掉冒烟批次暴露的 harness 缺陷并纠正 HANDOFF 的误述；(2) 实现**代码级后验证层（conformance）**：从已验证的 CIR 生成带 sid 标注的 Rust 骨架 → LLM 只填顺序空洞 → 静态 lint + 运行时轨迹回放到 ConcIR 参考解释器，证明生成代码的每一次观测执行都是模型的执行；(3) 把多臂主实验从"生成型"改为"修复型"，让缺陷由构造保证存在、false-accept 可测；(4) 每族补足 ≥2 个 buggy 用例；(5) 只在 3 个用例上跑一次 conformance 冒烟 + 一次修复型冒烟。不做全量 live、不做多模型、不写论文。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-smoke-working-tree/REVIEW.md（本轮全部问题与设计理由，§1 的 C-1..C-5 必修）
/Users/kevin/local-repos/ConcPlanVerify/experiments/EXPERIMENTS_V2_PROTOCOL.md、EXPERIMENTS_V2_HANDOFF.md
/Users/kevin/local-repos/ConcPlanVerify/benchmarks/FAMILIES.md、MANIFEST.json、build_families.py
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{arms,flash_smoke,rust_arm,revision_workflow,experiments_v2}.py
/Users/kevin/local-repos/ConcIR/doc/backend-design.md、backend-usage.md（支持矩阵、`run` 参考解释器）、src/interp/、src/main.rs

仓库边界不变：ConcIR 只放工具代码（本轮新增 `codegen`、`conform` 两个子命令和一个极小 `cir_trace` 运行时）；ConcPlanVerify 放 prompt/基准/数据/审阅；论文目录不动。

一、先修 harness（REVIEW §1 C-1..C-5、§2）

1. A1：解析 LLM 回复，去空白后等于 `NO_ISSUES` → `decision="self_no_issues"`，接受当前候选（accepted_round=该轮），**不再编译哨兵字串**；否则要求完整程序（必须含 `fn main`），不满足记 `format_error` 并把格式要求回送给下一轮。补回归测试：scripted 第 2 轮回 `NO_ISSUES` → accepted_round=2，rounds 长度 2。
2. A3 家族：`check` 返回 usage_error/解析错误（exit 2）时，**不终止**，把 stderr 的解析错误作为 `schema_error` 反馈回送 LLM，占用一轮；只有 provider 错误或 binary 不存在才 `tool_error`。同时在 CIR 生成 prompt 的 schema 摘要里把 `expr` 明示为字符串表达式并给一个 `write_shared` 例子。补回归：scripted 第 1 轮 expr 为对象 → 第 2 轮收到 schema 反馈 → PASS，rounds 长度 2。
3. `SUMMARY.{json,md}` 聚合：目录名与臂 id 统一（`A3p_ours_patch`、`A3_tool_repair` 各自独立目录），表中不得再出现"有结果但显示 None"。
4. A2 判定分列：记录 `build_ok`、`miri_green`、`lockbud_green`、`test_ok` 四列；PROTOCOL 增加 A2-m（build+miri）与 A2-ml（build+miri+lockbud）两档接受规则，冒烟批次两档都报。
5. `.gitignore`：加 `!experiments/**/*.txt`（或把归档改名 `.log`），让 `stdout/stderr/exit` 原始证据进仓库。已有批次补 `git add`。
6. `condvar/same_cv_different_locks`：去掉 `buggy` 标签（改 `variant_a/variant_b` 或只保留 correct），重建 MANIFEST；detection 统计中不再把 PASS 的用例算作 buggy。
7. ConcIR：`rust-toolchain.toml` 钉到 `nightly-2026-09-03`；把当前未提交改动分成 `chore: pin toolchain` 与 `chore: drop unused thiserror and stray comment` 两组，给出建议命令，不代提交。
8. 修正 HANDOFF：删除"A0/A2 accepted the lost wakeup, Miri/Lockbud miss it"与"A3 rejected (revision did not reach PASS)"两句，改为 REVIEW C-2/C-3 的事实描述；冒烟批次目录不删，标注 `superseded_by`。

二、一致性层（后验证）——本轮核心

A. ConcIR 侧（Rust）

1. `concir-backend codegen <program.json> --out <dir>`：从 CIR 生成 **std-only** Cargo 项目：
   - 每个模块 → `mod <name>`；每个资源 → `Arc<Mutex<_>>`/`Condvar`/`std::sync::mpsc::sync_channel(cap)`/自实现计数信号量（`Mutex<u32>+Condvar`，≤30 行，放进 `cir_trace`）/`Atomic*`/受保护变量（按 `protection` 放进对应 Mutex 内部）。
   - 每条语句 → 一段 Rust，行尾 `// @cir <sid>`；并发语句前插 `cir_trace::ev(<thread_tag>, "<sid>")`。`spawn/scope/join/call/return/goto/branch/switch` 按 CIR 语义生成控制流（goto 用 `loop + 'label` 或状态机，二选一并在 doc 写明）；`assign_local`/`read_shared`/`write_shared` 的 `expr` 能直译则直译，否则该处留 `// HOLE(<id>) expected: <type>` 并生成可编译占位（`Default::default()`）。
   - nobody 函数 → 函数签名 + `// HOLE(<fn>)` 体。
   - 线程 tag：主线程 `t0`；每个 `spawn/scope` 子函数在生成时由父线程按静态 sid 分配 tag `t<sid>`，写死在代码中并传入 `ev`。
   - 同时输出 `codegen.json`：sid→(文件,行号)、hole 列表、tag 表。
2. `cir_trace` 运行时（`<out>/src/cir_trace.rs`，≤80 行）：`ev(tag,sid)` 追加 `{"t":tag,"sid":sid}` 到全局 `Mutex<Vec<_>>`，进程退出时写 `CIR_TRACE_OUT` 指定的 jsonl；不用任何外部 crate。
3. `concir-backend conform <program.json> <trace.jsonl> [--contract]`：用参考解释器回放。规则：按事件顺序，取 `t` 对应线程的当前帧位置，检查 `sid` 是否是该线程此刻 enabled 的步（含 `condvar_wait` 的两段：进入等待/被唤醒后重取锁，两种都要能对应）；enabled 则执行，否则输出 `{"status":"violation","event_index":k,"expected":[...enabled sids],"got":sid}`。全部事件消费完 → `{"status":"conformant","events":n,"coverage":{"sids_seen":m,"sids_total":M}}`。轨迹中出现骨架外 sid → `unknown_sid`。
4. 测试：对 `benchmarks/families` 中每个 `fixed/correct` CIR，`codegen` 后 `cargo build` 必须通过（无 HOLE 时直接可跑）；对至少 abba_fixed、lost_wakeup_fixed、permit_leak_fixed、scope_bound_k_workers 四个用例，原生运行 20 次产生轨迹，`conform` 全部 `conformant`；再构造一条人为篡改的轨迹（交换两个 sid）验证 `violation`。`cargo test` 总数在 229 基础上只增不减。

B. ConcPlanVerify 侧（Python，新增 `python/cir_workflow/conformance.py`）

1. `fill_holes(skeleton_dir, requirements, llm)`：只把 HOLE 段与其上下 8 行、以及 requirements 交给 LLM；prompt 明示"只能写顺序局部计算，不得引入任何同步原语/线程/unsafe/新的 use"。回复解析为 `{hole_id: code}`。
2. `lint_filled(skeleton_dir, filled_dir)`：行级 diff，HOLE 以外不得有任何改动；HOLE 内正则禁列：`std::sync|std::thread|Mutex|Condvar|RwLock|channel|Semaphore|spawn|scope\(|\.lock\(|\.wait|notify|\.send\(|\.recv\(|Atomic|unsafe|cir_trace`；每个 sid 恰出现一次。输出 `lint.json`（通过/违规行）。
3. `collect_traces(filled_dir, native_runs=50, miri_seeds="0..64")`：原生运行 N 次 + `cargo miri run` many-seeds，每次一条 jsonl，超时（10 s）记为 `hang`（这本身就是 deadlock 观测，单列）。全部原始输出照 `calls/` 归档。
4. `conform_all(program, traces)` → `CONFORMANCE.json`：`traces_total / conformant / violation / hang / unknown_sid`，sid 覆盖率，lint 结果，全部 ConcIR 输出哈希。
5. 对照臂 `A3_free`：同一份已验证 CIR 交给 LLM 自由生成完整 Rust，要求它自己在每个并发操作前插 `cir_trace::ev(tag, sid)`；不做骨架 lint，只做 trace 回放。这是"为什么需要骨架"的消融，预期 conformance 低于骨架臂；如果不低，也照实记。
6. 单元测试：lint 对 HOLE 外改动、HOLE 内 `lock()`、缺 sid、重复 sid 四种情形各一条；`conform_all` 对 scripted 轨迹的计数正确。

三、修复型多臂主实验（替代生成型）

1. 基准：每个有 `buggy` 的族用例新增 `repair_task.json`：`{requirements, input_rust: rust/buggy.rs, input_cir: buggy.json, contract, ground_truth}`。当前 buggy 用例不足的族补齐到 ≥2：channel 加 `bounded_backpressure_lock_held`（持锁 send 满通道）；atomic-data 加 `counter_overflow_safety`（bounded Int 越界，safety FAIL）；semaphore 加 `acquire_twice_no_release`；condvar 加 `bare_wait_no_predicate`；structure 加 `nested_scope_lock_order`（buggy+fixed）。每个新用例 `spec.md/buggy.json/fixed.json/contract.json/ground_truth.json/rust/{buggy,fixed}.rs/rust/tests/behavior.rs`，`build_families.py` 通过且 petri==interp，MANIFEST 重建。
2. 臂：A1（自审修复 Rust）、A2-m、A2-ml（工具反馈修复 Rust）、A3（结构化诊断修复 CIR → codegen → 填洞 → conformance）。A0 在修复型任务里退化为"直接输出修复版"，保留作下界。
3. 终审 oracle（三者分列，绝不合成一个布尔）：
   - `oracle.behavior`：`rust/tests/behavior.rs` 带 10 s 超时，超时=挂起；
   - `oracle.miri64`：miri many-seeds 检出/未检出/tool_error；
   - `oracle.model`：对 Rust 臂输出，用"抽取 prompt"让 LLM 把 Rust 抽成 CIR，再用 conformance（LLM 在抽取时同时给出 sid 注释版 Rust）验证抽取是否与代码一致；**只有 conformant 的抽取才允许其 explore 结论进表**，否则该格记 `extract_unverified`。对 A3 输出，模型判定直接来自被接受的 CIR。
   - false-accept = 臂接受且 `oracle.behavior=hang` 或 `oracle.model=FAIL(conformant)`；behavior-loss = 接受但 preserved 目标不可达或 behavior 测试失败。
4. PROTOCOL 增补 §"修复型主实验"与 §"一致性层"，登记为对原 v2 协议的偏差 D-2/D-3 并写明理由（REVIEW §3）。旧的生成型冒烟结果保留为附表，注明 REVIEW C-1..C-3 的失效项。

四、冒烟（受预算门控，先离线全过再 live）

1. conformance 冒烟（无 LLM 也能跑的部分先跑）：abba_fixed、lost_wakeup_fixed、permit_leak_fixed 三个 CIR → codegen → 无 HOLE 直接构建 → 50 次原生 + miri 0..64 轨迹 → conform；输出 `experiments/conformance-v1/CONFORMANCE.{json,md}`。然后对 `scope_bound_k_workers`（有 HOLE）跑一次 Flash 填洞 + lint + conform，并跑一次 `A3_free` 对照。预算 ≤10 次请求。
2. 修复型冒烟：abba、partial_deadlock_bystander、lost_wakeup（用 `bare_wait_no_predicate` 或原 buggy）三个任务 × {A1, A2-m, A2-ml, A3}，K=4，预算 ≤48 次请求；输出 `experiments/flash-repair-smoke-v1/run-*/SUMMARY.{json,md}`，含三列 oracle 与 false-accept 判定。
3. 每次 live 前打印协议 sha 与预算，超预算立即停并记 stop_reason。

五、交付物与验收

- ConcIR：`codegen`、`conform` 子命令 + `cir_trace`；`doc/backend-usage.md` 新增两节；`cargo test` ≥229 且全过；新 release binary 的 sha256。
- ConcPlanVerify：`conformance.py` 与测试；修好的 A1/A3/SUMMARY/A2 分列；`.gitignore` 修正并补 add 证据；新基准用例与重建的 MANIFEST；PROTOCOL 增补；`experiments/conformance-v1/` 与 `experiments/flash-repair-smoke-v1/`；HANDOFF 追加"Round d"一节，含：修复项对照表、conformance 结果表（每用例 traces/conformant/violation/hang/coverage）、修复型冒烟表（每任务×臂：accepted、轮次、tokens、三列 oracle、false-accept）、偏差登记、未完成项。
- 验收线：C-1..C-5 全部有回归测试；三个无 HOLE 用例 conformance = 100%（若不是，先怀疑 `conform` 的 condvar 两段处理，不得放宽规则）；篡改轨迹能报 violation；修复型冒烟中至少一个 Rust 臂出现可测的 false-accept 或 A3 出现可测的接受（哪一种都算有效数据）；全部 Python 测试通过。
- 不做：多模型、全量 batch、论文正文、对 Lockbud 结果的任何过滤或调参。任何偏离本 prompt 的地方在 HANDOFF 中列为偏差并说明。
