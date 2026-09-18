# 诊断驱动组合修复：独立审阅

审阅日期：2026-09-17。对象：ConcIR 基于 `29ee9ed6133052d5adeeb30c686f0e5d36b99e6d` 的未提交工作区，包含新增的 search/benchmark 实现。审阅未修改 ConcIR；审阅前后源文件 SHA256 一致。版本、工具链和命令见 `review-metadata.json`，文件指纹见 `source-manifest.json`。

## 结论

组合修复的主流程已实现，但本轮暂不验收为可用于规模实验的闭环。需要先修正导出、预算、去重和搜索记录，再重新建立 A/B/C 的成本基线。没有观察到本轮探针把契约内的 FAIL/UNKNOWN 程序作为完整 PASS 接受；下列问题分别影响可交付结果、资源控制和实验可信度。

已确认的进展：

- 全量 `cargo test --offline --all-targets --no-fail-fast`：**208 passed / 0 failed**；禁用 snapshot 自动更新。四个历史 DOT snapshot 失败已消除。
- CLI `bench` 成功退出，产生 8 个案例 × 3 个策略 = 24 条记录。
- `two_cycles`：A 未找到单步修复；B/C 都找到两步修复。B 的最终程序经独立 CLI 保存、重新读取、重新验证为完整 PASS。
- 搜索确实保留并扩展中间 FAIL，候选读取当前节点诊断；最终接受仍要求总体完整 PASS，并使用原始契约。
- 当前诊断策略是阻塞资源相关性过滤。交付文档已注明其不是循环结构或源位置驱动的根因分析；现阶段不要扩大这一技术主张。

## E1 · P1：复杂类型的修复程序不能重新读取

位置：`src/ast.rs:227–234`（历史序列化缺陷，本轮的修复结果导出直接触发）；调用入口 `src/bin/concir-backend.rs:242–245`。

在 `single_cycle` 中添加一个合法但不参与锁操作的变量，分别使用 bounded Int、Enum、Struct、Array。四种模型均得到 `repaired`、退出码 0；导出的 `accepted_program` 均无法解析，退出码 2。

例：输入 `{"Int":[0,1]}` 被序列化为 `["Int",[0,1]]`。反序列化要求字符串或单键对象，不能接受该数组。嵌套类型同样受影响。

证据：`cli_summary.json` 的 bounded/enum/struct/array；对应 `*_accepted.json`、`*_reload.stderr.txt`。现有重载回归只覆盖锁模型，不能支持“导出保留所有类型与值域”的结论。

修复：统一类型的读写 schema，覆盖四种类型、嵌套类型和函数返回类型；检查完整 Program 的往返相等，以及修复文件重新读取后对冻结契约完整 PASS。不要仅在 CLI 拼接 JSON 修补。

## E2 · P1：SearchConfig.bounds 完全没有生效

位置：`src/repair/search.rs:43–50,181–184,324–327`。

将 `SearchConfig.bounds.max_states=1`、`max_depth=1` 后，`two_cycles` 的 B 搜索仍返回两步 `repaired`，验证 8 次，累计探索 15,656 个状态，与默认配置一致。把同样限制写入 `ContractSpec.bounds` 后，直接验证才返回 UNKNOWN、1 个状态。

根因：根节点与子节点调用 `verify_program` 时都只传原始 spec，完全没有使用 config.bounds。这里不是原契约被偷偷放宽，而是新增公开配置与实际执行不一致。调用者无法据此控制验证成本。

证据：`api-probes.json` 的 default、bounds_one、contract_bounds_one。

修复：明确唯一权威的验证 bounds。可以删除无效的重复字段，以冻结契约为准；若需要额外执行上限，应显式合成并记录，不得默默覆盖契约或放宽限制。导出实际生效的 bounds。

## E3 · P2：指纹去重发生在验证之后，浪费预算

位置：`src/repair/search.rs:324–327,365–369`。

`two_cycles` 的 B 执行 8 次验证，实际只有 7 个不同程序；逆向交换返回根程序后仍进行完整验证。`preserved_unfixable` 执行 9 次验证，只有 4 个不同程序。当前确实避免了重复节点入队，但没有避免重复验证。

将 B 的 verification_budget 设为 7 后返回 budget_exhausted；默认轨迹中第 8 次验证才得到 PASS，其中一次验证是已知重复。也就是说，成本和预算内成功率都受到重复验证影响。

证据：`cli_summary.json`、`two_cycles_b.stdout.json`、`preserved_unfixable_b.stdout.json`、`api-probes.json` 的 verification_seven。

修复：在昂贵验证和验证预算计数之前识别重复程序；保留重复候选的来源/引用，明确 proposal、unique program、verification 的计数口径。去重结果必须绑定相同契约及实际验证配置。修复后重跑 A/B/C，当前的 8 对 6 次验证不能全部归因于诊断定位收益。

## E4 · P2：导出节点 ID/parent 不一致，无法正确还原搜索树

位置：`src/repair/search.rs:332–334,369–378,420–423`。

`NodeReport.id` 使用 records 下标，parent 却使用仅保存新 FAIL 状态的内部 nodes 下标。出现重复或拒绝记录后，两者不再一致。

