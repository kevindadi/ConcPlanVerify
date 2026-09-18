# e8480c4：终止语义修正验收通过

日期：2026-09-17。提交和工具链见 review-metadata.json。ConcIR 在审阅开始、结束均为干净工作区，本次未修改源码。

## 结论

**本轮 H1/H2 验收通过。在本次代码审阅与独立检查范围内，没有发现新的阻塞问题。** 可以将该提交作为实验预跑基线，结束当前这一轮后端修复收尾。

这表示约定范围内的实现与记录链路通过验收，不构成所有 CIR 程序上的形式证明，也不意味着现有开发案例已经足以支撑论文的有效性或泛化结论。

## 独立检查结果

| 检查 | 实际结果 |
| --- | --- |
| 全量测试，禁用自动更新快照 | 229 passed / 0 failed / 0 warnings |
| H1：A/B/C × 6 种深度/编辑配置的真实输出 | 18/18 正确重放，策略 A 原有 3 个误拒绝已消除 |
| H2：删除/伪造截断、错误优先级、根 UNKNOWN 标志不一致 | 4/4 正确拒绝，退出码 4 |
| 上一轮 G1–G3 的补丁、诊断、成本和分类反例 | 14/14 正确拒绝 |
| 更早 F1–F3 的链、契约、编号和报告反例 | 11/11 正确拒绝 |
| 合法终止记录 | 9/9 正确重放 |
| 复杂类型修复程序保存、重读、独立验证 | bounded Int、Enum、嵌套 Struct、Array 均完整 PASS |
| 契约 max_states=1 | UNKNOWN、1 个状态，实际 bounds 正确导出 |
| CLI 预算与 INVALID/UNSUPPORTED 退出码 | 保持正确 |

9 类合法终止记录覆盖原本正确、INVALID、UNSUPPORTED、UNKNOWN、InvalidConfig、验证预算耗尽、权限拒绝、无可接受候选、含缓存复用的成功修复。

搜索成本保持：

| 案例/策略 | 候选 proposals | 实际验证次数 | 缓存命中 | 累计状态 |
| --- | ---: | ---: | ---: | ---: |
| two_cycles / B | 7 | 7 | 1 | 13445 |
| two_cycles / C | 5 | 6 | 0 | 11610 |
| preserved_unfixable / B | 8 | 4 | 5 | 176 |

这些是固定开发案例的复核数据，不能外推为诊断策略普遍优于无诊断策略。

## 实现核查

搜索与重放共用 TerminalFacts::classify 的终止优先级。重放根据已校验的节点、尝试、策略和预算重新推导事实，并比较 outcome、stop_reason、truncation、saw_unknown。

策略 A 仅根节点可扩展，第一层失败子节点到达深度 1 不再被误认为预算截断；B/C 的失败子节点则会按实际扩展规则处理。验证预算阻塞与候选预算优先级也纳入事实推导，未改变默认开发基准的搜索成本。

## 证据与命令

- full-tests.txt：全量测试日志。
- boundary-matrix-summary.json：18 个边界正例。
- terminal-flags-summary.json：4 个终止字段负例。
- residual-summary.json：14 个上一轮负例。
- summary.json：11 个更早负例与类型、bounds、去重检查。
- terminal-positive-summary.json、positive-summary.json：合法记录、CLI 与搜索成本。
- source-manifest.json、review-metadata.json：源码指纹、提交、构建命令及环境。

```sh
cd /Users/kevin/local-repos/ConcIR
INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo test --offline --all-targets --no-fail-fast
```

本目录的 probes.py、positive_checks.py、terminal_positive.py、residual_checks.py、boundary_matrix.py、terminal_flags.py 可按该顺序重跑。脚本使用上述 target 内的最新二进制，输出到各脚本所在目录。

## 下一阶段

冻结该提交作为 pilot 基线，做无 LLM 的实验预跑：统一 A/B/C 批处理、参数化小型模型、明确预算和外部超时、保存原始 artifact 与独立重放结果，观察组合能力、诊断收益及状态空间限制。

现有 8 个开发案例仍只作 smoke/regression；新生成的同模板变体也只作 pilot。正式论文实验需要另行建设有来源和独立期望的案例集。下一轮可执行任务见 NEXT_CURSOR_PROMPT.md。
