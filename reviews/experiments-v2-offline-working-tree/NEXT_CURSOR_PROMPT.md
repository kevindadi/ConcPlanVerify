本轮目标：修掉 experiments-v2 离线交付的复核问题；**按当前 ConcIR 的实际能力矩阵重建基准**（放弃论文 Table 1 的 P1–P9 编号，旧 cir2cvn 的 FnSummary/三值守卫/RwLock 等能力已不存在，而模块、scope/bound、有容量 channel、精确 condvar 等待集、计数信号量、有界数据域、safety/AG EF 性质等新能力论文未覆盖）；补齐终审 oracle；接通 Rust 臂 prompt 与 live 入口；最后只在少数就绪任务上跑一个 DeepSeek Flash 冒烟批次验证多臂链路。不做全量 live、不做多模型、不写论文。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-offline-working-tree/REVIEW.md
/Users/kevin/local-repos/ConcPlanVerify/experiments/EXPERIMENTS_V2_PROTOCOL.md
/Users/kevin/local-repos/ConcPlanVerify/experiments/EXPERIMENTS_V2_HANDOFF.md
/Users/kevin/local-repos/ConcPlanVerify/docs/REPOSITORY_BOUNDARIES.md
/Users/kevin/local-repos/ConcIR/doc/backend-design.md（§1 支持子集、§3 语义、§6 contract 与性质）
/Users/kevin/local-repos/ConcIR/doc/backend-usage.md（支持矩阵）
/Users/kevin/local-repos/ConcIR/src/explore/contract.rs（PropertySpec / PreservedSpec / PredicateSpec 的全部形状）
/Users/kevin/local-repos/ConcIR/tests/repro_bench、repro_round2..5、examples/（已有的合法模型与 contract，可作为基准种子）

仓库边界不变：ConcIR 只有工具代码；ConcPlanVerify 放 prompt/基准/数据/审阅；论文目录只有 tex。本轮所有交付进 ConcPlanVerify。

一、先恢复 ConcIR 可编译状态（REVIEW P1-3）

1. `git -C /Users/kevin/local-repos/ConcIR status`。当前未提交：`Cargo.toml` edition 2021→2024、`src/ast.rs` 一处 `ref` 修复、根目录文件名带前导空格的 `" rust-toolchain.toml"`、迁移产生的 `scripts/`、`doc/todo.md` 删除与 `.gitignore` 变更。
2. edition：保留 2024 并补齐 `src/validate/control.rs`、`src/validate/types.rs` 中 `ref` 绑定错误（只去掉多余 `ref`，不改逻辑）；若任何测试行为变化则回退 2021 并报告。二选一并写明理由。
3. 删除带空格的 `" rust-toolchain.toml"`。若需固定 toolchain，新建正确命名的 `rust-toolchain.toml`，用当前能通过 229 项测试的 nightly，并在 `doc/backend-usage.md` 记录。
4. `INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast` 229 通过；`cargo build --release --offline --bin concir-backend` 产出新 binary，记录 sha256；此后所有实验只用新 binary。
5. 给出 ConcIR 建议 commit 分组（迁移删除 / edition 修复），不代用户提交。

二、修复 harness 证据缺口（REVIEW P1-1、P1-2、P2-3、P2-4、P2-5）

1. `rust_arm._run`：每次工具调用在运行目录外的 `calls/<seq>-<tool>/` 落盘 `argv.json`、`env.json`（只记录 `MIRIFLAGS`、`CARGO_*`、`RUST_BACKTRACE`）、`stdout.txt`、`stderr.txt`、`exit.txt`、`wall_ms`；`ToolRun.as_dict` 输出路径与哈希；DETECTION/ARMS 记录引用路径。
2. 行为测试：解析 `cargo test` 输出 `test result: ok. N passed`；`N == 0` 或解析失败 → `behavior_test_ok = None`、`reason = "no_tests"`；exit≠0 → `False` 并保存输出。补回归：零测试项目不能得 `True`。
3. miri：exit≠0 且未匹配 deadlock/data race 文本 → `status = "tool_error"`，与 `detected=[]` 区分。样本量：探测本机 miri 是否支持 `-Zmiri-many-seeds=0..64`，支持则新增该模式，否则循环 seed 0..63；原 5 组冻结表继续跑并单列；新增部分在 PROTOCOL 登记为偏差 D-1（理由：单次 ≈0.2 s，N=5 不足以支撑漏报结论）。
4. `revision_workflow` / `arms.py`：A3 家族每轮写入 `rounds[]`（请求哈希、tokens 或 null、LLM 墙钟、工具墙钟、工具输出哈希、决策、反馈哈希）。补回归：scripted 三轮 FAIL→静态错误→PASS 的记录长度为 3。
5. 72 项旧测试全部保持通过。

三、按能力矩阵重建基准（替换 P1–P9）

3.1 目录与命名

