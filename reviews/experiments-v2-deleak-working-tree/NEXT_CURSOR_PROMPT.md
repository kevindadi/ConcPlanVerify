本轮唯一目标：产出第一批**可采信的修复型对照数据**。顺序不可调换：(1) 修 ConcIR 双引擎分歧 N-1 与 Miri 分类 N-2；(2) 诊断可读性 R-3；(3) Rust 臂三列 oracle（behavior 测试、Miri many-seeds、抽取 oracle）；(4) 在洗净输入上重跑 repair-smoke-v2（同 3 任务）并跑 live 填洞 + `A3_free`；(5) channel 一致性闭合 N-3。多模块 codegen 若时间不够可延后，其余不可。不做全量 batch、不做多模型、不写论文、不调 Lockbud。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-deleak-working-tree/REVIEW.md（N-1..N-4 与剩余项表）
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-conformance-working-tree/NEXT_CURSOR_PROMPT.md 的 §四、§六、§七（本轮原样执行，下面只写变更点）
/Users/kevin/local-repos/ConcIR/src/{petri/,interp/,validate/structure.rs,explore/,conform.rs,codegen.rs}
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{rust_arm,arms,conformance,revision_workflow}.py

仓库边界不变。每一节完成即提交一次（ConcIR 与 ConcPlanVerify 各自），HANDOFF 里每节记 commit hash。

一、ConcIR 正确性（N-1、N-2 分类需要的接口、N-4）

1. N-1 语义统一：在 `doc/backend-design.md` §3 写明"控制流落出函数体末尾 = 隐式 `return`（无返回值）"。Petri 翻译为每个函数末尾补隐式 return 迁移；解释器行为保持。校验器新增 `E1xx FallOffEnd`（warning）：函数最后一条语句不是 `return`/无条件 `goto`/`switch` 全分支跳转。`worker_with_payload` 两引擎必须一致 PASS。
2. 一致性回归：新增 `tests/engine_agreement.rs`，遍历 `examples/`、`tests/**/**.json` 与 `ConcPlanVerify/benchmarks/families/**/*.cir.json`（路径通过环境变量传入，缺省跳过外部目录）对每个模型 + contract 跑 petri 与 interp，断言 outcome 与 complete 相同。任何分歧立即失败并打印用例名。
3. N-4：`concir-backend` usage 文本补 `codegen`/`conform`/`schema`；`explore`/`conform`/`codegen` 的 JSON 输出加 `binary_sha256`（自身可执行文件哈希）与 `git_rev`（编译期 `env!` 注入，无 git 时 `unknown`）。
4. `cargo test` 全过；`cargo build --release`；HANDOFF 记 `git rev-parse HEAD` 与 binary sha，二者必须来自同一棵树。

二、诊断可读性（R-3，原 §四 全部照做）

变更点：`doom_state` 的 `holds`/`waiting_on` 以 FQN 给出；对 `always_reachable`/`preserved` 失败，`counterexample` 给到"目标首次不可达"的最短前缀之后，再附 `doom_state`。模板化 `repair_hints` 至少覆盖三类：锁序环（列出环上每条边 `thread: holds → wants`）、等待无人通知（waiter 的 cv 与谁本应 notify）、目标被死锁阻断（哪些线程卡在哪条 sid）。回归：partial_deadlock_bystander buggy 的诊断里能读到 `main::a … holds [main::mtx_a] waiting_on mutex main::mtx_b` 与其对称项。

三、Rust 臂 oracle（原 §六 照做，以下为变更与补充）

