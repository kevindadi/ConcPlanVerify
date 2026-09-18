请在 /Users/kevin/local-repos/ConcIR 做一次范围明确的终止语义修正，基线 41b2d09480ecca178fc3110c0f8a956c8c553220。

独立复核：227 passed / 0 failed / 0 warnings；上一轮 14 个反例全部正确拒绝，G1/G2 修正通过，默认正例及 B/C 搜索成本稳定。剩下 H1/H2 都在 G3 终止分类，不涉及内核或修复算法扩展。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/41b2d09/REVIEW.md

同目录 boundary_matrix.py、boundary-matrix-summary.json、terminal_flags.py、terminal-flags-summary.json 提供独立复现。

1. 修复 H1 的正常记录误拒绝。

策略 A 只扩展根。preserved_unfixable 使用 --strategy a --max-depth 1 或 --max-total-edits 1 时，搜索器正常穷尽单步候选并返回 no_acceptable_candidate；导出的 artifact 应成功重放。当前 replay 用最大生成节点的深度/编辑数达到上限推断截断，导致错误拒绝。

保留 A 的单步语义和合法的 NoAcceptableCandidate 结果。根据实际扩展决策判断是否被预算截断，区分到达边界与因边界停止。B/C 需要继续扩展却遭遇同样预算时，仍应正确报告 BudgetExhausted。

2. 修复 H2 的终止字段遗漏与优先级不一致。

在线搜索与 replay 共用终止语义/优先级定义，依据可独立核实的搜索事实生成或验证完整四元组：outcome、stop_reason、truncation、saw_unknown。不要仅在 replay 添加互不关联的字符串分支。

必须拒绝：删除真实 truncation、凭空加入 truncation、根 UNKNOWN 却 saw_unknown=false、depth/edit 同时到界时把 stop_reason 改成不符合实际优先级的另一项。若同一搜索在不同节点上实际产生多类截断，要按照真实事实和既有优先级处理，不能仅凭最终最大深度猜测。

必要时增加明确的可校验终止/扩展事实，保持 schema 和文档一致。记录字段不是自身的真实性证据；不要仅信任新增标记。

3. 用固定验收矩阵收敛。

- preserved_unfixable：A/B/C × 默认、depth=1、edits=1、两者=1、depth=0、edits=0，共 18 个由真实搜索原样生成的 artifact，全部成功重放；原搜索成功/失败类别符合策略。
- 对实际损坏的终止字段，terminal_flags.py 的四例全部明确拒绝。
- 保留原本正确、修复成功、UNKNOWN、INVALID、UNSUPPORTED、InvalidConfig、权限拒绝、缓存复用、验证预算阻塞的合法重放。
- 保留之前 11+14 个反例及复杂类型、语义回归。

通过针对性测试后运行 INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast。保持 two_cycles 的 B=7/C=6 次验证、preserved_unfixable=4 次；若改变需解释。

更新 Handoff 和 backend 文档的终止保证与实际测试数字：bbff35b 是 222/0，当前基线是 227/0。交付具体修复位置、真实测试命令/结果及 18+4 验收结果。

本轮不增加新原语、LLM、POR、搜索策略或其他审计功能，不改 ConcPlanVerify，不启动规模实验。完成这一终止语义修正后交由 Codex 独立复核，再进入实验预跑和独立样本集建设。
