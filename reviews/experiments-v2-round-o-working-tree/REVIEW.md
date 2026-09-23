# Review — Round o（freeze-5，2026-09-23）

范围：ConcPV HEAD `a0960a93`（tag `experiments-v2-freeze-5` = `f6606f64`，其后一个未打 tag 的提交），ConcIR `43168bb`（`concir-freeze-4`）。数据：`experiments/flash-gen-main-v2/run-20260923T005232`（312 格 / 680 请求）、`gen-llmcode-smoke-v1`、`gen-model-probe-v2`、`experiments/tables/*.tex`、论文 `paper.tex`（1513 行，可编译）。

## 结论（先说）

上一轮的架构修正已经落地：CIR 阶段很强（模型 PASS 59/72，v1 为 25/72，INVALID 因 v3.1 实体命名基本消失），LLM 从已验证 CIR 写出的 Rust 100% 可编译（smoke 45/45，v1 工具 codegen 只有 22/45）。**但代码阶段 31/59 通过 conform+monitor，G3 v2 接受率 0.431，低于工具 codegen 消融 0.833，触发了 §3 的止损条件。**

我逐格复现了失败格的 conform 违规（用 `concir-backend conform --op-resource` 重放存档 trace），**28 个失败格里我抽查的全部样本都是 harness 编码缺口，不是 LLM 对设计不忠实**（见 O-1）。这意味着：(a) 当前 RQ2 表里"conform 拒绝了 30/59 格"这一数字目前不能作为"后验证抓到 LLM 偏离"的证据；(b) 修 4 个工具侧问题后，代码阶段应显著回升——LLM 写出的 Rust 本身（我读了 condvar/semaphore/atomic 三个失败样本）是教科书级正确实现。这是好消息：瓶颈在我们的检查器编码，不在方法。

另有一个统计定义问题（O-2）使 G3 的 `RF_all` 被高估约 2.3 倍，必须在下一轮修正并重生成表。

## O-1（P0）代码阶段 28/59 失败 = harness 编码缺口，非 LLM 偏离

所有失败格的模式一致：3 轮 `post_verify_fail`，conform `violation 32/32`，`behavior_ok=true`，`monitor_fail=[]`。三轮完全相同 → 反馈无信息量或问题不可由 LLM 修。重放确认了四类缺口：

| # | 任务样本 | conform 输出 | 根因 | 归属 |
|---|---|---|---|---|
| 1 | `condvar/bare_wait_no_predicate` rep0 | `no enabled model step matches mutex_unlock on main::m at event 1`，expected `[condvar_notify, mutex_lock]`，got `mutex_unlock` | `cir_trace::sync::Condvar::wait(guard)` 让 guard Drop 先发 `mutex_unlock`，唤醒后再发 `mutex_lock`，最后才发 `condvar_wait`；CIR 的 `wait` 是单步（隐含释放/重获）。LLM 代码是标准 `while !*ready { ready = cv.wait(ready) }` | **instrument v2 编码 bug** |
| 2 | `semaphore/permit_leak` rep0 | `no model statement matches mutex_lock on "mutex"`，expected `[]` | std 无 Semaphore，LLM 用 `struct Semaphore { Mutex<usize>, Condvar }` 实现；instrument 看到的是 mutex/condvar 事件，CIR 资源是 `Semaphore`，`mapping_to_cir` 按 kind/order 映射不到任何东西 | **harness 缺 semaphore 运行时**；基线臂同样受害（G0/G1/G2 在 semaphore 任务上 `unmapped` 12/54） |
| 3 | `atomic-data/counter_overflow_safety` rep0 | `no model statement matches mutex_lock on c_mutex0 (thread t2)` | CIR 里 `c` 是受 `m` 保护的 `var`；LLM 给 `c` 单独包了一个 `Mutex<i64>`。这是**真实的设计偏离**（多了一把锁），但反馈只说"no model statement matches mutex_lock on c_mutex0 (event 1)"，LLM 三轮都没理解；`mapping_to_cir` 对多余 mutex 无处理，也没在 prompt 约定"CIR var → 保护锁内的字段，不加锁" | 反馈质量 + prompt 约定缺失 |
| 4 | `channel/bounded_backpressure_lock_held` rep0 | `no model statement matches mutex_lock on main::m (thread t1)`，expected `[]` | 线程 `t1` 在模型里没有任何可用步 → `mapping_to_cir` 把 spawn 顺序 zip 到 CIR 模块顺序的 worker 上，LLM spawn 顺序不同/同一 worker 多实例即错位 | **线程对齐按顺序，不按名字** |

另外 `run_llmcode_from_cir` 只把 `first_violation.detail` 拼进反馈，`expected/got/thread` 不给 LLM，也不写进 `CELL.json`（所以 `conform violation kind 分布`——上一轮 §3 要求的产出——实际没有）。

