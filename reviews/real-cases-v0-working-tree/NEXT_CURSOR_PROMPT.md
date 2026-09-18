本轮主任务：在 ConcPlanVerify 代码仓库实现现行 ConcIR CLI 的离线适配与 Python 工作流，为后续 LLM/agent 接入建立明确边界。先读审阅和职责文件，直接实施，不只给方案。

/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/real-cases-v0-working-tree/REVIEW.md
/Users/kevin/paper-review/papers/ConcPlanVerify/REPOSITORY_BOUNDARIES.md

一、严格区分三个目录

1. /Users/kevin/local-repos/ConcPlanVerify
   Python为主的应用代码仓库。放LLM/provider接口、提示词、CIR生成、反馈驱动的修改/修复编排、CLI client、工作流或agent、相关单元/集成测试。
2. /Users/kevin/local-repos/ConcIR
   Rust工具代码仓库。提供CIR、静态校验、Petri转换、验证、诊断、确定性修复、artifact/replay。上层经CLI调用，不在此加入LLM交互代码。
3. /Users/kevin/paper-review/papers/ConcPlanVerify
   论文与研究产物目录。放实验结果、迁移审阅、迭代记录、下一轮prompt。不要把应用源码写在这里。

本轮应用代码改动主要在第1个目录；CLI仍由第2个目录提供；交付证据放第3个目录的新目录 experiments/offline-integration-v1/。应用用 --out 接受任意输出路径，不把上述机器路径硬编码为默认依赖。

二、迁移旧代码，清理重复后端

- 先检查git状态、现有README、python/cir_workflow及测试、Cargo/src/unipn、旧实验/脚本的引用关系。
- 用户已授权直接删除ConcPlanVerify中确定无关的内容，无需再次请求清理许可。清理旧cir2cvn翻译/验证入口、重复Rust实现和只服务旧协议的依赖/脚本；根据引用关系决定具体删除对象。
- 不因名字或未跟踪状态直接删除整个目录。有价值的历史实验、唯一来源证据和可复用provider代码应保留或清单化迁移到论文目录；不触碰凭据，不删除.git。
- 可复用现有Python包，不为改名重写全部工程；移除旧schema/default-binary/--analyze/verified_safe等协议假设。淘汰旧测试时注明对应的废弃功能，新入口必须有实际回归覆盖。
- 交付迁移清单，说明删除、保留、替换的路径及原因；README只描述当前可用功能。

三、实现现行CLI的类型化Python适配

以 /Users/kevin/local-repos/ConcIR/src/bin/concir-backend.rs 和 doc/backend-usage.md 为准，不凭旧README猜协议。至少支持：
- check <program.json>
- support <program.json>
- explore <program.json> <contract.json> petri|interp
- repair <program.json> <contract.json> --strategy a|b|c ... --artifact <path>
- replay <artifact.json>

要求：
- 二进制路径显式可配；缺失时给出可操作错误，不自动构建旧cir2cvn。每次实验记录实际binary hash、代码版本/dirty身份、输入/contract hash、配置、命令、退出码、stdout/stderr、耗时和artifact。
- 子进程使用参数数组；文件路径可含空格；超时停止子进程及其子进程组。每次调用独立目录，旧artifact不能冒充本次结果。
- 区分进程/协议错误和后端语义结局。按各子命令协议核对exit与JSON；exit1/3/4/5不一概当成崩溃，exit0也不一概当成验证PASS。非JSON、空输出、缺文件、spawn error、timeout有独立状态。
- 保留PASS/FAIL/UNKNOWN/INVALID/UNSUPPORTED、report.complete、边界信息、诊断、计数。FAIL且incomplete可能已经有反例；UNKNOWN不等于安全；support/check成功不等于性质成立。
- repair outcome区分repaired/already_satisfied/no_acceptable_candidate/budget_exhausted/analysis_unknown/invalid/unsupported等实际值。repair产物必须与本次输入、冻结contract和配置绑定，合法退出映射及replay通过后才能作为已核验修复结果。
- 输入绑定复用Rust已有规范化/验证机制，不另写Python版CIR语义。若确有CLI能力缺口，明确报告；不得静默跳过绑定或模拟后端成功。

重要接口差异：当前flag策略repair输出可replay的搜索artifact；位置参数repair model contract patches.json [budget]仅输出legacy report。不要把它包装成同一种artifact，也不要假设外部LLM patch已经支持composite/replay。本轮先完成策略模式的严格闭环，外部patch协议缺口列入后续设计。

