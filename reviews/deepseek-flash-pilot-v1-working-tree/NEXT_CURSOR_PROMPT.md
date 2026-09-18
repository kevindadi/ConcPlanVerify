本轮目标：实现“LLM 提出一个受限 CIR 补丁 → Rust 验证完整冻结合同 → 拒绝反馈给 LLM → 接受产物独立回放”的最小闭环。先完成离线正确性，再用现有两个冻结缺陷模型做小规模真实 Flash 修复试验。不要扩展实验规模或引入 agent 框架。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/REPOSITORY_BOUNDARIES.md
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/deepseek-flash-pilot-v1-working-tree/REVIEW.md
以及同目录 evidence/audit.py、audit.json。

一、仓库职责与边界

- /Users/kevin/local-repos/ConcPlanVerify：Python；LLM、prompt、响应解析、反馈、多轮预算、工具编排。
- /Users/kevin/local-repos/ConcIR：Rust CLI；补丁上下文、hash、范围约束、应用、Petri 检测、完整合同验收和 artifact 回放。允许为外部补丁新增最小 CLI 协议，不添加 LLM/provider/prompt。
- /Users/kevin/paper-review/papers/ConcPlanVerify：论文与证据。新结果放 experiments/deepseek-flash-repair-v1/；旧 pilot 和历史产物原样保留。
- 先检查两仓 git 状态、局部 AGENTS 和现有实现，保留用户改动。记录构建输入与二进制身份；新功能需要新 release binary，不能仅用旧 binary 跑测试后宣称已验证新协议。

二、先补两个已复现的验收缺口

1. 对三个预声明生成任务增加任务级 fidelity 检查：入口、实际 scope 启动的两任务、指定共享锁/FQN/资源所有者、准确获取与释放顺序、跨模块声明。不要仅凭任意函数定义中的反序锁判断 faithful。最小回归包含删除一个 scope 成员、未启动的反序函数、不同模块的同名局部锁、提前释放第一把锁。保留原始模型与判定依据，不用 Python 重写一般 CIR 解释器；超出任务模板的结构标 unknown。modeling_mismatch 与 backend verdict 分列，不能输出需求成功；有效根模型冻结后不得以整篇重生成绕过修复限制。
2. 完善 replay payload 与 artifact 一致性：节点数、合法整数类型（bool 不能当计数）、outcome、接受节点、chain_len、accepted_ok。工作流报告 repaired 必须要求当前 artifact 绑定正确、回放成功且接受结果真正通过；合法失败 artifact 可以 replay 成功，但不等于 repair 成功。以审阅里的真实 artifact + 矛盾 payload 补回归，并保留真实 CLI 集成测试。

三、Rust：外部单补丁评估与回放协议

先用现有 patch.rs、repair/candidates.rs、repair/mod.rs、repair/search.rs 和 CLI 确定复用点。当前 legacy repair patches.json 的输出不能直接作为 replay artifact，不得凭 exit 0 接受。

提供两个逻辑能力，命令名可采用 repair-context 与 evaluate-patch，也可在清晰兼容的既有命令上扩展；在协议文档里固定输入、JSON schema、状态和退出码：

1. 导出修复上下文：输入冻结 model + contract；Rust 给出 model/contract fingerprint、允许范围、真实根验证与诊断、可定位的函数与 statement sid、Rust 算出的原函数 hash。包括模型必要上下文，但不输出预先求解的获胜补丁或把历史 accepted patch 当提示答案。Python 不复刻 hash/序列化/语义。
2. 评估显式外部 CirPatch：绑定冻结 model 和 contract、核查上下文 fingerprint/目标函数 hash/allowed_scope，再应用、静态检查、support、完整合同验证。只评估传入候选，不偷偷调用内部搜索帮它成功。

本轮每个候选仅允许一次相邻 mutex_lock 的 swap_statements，目标不同资源、非控制目标，完全复用 Rust 现有合法性约束。拒绝删除语句、整篇替换、修改 contract、改变启动/资源/保护关系、额外编辑或其他操作；不要增加修复操作种类。旧通用接口若已有其他操作可保持兼容，新受限接口必须自己 enforce，不能只靠 prompt。

每次候选都相对于同一个冻结初始模型，拒绝不更新当前模型；本轮不做多补丁累积搜索。UNKNOWN/UNSUPPORTED/预算耗尽/静态无效/越界/过期 hash/验证失败/工具协议错误分别记录。接受必须完整验证所有 properties 与 preserved；不能只让目标死锁消失就通过。

使用有版本标识的外部候选 artifact，或以明确区分的协议模式扩展现有 artifact；不把外部请求伪造成内部 A/B/C 搜索统计。复用验证实现，保留模型/合同、候选、实际配置、验证结果、来源和身份绑定。Rust replay 必须能够不联网从原输入重新应用并验证，拒绝被篡改的输入/contract/patch/报告/接受结论；支持接受与语义拒绝证据的复核。不可解析的原始 LLM 响应由 Python 保存为解析失败，不伪造 Rust artifact。旧 v1 search artifact 仍能回放，包括上一轮两个真实产物。

