# Offline integration v1 审阅

2026-09-18。主代码仓库：/Users/kevin/local-repos/ConcPlanVerify。独立执行现有Python测试：29 tests / OK，无skip输出，已指定真实ConcIR binary。交付已完成Python层迁移与离线主路径，可以开始小规模DeepSeek Flash连通及生成试验，前提是修复以下两个接受条件漏洞。

本次没有读取.env内容或调用付费API；仅确认仓库根目录.env文件存在。当前generation prompt有用户未提交修改，后续实现必须基于它继续，不恢复旧版本。

## 两个已复现的接受条件问题

1. **P1：artifact缺失仍报告repaired。** OfflineWorkflow在repair.artifact_path为None时直接返回repair.status，没有检查这个status是否属于成功。在真实CLI完成repair后、文件边界注入“artifact丢失”，工作流返回status=repaired、repaired_by_tool=true、replay=null、error=null。成功必须要求本次artifact存在、绑定成立且replay有效；缺文件只能报告工具/证据错误。
2. **P1：replay exit0但空/非JSON响应被当成功。** ConcirClient._invoke的replay分支吞掉JSON解析错误，仍返回semantic/replayed。可执行stub输出garbage并exit0，得到payload=null但status=replayed。真实CLI成功协议包含结构化ReplayResult；必须检查必要字段/类型和与输入artifact的对应关系，空/乱码/错误结构均属protocol_error。

现有正常路径测试通过并不能覆盖这两个故障路径。复现与结果见evidence/probes.py、probes.json。下一轮先补回归，再进行少量真实模型请求。

## 真实模型配置

用户只允许DeepSeek Flash，使用应用仓库env里的凭据。2026-09-18核对[DeepSeek官方入口文档](https://api-docs.deepseek.com/)，当前请求标识为 `deepseek-flash`。不得沿用旧Pro默认值或在失败时fallback。

现有llm.py仅在thinking_enabled为true时传thinking参数；false时不传不能保证关闭，因为[官方thinking文档](https://api-docs.deepseek.com/guides/thinking_mode/)说明默认开启。因此首轮须显式发送disabled并记录实际配置，避免成本/模式失控。

## 下一轮范围

只做“真实Flash生成CIR + 解析/静态反馈重试 + Rust验证 + 确定性工具修复 + replay”。这仍不是LLM提出受限repair patch的实验；外部patch证据协议尚未接通，不得混淆。3个明确建模任务、每例最多3轮、全批最多12次实际HTTP模型请求（含重试）足够做连通性和工程验收，不作论文效果统计。

生成结果需另外审核是否忠于输入任务。若任务要求重现既有ABBA，模型通过自行消除锁顺序反转而获得PASS属于建模偏差，不能当作生成正确或工具修复成功。