四、实现最小离线工作流，保留未来agent扩展点

不引入大型agent框架。可用显式状态机/普通Python类。提供接口清晰的provider和scripted/mock实现，真实provider调用本轮不启用、不读取密钥、不发网络模型请求。

最小流程：
1. 读取requirements和调用者提供的contract，冻结contract；本轮不让provider生成/修改contract。
2. scripted provider产生CIR JSON，保存原始响应；解析及静态检查失败时，将结构化反馈和原始requirements交给provider，在明确的最大次数内重试。
3. 一旦CIR通过静态检查，保存并冻结本轮初始CIR，做support/explore。
4. PASS按完整性条件结束；UNKNOWN/UNSUPPORTED/工具错误分别停止并报告，不能通过放宽contract或删除行为获得通过。
5. FAIL时生成结构化诊断反馈；本轮使用ConcIR策略repair完成确定性修复基线，再replay核验artifact，输出修复结局与证据。

明确记录每一轮candidate来源（scripted/LLM/工具）。第5步的成功是“后端确定性修复”，不能标成“LLM修复”。生成阶段的整模型重试不等于已验证初始程序后的任意整模型替换权限。初始CIR冻结后，禁止绕过后端allowed_scope直接替换完整程序并将PASS称为合法repair。

提示词/feedback模板放在ConcPlanVerify Python代码仓库，使用当前模块化CIR格式（modules、sid、kind、FQN等），参考现行Rust AST和文档；不继续使用旧平面op/transfer模式。记录模板版本/hash。反馈保留性质id、相关函数/语句、反例/边界信息、未满足preserved properties；不得把未知/工具错误改写成“无缺陷”。

离线演示至少包括：
- scripted先返回坏JSON，再返回合法CIR，证明反馈重试真正被使用。
- 已正确模型直接结束，不调用repair。
- 一个现有可修复案例：FAIL→工具repair→artifact→replay成功。
- bounds UNKNOWN、RwLock UNSUPPORTED分别停止。
- 预算耗尽/无可接受候选按原状态返回。

五、测试与证据

- 单元测试覆盖各命令exit/JSON映射、空/坏输出、deadline、含空格路径、缺失或过期artifact、contract不变、有限重试和provider来源标签。
- 集成测试必须调用真实ConcIR binary，使用已有小fixture；不要把所有CLI调用mock掉。验证PASS/FAIL/UNKNOWN/UNSUPPORTED、受限修复和失败replay的处理。
- 构建/依赖准备与运行成本分开。离线验收不得要求API key或真实provider网络可用。旧provider可以保留，但不能在import或测试时自动发请求。
- Python客户端只负责协议、编排和记录；不要复制Petri、语义解释器、candidate枚举、patch合法性或接受判定算法。
- 本轮不修改核心语义，不扩展RwLock/POR或修复操作，不将旧缩减模型说成完整项目验证，不重跑旧大批次。

六、随迁移完成几个小的研究台账收尾

如果继续使用ConcIR现有实验runner，只做以下明确修正，单独列出改动；新Python应用不要继承这些统计错误：
- attempted必须排除not_executed；repaired/not_executed另列执行缺失，不计作双方实际运行的胜负。
- cumulative cost按本attempt真正执行的search/replay阶段累计。replay-only恢复不能再次累计继承的search耗时；100ms搜索+20ms恢复replay应120ms，不能200ms。比较策略仍用明确的search成本。
- 实际生成器是real-cases-v0/build_cases.py，不能只给不存在的generate_cases.py记null。让生成代码身份显式可配/枚举并快照化；旧批次不回填为新的身份。
- Handoff“92 s”改为“92 states”，搜索时间原记录为8/8/9ms。通过新勘误记录纠正，保留原始结果。

七、交付

应用仓库：可运行Python入口、类型化CLI client、离线provider/工作流、现行模板、测试、使用说明和迁移清单。
论文目录 experiments/offline-integration-v1/：
- OFFLINE_INTEGRATION_HANDOFF.md
- REPO_MIGRATION.md
- CLI_CONTRACT.md（实际命令、字段、退出码、证据与已知gap）
- 测试命令与结果、离线演示原始记录及artifact/replay、manifest及hash清单
- REAL_CASES_V0_ERRATA.md（1个可修复issue缩减+1个支持边界；fixed hypothesis不是上游修复；单位与统计勘误）

完成后停止，报告哪些入口已经用真实CLI验收、哪些仍只是接口。下一轮再决定真实LLM接入和外部patch证据协议，不自动开展付费模型实验。
