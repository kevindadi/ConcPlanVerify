请在 /Users/kevin/local-repos/ConcIR 修复本轮组合修复闭环的独立审阅问题 E1–E8。

先读完整审阅与复现证据：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/repair-loop-working-tree/REVIEW.md

本次审阅基于 HEAD 29ee9ed6133052d5adeeb30c686f0e5d36b99e6d 上的工作区，包括新增 search/benchmark 文件；具体文件 SHA256 见同目录 source-manifest.json。若当前已提交或有后续修改，先确认差异，保留已有工作，不重置、不回滚。当前全量测试是 208 passed / 0 failed，但独立边界探针仍暴露以下问题。

本轮目标：使已实现的两步组合修复具备可靠的导出、预算、去重、搜索记录和 CLI 行为。保留已有原始契约验证、FAIL 中间节点扩展及完整 PASS 才接受的逻辑。不要重写解释器或 Petri 内核，不新增原语、LLM、POR，不修改 ConcPlanVerify 仓库，不扩充论文规模实验。

1. 修复复杂类型序列化（E1）

ComplexBaseType 当前输出元组数组，反序列化要求单键对象。修复共享 AST 序列化，使 bounded Int、Enum、Struct、Array 和嵌套组合都符合既有输入 schema；覆盖资源、局部/参数/返回类型等实际承载位置。不要弱化类型或在 CLI 临时转换。

加入有语义断言的 Program/contract/patch 往返回归，以及含复杂类型的锁修复案例：原输入合法且 FAIL，导出最终程序后重新读取，对完全相同的冻结契约完整 PASS；类型、值域、性质必须一致。

2. 明确验证 bounds 的唯一来源（E2）

SearchConfig.bounds 目前未被使用；把 max_states/max_depth 改为 1 仍会进行默认规模的搜索。优先消除重复配置，以 ContractSpec 的冻结 bounds 为权威并导出实际配置；如果保留独立执行上限，明确合成规则、在根与每个子节点一致执行并记录，不能静默忽略或放宽原契约。禁止仅为了测试通过而偷偷修改性质或 assumptions。

针对极小 states/depth/thread/frame 等实际支持边界验证 UNKNOWN/INVALID 分类；不能报告超出实际执行保证的 PASS。

3. 在验证之前完成去重（E3）

在合法补丁应用后、调用昂贵验证之前识别已经验证过的等价候选程序。重复候选不能消耗新的验证预算，也不能重复入队。以相同的冻结契约和实际分析配置为前提复用结果；保留来源和被复用节点的引用。

明确定义 generated proposals、unique candidate programs、actual verification calls、cache hits/duplicates 的计数口径。不要混淆候选预算与验证预算；相同配置下顺序和停止原因保持确定性。

回归覆盖交换后逆向交换、不同顺序得到同一复合结果，以及诊断/无诊断两种策略。当前 two_cycles 的 B 为 8 次验证但仅 7 个不同程序，preserved_unfixable 为 9 次验证但仅 4 个不同程序；新增测试应证明重复验证已消除，而不只检查去重容器长度。

4. 修复搜索记录身份并支持独立重建（E4）

records 和内部 nodes 的下标目前混用；拒绝记录还固定 id=0。采用统一稳定的节点 ID，或明确区分 node/attempt ID 并建立引用，不允许两套编号隐式混用。

记录真实父节点、父程序指纹、incoming patch 及基准、结果指纹、验证结果、代价和停止/拒绝/复用原因。没有产出新程序的拒绝尝试不应伪装成一个无父节点的根。重复尝试可以引用已存在节点，但父边必须仍指向本次实际来源。

验收须从序列化 artifact 独立重建并验证中间节点；不能只比较 depth 或成功链。覆盖 preserved_unfixable 中记录 7/8 错指向记录 3 的反例，以及 allowed_scope.modules 禁止当前模块时多个 id=0 的反例。

5. 统一预算边界与停止原因（E5）

verification_budget=0 当前仍验证根节点一次。明确根验证计入预算：预算不足时执行前返回明确状态或校验错误，不能超额。

B/C 因 max_depth/max_total_edits 截断时应保留具体截断原因，不能与策略候选已经穷尽混为一谈。区分 A 的单步策略限制与 B/C 的外部预算。若截断与 UNKNOWN 同时出现，保留两种事实并给出确定的主状态。

测试 0、1、恰好够用、不足一步等边界；修复后验证 two_cycles 在合理的去重预算内能成功。所有未找到结果仅表示当前策略/预算内未找到，不声称不存在修复。

6. 恢复 CLI 行为（E6/E7）

旧命令 repair model contract patches.json [budget] 的预算参数在 args[5]，当前误读 args[6]。修复并测试省略、0、1、非法数值/多余参数。

repair --strategy 的根 INVALID/UNSUPPORTED 应按公开约定返回 4/5；UNKNOWN 为 3。不能统一压成退出码 1。覆盖 A/B/C，并保留完整 JSON 诊断。统一公开文档、usage 与实际行为。

7. 交付完整复现 artifact（E8）

当前 repair JSON 与 bench JSON 的字段各缺一部分，Handoff 却声称都完整。设计共同结果格式，或在摘要中引用完整 artifact；不要仅依赖进程内 Node。

至少保存：输入 CIR、冻结 contract、实际 search/analysis 配置、可区分本次源码的版本信息（提交与 dirty 内容指纹，或等价源码标识）、全部节点/尝试及父子补丁关系、完整验证报告、最终 CIR、补丁链、各计数和停止原因、可复现命令。根 INVALID/UNSUPPORTED/UNKNOWN 也要保留报告。包版本 0.1.0 不能单独充当源码版本。

bench 的 accepted_chain 当前只是长度，请明确命名并关联真正的修复 artifact。复现工具不应依赖已退出进程的内部对象或未保存的输入。若提供 loader/replay CLI，必须真实读取 artifact，校验输入/契约/补丁基准并重新验证。

加入结果保存→另一次读取→重建中间节点→验证最终程序的端到端回归，以及损坏父引用/补丁基准/输入指纹等错误必须显式失败的检查。不要把记录丢失自动补成成功。

8. 重建基线并交接

保留此前语义回归与 4 个 DOT goldens，不跳过测试，不更新快照来掩盖语义失败。

先跑针对性 E1–E8 回归，再跑：
INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast

修复后重新运行 8 案例 × A/B/C 开发基准，报告成功结果、实际验证次数、去重命中、状态数、成本和截断原因。去重前的 8 对 6 次验证不能直接沿用为诊断策略收益；更新文档中所有受影响数字。开发基准仍不作为论文独立实验集。

更新 backend-design、backend-usage 和 REPAIR_LOOP_HANDOFF.md，使实际 schema、预算和保证完全一致。交接说明逐项列出 E1–E8 的修复位置、真实命令/结果及剩余限制，给出一个可直接执行的完整 artifact 重放命令。不要编造测试或性能结果。

完成后交由 Codex 独立复核，再决定是否进入规模实验和 LLM 接入。
