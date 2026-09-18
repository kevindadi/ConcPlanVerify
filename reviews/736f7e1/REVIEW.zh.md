# ConcIR 后端首轮代码审查

- 审查提交：`736f7e1d4c46d44b6baabff5d6a116dd253f95be`。
- 日期：2026-09-17。
- 审查范围：新增语义、解释器、Petri 执行器、探索、契约、修复及相关测试；不是完整形式化证明或穷尽审计。
- 结论：模块与端到端示例已经落地，但目前存在错误 PASS、错误修复接受和引擎分歧，不能据此开始宣称后端正确性或采集最终论文结果。下一轮优先修正语义与验收，不接入 LLM。
- 没有修改 ConcIR/ConcPlanVerify 源代码；ConcIR 审查前后工作树干净。

## 执行记录

运行全套现有测试：

```sh
env INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-736f7e1-target cargo test --offline --all-targets --no-fail-fast
```

实际结果：137 passed、4 failed。新增后端相关测试 33 个均通过。失败均为原有 DOT snapshot 测试；这些失败此前已经存在，不归因于本轮后端。完整输出见 `baseline-tests.log`。

审查额外创建最小 CIR/contract/patch 反例，见 `repro/`。`.stdout` 和 `.stderr` 是被审查提交的原始运行结果，`observations.json` 汇总了主要字段。它们是审查证据，不是性能实验数据。

重新构建后可运行以下探针；脚本打印观察值，不把当前错误行为当成期望：

```sh
env CARGO_TARGET_DIR=/private/tmp/concir-next-review-target cargo build --offline --bin concir-backend
env CONCIR_REVIEW_BIN=/private/tmp/concir-next-review-target/debug/concir-backend python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/736f7e1/repro/probe.py
env CONCIR_REVIEW_BIN=/private/tmp/concir-next-review-target/debug/concir-backend python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/736f7e1/repro/probe_more.py
```

建议修复者先将反例迁入仓库成为有明确期望的 Rust 回归测试，原始证据目录保留不动。

## R1 — P1：句柄存在线程上，被嵌套调用覆盖，造成错误 PASS

位置：`src/interp/state.rs:119`、`src/interp/exec.rs:607`、`src/petri/exec.rs:1066` 附近。

`handles` 存在 ThreadState/NetThread 中，使用局部 handle 字符串作为键。被调用函数与调用者使用相同局部名字时会覆盖同一个条目，调用返回后没有恢复调用者绑定。

反例 `nested_handle_false_pass.json`：

1. main 创建 a，保存为 h。
2. helper 创建 b，也保存为自己局部的 h，join 后返回。
3. main 执行 join(h)，随后才 release(gate)。
4. a 必须 acquire(gate) 才能返回；b 立即返回。

正确绑定下，main 等 a，a 等 main 发 gate，必然死锁。实际两个引擎都返回 PASS。仅将 helper 的 h 改名 local_h，两个引擎就返回 FAIL；两个输入均通过静态验证。

修正：句柄名称解析和所有权归属于调用帧；具体子线程身份可放全局表。覆盖调用、递归调用、返回和 join 生命周期。局部 alpha-renaming 不应改变性质结果。

## R2 — P1：补丁后沿用旧语句下标，删除必需行为仍被接受

位置：`src/explore/contract.rs:330`、`src/repair/mod.rs:142`、`src/repair/mod.rs:172`。

`StatementReached` / `ScopeCompleted` 将 sid 解析成 body 下标。修复后重新 lower 程序，但直接复用对旧程序解析的 VerificationContract。删除或交换语句后，下标可能指向另一条语句。

反例：`stable_sid_contract.json` 要求 `main::t2.s3` 可达；`delete_patch.json` 删除 t2 的 s1、s3、s5。修复器返回 repaired/PASS，但 s3 已不存在。对 `deleted_program.json` 用同一原始 contract 重新解析，正确报错 `has no statement 's3'`。旧下标 2 此时指向 return，导致虚假保留目标。

修正：保存不可变、符号化的 ContractSpec，逐候选重新绑定；或者直接使用稳定语句身份。冻结的是规格意义，不是旧程序的数组下标。目标被删除必须拒绝，不能忽略、重定向或自动削弱。

## R3 — P1：文件候选绕过补丁操作权限

位置：`src/repair/mod.rs:112`、`src/repair/patch.rs:108`。

修复入口只检查函数范围；allow_lock_reorder 仅在自动枚举器中检查，allow_statement_delete 没有在统一验收处执行。

实际复现：

- `forbidden_delete_contract.json` 禁止删除，文件候选仍成功删除三条语句并返回 repaired。
- `forbidden_reorder_contract.json` 禁止锁重排，文件候选仍重排并返回 repaired。

修正：所有 provider 的候选都经过同一个、provider 无关的权限检查。按 module::function 检查范围，逐条检查变更种类；补丁通过验证也不能豁免契约中的修改限制。

## R4 — P1：notify_one 固定唤醒队首，遗漏合法选择

位置：`src/interp/exec.rs:496`、`src/petri/exec.rs:564`。

