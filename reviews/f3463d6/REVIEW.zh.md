# ConcIR 第二轮独立审阅

审阅日期：2026-09-17。审阅版本：`f3463d6ca87b6c051c832996e16d537db2248d93`。

结论：上一轮主要反例已有实质修复，但还不能验收核心验证／修复闭环。本轮复现了错误 PASS 和错误修复接受，以及同步语义、契约绑定、有限状态化等问题。建议继续修正确性，再开展实验和 LLM 集成。

审阅开始时 HEAD 为 `736f7e1`，第二轮代码尚未提交；审阅结束时已提交为 `f3463d6`。开始时记录的 122 个文件 SHA-256 与结束时一致，没有新增文件。审阅者没有修改 ConcIR 源码。版本、散列及检查结果分别保存在 `revision.json`、`source-manifest.json`、`review-integrity.json`；提交差异在 `reviewed.diff`。

## 1. 测试和上一轮复核

实际运行：

```sh
cd /Users/kevin/local-repos/ConcIR
env INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-round2-target cargo test --offline --all-targets --no-fail-fast
```

结果：**161 passed，4 failed**，退出码 101。4 个失败仍是已有的 DOT snapshot 测试：`snapshot_function_only`、`snapshot_with_summary_dot`、`snapshot_producer_consumer_dot`、`snapshot_state_machine_dot`。没有通过更新快照或跳过测试来使其通过。完整记录在 `full-tests.log`。

其中 `round2_regressions` 18/18、`differential` 8/8、`semantics_regression` 13/13、`repair_e2e` 4/4 通过。还独立重跑了上一轮反例的副本，未覆盖上一轮审阅证据。

| 上轮项 | 本轮复核 |
|---|---|
| R1 帧内 handle | 原始错误 PASS 变为 FAIL；局部改名对照结论一致 |
| R2 契约重新绑定 | 删除必须保留的 sid 被拒绝，不再误指向后续语句 |
| R3 补丁权限 | 文件候选的删除／重排权限已强制检查；全限定函数范围仍未完成，见 B7 |
| R4 等待者选择 | notify_one 原反例已检出；rendezvous 仍只选队首，见 B3 |
| R5 空 notify_all | 两引擎 PASS、完整探索 |
| R6 统一验证入口 | 静态非法程序返回 INVALID；不支持的假设返回 UNSUPPORTED；早退报告元数据仍有小问题 |
| R7 有限状态化 | 普通调用循环 PASS、6 状态；循环 spawn/join 和 scope 仍 UNKNOWN，见 B6 |
| R8 CLI 退出码 | 原始用例的非 PASS 不再返回 0 |
| R9 无函数体入口 | 两引擎 PASS、1 状态 |
| R10 差分检查 | 已比较状态与带标签的边，并检查完整性；仍有投影信息损失和共同语义错误未覆盖 |

## 2. 必须修复的发现

### B1 — [P1] Petri 网遗漏声明的 Var/Atomic，导致错误 PASS 和错误修复接受

位置：[net.rs:550](/Users/kevin/local-repos/ConcIR/src/petri/net.rs:550)、[net.rs:621](/Users/kevin/local-repos/ConcIR/src/petri/net.rs:621)、[exec.rs:1254](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1254)、[exec.rs:1556](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1556)。

网的资源构建循环未为所有声明的 Var/Atomic 建立库所；这些库所主要由显式读写语句创建。初始化又仅遍历已有库所。一个只在契约中被观察、没有显式读写的变量，其初值因此消失。谓词把不存在的值当作 `VarEq=false`，取反后就变成错误的 true。

最小程序：声明 `main::x = 0`，main 只有 return；使用全限定资源名，排除命名空间问题。

| 契约 | 解释器 | Petri | 正确结论 |
|---|---|---|---|
| AG(x == 0) | PASS | FAIL | PASS |
| AG(!(x == 0)) | FAIL | PASS | FAIL |

Var 和 Atomic 都复现。相关用例：`r2_query_only_Var*`、`r2_query_only_Atomic*`。

**修复链路已复现错误接受**：原程序 `x=0; return`，x 初值为 0，契约 AG(!(x==0))，允许删除语句。原程序两引擎 FAIL。候选仅删除写语句，违规的初始状态仍存在，但网中 x 库所消失，repair 返回 `repaired`、`patched_outcome=PASS`、退出码 0。独立验证候选：解释器 FAIL，Petri PASS。见 `r2_query_only_repair_before`、`r2_query_only_repair`、`r2_query_only_repair_after`。

同一问题还影响只通过 `dst` 写入的共享变量：`atomic_load a -> x`，没有显式读写 x 时，Petri 报 E900 “no place for shared resource”，解释器能执行。见 `r2_bounded_dst_fixed`；该用例的目标范围检查是另一个问题，见 B5。

修正：先按照所有受支持资源声明完整建立并初始化库所，再生成操作模板；或者给出覆盖契约、表达式、dst 等全部引用的可靠裁剪方案。不要把缺失值默认为 false 作为解决办法。必须加入“删除最后一次读写后，初始状态仍违反契约”的修复拒绝回归。

