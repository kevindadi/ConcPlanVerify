请在 /Users/kevin/local-repos/ConcIR 继续第四轮实现。

先阅读独立审阅报告：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/round3-working-tree/REVIEW.zh.md

反例和原始输出在同目录 repro/；raw-oracle.txt 是独立原始状态 BFS 的结果。审阅基于 f3463d6 上的第三轮未提交工作区，准确文件版本见 source-manifest.json 和 revision.json。保留已有修改；如源码已变化，先确认问题仍存在。复制证据后复现，不覆盖原始审阅记录。

第三轮的主要 B1–B8 反例已得到预期结果。当前全量测试为 177 passed / 4 failed，失败仍是既有 DOT snapshots。但新增了状态误合并导致错误 PASS 的问题，取值域检查也仍有遗漏。本轮只处理 C1–C3 和相关正确性验收，暂不接 LLM、不开展论文实验、不扩展原语、不加入偏序约简，也不修改 ConcPlanVerify 仓库。

一、最高优先级：修正状态去重的正确性（C1）

生产 explore/verify 当前用 system.canonical(state) 的字符串作为唯一状态键；Value::Str 不转义，导致不同 Struct 值产生相同文本，错误合并可达状态。两引擎因此一致地产生错误结论。

必须复现：
- r3_canonical_collision_A：EF(scope_done && x=A)，当前两者 FAIL，正确应为 PASS。
- r3_canonical_collision_B：EF(scope_done && x=B)，正确应为 PASS。
- r3_canonical_false_pass：AG(!(scope_done && x=A))，当前两者 PASS，正确应为 FAIL。
- 原始 State Eq/Hash BFS oracle 已确认 A、B 均可达：各引擎 16 个原始状态；生产探索错误合并后只有 11 个状态。

实现要求：
- 分离诊断展示文本与语义去重键，优先使用动态身份规范化后的结构化、带类型的 StateKey。
- 如果使用序列化键，必须保证编码无歧义，正确处理类型标签、字符串转义、长度和嵌套结构；不能靠更换哈希算法解决。
- 审计所有影响未来行为和性质求值的字段，不能通过少存字段实现有限化。
- 保留线程、frame、scope、handle 的身份规范化和生命周期回收，不恢复永久增长的历史状态。
- 建立“相同 key 保持谓词真值，且后继商状态及可观察动作对应”的独立检查。
- 检查规范化前后反例轨迹能否以一致的具体身份回放；不得将代表状态之间的边拼成不可执行轨迹。

独立 oracle 仅用于确保有限的小模型，使用原始结构化 State 的 Eq/Hash，不得复用生产 canonical/key/renderer。比较性质真值和行为关系，不要求原始图与商图状态数相同。

二、补齐 Petri Channel 消息域检查（C2）

Channel base=Int[0,1]，sender 发送共享 Int 变量 a=2，receiver 接收到 `_`。静态合法。
r3_channel_domain_0 和 r3_channel_domain_1 分别覆盖容量 0 和 1：当前解释器 deadlock_free FAIL，Petri PASS。按既定“域外更新禁用”语义，两引擎都应完整 FAIL。

要求：
- 在所有引入 payload 的 Petri 路径检查 Channel base：SendRegister、SendPair、SendBuf、SendBufBlock，以及相关交付／恢复路径。
- 不能用接收者 dst 检查替代 Channel 自身的消息域检查，特别覆盖 dst=_。
- 阻塞发送保留冻结值，不在恢复时重新求值。
- 禁用步骤不得留下半执行的 token 注册、消息消费、控制推进或唤醒。
- 覆盖先 sender、先 receiver、多等待者、缓冲未满／已满、合法／非法 payload；保留合法非确定匹配。
- 不通过让解释器同样放行非法消息来消除差分。

三、递归验证复合值的类型和取值域（C3）

within_type 目前只检查最外层 BoundedInt，Struct/Array 直接 true，BoundedInt 遇到非 Int 值也返回 true。

r3_nested_domain：src:{n:Int}={n:2}，x:{n:Int[0,1]}={n:0}，执行 read_shared src -> x。当前两引擎都认为域外 x 可达；正确应禁用该更新，EF(!(x=={n:0} || x=={n:1})) 完整 FAIL。

要求：
- 对已解析类型递归检查 Struct 字段、Array 长度和元素、Enum 成员、基本类型以及 bounded Int。
- 初始化、直接赋值、read/load、Channel payload、recv dst、call 参数／声明返回类型／返回 dst 使用一致的类型域规则。
- 类型不匹配不能默认成功；区分非法模型、语义域禁用和分析预算边界。
- 检查复合值写入的原子性，失败时不得先消费消息或退栈。
- 加入多层 Struct、Array<bounded Int>、Struct/Array 混合嵌套的正反例，合法值必须仍可执行。
- 如某组合确实超出支持范围，应明确说明并在统一入口拒绝，不能静默按无约束值处理，也不要为绕过反例全面禁用已有复合类型支持。

四、验收与交付

- 先将独立反例迁入回归，确认修复前失败，再修代码。
- 保留 B1–B8 和 R1–R10 已修好的正确期望，特别是错误补丁拒绝、契约重绑定、补丁权限、多等待者选择及有限并发循环。
- 加入小模型 oracle、身份重命名元变换、特殊字符串编码、复合类型和全部值进入路径的系统性测试；不能只比较两引擎是否一致。
- 检查探索完整性、性质真值、反例可回放、CLI 结果及修复验收，不能把 UNKNOWN 当 PASS。
- 分阶段运行相关测试，最终运行全量测试并记录真实结果。既有 4 个 DOT 失败单独说明，不跳过或未经检查更新快照。
- 更新设计文档，说明 StateKey 的完整字段、身份等价规则、资源域检查位置、原子性和支持边界。
- 新增 CODE_REVIEW_ROUND4.md，逐项写明 C1–C3 的根因、修改位置、修复前后正确期望、实际命令与结果，以及仍未解决的限制。
- 不宣称仅凭两个引擎一致就完成了语义正确性证明。交付后由 Codex 再次独立复核。
