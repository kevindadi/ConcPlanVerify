# 41b2d09：G1–G3 复核

2026-09-17，提交 `41b2d09480ecca178fc3110c0f8a956c8c553220`。工作区审阅前后干净，未修改 ConcIR 源码。

## 结论

**227 passed / 0 failed / 0 warnings**。上一轮 G1–G3 的 14 个反例全部正确拒绝，更早的 11 个反例也继续拒绝。G1 尝试补丁校验、G2 成本和诊断比较的独立反例已通过。9 类默认配置下的合法终止记录均能重放。

搜索成本保持：two_cycles B=7 次验证/1 次缓存命中，C=6 次验证；preserved_unfixable=4 次验证/5 次缓存命中。此前类型往返、bounds 和 CLI 回归继续通过。

但 G3 的终止校验仍与搜索器分开实现，用最大节点深度推断截断，新增了正常记录被误拒绝的问题；终止标记也未完整核对。建议只统一这部分规则，不再扩展内核或修复算子。

## H1 · P2：策略 A 的合法单步结果被 replay 当作截断拒绝

位置：`src/repair/search.rs:1092–1096`。

对 preserved_unfixable 使用策略 A，设置 `--max-depth 1` 或 `--max-total-edits 1`。搜索器正常尝试所有单步修改，返回 no_acceptable_candidate。策略 A 本身不扩展第一层子节点，因此未发生预算导致的搜索截断。

然而 replay 把“最大已生成节点深度/编辑数达到上限”直接当作截断证据，拒绝原样导出的记录：

```
artifact replay failed: no_acceptable_candidate must not have depth truncation
```

这是正常调用产生的回归，不需要损坏 artifact。A/B/C × 默认、depth=1、edits=1、两者=1、depth=0、edits=0 共 18 个正例中，A 的 depth=1、edits=1、两者=1 三例错误退出 4，其余 15 例成功重放。

证据：boundary-matrix-summary.json、boundary_matrix.py、matrix_a_depth1.json 等。

复现：

```sh
cd /Users/kevin/local-repos/ConcIR
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend repair tests/repro_bench/preserved_unfixable.json tests/repro_bench/preserved_unfixable_contract.json --strategy a --max-depth 1 --artifact /private/tmp/concir-a-single.json
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /private/tmp/concir-a-single.json
```

搜索退出 1 是预期的未修复结果；重放应退出 0，表示该记录真实一致，现在却退出 4。

修复：基于策略以及实际可扩展节点/截断事件判断原因，不能仅比较已生成节点的最大深度。保留 A 的单步含义及原搜索分类；不要把合法 NoAcceptableCandidate 改为 BudgetExhausted 来迎合错误校验。

## H2 · P2：truncation、saw_unknown 和原因优先级仍未被统一核对

位置：`src/repair/search.rs:1063–1067,1113–1126`，以及 validate_outcome_evidence 未读取 truncation。

四个单字段反例当前均被接受：

1. 正常 B 的 depth=1 截断记录，把 truncation 从 max-depth 改为 null。
2. 正常未截断记录，把 truncation 从 null 改为 max-depth。
3. B 的 depth=1、edits=1 同时到达边界，搜索器按实现先判深度，记录 stop_reason=max-depth、truncation=max-depth；仅把 stop_reason 改为 max-total-edits，replay 仍成功，既没有发现字段矛盾，也没有遵循搜索器优先级。
4. 根报告为 UNKNOWN，把 saw_unknown 从 true 改为 false，仍通过。

证据：terminal-flags-summary.json、terminal_flags.py 及 wrong_truncation/fake_truncation/wrong_priority/root_unknown_flag_false.json。

这说明原 G3 的“终止字段与实际优先级一致”尚未完成。实现应从已核实的搜索事实统一得到 outcome/stop_reason/truncation/saw_unknown；记录只是待比较的输出，不能作为自身真实性依据。

修复建议：让在线搜索与 replay 共用终止语义和优先级定义；必要时显式记录可校验的扩展/截断事实。区分“节点到达边界”“该策略会继续扩展它”“确实因预算停止”三件事。验证完整字段组合，而不只判断每个字符串分别是否看起来合理。

## 已验证范围与文件

- full-tests.txt：完整测试日志；review-metadata.json：构建版本及命令。
- summary.json：此前 11 个反例、类型、预算、去重。
- residual-summary.json：上一轮 14 个反例全部退出 4。
- terminal-positive-summary.json：原本正确、INVALID、UNSUPPORTED、UNKNOWN、InvalidConfig、预算耗尽、拒绝、未找到、缓存复用等 9 个合法记录全部通过。
- positive-summary.json：CLI、模块权限与 B/C 成本。
- boundary-matrix-summary.json：18 个策略/预算边界正例，3 个误拒绝。
- terminal-flags-summary.json：4 个终止字段损坏反例仍错误接受。

完整测试命令：

```sh
cd /Users/kevin/local-repos/ConcIR
INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo test --offline --all-targets --no-fail-fast
```

本目录 Python 探针读取上述最新二进制，结果写到脚本所在目录。重跑 boundary_matrix.py 可直接再现 H1；已有输入文件可以直接交给 replay。

文档仍有数字不同步：bbff35b 应为 222/0；当前全量结果为 227/0，但 Handoff 命令示例还写 225/0。修正实际记录即可，无需增加功能。