失败家族分布（channel、condvar、semaphore、atomic-data 全挂；structure、lock-order 全过）与四类缺口一一对应。**判断：修完 1/2/4 + 改进 3 的反馈和约定后，代码阶段应接近 smoke 的 build 率。** 需要下一轮用零请求的离线重放先量化"缺口占比"，再活跑。

## O-2（P0）`RF_all` 定义漂移：G3 被高估

`gen_results.aggregate` 的 `rf = _mean([c.get("rf") for c in cells])`，`_mean` 丢弃 `None`。G3 v2 未接受格 `coverage=None`（`run_g3_v2` 只在接受时写 coverage），于是 G3 的 `RF_all` 只在 31 个接受格上求平均 → `RF_all == RF_acc == 0.670`。而 G0/G1/G2 未接受格仍有 monitor 覆盖值，`RF_all` 真的是全格均值。两种口径混在同一列。

按"未接受记 0"（上一轮 prompt §3 的明文定义）粗算：G0 ≈ 0.52、G1 ≈ 0.50、G2 ≈ 0.43、**G3 v2 ≈ 0.29**、G3_codegen ≈ 0.64。`RF_acc` 不受影响（G3 0.670 vs G0 0.531 仍成立）。论文 `gen_main.tex` 当前 G3 `RF_all 0.670` 是错的，必须重算。

## O-3（P1）G3 的 RF 是有界口径，论文措辞要对

`combined["coverage"]` 取自 `bounded_monitor.coverage(...)`，状态是 `PASS_bounded`。即四臂的 RF 都由 monitor 在最终代码上有界测得——这是公平的，但"穷尽"只对 CIR 阶段的契约 PASS 成立。论文 §RQ2 / 表注需写成：设计层契约穷尽 PASS（`cir_pass` 列），代码层 RF 有界（所有臂同口径），conform 连接两者。`gen_g3_stages.tex` 已有阶段拆分，措辞待核。

## O-4（P1）`_defect` 对 G3 v2 失效

`_defect` 读 `rec["conform"]`，v2 里在 `rec["code_stage"]["rounds"][-1]["conform"]`，恒为 0。因接受条件已含 conform PASS，结论不变，但定义应统一为 `accepted ∧ (hang ∨ monitor FAIL ∨ Miri detected)`，对所有臂一致。

## O-5（P1）三个 `unsupported` G3 格

`same_cv_different_locks` 3 rep 的 CIR 被 W1xx 判 UNSUPPORTED（一个 condvar 绑多把 mutex）。v3.1 已把需求改写为意图级，模型仍然选择了该结构。作为已知限制写入 PROTOCOL/论文 Threats，不需再改需求。

## O-6（P2）冻结后的提交

`a0960a93`（run_g3 toggle / run_g3_v2）在 `experiments-v2-freeze-5` 之后。若它只是把已用于跑批的代码补提交，下一轮 freeze-6 覆盖即可；若改了逻辑，需在 HANDOFF 说明主批次跑的是哪个版本。

## 已完成、且质量可用的部分

- Benchmark v3.1（实体命名进需求、same_cv 意图改写、无启发式对齐）；CIR 阶段 PASS 59/72。
- G3 v2 流程、G3_codegen 消融、smoke（build 45/45；23 个 codegen 编不过的 CIR 中 8 个已过 conform）。
- kimi-k3 探针 v2：G3 `RF_acc` 与 awp 11/24，与 Flash 趋势一致（同样受 O-1/O-2 影响，重算后仍可用作"方法不依赖单一模型"的证据）。
- 分档：Complex 档 G3 RF 0.91 vs G0 0.361 是当前最强信号；Simple 档 G3 < G0，要在论文里如实说（CIR 阶段的开销在简单任务上不划算）。
- 论文：§3.9"Code construction from a verified model"已按 LLM-写代码架构重写，RQ 结构（RQ1–RQ6）就位，可编译。

## 未完成

- 生成格的 agent-proxy 专家标注（rubric v3）与 ≥ 8 格人工抽样队列——未做。
- loom——未做，建议放弃（时间）。
- `HUMAN_DISAGREEMENT_EVIDENCE.md` 两条 `owner_verdict`、人工原因列——仍待用户。

## 对下一轮的判断

9 天到截稿。O-1/O-2 是工具侧与统计侧的 bug，修复成本 1–2 天，收益是把方法的主要指标从"止损"拉回。因此下一轮仍是"半天工具 + 一天重跑 + 其余全部写论文"，而不是直接进入写作。重跑只做代码阶段（复用 59 个已验证 CIR，零 CIR 请求），Flash ≤ 59×3 + kimi ≤ 24×3。
