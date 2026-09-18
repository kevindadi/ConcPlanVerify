本轮在 /Users/kevin/local-repos/ConcPlanVerify 实现并运行首个 DeepSeek Flash 小规模真实生成实验。用户已授权使用该仓库env中的DeepSeek key，但明确禁止Pro，先只用Flash。先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/offline-integration-v1-working-tree/REVIEW.md
以及同目录 evidence/probes.json、probes.py。

一、仓库与模型边界

- Python应用、LLM/provider、提示词与编排：/Users/kevin/local-repos/ConcPlanVerify。
- Rust CLI及形式语义：/Users/kevin/local-repos/ConcIR。此次不扩展核心修复操作。
- 论文、报告、原始实验结果：/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/deepseek-flash-pilot-v1/，每批用独立子目录。
- 先检查git状态；当前concir_generation_v1.md有未提交修改，保留并在其基础上完善，不能直接覆盖/恢复。
- 运行时从应用仓库根 .env 读取DeepSeek API key；不要cat/打印整个env，不把密钥、Authorization头写进prompt、日志、异常或manifest，不提交凭据。只加载本轮需要的DeepSeek配置。
- provider只允许deepseek；请求模型固定deepseek-flash，官方endpoint为https://api.deepseek.com。每次请求前执行精确允许列表检查；CLI/env/config/fallback都不得绕到Pro、旧未知alias或其他provider。非法模型在发请求前失败。
- 2026-09-18官方文档：https://api-docs.deepseek.com/ 。如执行时Flash不可用，报告阻塞，绝不自动换Pro。保存requested model与响应实际model，出现非Flash身份停止后续请求并报告。
- 本轮沿用Chat Completions，显式关闭thinking：extra_body={"thinking":{"type":"disabled"}}。现有thinking_enabled=False时省略参数不能保证关闭。按官方接口核实参数，不顺手迁移整个provider栈。

二、先补两个接受漏洞并跑离线测试

1. OfflineWorkflow在repair.artifact_path=None时直接返回repair.status；因此repaired无artifact也能成功。对于repaired/already_satisfied的repair路径，必须要求当前artifact存在、schema与输入/冻结contract/config绑定有效且replay通过；否则是工具/证据错误，repaired_by_tool必须false。
2. ConcirClient的replay分支exit0时吞掉JSON错误；空stdout、garbage、错误结构也会status=replayed。按真实Rust ReplayResult检查必要字段和类型，不能仅靠exit0；失败replay不得升级为成功。

用真实repair产物＋文件边界故障、可控subprocess stub补回归。保留现有29项测试及真实CLI集成；不要把受测判断替换成mock逻辑。

同时让每个工作流run有独占输出目录，重复命令不能覆盖旧generation/contract/report。冻结contract校验覆盖所有退出路径，工具错误与缺失证据明确记录。

三、接通受控live入口

- 在现有CLI增加明确的live/generate入口，复用provider协议和工作流。scripted入口与离线测试继续无需key/网络。
- 把LlmCandidateProvider接入；保留requirements、原candidate、真实check反馈的多轮上下文。解析和静态格式失败可有限重试，网络/认证错误与建模失败分开。
- 正确处理“JSON语法合法但不符合Rust反序列化schema”的模型输出：不要把它与缺二进制/命令错误混淆而直接吞掉。参考CLI实际错误分类；不要在Python重写完整CIR语义，也不要用隐藏的JSON修补器改出成功。
- 成功通过静态校验后冻结初始CIR，调用support/explore；UNKNOWN/UNSUPPORTED保持其结局。
- FAIL后仍使用Rust策略repair，再验证artifact/replay，标注repair_source=tool；这轮不是LLM外部patch修复。不得在冻结后让LLM整篇重写CIR绕过allowed_scope。
- prompt对齐现行AST/支持范围。不要主动推荐unsupported的abstract_step；任务真需要未支持语义时明确报告，不用Mutex或空操作偷换。只选当前支持的小任务运行首轮。

四、预算与错误处理必须在真实请求前实现

- 3个任务，每任务最多3轮candidate生成。
- 单次max_tokens=4096，timeout=90秒；全批最多12次实际模型HTTP请求，包含失败请求/传输重试，最多20分钟。首个正常任务的首次请求兼作连通性测试，不另跑聊天测试。
- 最多1次传输重试，仅用于可重试的超时/限流/服务错误；401/403/余额不足/无效model/参数错误停止批次，不盲目重试。避免SDK和外层各自重试叠加。
- 所有调用共用请求计数和总deadline，失败与中断也消耗已发出的调用额度；重启不能悄悄重置同一批预算。
- 上限是保守默认。达到上限就交付已获得的结果及停止原因，不扩大批次直到成功。
- token usage如缺失就记unknown/null，不当成0。不要求为单轮试验引入大型计费系统。

五、冻结3个建模任务再调用模型

任务类型：
1. 已正确的双mutex同序获取模型。
2. 给定两个线程反序获取相同mutex的既存ABBA行为，要求忠实建模初始缺陷。
3. 两模块通过FQN共享资源的既存ABBA行为。

运行前写TASKS/PROTOCOL，固定每例requirements、命名约定、contract、结构检查项、支持子集、预算、提示词hash和case id。可以沿用已有小fixture的行为设定，但不得把完整目标CIR或修复答案塞给模型，也不能用scripted候选冒充真实响应。

结构审核与性质验证分开：是否保留线程、共享锁身份、锁顺序、跨模块引用、解锁与调用者要求。尤其任务2/3，模型在生成阶段自行改成同序锁以得到PASS，记modeling_mismatch；不能报告“已满足需求”或“工具修复成功”。保留原始输出、实际验证结果及该判定依据。

先跑第1个任务，确认传输/日志/CLI链路正常再继续其余任务。接口失败先修接口，不能通过切模型解决。即使某例失败也保留，禁止只展示成功结果。若真实模型没有触发反馈重试，如实报告；用离线回归覆盖反馈路径，不人为注入错误伪装模型犯错。

六、证据与测试

每次实际请求保存脱敏后的精确system/user messages、prompt hash、原始assistant文本、request id、requested/response model、thinking设置、finish_reason、usage、耗时与传输重试号；不包含key/请求头。
保存Rust调用原始日志、binary/input/contract/artifact hash、check/support/explore/repair/replay结果与来源标签。模板、应用代码（含dirty改动）、后端binary身份可追溯。

离线测试补充：非Flash被拒且SDK零调用；配置/环境不能触发Pro fallback；thinking=false显式发送disabled；全局调用上限和非重试错误；密钥脱敏；缺artifact/坏replay；原有scripted测试不联网。

七、交付并停止

在论文目录的新批次输出：
- DEEPSEEK_FLASH_PILOT_HANDOFF.md
- PROTOCOL.md、任务清单、代码/模板/模型/输入manifest
- 全部LLM与CLI原始证据、汇总表、离线测试结果
- 每例：调用数、静态合法性、建模忠实性、根验证、工具修复、replay、失败/停止原因，分开报告。

结论只能是少量真实Flash调用下的链路/生成可行性；不得声称LLM完成了受限patch修复、已达到FSE实验规模或普遍有效。本轮不新增provider，不用Pro，不开展大规模评测、不引入agent框架、不扩展Rust核心。完成后停止，交付供独立审阅。
