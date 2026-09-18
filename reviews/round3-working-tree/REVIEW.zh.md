# 第三轮实现复核：下一轮仍应优先修正确性

审阅对象为基于 `f3463d6ca87b6c051c832996e16d537db2248d93` 的未提交第三轮工作区，含 `CODE_REVIEW_ROUND3.md`。源文件清单／散列见 `source-manifest.json`，差异见 `reviewed.diff`，版本和完整性检查见 `revision.json`。未修改 ConcIR 源码。

实际执行全量测试：

```sh
cd /Users/kevin/local-repos/ConcIR
env INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-round3-target cargo test --offline --all-targets --no-fail-fast
```

结果为 **177 passed，4 failed**，失败仍是既有 DOT snapshots。第三轮回归 15 项、第二轮回归 18 项、差分测试 9 项均通过。独立重跑上一轮 Python 反例，主要错误接受、变量遗漏、cv 锁关联、命名空间、全限定补丁范围、简单动态实例循环和信号量溢出均得到预期结果。完整记录分别在 `full-tests.log`、`old-probe-summary.txt`。

但本轮新增的状态去重方式存在错误状态合并；B5 取值域检查仍有未覆盖路径。因此不建议直接进入实验或 LLM 集成。

## C1 [P1] 展示字符串被用作状态键，字符串未转义导致错误 PASS

位置：[explore/mod.rs:309](/Users/kevin/local-repos/ConcIR/src/explore/mod.rs:309)、[sem/value.rs:115](/Users/kevin/local-repos/ConcIR/src/sem/value.rs:115)。`explore` 和 `verify` 都改用 `system.canonical(state)` 作为唯一去重键，但值的文本表示不是无歧义编码。

例如以下两个合法、不同的 Struct 值：

```json
{"a":"X\",b:\"Y","b":"Z"}
{"a":"X","b":"Y\",b:\"Z"}
```

都被编码为 `{a:"X",b:"Y",b:"Z"}`。这不是 HashMap 的随机哈希碰撞，而是两个不同状态具有完全相同的键。

反例由 main scope 两个线程组成，每个线程把不同的上述常量写入共享 x 后返回。两个执行顺序都合法，所以 scope 完成后的 x=A 和 x=B 均应可达。

| 用例 | 正确期望 | 解释器 | Petri |
|---|---|---|---|
| r3_canonical_collision_A：EF(scope_done && x=A) | PASS | FAIL | FAIL |
| r3_canonical_collision_B：EF(scope_done && x=B) | PASS | PASS | PASS |
| r3_canonical_false_pass：AG(!(scope_done && x=A)) | FAIL | PASS | PASS |

全部静态合法；两引擎均报告完整、11 状态。为避免探索器共享缺陷导致循环论证，额外编写了直接使用原始 State 的 Eq/Hash 做 BFS 的有限小模型 oracle，不调用生产 explore/verify 的去重代码。结果两引擎各有 16 个原始状态，A、B 均可达，并发现相同 canonical key 对应不同谓词真值。见 `raw-oracle.txt` 和 `repro/raw-oracle/src/main.rs`。

下一轮必须将动态身份规范化后的结构化状态键与诊断文本分离，或采用可证明无歧义的类型化编码。保留完整行为相关字段。不能仅把哈希函数换一个，也不能恢复无限身份增长来逃避问题。身份商图需要检查谓词保持和后继保持；同时保证反例轨迹的具体身份可以回放。

## C2 [P1] Channel 消息域检查仅在解释器实现，Petri 仍接受域外消息

位置：[interp/exec.rs:442](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:442)、[petri/exec.rs:704](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:704)、[petri/exec.rs:739](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:739)、[petri/exec.rs:825](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:825)。

声明 Channel 的 base 为 Int[0,1]，共享 a:Int 初值 2，sender 发送 a，receiver 接收到 `_`。静态合法。因为接收者丢弃值，dst 检查不能替代 Channel 本身的 payload 域检查。

| 用例 | 解释器 | Petri | 当前域语义下的正确期望 |
|---|---|---|---|
| r3_channel_domain_0：capacity=0 | FAIL，3 状态 | PASS，9 状态 | 两者 deadlock_free FAIL |
| r3_channel_domain_1：capacity=1 | FAIL，3 状态 | PASS，12 状态 | 两者 deadlock_free FAIL |

解释器禁用域外发送，Petri 允许配对／入队并正常结束。必须在所有实际引入 payload 的网步骤检查 Channel base：注册阻塞发送、直接 rendezvous、缓冲发送等，并保证冻结值和步骤原子性。审计恢复路径，避免新鲜求值和冻结值采用不同规则。

## C3 [P1] within_type 不递归检查 Struct/Array 中的有界成员

位置：[sem/value.rs:154](/Users/kevin/local-repos/ConcIR/src/sem/value.rs:154)。该函数仅在最外层类型是 BoundedInt 时检查区间，其余类型直接 true；即使是 BoundedInt，值类型不匹配也返回 true。

`r3_nested_domain`：src:{n:Int} 初值 {n:2}，x:{n:Int[0,1]} 初值 {n:0}，执行 read_shared src -> x。静态合法。查询 EF(!(x=={n:0} || x=={n:1}))，两引擎均完整 PASS、3 状态；按域外更新禁用的现有语义，正确期望为 FAIL。

需要完整递归验证结构体字段、数组长度和元素域、Enum 成员、基本类型，并统一初始化和运行时赋值语义。复合值不能因最外层是 Struct/Array 而绕过域限制。函数签名、参数和返回值也应纳入同一检查；若某类组合确实不支持，应明确拒绝，而不是静默放行。

## 验收建议

1. 优先 C1，随后 C2/C3。保留已经正确的 B1–B8 回归和动态循环有限化。
2. 使用有限小模型原始状态 oracle 验证规范化商图，不要让 oracle 复用被测试的 key/renderer。
3. 系统覆盖特殊字符串、不同值形状、动态身份重命名、Channel 容量和不同阻塞顺序、复合类型与不同写值路径。
4. 完整检查性质结果、探索完整性、消息／返回步骤原子性、CLI 类别和修复验收；不能只检查双方引擎一致。
5. 下一轮继续集中于正确性，不同时扩展多点修复、LLM 或大规模实验。

## 复现

`repro/probe_new.py` 依赖同目录 `old-repro/probe.py` 的公共构造函数。请复制整个 repro 到临时目录再执行，避免覆盖审阅输出。

```sh
env CONCIR_REVIEW_BIN=/private/tmp/concir-audit-round3-target/debug/concir-backend python3 /private/tmp/concir-audit-round3/probe_new.py
env CARGO_TARGET_DIR=/private/tmp/concir-audit-round3-target cargo run --offline --quiet --manifest-path /private/tmp/concir-audit-round3/oracle/Cargo.toml -- /private/tmp/concir-audit-round3
```

以上为实际使用的临时路径。复现时改为复制目录中的 probe_new.py、raw-oracle/Cargo.toml 和包含 r3_canonical_collision_A/B JSON 的目录。原始输出保存在 `repro/*.stdout`、`repro/*.stderr`，摘要为 `new-probe-summary.txt`。