### B2 — [P1] 条件变量的锁关联在解释器中仍是全局单值

位置：[state.rs:40](/Users/kevin/local-repos/ConcIR/src/interp/state.rs:40)、[exec.rs:464](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:464)、[exec.rs:473](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:473)、[exec.rs:715](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:715)。

每次 wait 都覆盖 `cv.lock`，notify/notify_all 再让所有被唤醒者重取最后写入的那一把锁。Petri token 已按等待者携带 lock，解释器却没有同步采用该语义。

`r2_multi_lock_cv`：两个线程分别持有 m1/m2 等待同一 cv；notifier 确认二者均已等待后通知。静态检查合法。解释器 INVALID、145 状态，出现错误锁所有者／解锁空锁的 E510；Petri PASS、163 状态，完整探索。

当前设计文档描述“重新获取同一把锁”，且统一入口没有拒绝此模型。应让解释器逐等待者保存锁；如果项目选择限制一个 cv 只能关联一把锁，应在统一支持检查中明确拒绝该模型，并调整文档。不能让同一输入一边 PASS、一边 INVALID。本审阅不把该扩展语义等同于特定语言运行库的 Condvar 保证。

### B3 — [P1] 零容量 Channel 仍删除非队首等待者的合法匹配

位置：[解释器 exec.rs:361](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:361)、[Petri exec.rs:1358](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1358)、[Petri exec.rs:1380](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1380)。

notify_one 已改为枚举，但 rendezvous 仍使用 `pending_recv.pop_front()`／`pending_send.pop_front()`，网的 ControlWait/Pair 也只取 `.first()`。消息 FIFO 不能推出等待接收线程必须按排队顺序获得消息；当前使用文档又明确说等待线程没有 FIFO 顺序。

独立 Rust 探针从合法程序的初始状态做 BFS，找到两个 receiver 已阻塞、一个 sender 可以发送的可达状态。该状态下只有一次发送可推进，默认非 FIFO 接收者语义应有两个不同匹配后继；**两个引擎均只生成 1 个后继**，且总留下第二个等待者。见 `rendezvous-probe-summary.txt` 和 `repro/rendezvous_probe.rs`。

这是直接的漏边证据；本轮没有声称已经用这个独立案例证明最终 deadlock 判定出现错误 PASS。修正应枚举允许的 rendezvous 配对，同时明确消息顺序与线程调度顺序；若要支持 FIFO 等待策略，应是显式、受检的语义配置。不能只改文档来替代默认非确定语义。

### B4 — [P1] 契约短名绑定到 ModuleId(0)，声明重排会改变被验证的对象

位置：[contract.rs:302](/Users/kevin/local-repos/ConcIR/src/explore/contract.rs:302)。

固定入口 `main::main`，main::x=0，other::x=1，契约 AG(x==0)。两个模块各放显式 read_shared，确保所有变量都有库所，排除 B1 干扰。

只把 modules 数组倒序，两引擎结果均从 PASS 变成 FAIL，探索都完整、都是 3 状态。契约从观察 main::x 静默转成 other::x。见 `r2_namespace_used_False/True`。

建议使用显式、稳定的契约命名空间，或将未限定名绑定到入口模块并明确文档；歧义也可直接拒绝。全限定名应精确解析。模块／函数／资源独立声明重排的元变换测试应检查有实际目标的契约，而不只检查 deadlock_free。

### B5 — [P1] 经 dst 写入的值绕过 bounded Int 的取值域检查

位置：[state.rs:354](/Users/kevin/local-repos/ConcIR/src/interp/state.rs:354)、[Petri exec.rs:1181](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1181)、[解释器 exec.rs:293](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:293)。

显式赋值／store 会调用 within_type，但两个 write_dst 都直接插入值。`atomic_load a -> x` 可以把 2 写入范围 [0,1] 的 x。为隔离 B1，在一个未调用函数中加入对 x 的显式读取，确保网中存在库所。

`r2_bounded_dst_with_place` 静态检查合法，契约为 EF(!(x==0 || x==1))，两引擎都 PASS、3 状态、完整。这证明两者都到达了声明域外的状态。根据现有文档“越出语义域的更新禁用”，该目标不应可达。

修正应统一覆盖直接赋值、read/load、recv、call 参数／返回等写值路径；区分语义域边界与探索预算，并保证禁用步骤不会留下半执行的消息消费、解锁或返回动作。不可仅调整差分测试，让两个实现继续一致地越界。

### B6 — [P2] 完成监视器饱和后，历史线程／scope／handle 仍使有限并发循环无限增长

位置：[state.rs:176](/Users/kevin/local-repos/ConcIR/src/interp/state.rs:176)、[解释器 exec.rs:105](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:105)、[Petri exec.rs:225](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:225)。

完成函数计数的饱和确实修好了普通 call 循环，但 threads、finished、scopes、handle_children 中的历史对象没有按可观察性回收，具体身份也继续进入状态键。

两个模型均没有增长的数据、最多同时两个线程、没有递归：