两个引擎都通过 pop_front/take_first 固定选择一个等待者。这是共同实现的欠近似，不会被“两者结果相同”的测试发现。原语及契约没有声明 FIFO 条件变量策略，标准 Condvar 也没有给出必须唤醒最早等待者的保证。参考：[Rust Condvar](https://doc.rust-lang.org/std/sync/struct.Condvar.html#method.notify_one)。

反例 `notify_choice_false_pass.json` 使用两个 semaphore 和 mutex 保证 w1、w2 依次登记等待，随后 notifier 执行一次 notify。w1 醒来会 notify_all，w2 醒来直接退出。当前两个引擎均 PASS：固定选 w1 后所有线程完成。允许选择 w2 时，w1 永久等待，存在死锁执行。

修正：枚举每个当前等待者的合法绑定。确定性指枚举顺序和报告可复现，不是把程序非确定性替换为一个固定选择。同时审计 mutex/semaphore/channel 的 FIFO 强制策略和 eager handoff；若没有显式调度契约或性质保持证明，不能任意删去竞争。

## R5 — P1：无 wait 位置的 notify_all 在网中没有后继

位置：`src/petri/net.rs:830`。

只有能从 wait 位置推断 condvar_lock 时才生成 notify_all 变迁。程序只含 `notify_all(cv); return` 时，解释器正确返回 PASS，而 Petri 引擎以单状态 FAIL 报告死锁。输入静态合法。

反例：`notify_all_without_wait.json`。

修正：无等待者的通知应当正常前进；模板不得依赖必须存在 wait 使用点。空等待集合与不存在静态 wait 位置均需覆盖。

## R6 — P1：验证入口跳过静态验证，契约语义开关不生效

位置：`src/bin/concir-backend.rs:163`、`src/explore/contract.rs:272`、`src/explore/mod.rs:214` 附近。

两个独立观察：

1. `invalid_protected_write_fixed_fixture.json` 未持锁写受保护变量。check 返回 valid=false/E309，而 explore 在两个引擎均返回 PASS、complete=true。入口直接 lower，并未通过原有静态验证。
2. `ignored_assumptions_contract.json` 指定 sequential_consistency=false、no_spurious_wakeups=false，仍得到 PASS。配置被复制进契约，但未改变执行语义或触发 Unsupported。

修正：提供从 Program + ContractSpec 开始的统一受检入口，CLI 和修复共用；低层 raw engine 若保留，明确它的前置条件且不对外包装成完整验证。暂不支持的语义配置必须拒绝或 Unsupported，不能静默忽略。报告记录实际使用的语义配置和分析边界。

## R7 — P2：有限控制的重复调用被历史计数人为变成无限状态

位置：`src/interp/exec.rs:730` 及 Petri 返回路径、双方 allocation/state 定义。

`finite_call_loop.json` 只有 main 无限调用一个立即返回的 helper，没有增长的数据，也没有递归。契约仅检查 deadlock_free。两个引擎均走到 40 深度，产生 41 状态并 UNKNOWN。每次返回无界增加 completed_functions，而且动态身份未规范化。

这次没有错误 PASS，但会使基本循环无法完成状态探索，影响后续任何规模实验。

修正：历史监视量按契约需求有限化，例如 FunctionCompleted 使用布尔，AtLeast(n) 饱和于所需最大 n；只在不影响性质的前提下进行身份复用或规范化。不得粗暴从 Eq/Hash 中删除仍影响未来行为的信息。真正越过分析边界仍需 Unknown。

## R8 — P2：CLI 将 UNKNOWN/INVALID/UNSUPPORTED 作为退出成功

位置：`src/bin/concir-backend.rs:184`。

仅 Outcome::Fail 使用非零退出码。实际 `finite_call_loop` 输出 UNKNOWN 但 exit=0；`runtime_invalid_exit` 输出 INVALID 但 exit=0。这会让脚本和 CI 将未完成/无效验证当成功。

修正：文档化稳定退出码；只有总体 PASS 为成功，其他验证结论均非零，JSON 中保留细分类别。

## R9 — P2：另外存在已复现的引擎分歧

`bodyless_entry.json` 是静态合法、无副作用且 may_block=false 的空入口。解释器 initial 创建空帧，随后报 E602/INVALID；Petri initial 直接完成并 PASS。

按统一支持策略处理：若允许此类空函数，两个引擎都应完成；若不允许，均明确拒绝。不能继续因入口位置与普通 call/spawn 的路径差异产生不同解释。

## R10 — P1（验收缺口）：所谓完整差分测试实际只比较共享状态集合

位置：`tests/petri_projection.rs:339`。

project_it/project_pn 定义了更完整的投影，但未被调用；实际只使用 shared_it/shared_pn。编译器已有 dead_code 警告。局部变量、调用栈、完成事实、具体目标结果及边关系没有被这项断言比较；等待发送的消息值也未完整进入投影。且没有先断言双方探索完整、无错误。

没有共享资源的两个错误执行器尤其可能都投影成唯一空状态而通过。状态集合相同也不能证明 AG EF 一致，因为边关系决定后续可达性。

修正：比较足够保留行为和性质的规范化状态与可观察边/弱步关系，检查阻塞与终止状态；若网含辅助步骤，显式处理 stuttering，不能简单删除控制信息。每次差分先断言完整性和错误状态。增加性质驱动、alpha-renaming、系统枚举和手写语义反例，避免两个引擎共同实现同一个错误。

## 尚不能据此声称完成的事项

- 上述是有复现与源码依据的首轮问题清单，不代表未列出的原语已全部正确。
- 网的多来源事件、实际读写弧与模板副作用一致性还需要后续审查。当前 StepLabel 只有单个 origin，rendezvous 等跨实例动作不能完整表述双方来源。
- 当前修复器枚举原程序的独立候选，RepairContext 不含上轮结构化诊断。诊断驱动、多处组合修复可以在本轮正确性问题清除后开展。
- 先清除错误结论，再扩展 RwLock/select/async、状态空间优化、LLM 和论文实验。