`benchmarks/patterns/` 改为 `benchmarks/families/<family>/<case>/`。旧 `patterns/P1..P9` 目录整体移入 `benchmarks/legacy-paper-patterns/`（含其 `legacy_brief.md` 与 Rust 参考程序），MANIFEST 中标 `status: legacy`，不参与新实验；其中 P1/P4 已验证的 CIR 与 contract 可被新用例引用复用，但需在 `provenance` 写明来源（P1 buggy CIR 是 `deepseek-flash-pilot-v1/t2_abba` 的 LLM 生成模型，sha `d2d4958b…`，必须声明为 `llm_generated` 或由人重写）。

每个 case 目录：`spec.md`（意图设计，不指出缺陷位置）、`buggy.cir.json` + `fixed.cir.json`（缺陷用例）或 `correct.cir.json`（正确用例）、`contract.json`、`ground_truth.json`（defect family、涉及资源/语句 FQN+sid、预期 outcome、预期修复类别、`provenance`）、`rust/{buggy.rs,fixed.rs}` 或 `rust/correct.rs`（std-only 单文件参考程序，用于 Rust 臂与 Track D）、`rust/tests/behavior.rs`（真实存在的行为测试：5 s 超时守护，fixed/correct 必须通过并断言预期结果，buggy 允许失败）。

3.2 族与最小用例集（每族至少列出的用例都要做；名称固定，后续记录以 `<family>/<case>` 引用）

- `lock-order/`：`abba_2lock`、`cycle_3lock`、`cross_module_cycle`（两个模块、`requires.resources`）、`two_independent_cycles`（种子：`tests/repro_bench/two_cycles.json`）、`call_chain_hold`（`call` 内持锁形成跨函数链，callee 带 `requires_held`）、`partial_deadlock_bystander`（两任务互锁 + 旁观任务持续完成；contract 用 `always_reachable` 表达两任务完成；预登记 `deadlock_free` PASS 而 `always_reachable` FAIL）。
- `condvar/`：`lost_wakeup_notify_before_wait`（预登记 FAIL）、`bare_wait_no_predicate_loop`、`notify_one_multi_waiter_wrong_pick`（种子：`repro_round2/notify_choice_false_pass.json`）、`notify_all_vs_one`（同一设计两版，一版 FAIL 一版 PASS）、`same_cv_different_locks`（种子：`repro_round3/r2_multi_lock_cv.json`）、`dual_cv_cross_wait`。P2 类 contract 加 `var_eq ready=true` 的 preserved。
- `channel/`：`rendezvous_both_send`（capacity 0）、`send_while_holding_mutex`（种子：`repro_bench/channel_deadlock.json`）、`bounded_buffer_full_holding_lock`（capacity 1–2）、`producers_k_consumer_1`（`scope` + `bound=k`，capacity n，正确/错误两版）、`pipeline_two_stages`。contract 可用 `channel_empty` / `channel_at_least` 谓词。
- `semaphore/`：`throttle_n_permits`（正确）、`permit_leak`（某路径 acquire 不 release；`always_reachable` 失败）、`sem_plus_mutex_order`。
- `atomic-data/`：`cas_retry_bounded`（有界 Int 自旋，正确；contract 含 `safety` 不变式）、`atomic_handshake_flag`（正确/错误两版，错误版用 `unreachable` 坏状态或 `safety` 抓）、`bounded_counter_invariant`（`safety`：计数不越界）、`data_dependent_lock_path`（`branch`/`switch` 依据有界 Int/Enum 选择加锁顺序，只有某些值下死锁——需要精确数据域，正确版与错误版都要）。
- `structure/`：`scope_bound_k_workers`（k=2..4 版本）、`spawn_join_loop_finite`（`goto` 回边 + 有界计数）、`nested_scope`、`finite_call_loop`（种子：`repro_round2/finite_call_loop.json`）。
- `boundary/`（负例，证明不会假 PASS）：`rwlock_unsupported`（预登记 UNSUPPORTED，可复用 real-cases/dashmap-369）、`async_select_unsupported`（预登记 UNSUPPORTED；种子：`examples/async_workers.json`）、`unbounded_int_unknown`（预登记 UNKNOWN，`max_states` 有限）。这些不需要 Rust 参考程序与行为测试。
- `real-cases/`：保留 `rmw-zenoh-998`；再从 GitHub 收集 ≥2 个有明确复现描述的 Rust 死锁/丢失唤醒 issue 做 CIR 归约，记录来源 URL、归约假设、与源码的对应关系；未能验证的如实标 `hypothesis`。

3.3 contract 口径

- 基础：`deadlock_free` + 每个任务 `function_completed` 的 `preserved.reachable`。
- 完成类缺陷（partial deadlock、permit leak）用 `always_reachable`；数据类用 `safety` / `unreachable`；通道类可加 `channel_empty`。
- `bounds` 统一（沿用 pilot 的 8/8/20000/64/256），`boundary/unbounded_int_unknown` 例外并注明。`allowed_scope` 只开 `allow_lock_reorder`。
- 目标由人写并冻结，不由 LLM 推导；这与论文"LLM 可推导业务目标"的表述不同，在 PROTOCOL 中明确为设计变化。