- `scope(worker); goto s1`，worker 立即 return：两引擎 UNKNOWN，41 状态，深度 40 截断。
- `spawn worker h; join h; goto s1`：解释器 UNKNOWN、51 状态，Petri UNKNOWN、89 状态，同样深度截断。

见 `r2_scope_loop`、`r2_spawn_join_loop`。这些程序在现有契约只观察 deadlock_free 时具有有限的行为商图。需要回收不再可引用的完成对象，规范化剩余身份，并保存仍可被 join／契约观察的必要事实。提高预算不会解决根因；也不能直接从状态键中删除仍影响行为的数据。

### B7 — [P2] 文档宣称全限定补丁范围，实际仍只比较函数短名

位置：[patch.rs:247](/Users/kevin/local-repos/ConcIR/src/repair/patch.rs:247)、[candidates.rs:41](/Users/kevin/local-repos/ConcIR/src/repair/candidates.rs:41)、[contract.rs:58](/Users/kevin/local-repos/ConcIR/src/explore/contract.rs:58)。

对现有 lockorder 示例设置 `allowed_scope.functions=["main::t1"]`、允许锁重排，合法的 main::t1 交换候选被文件 provider 拒绝为 “function 't1' is outside ...”；自动 provider 直接生成 0 个候选。见 `r2_fqn_scope_file/auto`。单独增加 modules 列表并没有实现 module::function 标识；多模块同名函数还会产生范围表达歧义。

解析允许范围为稳定的限定目标，所有 provider 共用同一判断；测试 main::t1 与 other::t1 的区分。`CODE_REVIEW_ROUND2.md` 对 R3 完成情况的描述需要同步纠正。

### B8 — [P2] SemaphoreRelease 的加法可使整个验证进程 panic

位置：[解释器 exec.rs:507](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:507)、[Petri exec.rs:642](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:642)。

Semaphore 初始 count 为 i64::MAX，执行 release(1)。`check` 返回合法，但两个 debug 引擎均在 `available + count` panic，退出 101，没有 JSON 报告。见 `r2_semaphore_overflow` 和两份 stderr。

应使用 checked arithmetic，并按约定返回结构化 INVALID 或表示范围不足的边界结果；统一位置和退出码。此次实际复现的是 debug panic，没有将未经运行的 release 行为写成已验证事实。

## 3. 验收测试仍需补强

1. [differential.rs:166](/Users/kevin/local-repos/ConcIR/tests/differential.rs:166) 和 [differential.rs:390](/Users/kevin/local-repos/ConcIR/tests/differential.rs:390) 把 frame.handles 投影成排序后的 child 集合，丢掉“哪个局部 handle 指向哪个 child”。例如 h1→a,h2→b 和 h1→b,h2→a 投影相同，但后续 join(h1) 行为不同。应保留规范化后的名字／符号槽到 child 的对应关系；alpha-renaming 应通过已知映射规范化，不能删掉绑定。
2. 当前声明重排测试只重排单模块内函数，并只比较 deadlock_free。它无法发现 B4。
3. B3、B5 是双方实现相同错误，说明“两个引擎一致”仍需独立的规则断言、小模型枚举和负例支持。
4. 错误入口的报告元数据仍由 synthetic 的默认 assumptions/bounds 填充，finish_meta 只补指纹。应区分请求配置、实际采用配置以及“分析未启动”；不要让被拒绝配置的报告看起来使用了默认配置。这不是本轮错误 PASS 的根因。

## 4. 下一轮验收条件

优先处理 B1、B2、B3、B4、B5，随后 B6–B8；同步补强上述测试。

- 本轮错误补丁必须被拒绝，独立解释器和 Petri 对其都 FAIL。
- 原有 R1–R10 的正确期望不能退化。
- 零容量 Channel 的两个可匹配 receiver 必须有两个分支，或以明确受检的另一语义处理。
- bounded Int 在每种值进入路径都不能越域。
- 有限 spawn/join、scope 循环在小预算内完整探索，不靠增大预算。
- 命名空间与限定补丁目标对声明重排稳定。
- 非法／超范围输入返回结构化结果，不 panic。
- 差分比较保留行为相关的绑定；加入不能依赖两个实现共识的独立期望。

本轮不建议扩展 LLM、原语覆盖、偏序约简或论文大规模实验。先获得可信的语义与修复验收基础。

## 5. 证据与重现

`repro/probe.py` 生成并运行本轮 CLI 反例，完整输出在 `repro/*.stdout`／`*.stderr`；摘要在 `observations.json` 和 `observations.txt`。`rendezvous_probe.rs` 是直接调用公开状态系统接口的独立探针，没有修改仓库测试。

重新运行会覆盖输出，请复制到临时目录运行，保留此审阅记录：

```sh
cp -R /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/f3463d6/repro /private/tmp/concir-round3-repro
env CONCIR_REVIEW_BIN=/private/tmp/concir-audit-round2-target/debug/concir-backend python3 /private/tmp/concir-round3-repro/probe.py
```

重新构建后请将 CONCIR_REVIEW_BIN 指向新二进制。Rust 探针的临时 crate 构建说明见 `repro/README.md`。本轮后续实现任务见同目录 `NEXT_CURSOR_PROMPT.md`。
