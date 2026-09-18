# Cursor 下一轮实现任务：修复 ConcIR 后端的语义与验收漏洞

请在 `/Users/kevin/local-repos/ConcIR` 继续工作。

先阅读：

`/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/736f7e1/REVIEW.zh.md`

反例、契约、候选补丁和原始运行结果在同目录 `repro/` 中。审查基线是提交 `736f7e1d4c46d44b6baabff5d6a116dd253f95be`。保留已有工作；如果代码已经变化，先确认各问题是否仍存在。

两个仓库职责不变：核心工作在 ConcIR，暂时不修改 ConcPlanVerify，不接入 LLM。不要开始论文实验、扩大原语支持或加入偏序约简。本轮目标是消除错误 PASS、错误修复接受和引擎分歧，并建立有效的正确性回归。

## 工作顺序与必须完成的修正

### 1. 先复现，再加入失败回归

将审查提供的 CIR/contract/patch 反例迁入本仓库测试夹具，给每个案例写清正确的期望和理由。先验证它在修复前确实暴露问题。不能将原始输出中的错误 PASS 作为 golden 结果。

测试至少断言：静态合法性（或预期静态错误）、探索完整性、双方验证结论、目标/阻塞事实以及 CLI 退出码。原始审查目录保留，不覆盖其记录。

### 2. 修正实例隔离（R1）

- handle 的名字绑定归属于 Frame/activation，不能存在整个 Thread 的共享名字表。
- 具体句柄和子线程身份应准确配对，返回、join、scope、递归调用时生命周期明确。
- `nested_handle_false_pass` 必须由 PASS 改为 FAIL；改名后的对照程序也为 FAIL。
- 对调用者和被调用者的局部 handle、local、param 做 alpha-renaming 回归，性质结果必须不变。
- 防止一次修复只在解释器或 Petri 引擎生效。

### 3. 修正不可变契约与补丁权限（R2、R3）

- 保存原始符号化 ContractSpec，每个候选重新 lower 后，重新解析同一份规格；或者实现具有同等保证的稳定 ID 方案。
- 不能把旧程序的 body 下标直接用于新程序。目标不存在就拒绝候选，不得转向另一语句、丢弃目标或自动削弱。
- 同时覆盖 StatementReached、ScopeCompleted、语句交换、删除前序语句、目标删除、跨模块目标。
- 所有 CandidateProvider 的补丁都经过统一的权限检查：完整 module::function 范围、每个操作类别和 allow_* 配置。
- `allow_statement_delete=false` 和 `allow_lock_reorder=false` 对文件候选也必须强制执行。
- 候选去重应使用规范化变更内容或候选程序内容，而非只依赖用户任意指定的 id。
- 同一目标上的冲突或重复修改要明确拒绝或有可解释的顺序语义，不能依靠 Debug 字符串碰巧识别。

### 4. 修正同步选择与模板遗漏（R4、R5、R9）

- notify_one 枚举所有当前等待者的合法选择。实现的遍历顺序可以确定，但不得因此删除程序的非确定性。
- `notify_choice_false_pass` 必须发现选择 w2 后 w1 永久等待的反例，而不是继续 PASS。
- 审计 mutex/semaphore/channel 中固定队首、立即移交和批量推进的规则。FIFO 消息顺序不等于等待线程必须 FIFO；不得无依据地把竞争步骤合并为强制原子操作。
- 对每类移交明确其线性化点与可观察状态。若保留特定 FIFO 原语策略，必须是显式、受检的语义配置，不能替代通用 CIR 原语的默认语义。
- notify_all 在等待集合为空、甚至没有任何静态 wait 位置时仍应正常前进。`notify_all_without_wait` 两个引擎均 PASS。
- 对空、无副作用入口和普通 call/spawn 的空函数采用同一支持策略。`bodyless_entry` 不能再一边 INVALID、一边 PASS。
- 不要通过让参考解释器复制网的错误行为来消除差异。

### 5. 统一验证入口和结论边界（R6、R8）

- 提供从 Program + ContractSpec 开始的受检公开入口，统一执行静态验证、语义支持检查、配置验证、契约绑定、探索与性质检查。CLI 和修复验收复用此入口。
- `invalid_protected_write_fixed_fixture` 必须返回 Invalid，不能因为 lower 成功就继续给 PASS。
- 不支持 sequential_consistency=false 或 no_spurious_wakeups=false 时，明确 Unsupported/配置错误；不能静默忽略。
- 定义合法的 bounds、property id 和谓词资源类型；错误输入返回结构化错误，不 panic。
- 报告包含模型/契约指纹、实际语义配置、分析边界与完整性。
- CLI 只有总体 PASS 返回成功；Fail、Unknown、Invalid、Unsupported 都应有文档化的非零退出码。保留 JSON 中的详细类别。

### 6. 使有限程序能形成有限状态图（R7）

- 避免未被性质观察的累计调用次数、历史线程记录和永久递增身份使简单循环不断产生新状态。
- 按契约需要生成有限监视器：完成一次可用 bool；AtLeast(n) 可按最大相关阈值饱和。列清哪些观察量需要保存。
- 安全地回收/复用/规范化线程、帧与句柄身份，避免陈旧句柄混淆；不能直接忽略影响后续执行的信息。
- `finite_call_loop` 在合理小边界下应探索完整并对 deadlock_free 返回 PASS，而不是靠 max_depth 截断为 UNKNOWN。
- 真正越界、无界数据增长、递归栈增长和搜索预算耗尽仍必须给 Unknown，不能伪装为完整。

### 7. 重建真正的差分验收（R10）

- 当前 petri_projection 只比较 shared_*；完整 project_* 未使用。替换此薄弱断言，不要删掉 dead_code 警告来掩盖问题。
- 规范化投影保留控制位置、调用栈、局部值、帧内句柄、具体等待关系、消息及其值、完成监视器和阻塞/终止事实。
- 比较可观察边关系或适当的弱步关系；不能只比较可达状态集合或状态数。
- 明确辅助网步骤的投影与 stuttering 条件，尤其说明对 EF/AG EF 的影响。
- 差分测试先断言双方探索完整、没有 Invalid/Unsupported/边界事件，再比较状态与行为。
- 增加小程序系统枚举和元变换检查（局部改名、模块改名、独立声明重排）。不依赖两套引擎共用同一个同步 helper。
- 除了两者一致，还要用手工定义的正确期望捕捉共同错误，例如 R1 和 R4。

## 测试与交付

按逻辑修正分阶段提交工作，每阶段运行针对性回归。最终运行完整测试，保留真实输出。原有 4 个 DOT snapshot 失败需与新增失败区分；若一并修复，人工检查生成的图再添加正确 golden，不得直接跳过测试。

更新 doc/backend-design.md、doc/backend-usage.md、CODE_REVIEW_HANDOFF.md，明确语义变化、兼容性、已实现范围和限制。

新增 `CODE_REVIEW_ROUND2.md`，逐条列出 R1–R10：
- 根因及修改位置；
- 复现用例和修复后期望；
- 实际执行的命令与结果；
- 引擎独立性和状态有限化依据；
- 仍未解决的问题。

保留无 LLM 的锁顺序修复演示；同时证明禁止的补丁、目标被删除的补丁、出现新错误的补丁和 Unknown 候选都不会被接受。

不要用修改测试期望、弱化契约、强制 FIFO、默认 Unknown 值或共享错误语义让测试通过。不要宣称完成形式证明；交付可审查的实现与实际证据。完成后交由 Codex 再次独立审查。