3.4 验收

- 每个 CIR 过 `check`/`support`；所有用例 petri 与 interp 差分 outcome 一致；预登记结果（每个用例写在 `ground_truth.json`）与实际逐项比对，偏差如实报告；判定为后端缺陷的走 Rust 回归修复并记录。
- `benchmarks/build_patterns.py` 改为 `benchmarks/build_families.py`：校验目录结构、运行 check/support/explore、重建 `MANIFEST.json`（每文件 sha256、每用例 status/provenance/预登记/实际），失败即退出非零。
- 每个 Rust 参考程序 `cargo build` 通过，行为测试真实存在（N ≥ 1）。
- 输出 `benchmarks/FAMILIES.md`：族 × 用例 × 覆盖的原语/性质矩阵，标出哪些能力是论文未覆盖的新能力。

四、Track D 重跑

用新 binary、新 miri 配置、原始输出归档，对全部非 boundary 用例重跑 `detection`（boundary 族只跑 ConcIR，记录 UNSUPPORTED/UNKNOWN），输出 `experiments/detection-v2/`（不覆盖 detection-v1）。lockbud：按其 README 尝试安装（记录 commit、toolchain 与结果），成功则加入并给出全部用例结果，失败记 `unavailable` 与原因，不替代。DETECTION 表按族汇总：ConcIR 完整分辨率、miri 检出率（按 seed 数）、lockbud 检出率、各工具墙钟。

五、Rust 臂 prompt 与 live 接线

1. `prompts/rust_generation_v1.md`（从 spec 生成单文件 std-only Rust，`fn main`，无外部 crate，不得用 sleep 规避竞争）、`prompts/rust_self_review_v1.md`（A1：检查并修复并发缺陷，输出完整程序或明确回答 `NO_ISSUES`）、`prompts/rust_tool_feedback_v1.md`（A2：附 cargo/miri/lockbud 原始诊断，截断 8 KiB）。sha256 进 manifest。
2. `arms.py` live 入口复用 `live.py` 的 Flash 客户端、预算、模型身份检查与脱敏记录；`assert_protocol_confirmed` 以冒烟 PROTOCOL 文件 sha256 作为确认凭据。
3. 终审 oracle 按 B4 四项分列；Rust 产物的 `bug_present` 写成可执行检查（miri 多 seed 检出、行为测试超时）+ 人工对照字段，不合成单一分数。CIR 产物用 `explore` 的 counterexample/blocked facts 与 `ground_truth.json` 的资源/语句对照。
4. A3 的 fidelity 模板改为按 `ground_truth.json` 中的资源/语句结构逐用例定义；模板外结构标 `unknown`。

六、Flash 冒烟批次（小、只验证链路）

- 目录 `experiments/flash-arms-smoke-v1/`，先冻结 `PROTOCOL.md`（引用主协议，登记偏差与本轮用例选择理由）。
- 任务 3 个，覆盖三种反馈类型：`lock-order/abba_2lock`（死锁，可被单补丁修复）、`condvar/lost_wakeup_notify_before_wait`（死锁，不能被 swap 修复，检验 A3 整份修订）、`lock-order/partial_deadlock_bystander`（`always_reachable` 失败，检验目标层反馈）。
- 臂：A0、A1、A2、A3、A3p（仅 abba）、A3_tool_repair；消融臂不在冒烟批次。
- K=4；全批 HTTP ≤ 48；墙钟 ≤ 60 分钟；`budget.json` 共享，重启不重置。模型规则同主协议（Flash only、thinking disabled、max_tokens 4096、temperature 0、timeout ≤ 90 s、SDK max_retries 0、≤1 次 transient 重试、响应 model 不一致停批）。key 只从 `.env` 读，不落盘，交付前扫描证据。
- 输出 `SUMMARY.md/json`：每臂×任务的接受/终审四项/轮次/tokens/LLM 墙钟/工具墙钟；A2 与 A3 每轮工具耗时对照。首轮即接受的如实写"反馈未触发"。

七、交付后停止

交付：ConcIR 状态说明与 commit 建议；`benchmarks/families/` 全部 ready + `FAMILIES.md` + 重建的 `MANIFEST.json`；`benchmarks/legacy-paper-patterns/`；`experiments/detection-v2/`；`experiments/flash-arms-smoke-v1/`；更新的 `EXPERIMENTS_V2_PROTOCOL.md`（B1 任务集改为能力族、偏差登记、"目标由人写"的设计变化）；`experiments/EXPERIMENTS_V2_HANDOFF.md` 追加本轮章节：对 REVIEW.md 各条的逐项回归结果、二进制/miri/lockbud/rustc 版本、复现命令、每族预登记 vs 实际、预算使用与停止原因。

结论限定为"harness 证据缺口已修、按当前能力矩阵的基准就绪并经工具验证、多臂链路在 3 个任务上跑通"。全量 live 留待下一轮。完成后停止，等待独立审阅。