1. N-2：`rust_arm` Miri 分类新增 `thread_leak`（匹配 `the main thread terminated without waiting for all remaining threads`），与 `deadlock`/`data_race`/`timeout`/`tool_error` 并列；`miri_green` 要求全部种子 `clean`；detection 与 SUMMARY 单列该档。回归：把 smoke-d 该 stderr 原文喂给分类器 → `thread_leak`。
2. Miri 进臂 oracle：每个候选跑 seeds 0..15（每种子 8 s 超时，超时记 `timeout` 且 `hang_suspect=true`），不再是单种子；many-seeds 若能用则替代循环，但输出必须能按种子拆开。
3. `behavior.rs` ×6：由 harness 注入到候选项目 `tests/`，不进 prompt；每个测试 10 s 超时 → `hang`。测试只断言"需求"层面（终止、终态），不依赖实现细节（不 grep 源码）。
4. 抽取 oracle：按原 §六.3；抽取 prompt 与 A3 的 v2 schema 表共用同一份 `concir-backend schema` 输出；抽取结果同样经 `normalize.py`。`extract_validated` 需要 20 native + 8 miri 种子全部 conformant。

四、Live（受预算门控，先离线全过）

1. `experiments/flash-repair-smoke-v2/`：任务 abba_2lock、partial_deadlock_bystander、bare_wait_no_predicate，全部用 `repair_input/`；臂 A0、A1、A2-m、A2-ml、A3；K=4；预算修复臂 ≤48、抽取 ≤24、总 ≤72。`SUMMARY.md` 含 v1→v2 对照表（accepted/round/tokens 变化 + 原因标注：去泄漏 / schema / 诊断），每格三列 oracle 原始值 + `false_accept`/`inconclusive`。A3 每轮 `decision` 与反馈摘要列进 HANDOFF。
2. `experiments/conformance-v3/`：`worker_with_payload`（N-1 修复后恢复为正常基准）跑 Flash 填洞 + lint + 50 native + 16 miri → conform；同时 `A3_free`；并列报 conformant/violation/coverage。预算 ≤6。conformance-v2 的 4 个用例用新 binary 重跑一遍进 v3（验证 binary_sha256 字段与结果绑定）。

五、channel 一致性（N-3）

规则写进 `doc/backend-usage.md` 的 conform 一节：`channel_send`/`channel_recv` 的 `ev` 在调用返回后发出，视为完成步；cap=0 时 send/recv 两个完成事件顺序任意，frontier 同时保留两种绑定。`conform` 与 `codegen` 按此修改；`channel/rendezvous_both_send(fixed)`、`channel/send_while_holding_mutex(fixed 或 correct)`、`real-cases/rmw-zenoh-998` 进 conformance-v3 表；有 violation 就如实记并分析是 codegen 还是 conform 的问题，不放宽规则。

六、可延后（仅当一至五全部完成且提交后）

多模块 codegen（`mod` + `pub(crate)` + FQN 引用），目标 `cross_module_cycle(fixed)` 可 codegen + build + conform。

七、交付物与验收

- ConcIR：N-1 修复 + `FallOffEnd` 诊断 + `engine_agreement` 测试；R-3 诊断；usage/`binary_sha256`/`git_rev`；channel conform 规则；全部测试通过；每节一个 commit。
- ConcPlanVerify：`thread_leak` 分类、many-seeds 臂 oracle、`behavior.rs` ×6 注入、抽取 oracle、repair-smoke-v2、conformance-v3、PROTOCOL 偏差登记（D-10 thread_leak、D-11 miri 16 种子、D-12 隐式 return 语义）、HANDOFF Round f（N-1..N-4 与剩余项对照表、v1→v2 对照表、conformance-v3 表、A3 每轮决策、每节 commit hash、未完成项）。
- 验收线：`engine_agreement` 对全部基准零分歧；partial_deadlock 诊断含对称的 holds/waiting_on；smoke-d 的 thread_leak stderr 被正确分类；repair-smoke-v2 每个 Rust 臂 accept 格都有三列原始值且 `inconclusive` 比例写进 HANDOFF；`worker_with_payload` 骨架臂与 `A3_free` 都有数字；conformance-v3 的 JSON 带 `binary_sha256` 且与 HANDOFF 一致。
- 若某节做不完：停在那一节，把已完成节全部提交，HANDOFF 写清停在哪、为什么；不要跳到后面的节。
