# real-cases-v0 审阅

2026-09-18。本轮只读审阅代码仓库，测试输出写入 /private/tmp；论文目录保存报告。

结论：上一轮三项问题的主要回归已通过，可以推进 ConcPlanVerify 的离线 CLI 适配与 Python 工作流。当前真实案例 v0 是 issue 机制的可行性缩减，尚不能作为源码修复效果的正式实验。另有统计台账和生成脚本身份的小项需收尾，不需要再重写核心或增加算法。

## 独立验证

- Rust core：229 passed / 0 failed / 0 warnings。
- 当前 lifecycle：11/11，通过其中的外部输出目录 behavior 检查。
- 新交付共9条记录（3 root、6 repair），auditor 0 issues；6个 repair artifact 重新 replay 全通过（包括3个 unsupported outcome）。
- 独立运行 Petri/interpreter：rmw buggy 两者均 FAIL/complete，49 states；fixed hypothesis 两者均 PASS/complete，39 states；DashMap 两者均 UNSUPPORTED/incomplete，0 states。
- rmw A/B/C 均2次验证、累计92 states，交付搜索时间分别8/8/9 **毫秒**。Handoff 的“v2 / 92 s”需改为“2 verification calls / 92 states”，不能读作92秒。

## 研究证据的边界

两个 issue 的机制描述能够从上游页面核实：[rmw_zenoh #998](https://github.com/ros2/rmw_zenoh/issues/998)、[DashMap #369](https://github.com/xacrimon/dashmap/issues/369)。前者描述锁顺序反转，并建议缩短持锁范围；后者提供跨 shard 的读锁/写锁交互示例。这只核实了报告内容，不代表本次独立复现了原始项目故障或确认了维护者认可的修复。

交付的两个 provenance 都没有完整、已核实的 buggy/fixed commit 对；rmw 的 fixed.cir.json 已明确标成 hypothesis，应该继续保留该标签。其模型省略了两次锁获取之间的操作/调用，使当前 adjacent-swap 修复更容易适用。不能据此声称工具生成了上游源码补丁。当前可表述为“1个可验证/可修复的来源于issue的缩减模型 + 1个不支持边界模型”。

DashMap 的不支持结果应保留，不能替换 Mutex 来换取成功。后续正式实验仍需要源码版本固定、抽象映射审核与独立缺陷来源分组。模型中 EF(function_completed) 表示可达完成，不代表所有调度最终完成。

## 收尾项

1. `pilot_analyze.py` 的 both_attempted 只看 B/C 记录是否存在。当前回归中有1对含 not_executed，却仍报告4对 attempted（应将“记录存在”和“实际执行”区分）。不能把 repaired/not_executed 当成有执行预算下的胜负。成功/timeout 的差异现在已保留，这是正确改进。
2. `_cumulative_attempt_ms` 对每行 wall_ms 求和，但 wall_ms 已改成搜索成本；replay-only recovery 继承旧 search_wall_ms，因而会重复计搜索、漏计 replay。最小输入：原搜索100ms、恢复仅 replay20ms，当前返回200ms，实际执行成本应120ms。搜索性能比较本身现在使用 search_wall_ms，不受此辅助累计字段问题影响。
3. runner 的代码身份硬编码 generate_cases.py；真实案例生成器叫 build_cases.py，rc-abc environment 对 generate_cases.py 记 null，实际生成器未纳入 code snapshot/hash。已运行输入文件仍有哈希，不能因此推断验证结果错误；但实验生成过程的来源身份不完整。

这些可在迁移附带的小修中解决，保留旧批次，使用新输出目录。不要为修表重跑全部 heavy。

## 下一轮

主要工作转到 `/Users/kevin/local-repos/ConcPlanVerify`，以Python构建调用现行 ConcIR Rust CLI 的适配与离线工作流。上层负责模型生成与反馈迭代，工具层负责形式验证和受限修复接受。先使用 scripted/mock provider 与真实 CLI；不调用付费 API，不以 mock 的结果作为 LLM实验。

注意现有CLI有两个repair协议：flag策略模式产出可replay的搜索artifact；位置参数patch文件模式只产出legacy report。Python不能把两者混为一种证据，也不能假设外部patch已能进入composite search artifact。下一轮先完成策略模式闭环和生成阶段反馈重试，外部LLM patch接受留明确接口缺口。

实验结果与本轮迁移说明存放论文目录。职责以 `../../REPOSITORY_BOUNDARIES.md` 为准。
