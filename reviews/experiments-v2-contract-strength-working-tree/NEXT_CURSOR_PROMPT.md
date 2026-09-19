本轮目标：(1) 用冻结的人工 contract 重算契约强度表（离线，脚本入库）；(2) 把 A3 的主模式改为局部再生成 `A3_local`，整份重发降为消融臂，并放开 sid 规范化；(3) 抽取 oracle 改双围栏协议 + sid 补齐，盘上 12 个候选补跑；(4) 跑第一批可进论文的多臂批次 `flash-repair-smoke-v3`：8 个 hard 任务 × {A0, A1, A2-ml, A3_local, A3_whole}，带终态要求与新契约，三列 oracle 齐全。不做多模型、不写论文、不调 Lockbud、不改 conform 规则。

先读：
/Users/kevin/local-repos/ConcPlanVerify/reviews/experiments-v2-contract-strength-working-tree/REVIEW.md（G-1..G-6）
/Users/kevin/local-repos/ConcPlanVerify/experiments/contract-strength-v1/CONTRACT_STRENGTH.json、a3-partial-rerun/run-*/calls/*-check/stdout.json（G-2 的 E208/E931 实物）
/Users/kevin/local-repos/ConcPlanVerify/python/cir_workflow/{revision_workflow,normalize,extract,arms,flash_smoke}.py、prompts/{concir_generation_v2,concir_patch_v1,rust_to_cir_extract_v2}.md
/Users/kevin/local-repos/ConcIR/src/validate/（E208/E931 的产生处）、doc/error_codes.md

仓库边界不变；每节一个 commit；所有实验 JSON 带 `binary_sha256`/`git_rev`/`contract_sha256`。

一、契约强度重算（G-1，离线）

1. 新建 `python/cir_workflow/contract_strength.py` + CLI `python -m cir_workflow contract-strength --batches …`：输入历史批次目录，枚举所有 A3 接受版 CIR，对每个用**该任务冻结的** `benchmarks/families/<task>/contract.json` 跑 explore（petri），同时保留旧契约（该批次 run 目录里的 `contract.json`）结果做对照。
2. record 字段：`run, task, accepted_version, cir_sha256, old_contract_sha256, old_outcome, frozen_contract_sha256, new_outcome, rejected[], binary_sha256, git_rev`。
3. 输出覆盖 `experiments/contract-strength-v1/CONTRACT_STRENGTH.{json,md}`（旧表移到 `CONTRACT_STRENGTH.derived-v0.md` 并注明"推导契约，作废"）。md 开头写明契约有效性前提：`build_families.py` 在冻结契约下 21 buggy FAIL / 全部 fixed PASS。
4. 单元测试：一个 scripted 接受版 + 冻结契约 → 记录字段齐全；`rejected` 与 explore 的 preserved 失败项一致。

二、A3 主模式改为局部再生成（G-2）

1. `A3_local`（新主臂）：反馈 = 结构化诊断 + `doom_state` + `related_functions`；prompt（`prompts/concir_local_revision_v1.md`）要求只回 `{"functions": {"<fqn>": [<body>]}, "new_resources": [...], "removed_resources": [...]}`；harness 把回复合并到上一版（未提及函数与资源原样保留），跑 normalize → check → support → explore。允许 LLM 在 `functions` 里新增函数（须同时出现在某个 `scope/spawn/call` 里，否则 E-孤立函数 回送）。
2. `A3_whole` = 现有整份重发，降为消融臂。
3. `normalize.py`：缺 sid → 按序补 `s<n>`；不合规 sid → 确定性重命名并重写 `goto/branch(then/else)/switch(cases)` 目标；`normalizations[]` 记 `{rule: sid_rename, function, from, to}`。取消旧的"sid 不合规只回送"。
4. 反馈增强（Python 侧渲染，不改 ConcIR）：E208 附 `declared_base`、`observed_init`、合法例子；E931 附该函数 `params/locals` 列表与"如需新局部请在 `locals` 声明"的提示。
5. 离线回归：收集 smoke-d、v2、a3-partial-rerun 里全部 `check_invalid`/`sid_invalid`/`schema_error` 的候选原文（≥8 份），经新 normalizer 后统计 valid 数，写进 HANDOFF（"规范化后直接 valid x/y"）。
6. 局部合并的单元测试：只回一个函数 → 其余不变；回复引用未声明资源 → 回送而非崩溃；sid 重命名后 goto 目标一致。

三、抽取 oracle（G-3）

1. `prompts/rust_to_cir_extract_v3.md`：输出两个围栏块 ```json（CIR）与 ```rust（标注源码），不再要求单个 JSON 对象；sid 可缺省（由 normalizer 补），但 `ev` 里的 sid 必须与 CIR 一致——为此要求 LLM 先写 CIR 再写 Rust，且 Rust 中的 `ev` 引用 CIR 里已写出的 sid。
2. `extract.py`：解析双围栏；normalize（含 sid 补齐）；构建；20 native + 8 miri；全部 conformant 才 `extract_validated`。
3. 对 v2 的 12 个盘上候选补跑（≤24 请求），更新 `oracle.model`（旧值留 `oracle.model_v2`）。报告 `extract_validated` 数与未验证原因分布；对未验证的给首个 violation 的事件索引与 frontier。
4. 可选：对 `A3_free` 程序跑一次抽取（G-5），若验证通过则说明 66 违规是标注错而非代码错，写进 HANDOFF。

四、`flash-repair-smoke-v3`（第一张可进论文的表）

1. 任务（8，全部用 `repair_input/`，requirements 含终态 `DONE` 行）：`lock-order/partial_deadlock_bystander`、`lock-order/cross_module_cycle`、`lock-order/cycle_3lock`、`structure/nested_scope_lock_order`、`condvar/notify_one_multi_waiter_wrong_pick`、`channel/bounded_backpressure_lock_held`、`channel/send_while_holding_mutex`、`semaphore/acquire_twice_no_release`。任一任务缺 `repair_input/` 或冻结契约未通过 buggy FAIL/fixed PASS，先补齐再跑。
2. 臂：A0、A1、A2-ml、A3_local、A3_whole。K=4。Miri 16 种子 ×8 s。
3. 预算：修复臂 ≤200 请求（8×5×4=160 + 重试余量），抽取 ≤80，总 ≤280；`max_seconds` 3 h；超限即停并记 `stop_reason`，已完成的格照常入表。
4. Oracle 三列齐全：`behavior_status`（`terminated_ok/terminated_wrong_state/hang/no_output/no_build`）、`miri` 状态计数、`model`（A3 用接受版 CIR 在冻结契约下的 verdict；Rust 臂用抽取）。`bug_present`、`false_accept`、`inconclusive` 按既定规则。
5. `SUMMARY.md`：主表（任务 × 臂：accepted/round/tokens/llm_ms/tool_ms/三列 oracle/false_accept）、按臂汇总（accept 率、false-accept 率、inconclusive 率、平均轮次、平均 tokens）、A3_local vs A3_whole 的每轮 decision 分布（`check_invalid` 占比是关键指标）。全部数字来自 SUMMARY.json，不手填。
6. PROTOCOL 增补：任务集与选择理由（工具漏报且 LLM 会错的类型；教科书两例 abba/bare_wait 移出主表作附表）、臂定义（A3_local 为主）、预算、oracle 规则；登记偏差 D-16（A3 主模式）、D-17（sid 规范化）、D-18（抽取双围栏）。

五、交付物与验收

- ConcPlanVerify：`contract_strength.py` + 重算表；`A3_local` 与合并逻辑、v1 局部 prompt、normalizer 升级与回归数字、抽取 v3 与 12 格补跑、smoke-v3 完整目录（SUMMARY/PROTOCOL/llm/calls）、HANDOFF Round h（G-1..G-6 对照表、重算后的强度表、规范化回归数字、抽取结果、v3 主表与按臂汇总、每节 commit、未完成项）。
- ConcIR：如无需改动则只记录 `cargo test` 数量与 binary sha；若为 E208/E931 增加诊断字段则单独 commit。
- 验收线：强度表每条 record 带两个契约 sha，且 v2 partial v3 在冻结契约下 FAIL；规范化回归 valid 数有明确 x/y；抽取 `extract_validated` >0 或每格有 violation 定位；smoke-v3 至少 6 个任务跑完全部 5 臂，`A3_local` 的 `check_invalid` 轮次占比在 HANDOFF 中给出并与 `A3_whole` 对照；Rust 臂每个 accept 格的三列 oracle 都有非 `no_output` 值。
- 做不完则停在当前节，已完成节全部提交，HANDOFF 写清停点。
