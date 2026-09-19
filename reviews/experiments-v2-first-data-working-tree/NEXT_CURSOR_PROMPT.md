本轮目标：**先把契约补强到能拦住空洞修复，再把现有 12 格 Rust 数据补完整**；不加新任务。顺序：(1) ConcIR 新增 `holds_all` 谓词与互斥不变式，重写全部 buggy 用例的 contract，离线复检 v2 中 A3 的三个接受版；(2) 修抽取 oracle 的三个琐碎错配并对盘上 12 个候选补跑抽取（≤24 请求）；(3) 行为 oracle 改为需求层可观测终态，改掉所有 "bug fixed" 措辞，SUMMARY 展示原始状态计数；(4) `conform` 支持 rendezvous 复合步，channel 三个用例 + real case 进一致性表；(5) `A3_free` 补 `finish()` 重跑；(6) 如果一至五全部完成，A3 用新契约在 partial_deadlock 上重跑一次（≤8 请求），看能否得到保设计的修复。不做全量 batch、不做多模型、不写论文、不调 Lockbud。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-first-data-working-tree/REVIEW.md（F-1..F-6）
/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/lock-order__partial_deadlock_bystander/A3_ours_revision/*/revision-3.cir.json 与 benchmarks/families/lock-order/partial_deadlock_bystander/fixed.cir.json（F-1 的对照实物）
/Users/kevin/local-repos/ConcIR/src/explore/contract.rs、src/interp/state.rs、src/petri/、doc/backend-usage.md 的 conform 一节
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{extract,normalize,conformance,arms}.py、prompts/rust_to_cir_extract_v1.md

仓库边界不变；每节一个 commit，HANDOFF 记 hash；每个实验 JSON 带 `binary_sha256`/`git_rev`。

一、契约补强与空洞修复复检（F-1）

1. ConcIR `PredicateSpec` 新增 `HoldsAll { function, resources: [FQN] }`：存在某个正在执行 `function`（任意帧深度）的线程，同时持有全部 `resources`（mutex 持有 / semaphore 至少 1 permit 计入；其他类型报 `unsupported`）。interp 与 petri 都实现，`engine_agreement` 覆盖。
2. `PropertySpec::Safety` 新增不变式 `MutexExclusive { resource }`（同一时刻至多一个持有者——用于 codegen/抽取模型的自检）与 `NeverHoldsAll { function, resources }`（禁止某函数同时持有某组资源，用于表达"不得嵌套"的设计）。
3. `schema` 子命令与 v2 prompt 的 contract 表同步；`doc/backend-design.md` §6 补语义。
4. 重写全部 19 个 buggy 用例的 `contract.json`：每个用例的 `preserved` 必须包含至少一条**设计意图**谓词（`holds_all` / `var_eq` 终态 / `function_completed_at_least` 等），并在 `spec.md` 用一句话说明该条为什么代表意图。`partial_deadlock_bystander`：`preserved reachable holds_all(main::a,[main::a,main::b])` 与 b 的对称项。`build_families.py`：人工 `fixed` 必须在新契约下 PASS，`buggy` 必须仍 FAIL，否则不得进 MANIFEST。
5. **离线复检**（无 LLM）：对 smoke-v1、smoke-v2 中所有 A3 接受版 CIR（含 smoke-c 的 abba、smoke-d 的 abba、v2 的三个）用新契约 explore，产出 `experiments/contract-strength-v1/CONTRACT_STRENGTH.{json,md}`：每个接受版在旧/新契约下的 outcome、被哪条 preserved 拒绝。这是本轮主结果。

二、抽取 oracle（F-2）

1. `normalize.py`：顶层未知字段（`contract`、`description`、`notes`…）剥离并记录 `rule: drop_top_level`；`extract.py` 的解析回退：从 ```json 围栏中取对象；不是 `{cir,rust}` 时发一次格式反馈重试（计入预算）。
2. `prompts/rust_to_cir_extract_v2.md`：与 `doc/backend-usage.md` conform 规则一致——lock/acquire/wait/channel 的 `ev` 在调用**返回后**，unlock/notify/release/spawn/scope/join 在语句处；末尾必须 `cir_trace::finish()`；不要输出 contract；tag 规则照 codegen。
3. 对 v2 批次盘上 12 个候选 Rust 补跑抽取（≤24 请求），写回同一 `SUMMARY.json` 的 `oracle.model`（保留旧值为 `oracle.model_v1`）。目标：`extract_validated` 比例 >0；仍未验证的写明原因分布。

三、行为 oracle 与措辞（F-3、F-4）

1. 全部 `repair_input/requirements.txt` 追加一句可观测终态（如 "On completion the program prints exactly one line `DONE <k>=<v> ...`"，k/v 由用例决定：`ready=true`、`permits=N`、`a=1 b=1`）。看门狗解析该行；`behavior_status` 取值 `terminated_ok / terminated_wrong_state / hang / no_output / no_build`。词表 lint 继续通过。
2. HANDOFF、SUMMARY、PROTOCOL 中所有 "bug fixed" 改为状态原文；`terminated` 且 `oracle.model` 未验证 → 显示 `terminated, unverified`。
3. `SUMMARY.md`：`oracle.miri` 列改为状态计数（`clean 16` / `timeout 16` / `thread_leak 3, clean 13`）；`oracle.model` 不截断，长文本放脚注；`bug_present`/`false_accept` 旁标注依据列。
4. v2 批次的 12 格用盘上候选**重跑行为 oracle**（无 LLM），更新表。

四、channel 一致性（F-5）

1. `conform`：`channel_send`/`channel_recv` 在 cap=0 时视为**复合步**：模型的一次 rendezvous 消费两个完成事件（send-done、recv-done），顺序任意，两事件之间允许其他线程的事件；cap≥1 时 send-done 对应入队步、recv-done 对应出队步。规则写进 `doc/backend-usage.md`，替换现在的"归到后到者"叙述。
2. `codegen` 的 channel 运行时与 `ev` 落点按该规则；`rendezvous_both_send(fixed)`、`bounded_backpressure(fixed)`、`send_while_holding_mutex`（fixed/correct）、`rmw-zenoh-998`（buggy 与若有的 fixed）进 `experiments/conformance-v4/`。目标 fixed/correct 全部 100%；达不到则给出首个 violation 的事件前缀与模型 frontier，判定是 codegen 还是 conform 的问题，不放宽规则。

五、`A3_free`（F-6）

自由生成 prompt 加硬性要求：必须 `cir_trace::finish()`、必须 `ev` 每个并发操作、按 conform 规则落点；1 次请求重跑 `worker_with_payload`，与骨架臂并列进 conformance-v4。

六、可选（一至五全部完成并提交后）

新契约下 A3 在 `partial_deadlock_bystander` 上重跑一次（K=4，≤8 请求）：记录是否得到保持 `holds_all` 的修复、轮次、每轮 decision；与 v2 的 v3 接受版并列。

七、交付物与验收

- ConcIR：`HoldsAll`/`MutexExclusive`/`NeverHoldsAll`、conform 复合步、schema/doc 同步、全部测试通过、`engine_agreement` 零分歧。
- ConcPlanVerify：19 个 contract 重写 + `spec.md` 意图说明、`contract-strength-v1`、抽取 v2 prompt 与补跑、行为 oracle 终态、SUMMARY 展示、conformance-v4、PROTOCOL 偏差（D-13 契约补强、D-14 终态可观测替代 behavior.rs、D-15 rendezvous 复合步）、HANDOFF Round g（F-1..F-6 对照表、契约强度表、12 格补全后的表、conformance-v4 表、每节 commit）。
- 验收线：`build_families.py` 在新契约下 19 个 buggy FAIL / 全部 fixed PASS；`CONTRACT_STRENGTH` 至少覆盖 5 个 A3 接受版并明确列出被拒的与拒因；抽取 `extract_validated` >0；12 格 `behavior_status` 均为新枚举；channel fixed/correct 用例 conformance 100% 或给出定位；`A3_free` 有非 missing 数据；HANDOFF 里无 "bug fixed" 字样。
- 做不完则停在当前节，已完成节全部提交，HANDOFF 写清停点。