四、Python：LLM 补丁与反馈状态机

增加独立 repair provider/入口；复用 Flash 客户端和证据记录机制，保留生成与工具修复旧入口。

流程：读取冻结模型和合同 → Rust 上下文 → LLM 返回单个结构化候选 → Rust evaluate → 若拒绝且可继续，将真实结构化原因、原候选及冻结上下文反馈 → 最多三轮 → 接受后 Rust replay → 输出最终结果。

- patch prompt 明确 schema、sid、原函数 hash、允许操作与禁止改变合同；LLM 只选目标和修改，不决定验证结论。
- 不在 Python 隐藏修正目标/hash/sid 或把无效模型回答替换为枚举器答案。若采用由应用填入绑定元数据的设计，事先在协议写明固定字段与模型决策字段，并分别保存原始与提交内容。
- 解析/候选拒绝可以在预算内反馈；认证、模型身份、工具进程/协议错误停止。拒绝的候选与原因全部保留。
- 标签区分 candidate_source=llm、validator=tool、repair_mode=external_single_patch；离线 scripted 测试明确标 scripted。不能把确定性工具修复计入 LLM 成功。
- 成功关口要求 artifact 绑定、完整 PASS、replay 接受确认，冻结文件在所有退出路径上保持身份一致。重复候选按内容识别并记录；不得虚增成功数。
- 更新 README/CLI docstring，移除“尚未接通真实模型入口”等过时描述。

五、离线测试优先，全部通过才执行 live

必要场景：合法单 swap；错 hash/sid；越界；非法或多项编辑；修改输入/合同；静态无效；完整合同的 preserved 失败；UNKNOWN/UNSUPPORTED；缺/坏 artifact；矛盾 replay；篡改 artifact；预算终止。用真实 Rust CLI 验证新路径和旧 artifact 兼容。

用 scripted provider 明确演示“无效补丁被真实 Rust 拒绝 → 第二轮请求确实包含该拒绝原因 → 合法补丁被验证和 replay 接受”。测试不联网、不读取真实密钥；证明反馈传递而非只断言最终成功。保留现有 Python 全部 44 项与 Rust 相关测试，并运行适当的 Rust 全套回归。

六、只跑两个冻结缺陷样本的真实 Flash 修复 pilot

复用 experiments/deepseek-flash-pilot-v1/SUMMARY.json 指向的 t2_abba、t3_cross_module_abba frozen_initial.cir.json 和对应原合同。引用并校验原哈希；不重新生成根模型，不读取旧 accepted patch 给 LLM。新版本 backend 上先重新确认根完整 FAIL。

运行前冻结 PROTOCOL、任务/输入/合同/提示词 manifest 和结果字段。只有两个任务，每任务最多三轮补丁请求；全批最多 6 次实际 HTTP 请求（含失败及传输重试），最多 20 分钟。已有生成 pilot 的“剩余 9 次”不是本轮额外额度。

- 运行时只使用 ConcPlanVerify 根 .env 的 DeepSeek key，绝不打印文件、密钥或 Authorization，不把异常中的凭据写入日志。先完成本地 SDK/二进制依赖检查，避免将本地 import 失败混作实际 HTTP 请求；预算预留可以保守消耗，但须分清“预留次数”和“已发请求”。
- provider 固定 deepseek，精确模型固定 deepseek-flash；只能 Flash，禁止 Pro/其他 provider/自动 fallback。请求前检查，响应身份不一致立即停止。SDK 实际 base_url 与记录一致。
- 显式 thinking disabled，max_tokens=4096，单请求 timeout≤90 秒且不超过全批剩余时间；最多一次 transient 传输重试，SDK 不叠加重试。401/403/余额/模型/参数错误停止，不切模型。
- 全批共享持久化计数和 deadline；不能通过重启或新目录继续同一次实验以绕过上限。未完成也交付全部结果，不追跑到成功。
- 保存原始脱敏 messages、响应、requested/response model、request id、usage、耗时、finish_reason、轮次、反馈及 Rust 完整原始日志和 hash。缺 usage 记 null。
- 若第一轮即成功，如实写真实反馈重试未触发；离线脚本测试与真实模型证据分开。若失败，如实报告，不放宽 contract/范围/预算，不自动工具兜底冒充 LLM 成功。

七、交付后停止

新论文实验目录输出 PROTOCOL.md、SUMMARY.md/json、manifest.json、所有原始证据、离线测试记录和 LLM_PATCH_REPAIR_HANDOFF.md。报告两个仓库变更、CLI/schema、版本/二进制身份、复现命令、每例每轮接受/拒绝及来源、原合同保持情况、replay、请求/usage 与停止原因。记录对前一轮两个审阅问题的回归结果。

结论限定为“单步受限补丁闭环可行性”：两个样本不支持大规模效果结论，也不代表源码级修复。此次不做新 provider、多步 agent、大型 benchmark、论文实验结论补写或提交发布。完成后停止，等待独立审阅。
