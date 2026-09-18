请在 /Users/kevin/local-repos/ConcIR 完成本轮收尾，只修复 artifact/replay 内部一致性与预算截断记录，不再扩展搜索能力。

基线提交 bbff35b869b19f0fe05f2f68ff7f987c89256b5a，独立全量测试 222 passed / 0 failed。上一轮 E1–E7 的主要修正已经通过，保留这些实现。E8 仍有 F1–F3 漏检，正常预算停止还有 F4 漏记。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/bbff35b/REVIEW.md

同目录有实际输入、输出、summary.json、positive-summary.json、probes.py，均是独立复现证据。不要只根据已有测试通过就宣布完成。

本轮范围：

1. F1：把最终结果绑定到真正的修复链。

当前 replay 忽略 artifact.patch_chain，单独验证 accepted_program。因此清空链、损坏链的函数基准、仅向 accepted_program 添加与链无关的资源，都错误返回 repaired/accepted_ok=true。

明确接受节点身份。从 input_program 应用每个最终补丁，核对基准、父子指纹和代价；结果须与接受节点、accepted_program 和 accepted_report 一致。校验整体 RepairOutcome 与根状态、接受结果是否相容。没有链的 repaired、链终点不是接受结果、报告与终点不一致都要拒绝。

2. F2：补丁权限与冻结契约必须在 replay 中重新检查。

对每个需要应用的补丁调用统一权限检查，不能仅 patch::apply。在线搜索和 replay 使用相同契约规则。

校验各 VerificationReport 中的 model/contract fingerprints、assumptions、bounds、complete、required/preserved 性质等规范性字段与独立重验一致。只比较总体 outcome 不够。把 allow_lock_reorder 改成 false 或删除 preserved、却保留旧补丁和报告的 artifact，必须明确失败。

要求是记录内部一致性，不要求加入加密签名或认证服务，也不承诺阻止能一致地重写全部证据的人。

3. F3：验证所有公开的规范性字段，不把部分检查当完整验收。

先做结构校验，再进行昂贵重验：
- 节点 ID 唯一、根结构合法、父引用存在且无环，depth/total_edits 与边一致。
- incoming 的父/子指纹、函数基准与实际重建结果一致。
- attempts 的 ID、parent、patch、result、reused_node、指纹和 outcome 互相对应。验证、拒绝、复用、预算阻塞应有明确类型/规则；拒绝尝试不能被当成成功节点。
- effective_config 与冻结契约的实际 bounds 一致，预算/策略与记录相容。
- 从完整记录推导 proposals、verification_calls、cache_hits、状态累计数等可推导计数，与 counts 对照；明确根是否计入 unique program。
- 存档报告和 accepted_report 的规范性内容必须与重验一致。跨构建版本不宜直接比较的内容，应明确比较规则或版本不兼容状态，不能忽略矛盾后仍报告全面验收成功。

报告中的 bad_attempt_parent、false_counts、false_effective_bounds、false_node_complete、false_root_properties、bad_incoming_result_hash 都应被拒绝。合法的原本正确、未修复、UNKNOWN、INVALID、UNSUPPORTED 和预算耗尽 artifact 也要有明确可校验语义；InvalidConfig 的零节点结果需要单独处理，不要强行要求存在已验证根。

4. F4：记录被验证预算阻塞的候选。

two_cycles --strategy b --verification-budget 1 当前输出 proposals=1、attempts=[]。保存已生成候选的 parent、patch 和 budget-blocked 原因，不能伪造验证结果或新节点。保证计数口径一致；保持重复候选不消耗新验证预算的既有行为。

5. 验收与交付。

针对 F1–F4 加入真实端到端负例：每次只损坏一个规范性字段，运行 replay，必须非零退出并提供可定位原因；不要依靠其中另一个损坏字段碰巧拒绝。

增加正例：干净 artifact、包含 reused/denied/budget-blocked 尝试的 artifact，以及原本正确和各未成功状态；保存后由另一次读取独立重建。保留复杂类型往返、两步修复和以前语义回归。

先跑针对性测试，再运行：
INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast

复核运行 B/C 的 two_cycles：目前 B 为 7 次验证、C 为 6 次；preserved_unfixable 为 4 次验证。此次收尾不应无故改变搜索语义或成本；若有变化请解释。

更新 backend-usage/design、REPAIR_LOOP_HANDOFF.md，写明 replay 实际保证、各状态与版本比较规则，逐项列出 F1–F4 证据和真实测试结果。纠正历史基线：上一轮 208/0，更早 29ee9ed 为 192/4。

不引入 LLM、新原语、POR 或新的修复算子，不修改 ConcPlanVerify，不开始大规模论文实验，不编造测试数据。完成后交给 Codex 独立复核；通过后再进入实验预跑和独立样本集建设。
