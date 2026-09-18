# 803336c：F1–F4 独立复核

日期：2026-09-17。提交：`803336cee10d7c235cc289d1e40262821d571762`。审阅前后 ConcIR 工作区干净，未修改源码。

## 结论与已通过部分

全量测试 **225 passed / 0 failed / 0 warnings**。上一轮 11 个独立损坏记录反例均已返回退出码 4；F1 最终结果绑定、F2 契约与权限检查、F4 预算漏项的原始反例均已修复。F3 仍有下列三类明确遗漏，均属于上一轮已要求的记录内部一致性，不涉及扩展修复算法。

正例复核通过：原本正确、根 INVALID、根 UNSUPPORTED、UNKNOWN、零验证预算、验证预算耗尽、权限拒绝、无可接受修复、包含缓存复用的两步修复。它们的合法 artifact 均能独立重放。

搜索成本保持：two_cycles 的 B=7 次验证/1 次缓存命中，C=6 次验证；preserved_unfixable=4 次验证/5 次缓存命中。复杂类型导出重载、契约小 bounds、CLI 退出码与旧入口预算也继续通过。

没有发现新的在线搜索误接受。剩余问题影响完整记录、成本和失败分类能否被 replay 独立确认；在修复前，不宜把 replay 成功解释为这些字段已全部复核。

## G1 · P2：attempt.patch 仍未参与实际校验

位置：`src/repair/search.rs:1038–1093` 的 attempts 校验；replay_artifact 的重建循环只应用 node.incoming 和最终 patch_chain。

结构校验检查 parent、指纹、outcome 与引用，却只要求 attempt.patch 存在，没有对它执行权限、函数基准、应用或类型验证，也没有比较它和对应 verified 节点的 incoming。

单字段反例：

- 成功单步 artifact 的 attempts[0].patch.original_hash 改为 broken。
- changes[0].a 改成不存在的 sid。
- patch.function 改成不存在的函数。
- 合法 budget-blocked artifact 的 attempt.patch.original_hash 改为 broken。

四种 replay 均返回 0。前三种仍报告 repaired/accepted_ok=true。节点和最终链是正确的，但声称产生该节点的尝试补丁无法应用。

证据：residual-summary.json 的 attempt_bad_hash、attempt_missing_sid、attempt_wrong_target、blocked_patch_bad_hash，以及同名输入和输出。

修正要求：基于已重建父程序，校验每次尝试的实际补丁和记录分类。verified/reused/budget-blocked 的结果指纹应来自实际应用；denied/apply-error/static-invalid 的类别应由相应检查确认。验证尝试和节点之间需要可核对的一一对应关系，复用指向已存在结果。budget-blocked 仍不得触发一次虚构的验证调用。

## G2 · P2：报告比较忽略实际成本和反例内容

位置：`src/repair/search.rs:917–967`，reports_match；`1103` 仅对存档状态数求和。

reports_match 不比较 states_explored、transitions_explored、analysis_started。diagnostics 被压缩为 (property,outcome)，没有比较或回放 counterexample、blocked 等具体事实。counts.states_explored 只与存档报告的加和比较，没有与本次实际重验比较。

反例：

- 只将根报告 transitions_explored 改成 0，仍通过。
- 只将根报告 analysis_started 改成 false，仍通过。
- 只清空根诊断的 counterexample，或只清空 blocked，仍通过。
- 将各节点及接受报告的 states_explored 都改为 0，同时将总 states_explored 改为 0，仍通过。这个反例有意同步改动派生总数，验证的是“存档字段彼此一致，但与独立实际重验不一致”的情况。

证据：residual-summary.json 的 false_transitions、false_analysis_started、erase_counterexample、erase_blocking_facts、zero_states_consistently。全部退出 0；原单步修复实际累计探索 92 个状态，记录可以被改为 0 而被接受。

修正要求：在固定版本、相同语义配置下，将实际重验的计数、分析标记及结构化诊断证据与存档比较；反例可采用连续回放和终态事实校验。人类可读措辞不必作为语义依据。如果跨版本不能复现历史成本/轨迹，应明确报告该部分未复现或版本不兼容，而不能将其归入完整验收成功。

## G3 · P2：未成功的终止分类和原因未与证据关联

位置：`src/repair/search.rs:1200–1207`。AnalysisUnknown、NoAcceptableCandidate、BudgetExhausted 使用相同检查，只要求有根且没有接受结果；stop_reason、saw_unknown、truncation 没有与运行事实核对。

单字段反例：

- 对真正 verification-budget 截断的 artifact，只把 outcome 改为 no_acceptable_candidate，仍通过。
- 对 preserved_unfixable 的正常搜索，全部节点均为 FAIL、没有 UNKNOWN，只把 outcome 改为 analysis_unknown，仍通过。
- 同一个未截断 artifact，只把 outcome 改为 budget_exhausted，仍通过。
- 只把预算停止的 stop_reason 改为 solved，或将正常耗尽候选的 stop_reason 改为 verification-budget，仍通过。

证据：residual-summary.json 的 budget_as_no_candidate、budget_reason_solved、unfix_as_unknown、unfix_as_budget、unfix_wrong_stop_reason。

影响：实验中 UNKNOWN、预算截断和未找到修复的分类，可以与搜索证据矛盾而仍获 replay 成功。在线搜索当前没有因此改变结果；缺陷在记录验收。

修正要求：为终止状态建立共享校验规则，核对 outcome、stop_reason、truncation、saw_unknown、根/子节点状态、预算及阻塞尝试。按搜索器的实际优先级处理多种事实同时出现。NoAcceptableCandidate 仍只表示当前策略范围内未找到，不要求证明不存在任意修复。

## 证据与复现

- full-tests.txt：完整全量测试日志。
- summary.json：上一轮探针复测、预算、去重、类型与 bounds 结果。
- positive-summary.json：CLI、权限与搜索成本检查。
- terminal-positive-summary.json：9 种合法 artifact 重放结果。
- residual-summary.json：14 个 F3 残留反例，当前全部被错误接受。
- probes.py、positive_checks.py、terminal_positive.py、residual_checks.py：独立 CLI 脚本，输出到脚本同目录。

```sh
cd /Users/kevin/local-repos/ConcIR
INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo test --offline --all-targets --no-fail-fast

python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/probes.py
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/positive_checks.py
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/terminal_positive.py
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/residual_checks.py
```

单独复现：

```sh
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/attempt_bad_hash.json
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/zero_states_consistently.json
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/803336c/unfix_as_unknown.json
```

三者当前均返回 0；修正后应明确拒绝。固定同一构建下的合法记录应继续通过。

文档小误：REPAIR_LOOP_HANDOFF.md 把 bbff35b 标成 208/0；该提交独立实测为 222/0。208/0 是更早未提交的组合搜索工作区。
