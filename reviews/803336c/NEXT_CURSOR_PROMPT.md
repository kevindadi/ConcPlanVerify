请在 /Users/kevin/local-repos/ConcIR 收敛上一轮 F3 尚未完成的三项记录一致性要求，基线提交 803336cee10d7c235cc289d1e40262821d571762。

独立结果：225 passed / 0 failed / 0 warnings；F1、F2、F4 的原始反例已通过。保留已有修复。本轮只处理 G1–G3，不新增搜索策略、原语、LLM、POR，不修改 ConcPlanVerify，不开展规模实验。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/REVIEW.md

同目录 residual-summary.json 和 residual_checks.py 有 14 个可直接复现的遗漏。应围绕以下三条通用性质实现共享校验逻辑，使反例因真实语义矛盾被拒绝。

G1：每个 attempt 的实际补丁必须能解释记录的结果。

当前仅检查 patch 存在及引用关系。即使 original_hash 错误、目标函数/sid 不存在，replay 仍成功。

在重建父程序后，按实际搜索顺序检查 attempt.patch 的权限、基准、应用和静态合法性：
- verified：对应唯一新验证节点，实际补丁结果和该节点 incoming/指纹一致。
- reused：实际补丁结果等于其引用的已验证结果，且缓存复用顺序合法。
- budget-blocked：补丁原本可应用且合法，确因验证预算不足停止；不生成 outcome，也不增加验证次数。
- denied/apply-error/static-invalid：独立确认对应检查会在该阶段拒绝。

复用 patch::check_allowed/apply/validate，保留验证前去重；逐一核对身份和计数，避免只有父引用合法但尝试内容完全不相关。

G2：报告证据和成本必须绑定到真实重验。

当前 reports_match 忽略 states_explored、transitions_explored、analysis_started，只比较 diagnostic 的 property/outcome。把所有存档状态数及派生总数改为 0 仍能通过；清空反例/阻塞事实也能通过。

对固定构建、相同语义配置的重放，比较真实重验计数及分析标记，并校验结构化诊断的反例、具体绑定、阻塞/持有事实、边界事实。可采用规范化结构比较或从初始状态连续回放并核对失败终态；人类可读描述无需作为语义依据。总成本从已核实的报告推导。

明确跨版本策略：若历史计数或轨迹不能复现，应提供明确不兼容/未复现结果，不得输出表示整份 artifact 全部一致的成功。无需增加签名、认证或外部服务。

G3：终止状态必须由记录事实支持。

为 outcome、stop_reason、truncation、saw_unknown 与节点/尝试/预算建立共享规则：UNKNOWN 须有对应分析事实，预算截断须与真实预算及截断记录一致，未找到修复不能掩盖已经记录的预算阻塞。处理多种事实同时出现时采用与搜索器相同的确定性优先级。

验收涵盖：将正常未找到改成 UNKNOWN/预算耗尽，将预算耗尽改成未找到，以及仅把 stop_reason 改成 solved/错误预算原因，都明确拒绝。NoAcceptableCandidate 不应被解释成任意编辑空间下不存在修复；本轮无需增加全局最优或完备性证明。

验收与交接：

1. 针对三条性质加入端到端负例，保留上一轮 11 个已修复反例。residual_checks.py 中各例都应得到明确拒绝，错误信息能定位矛盾。同步篡改存档状态数与总数的例子专门检查独立重验，不应仅依靠记录之间的加和。
2. 同一构建的合法 artifact 继续通过：原本正确、修复成功、中间 FAIL、reused、denied、budget-blocked、无可接受候选、UNKNOWN、INVALID、UNSUPPORTED、InvalidConfig。
3. 针对性测试完成后运行 INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast。
4. 保持开发基准成本：two_cycles B=7/C=6 次验证，preserved_unfixable=4 次；若搜索行为改变需说明原因。
5. 更新文档，列清实际校验的规范性字段、跨版本规则和未支持保证。纠正历史数字：bbff35b 为 222/0；208/0 是此前的未提交工作区。
6. 交付真实测试命令、结果、G1–G3 修复位置和可运行重放命令。完成后独立复核，再进入实验预跑及独立样本集建设。
