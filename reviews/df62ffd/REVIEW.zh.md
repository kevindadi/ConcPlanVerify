# 第四轮实现复核

审阅版本：`df62ffd`，完整 SHA 及源码完整性检查见 `revision.json`。审阅期间源码未变化，工作区干净，未修改 ConcIR 核心代码。

结论：C1–C3 的原始反例均已修正。本轮复现 **1 个剩余 P1：函数自身声明的返回类型仍未在运行时检查**。此外，状态商图后继保持／反例回放的验收证据尚未补齐；本轮未据此声称发现第二个状态合并错误。

## 验证结果

实际执行：

```sh
cd /Users/kevin/local-repos/ConcIR
env INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-round4-target cargo test --offline --all-targets --no-fail-fast
```

**186 passed，4 failed**，退出码 101。4 个失败仍为已知 DOT snapshot 测试。第四轮回归 9 项、第三轮 15 项、第二轮 18 项、差分 9 项均通过。完整记录见 `full-tests.log`。

独立重跑上一轮 CLI 反例：

| 用例 | 解释器 / Petri | 完整性 |
|---|---|---|
| Channel 域外 payload，容量 0/1 | FAIL / FAIL，均 3 状态 | 完整 |
| EF(scope_done && x=A) | PASS / PASS，均 16 状态 | 完整 |
| EF(scope_done && x=B) | PASS / PASS，均 16 状态 | 完整 |
| AG(!(scope_done && x=A)) | FAIL / FAIL，均 16 状态 | 完整 |
| 内嵌 bounded Int 域外写入 | FAIL / FAIL，均 1 状态 | 完整 |

另重跑独立原始 State Eq/Hash BFS，将其比较键改为本轮新增的 state_key。双方各 16 个原始状态，A/B 均可达，没有相同 key 却目标谓词真值不同的冲突。见 `raw-oracle.txt`；oracle 源码在 `raw-oracle/src/main.rs`。这些结果支持原始反例已修复，不构成所有身份归一化情形的证明。

## D1 [P1] 返回路径只校验调用者 dst，漏掉被调用函数自己的 returns 类型

位置：

- [解释器 Return](/Users/kevin/local-repos/ConcIR/src/interp/exec.rs:826)
- [Petri ReturnInner](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1211)
- [Petri ReturnFinal](/Users/kevin/local-repos/ConcIR/src/petri/exec.rs:1236)

最小情况：共享 `src:Int = 2`；函数 two 声明 modeled 返回值类型 `Int[0,1]`，执行 `return src`。使用变量表达式而非越界字面量，模型静态检查合法；需要运行时域检查。

当前两个引擎仅检查返回值是否能写入调用者的 dst。如果 dst 是普通 Int 或调用者不接收结果，函数自身的 [0,1] 约束完全没有执行。线程最外层 return 也直接记录完成。lower 已保留 `SemFunction.returns` 的解析类型，运行时没有使用它。

独立反例结果：

| 反例 | 查询 | 实际两引擎 | 现有“域外更新禁用”语义下期望 |
|---|---|---|---|
| r4_return_domain_wide_dst | EF(two completed)，dst 为 Int | PASS，4 状态 | FAIL |
| r4_return_domain_discard | EF(two completed)，call 省略 dst | PASS，4 状态 | FAIL |
| r4_return_domain_entry | EF(main completed)，入口直接返回域外值 | PASS，2 状态 | FAIL |
| r4_return_domain_valid | 入口返回合法值 1 | PASS，2 状态 | PASS |

全部 check 合法、探索完整。忽略调用结果的有效写法是省略 dst；显式 `dst="_"` 在当前 call 的静态规则下是 INVALID，未把它作为有效反例。

这会允许违反函数签名的返回动作，错误满足 function_completed 和后续可达性契约。不是两个引擎一致就能接受的行为，也说明 `CODE_REVIEW_ROUND4.md` 中“所有 call returns 均检查类型域”的说明仍不完整。

修正要求：

1. 返回表达式求值后，先按被调用函数自身已解析的 returns 类型检查，再独立检查调用者 dst。
2. 所有返回路径一致：普通嵌套调用、省略 dst、入口／线程最外层返回；复合返回类型同样递归检查。
3. 保持现有语义：非法域更新禁用整个步骤，不先退栈、写 dst、唤醒 join/scope 或更新完成监视器。
4. 对 modeled/unmodeled 和无返回值函数按已有支持策略明确处理，不隐式扩展支持范围。
5. 增加域内边界值、域外动态值、宽 dst、窄 dst、无 dst、嵌套 Struct/Array 返回的回归。合法返回继续执行。

## 尚未补齐的 C1 验收证据

`tests/round4_regressions.rs::raw_oracle` 的 key 表只保存一个 goal 的布尔值，检查的是这两个固定目标的真值保持；虽然遍历 successors，但没有比较“相同 key 的不同原始状态是否拥有相同的后继商行为”，也没有反例轨迹回放断言。

当前碰撞夹具的原始图和生产图都为 16 状态，没有直接覆盖动态身份归一化实际合并不同原始状态的情况。建议新增显式构造的身份平移／重命名状态对，在保持全部引用关系的前提下检查 key、谓词和后继；再加一个归一化后轨迹可回放的用例。

设计文档 [backend-design.md:333](/Users/kevin/local-repos/ConcIR/doc/backend-design.md:333) 将该测试描述为同时检查后继商行为，应补实现或缩小声明。本项是验收与文档缺口，**未复现额外的状态丢失，不计为第二个 P1**。

## 证据与复现

本轮反例及输出在 `repro/`，摘要在 `new-probe-summary.txt`。可复制该目录到临时目录后运行：

```sh
env CONCIR_REVIEW_BIN=/private/tmp/concir-audit-round4-target/debug/concir-backend python3 /path/to/copied/repro/probe.py
```

脚本会在自身目录写入结果，请保留本审阅的原始输出。下一轮应集中补上返回签名检查及上述验收证据，无须再次重写整个后端。
