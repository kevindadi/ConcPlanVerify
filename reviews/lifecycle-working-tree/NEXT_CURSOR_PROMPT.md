继续负责 /Users/kevin/local-repos/ConcIR。本轮任务是“修正三处实验有效性问题 + 真实案例建模 v0”，完成后停在可独立审阅的交付状态。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/lifecycle-working-tree/REVIEW.md
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/lifecycle-working-tree/evidence/probes.json
/Users/kevin/paper-review/papers/ConcPlanVerify/REPOSITORY_BOUNDARIES.md
复现代码在同审阅目录 evidence/probes.py。不要对原始实验目录执行破坏性复现；用独立临时目录。

一、固定仓库职责（用户明确要求）

- ConcIR 是 CLI tools：CIR、静态校验、Petri 转换、验证、诊断、确定性修复、artifact/replay 和不依赖 LLM 的模型实验。
- /Users/kevin/local-repos/ConcPlanVerify 才负责 LLM 调用、提示词、诊断反馈、迭代修改与端到端编排。不能在 ConcIR 加入 provider、prompt、LLM loop，也不能在 ConcPlanVerify 再实现验证语义。
- 用户允许未来直接清理 ConcPlanVerify 中无关内容。迁移时按依赖关系交付删除/保留/迁移清单，不再重复询问这项授权；原始实验和有价值的来源/LLM代码要识别后处理，凭据不进入输出。
- 本轮仍完成核心工具的真实案例 v0，不接真实 LLM，不为以后迁移提前做大规模删除。ConcPlanVerify 当前的 cir2cvn --analyze、旧平面 CIR schema 和旧翻译器不能视为现行 CLI 适配；之后单独迁移并做离线集成测试。

二、在跑新比较实验前，修复以下三个已复现问题

R1：失效旧证据仍占据 valid_index（P1）
- 场景：旧 B=complete/repaired；artifact 丢失；同 cohort resume 后新 search exit=2、no_artifact；旧 complete 因 _best 优先级仍留在 index，summarize 返回0并计入成功。
- 选择有效记录之前先验证旧证据资格。缺文件/hash不符/字段或绑定不完整的记录保留历史，但退出有效成功集合。重试失败、预算不足也不能保留失效成功。
- _commit 的注释声称执行完整性校验，当前实现没有。落实统一的、按记录类型定义的完整性检查；summarize 应拒绝失效 complete 的成功统计。不要每次统计重新执行昂贵 replay，只核验证据及其绑定。
- 回归覆盖：旧 artifact 缺失与 hash 失配后，新 attempt 失败；失效记录不得继续算成功。没有完整证据的 repaired 必须被拒绝。

R2：普通/恢复路径耗时不可比（P2）
- 普通 wall_ms=search；recovery wall_ms=search+replay，但 analyzer 直接比较二者。实测同样 search9ms/replay5ms，正常A/C记9ms，恢复B记14ms。
- 统一字段语义。策略性能比较明确用 search_wall_ms；replay_wall_ms、端到端/累计重试成本分别定义和报告，不混为一个指标。
- 同一个合法搜索结果，无论是否补做 replay，搜索性能统计应不变；恢复与重试本身的时间不能丢失，放在明确命名的独立成本项。
- 旧批次保持冻结，不静默改口径；新表格明确口径和可比较范围。

R3：配对把 timeout 丢掉（P1）
- 同一身份/repeat 的 B=repaired、C=search_timeout，目前 summarize 报 pairs=0、success_differs=[]。
- outcome/success 比较从完整计划与有效记录构建，保留 timeout、UNKNOWN、no_acceptable、工具失败等实际结局；not_executed/证据无效单独列出并明确分母。
- 成本比较可单独使用 both_repaired 子集；不能先过滤到 complete 再声称是全部策略配对。
- 表格至少明确：planned pairs、已执行双方结局、单边未执行/无有效证据、双方 repaired，以及结局差异。重复次数不是独立案例数。
- 回归覆盖 B成功/C超时、B超时/C成功、单边not_executed、双方成功；验证成本子集正确，结局差异不漏记。

只针对这些问题补真实路径测试，复用现有实现，不新建泛化实验平台。保持既有 lifecycle/behavior 回归通过。源数据冻结于原目录，新输出用新 cohort/batch。

三、建立有来源的真实并发问题小样本

