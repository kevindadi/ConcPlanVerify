请在 /Users/kevin/local-repos/ConcIR 做一轮范围明确的收尾修正。

先阅读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/df62ffd/REVIEW.zh.md

基线是 df62ffd，反例和实际输出在同目录 repro/。保留已有工作；复制反例后复现，不覆盖原始审阅证据。C1–C3 原始反例已通过独立复核，当前全量测试为 186 passed / 4 个既有 DOT snapshot failed。本轮不重写后端，不扩展 LLM、原语、实验或多点修复。

1. 修复 D1：函数自身的返回类型域必须检查

函数 two 声明 modeled returns Int[0,1]，却返回共享 src:Int=2。静态检查合法，但运行时只检查调用者 dst，导致普通 Int dst、省略 dst、入口最外层 return 都错误记录函数完成。

要求：
- 使用 SemFunction.returns 已解析的类型检查返回表达式值，再独立检查调用者 dst。
- 覆盖解释器 Return、Petri ReturnInner/ReturnFinal；省略 dst 或最外层 return 不能绕过声明返回类型。
- 复合返回值递归检查；modeled/unmodeled、无返回值按既有支持策略处理。
- 按现有语义，域外返回禁用整个动作；不能先退栈、写入 dst、唤醒等待者或记录函数／scope 完成。
- r4_return_domain_wide_dst、r4_return_domain_discard、r4_return_domain_entry 中的 EF(function_completed) 均应完整 FAIL。
- r4_return_domain_valid 应完整 PASS。
- call 忽略结果的合法写法是省略 dst，不要拿当前静态非法的 dst="_" 替代反例。
- 加入动态域外返回、域内上下界、宽/窄/无 dst、嵌套调用、Struct/Array 返回值的正反例。保留既有调用者 dst 约束。
- 先加入失败回归，再修实现，保证两引擎独立实现并分别满足手写期望。

2. 补齐 C1 的剩余验收证据

当前 raw_oracle 只比较两个固定目标在同 key 下的布尔值，并未检查后继商行为或轨迹回放。

- 显式构造同一合法状态经动态身份平移／重命名得到的状态对，完整更新 thread/frame/scope/handle 引用。
- 对实现声称等价、会合并的状态，验证 key 相同、相关谓词真值一致、后继商状态及规范化动作对应。
- 有限小模型 oracle 使用原始 State Eq/Hash 建图，不得依赖生产 key 决定去重。
- 增加归一化后反例轨迹可回放的测试，避免代表状态之间的边包含不一致的具体身份。
- 不要求任意图同构均得到同一个 key；明确当前身份规范化覆盖范围，区分漏合并与错误合并。
- 保留已经正确的值编码、动态实例有限化和原 C1 碰撞负例。
- 修正文档中尚未由测试支持的“已检查后继商行为”声明；不要宣称完成形式证明。

3. 验收与交付

- 保留 R1–R10、B1–B8、C1–C3 的正确回归。
- 运行相关测试和最终全量测试；186/4 是修复前基线，4 个历史快照失败单独报告，不跳过或未经检查接受新快照。
- 更新 CODE_REVIEW_ROUND4.md 和相关语义文档，明确此前返回路径仅检查 dst，本轮补上声明 returns 类型。
- 新增 CODE_REVIEW_ROUND5.md：列出根因、修复位置、反例及正确期望、实际命令和结果、剩余限制。
- 不通过弱化 returns、改变期望或让两引擎共同放行域外值来通过测试。
- 完成后交由 Codex 独立复核。
