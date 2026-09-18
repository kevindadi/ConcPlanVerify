# 生命周期交付审阅

日期：2026-09-18。ConcIR HEAD 仍为 `e8480c4a54840e68d7cf3a62c5075e4fc02e6966`；实验代码为未跟踪交付。本次只读审阅两个代码仓库，测试输出写入临时目录。

**结论：上一轮的 batch 覆盖、恢复字段丢失和外部输出目录崩溃已得到实质修复。核心继续冻结。本轮新交付的数据核验通过；另有三项已复现的实验有效性问题，建议与真实案例 v0 同轮完成，不再单独开启一轮纯框架扩张。**

## 独立验证

- 核心：offline/all-targets/no-fail-fast，229 passed / 0 failed / 0 warnings。
- 以仓库外临时目录运行真实 lifecycle：8/8 通过，包含它内部启动的 external-out behavior 11/11。
- smoke-lifecycle 24 条 complete repair、matrix-lifecycle 12 条 complete explore，plan/index 对应正确。
- 24 个 smoke artifact 全部重新 replay 通过；检查输入/contract、有效配置、退出码和 raw 文件对应；交付 auditor 0 issues；代码快照与当前代码 hash 一致。
- matrix 的 outcome 与 report_complete 同 raw report 一致，保留 FAIL/incomplete 的区别。
- frozen-data-sha256.txt 的 3849 文件当前全部匹配；pilot-v1 还与上一轮独立审阅存档哈希逐一比较，无变化。
- 未重跑旧 180 次 pilot/heavy；未调用任何 LLM；未清理 ConcPlanVerify 文件。

## 发现

### R1 [P1] resume 拒绝失效旧记录后，没有将它移出有效索引

位置：`scripts/run_pilot.py:963–1004, 1199–1202, 1238–1265, 913–925`。

旧 index 先整体加载。load_resume_index 检测到缺失/哈希失配 artifact 后拒绝复用，但旧记录仍留在 valid_index；新失败状态在 _best 中低于旧 complete，于是旧“成功”继续赢得选择。summarize 目前只检查字段存在，不检查引用文件存在/内容一致。

真实 do_run 复现（仅在临时目录破坏自己的测试 artifact）：
1. 单案例 A/B/C 正常完成。
2. 删除 B 的 artifact。
3. 同 cohort --resume，subprocess 边界注入本次 repair exit=2、不产文件。
4. 新 attempt 正确记为 no_artifact，然而 index 仍是 complete/repaired，指向不存在的 artifact；do_run 和 summarize 均返回 0。audit 能报错，但统计阶段仍产生了成功结果。

修复：有效索引资格应先取决于证据是否仍有效，然后才比较 attempt 质量。失效旧记录保留在历史日志，不能继续参与 valid_index 选择；低预算或本次重试失败也不能让失效成功复活。complete 写入和 summarize 接受应统一验证必需字段、引用证据、绑定身份与关键计数（无需重复执行昂贵 replay）。当前 _commit 并没有调用其注释声称的完整性检查。

此问题未在已交付新 smoke/matrix 中触发，现有数据不因此作废。

### R2 [P2] 恢复与普通运行的 wall_ms 不是同一个量

位置：`scripts/run_pilot.py:753–754, 868–869`；`scripts/pilot_analyze.py:200–217`。

普通运行 wall_ms=search_wall_ms，恢复运行 wall_ms=search_wall_ms+replay_wall_ms；统计直接相加/比较 wall_ms。独立 lifecycle/b5 实测：A、C 都是 search=9ms、replay=5ms、wall=9ms；恢复的 B 同样 search=9ms、replay=5ms，但 wall=14ms。

字段标签虽不同，表格没有按标签分开。相同搜索成本因是否恢复而被报告为不同性能。统一所有路径的字段定义；策略比较明确使用 search_wall_ms，另列 replay 和可选总耗时。历史不同口径不能无提示混合；不能把重试等成本偷偷抹掉，若报告全过程成本应另设字段并定义清楚。

### R3 [P1] success/timeout 的策略差异被 complete-only 配对筛掉

位置：`scripts/pilot_analyze.py:183–203`。

构造同 case/config/repeat/identity 的 B=repaired、C=search_timeout 两条记录，通过真实 summarize_batch，结果 pairs=0、success_differs=[]、exit=0。原因是在构建全部配对之前先过滤 evidence_status != complete。

共同成功样本的验证成本比较可以限于双方成功，但成功率/结局比较必须保留超时、UNKNOWN、失败等结果。需要分开定义 planned pairs、attempted/outcome pairs、双方证据完整、双方修复成功、单边未执行，显式报告排除原因。timeout 不是不存在的样本。已有两个新批次全 complete，未暴露此偏差；后续真实模型更可能受其影响。

## 后续研究范围

下一轮开始有来源的真实并发问题模型 v0，并在运行其 A/B/C 比较前完成上述三项修正。目标是检验“实际问题能否忠实映射到当前 CIR 子集、原缺陷与上游修复是否仍可表达”，不是马上追求大样本或宣称验证整个项目。来源必须可追溯，抽象假设、无法表达的语义和模型→源码的界限必须写清。不要将旧手写 benchmarks/rust 或同构合成模型自动标为真实案例。

仓库边界按用户最新指示：ConcIR 作为 CLI tools；未来 LLM/prompt/反馈修改全部在 ConcPlanVerify。检查发现 ConcPlanVerify 仍使用旧 cir2cvn 协议、旧 schema 和旧翻译器，迁移需要独立离线适配阶段。用户已允许清理该仓库的无关内容，本轮无需为了未来接入先删除文件。详见 `../../REPOSITORY_BOUNDARIES.md`。

证据：`evidence/probes.json`、`probes.py`、`lifecycle.log`、`delivery-audit.json`、`core-tests.txt`。复现脚本应在新临时根执行，不能用于原始实验目录。
