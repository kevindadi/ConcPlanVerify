# DeepSeek Flash pilot v1 独立复核

日期：2026-09-18。范围：已交付代码、实验原始记录及离线重验；未读取 `.env`，未新增付费请求，未修改两个代码仓库。

## 结论

接受这轮作为“真实 Flash 生成 CIR → Rust 检测 → 工具修复 → artifact 回放”的小规模链路证据。不能据此声称 LLM 已完成补丁修复、真实反馈迭代有效，或已完成 FSE 所需的效果评估。下一轮建议实现外部候选的可回放验证协议，然后接入受限 LLM 补丁闭环。

独立证据见 `evidence/audit.py`、`evidence/audit.json`、`evidence/tests.txt`：

- 重跑 Python 全部 44 项离线测试通过，含现有真实 CLI 集成。
- manifest 中应用代码、提示词、合同、文档、原始证据及 Rust 二进制共 105 项哈希全部匹配。
- 交付记录显示实际成功批次 3 次调用，请求和响应均为 `deepseek-flash`，显式关闭 thinking，无传输重试，合计 6666 tokens。此次复核未向服务端重新确认这些请求。
- 三个冻结 CIR 独立静态检查均合法；重新探索的完整结果依次为 PASS、FAIL、FAIL。
- 两个修复 artifact 重新调用 Rust replay 均成功，`accepted_ok=true`，各一项补丁。
- 人工核对三份实际 CIR：入口 scope 包含两个任务、共享锁身份和获取顺序符合任务；跨模块案例拥有正确的 FQN、所有者和 requires。
- 三次真实生成的 `had_feedback=false`。修复来源均为工具，真实 LLM 拒绝反馈与补丁提出尚未验证。

## 需在下一轮先修的两处问题

### P2：结构审核没有检查要求的并发启动

位置：`/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/structural.py:112`，相关跨模块检查在 124 行。

复现：从交付的 t2 ABBA 模型中仅删除 scope.funcs 中的第二个任务，保留其函数定义。Rust check 仍合法，`run_structural_check('abba_inversion', mutant)['ok']` 仍为 true。实际只启动一个任务，不能忠实表达用户要求的双任务 ABBA。完整 contract 会因 t2 无法完成而 FAIL，这并不挽救独立的 `modeling_ok` 错判。

当前检查仅遍历函数定义、比较资源原始字符串和净计数。扩展前应增加预声明任务的入口、scope 成员、资源身份和具体操作顺序核对；复杂或无法判定的结构标 unknown，不冒充完整语义分析器。此次三份实际输出没有该缺陷，不否定现有 pilot 结果。

### P2：replay 字段一致性仍有缺口

位置：`/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/concir_client.py:501`。

已修复空 stdout、坏 JSON 和缺字段，但真实 t2 artifact 搭配矛盾 payload（`nodes=999`、`accepted_ok=false`、`outcome=no_acceptable_candidate`，其余字段保持真实值）仍被 `_validate_replay_payload` 返回 None 接受。工作流只检查 `status=replayed` 后沿用 repair 成功状态，会漏掉这类协议矛盾。需要检查节点数、outcome、接受节点及 accepted_ok 的一致性，并在工作流成功关口明确检查接受验证成功。注意合法的“无接受补丁 artifact 回放成功”仍是回放成功，但不能当作修复成功。

这是以受控矛盾输出复现的客户端验收缺口；本次真实 Rust 回放未出现该矛盾。

## 下一步依据

Rust 的 `CirPatch` 已包含 module/function/original_hash/changes，`function_hash` 由 Rust 定义。现有位置参数 `repair model contract patches.json` 接受外部候选，但仅输出 legacy report，没有可供当前 replay 消费的完整 artifact。带 flags 的 repair 导出 search artifact，却使用内部确定性枚举器。

因此不能简单把 LLM JSON 塞给旧入口后凭 exit 0 宣称端到端完成。需补齐“外部候选 → Rust 受限应用和完整合同验证 → 可回放证据”的工具接口；Python 负责 LLM 提出与反馈编排。协议扩展不应伪装成现有 A/B/C 搜索或把旧工具答案送给 LLM。

完整可执行要求见同目录 `NEXT_CURSOR_PROMPT.md`。