`preserved_unfixable` 中，记录 7/8 的 parent=3；记录 3 的指纹是原程序 `3846e855927db5e2`。实际父节点是双交换程序 `cabf6e2feb17a5ee`，对应记录 4。由于同一深度可能有多条记录，只检查 parent.depth+1 不足以发现这个错误。

另一个独立反例：allowed_scope.modules 只允许其他模块时，根节点及两个拒绝记录全部 id=0，拒绝记录 parent=null。

证据：`preserved_unfixable_b.stdout.json`，`api-probes.json` 的 parent_identity_map，`cli_summary.json` 的 denied_modules。

修复：统一稳定节点 ID，或显式区分节点与候选尝试 ID；每条尝试关联真实父节点。保存 incoming patch/程序基准，支持从导出数据独立重建。验收应重新应用补丁并比较父子程序指纹，而不只检查深度和最终成功链。

## E5 · P2：部分预算边界未执行或未正确报告

位置：`src/repair/search.rs:181–184,257–259`。

- verification_budget=0 仍先执行一次根验证，报告 verifications=1。若根验证必须保留，应在运行前拒绝不足的预算，而不是静默超额。
- two_cycles 的 max_depth=1 或 max_total_edits=1 均被预算截断，却返回 no_acceptable_candidate；没有记录截断原因。这与候选/验证预算返回 budget_exhausted 的口径不一致。

证据：`api-probes.json` 的 verification_zero、depth_one、edits_one。

修复：统一预算定义及验证前检查；记录真实截断原因。区分 A 本身只搜索单步与 B/C 被外部深度/编辑预算截断。所有“未找到”都限于实际策略和预算，不得解释成不存在修复。

## E6 · P2：旧 CLI 的显式 budget 参数失效

位置：`src/bin/concir-backend.rs:214,226`。

文档命令 `repair model contract patches.json 0` 中预算在 args[5]，实现读取 args[6]，回退到默认 16。独立调用实际执行了 1 个候选并返回 repaired，违反调用者指定的零候选预算。

证据：`legacy-budget-zero.json`，使用 `patches.json`。

修复参数下标或使用统一解析逻辑；覆盖省略、0、1、非法数值及多余参数。保持旧 CLI 兼容。

## E7 · P2：新 repair 路径把 INVALID/UNSUPPORTED 的退出码压成 FAIL

位置：`src/bin/concir-backend.rs:258–261`。

同一不支持的契约，explore 返回 5/UNSUPPORTED，repair --strategy c 返回 1/unsupported；同一静态无效模型，explore 返回 4/INVALID，repair 返回 1/invalid。JSON 仍区分状态，但基于文档退出码的调用脚本会误分类。

证据：`cli_summary.json` 的 unsupported_*，`additional-summary.json` 的 invalid_*。

修复：根据具体 RepairOutcome 映射退出码，并测试所有策略入口。

## E8 · P2：交付的结果还不是可独立复现的完整记录

位置：`src/repair/search.rs:95–111`，`src/repair/benchmark.rs:43–65,203–228`，`src/bin/concir-backend.rs:246–255`；对照 `REPAIR_LOOP_HANDOFF.md` 第 5 节。

- repair JSON 没有冻结 contract、实际配置、代码版本及逐节点完整 VerificationReport；未接受节点也没有可应用的 incoming patch。root INVALID/UNSUPPORTED/UNKNOWN 时 nodes 为空，丢失原始诊断。
- bench 的 accepted_chain 是长度，不是补丁链；没有最终程序和完整报告。code_version 固定为 Cargo 包版本 `0.1.0`，不能区分本次未提交实现与其他同版本实现。
- Handoff 声称两种输出均包含这些字段，实际输出不符合。

证据：`bench.json`，任一 `*_b.stdout.json`，unsupported_repair 的空 nodes。

修复：提供共同的可导出 artifact 或由摘要引用的独立完整文件，包含输入、冻结契约、实际配置、源码版本/dirty 指纹、节点/尝试及补丁关系、验证报告、最终 CIR 和复现命令。不能依赖进程内私有 Node 或原工作目录才能复核。文档应与实际 schema 一致。

## 复现入口及下一步

原始全量日志见 `full-tests.txt`；CLI 探针见 `cli_probes.py`；API 探针见 `probe/src/main.rs`。它们读取 ConcIR，用 `/private/tmp/concir-audit-repair-loop` 存放运行输出，需先创建该目录。

```sh
cd /Users/kevin/local-repos/ConcIR
INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo test --offline --all-targets --no-fail-fast
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/repair-loop-working-tree/cli_probes.py
CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo run --offline --manifest-path /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/repair-loop-working-tree/probe/Cargo.toml
```

API 探针生成 patches.json 后，可复现旧入口预算：

```sh
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend repair tests/repro_bench/single_cycle.json tests/repro_bench/single_cycle_contract.json /private/tmp/concir-audit-repair-loop/patches.json 0
```

下一轮只收敛 E1–E8 和针对性回归，随后重跑开发基准。暂不扩展原语、LLM 接入或论文规模实验。可直接交给 Cursor 的任务见 `NEXT_CURSOR_PROMPT.md`。