目标是建模可行性与工具适用性检查，不是正式 held-out 评估，也不是验证整个上游项目。

1. 候选调查
- 优先调查真实 Rust 并发问题；至少查阅4个候选，来自至少2个独立上游项目。只使用能核实的源码、issue、PR/commit 和原始 reproducer。
- 每个候选记录稳定 URL、仓库、buggy/fixed commit SHA、相关路径/行与片段哈希、原始问题机制、上游修复思路、许可/引用来源。
- 若不能核实 fixed revision，明确留空并解释，不能编造。若网络不可用，记录实际阻塞，不把本地合成例子冒充真实案例。
- 按当前 CIR/后端支持范围决定是否能建模；保留排除与不支持清单及具体语义原因。不能只保留 A/B/C 修好的案例。
- 同一上游缺陷的变体、缩减和不同配置属于同一来源组，不计为不同真实缺陷。旧 ConcPlanVerify benchmarks/rust 只有核实来源后才可归入真实案例。

2. 先做2个可审查的最小模型，前提是来源和映射成立
- 不为凑数量扭曲源程序；不足2个时如实交付调查清单、已完成模型和阻塞证据。
- 每个案例提供 provenance.json、原始相关片段或获取脚本、buggy.cir.json、fixed.cir.json（确有可建模的上游修复时）、冻结 contract.json、mapping.md、预期结果与来源依据。
- mapping.md 逐项对应：线程/任务、资源身份与共享关系、锁/解锁和RAII释放点、等待/通知、调用/作用域、关键控制流、preserved properties、可编辑范围。列出省略的代码、有限边界、调度/内存模型假设与适用限制。
- 特别说明缩减是否可能产生伪反例、遗漏反例或改变可修复性；不能声称源码→CIR 完整语义等价，除非另有证据。
- 预期来自源程序/issue/上游修复，不能把本工具输出反填为 ground truth。独立记录“缺陷可表示”“验证可完成”“当前 patch 空间可修复”，这三件事不能混淆。
- 上游修复模型不自动等于合法 repair witness。只有通过现有 patch API 和 allowed_scope，并在同一冻结 contract 下验证的编辑链，才称为合法 witness；不在当前相邻锁交换能力内则明确报告，不能为了修好改权限或扩充核心操作。

3. 小规模验证
- 复用现有 manifest/CLI/runner/auditor，产物放 experiments/real-cases-v0/，不要复制一套验证或统计实现。
- 运行前写 PROTOCOL.md，固定入选/排除标准、模型哈希、contract、A/B/C 配置、状态界限、搜索/replay timeout 和总预算。初轮每案例每策略1次用于可行性，不做统计显著性或稳定速度结论。
- 对 buggy 和 fixed 模型先运行静态检查及根验证，记录 outcome 和 report_complete；用适当小案例交叉检查 interpreter/Petri 结果。当前不支持的模型保留 unsupported/限制说明。
- 再运行有合法输入的案例 A/B/C；保存所有成功、失败、UNKNOWN、timeout、not_executed 和排除原因；complete repair 通过 replay。
- 接受验证始终使用原冻结 contract。若需扩大分析界限，只能作为另一个显式配置/批次，不覆写旧 contract 或把结果混在同一比较中。
- 若某案例无法在预算内完成，报告实际界限，不持续调参直到成功，不因此加入 POR/缓存/新搜索算法。

四、交付与停止条件

交付：
- 三项修复与回归的对应表，以及必要的核心 offline 测试结果。
- experiments/real-cases-v0/PROTOCOL.md、候选来源清单、纳入/排除记录、逐例映射与冻结输入。
- 全部有效索引、attempts、artifact/replay/根验证证据、自动生成的结果表。
- REAL_CASES_V0_HANDOFF.md：来源有多少、实际建模多少、根验证结局、A/B/C结局、合法修复多少、未支持/未完成什么；独立案例数与运行数分开。
- 确认旧 pilot 数据未改动，ConcIR 未增加 LLM 代码；下一阶段接口边界与潜在适配缺口简要记录即可。

完成上述交付后停止，不自动进入真实 LLM 实验、全项目源码解析器、ConcPlanVerify 重构或新的后端算法。本轮的研究产出应是“真实问题→明确假设下的CIR→验证/修复证据”的可追溯链。
