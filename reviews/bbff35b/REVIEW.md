# bbff35b：E1–E8 复核及 artifact 收尾问题

日期：2026-09-17。提交：`bbff35b869b19f0fe05f2f68ff7f987c89256b5a`。ConcIR 工作区在审阅开始、结束均干净；本次未修改源代码。命令、版本及文件指纹见 review-metadata.json、source-manifest.json。

## 结论

全量测试 **222 passed / 0 failed**，未出现编译警告。上一轮复杂类型、验证 bounds、验证前去重、主要节点编号、预算边界及 CLI 回归均已修复并通过独立检查。E8 已具备完整数据导出与基本重建能力，但 replay 对 artifact 内部一致性的校验仍不足；另发现预算停止时漏记一次 attempt。

建议本轮只收敛下列 F1–F4，不再扩展搜索能力。收敛后即可做实验预跑和独立样本集设计；目前不将 replay 的成功当作整个修复记录已完整验收的证据。

没有发现这些反例导致在线搜索把原契约内的 FAIL/UNKNOWN 程序当作完整 PASS 接受。问题集中在导出记录及其独立验收；这里要求的是内部一致性检查，不是加密签名或对抗能整体重写全部证据的攻击者。

## 上一轮问题的独立复核

| 项目 | 实际结果 |
| --- | --- |
| E1 复杂类型 | bounded Int、Enum、嵌套 Struct、Array 四个模型均修复成功，保存最终 CIR 后用独立 CLI 重新读取、验证，全部完整 PASS |
| E2 bounds | 无效的 SearchConfig.bounds 已移除；冻结 contract.max_states=1 返回 UNKNOWN、1 个状态，导出实际 bounds=1 |
| E3 去重 | two_cycles 的 B：7 次 proposals、7 次验证、1 次命中；verification_budget=7 可以完成两步修复。C 为 5 次 proposals、6 次验证。preserved_unfixable 为 4 次验证、5 次命中 |
| E4 编号 | preserved_unfixable 的 4 个节点编号、父引用正确；禁止模块的两个拒绝尝试均 parent=0，只有一个真实根节点。预算停止仍有 F4 漏记 |
| E5 预算 | verification_budget=0 在执行前返回 invalid_config，0 次验证；depth=1/edits=1 分别报告 max-depth/max-total-edits 截断 |
| E6 旧 CLI | budget=0 不试候选；budget=1 修复成功；非法数字返回 2 |
| E7 退出码 | A/B/C 对 INVALID 均返回 4，对 UNSUPPORTED 均返回 5 |
| E8 artifact | 干净 artifact 能重建；损坏 input 或 node.incoming 的函数基准会返回 4。下面 F1–F3 的其他内部矛盾却仍被接受 |

证据：summary.json、positive-summary.json、full-tests.txt。全部独立 CLI 输入、输出和错误日志保存在本目录。

## F1 · P1：最终结果没有绑定到实际重建的节点和补丁链

位置：`src/repair/search.rs:954–966`，replay_artifact。

重放每个 node.incoming 后，函数另外验证 accepted_program 是否完整 PASS，却没有要求它等于重建的接受节点，也没有读取和重放 artifact.patch_chain。

独立复现（每次只改一个位置）：

1. 将成功的单步修复 artifact.patch_chain 清空。
2. 将 patch_chain[0].original_function_hash 改为 bad-hash。
3. 仅向 accepted_program 添加一个与修复无关的 Mutex 资源；input_program、全部 nodes 和 patch_chain 保持原样。锁交换不可能生成这个新增资源，因此它不属于记录中的任何修复结果。

三者 replay 均退出 0，并报告 `accepted_ok=true, outcome=repaired`。第三种程序本身仍满足性质，问题是它不是记录的补丁产物。

证据：empty_chain.json、bad_chain_hash.json、unrelated_accepted.json 及对应 *_replay.stdout.json；汇总见 summary.json。

修复要求：建立明确的接受节点引用或唯一解析规则，从根依次应用最终 patch_chain，校验每步基准及父子指纹；最终结果必须与接受节点、accepted_program、accepted_report 一致。总体 outcome、根状态、是否存在接受结果也必须相容。不要只重新验证一个单独提供的最终程序。

## F2 · P1：replay 没有重新检查补丁权限，也没有核对冻结契约与存档报告

位置：`src/repair/search.rs:918–928,944–949`。

在线搜索先调用 patch::check_allowed，replay 只调用 patch::apply，后者不检查 ContractSpec.allowed_scope。报告重验仅比较总体 outcome，不比较 contract_fingerprint 或各性质。

复现：把单步成功 artifact.frozen_contract.allowed_scope.allow_lock_reorder 改成 false，保留原来的锁交换和报告。replay 仍返回 repaired/accepted_ok=true。用这个相同契约重新执行在线搜索，得到 no_acceptable_candidate、0 proposals，确认两个入口的权限规则不一致。

另一反例：只删除 frozen_contract.preserved。原报告仍记录这些性质和原契约指纹，replay 却成功，因为根总体仍为 FAIL、最终总体仍为 PASS。

证据：forbidden_scope.json、deleted_preserved.json 及其 replay 输出；forbidden_fresh_search.stdout.json 是对照。

修复要求：每一步按冻结契约重新进行权限检查；校验报告的契约指纹、模型指纹、assumptions、bounds、required/preserved 结果与当前重验一致。契约被替换但旧报告没更新时，必须拒绝这份自相矛盾的记录。无需引入签名体系。

## F3 · P2：replay 成功不代表 attempts、有效配置和验证证据已一致

位置：`src/repair/search.rs:887–973`，尤其 `944–949` 的仅比较 outcome，以及 `968–972` 的无条件成功返回。

下列单字段损坏都仍退出 0、accepted_ok=true：

- attempts[0].parent=99999：引用不存在的父节点；attempts 根本未参与重放。
- counts.verification_calls=0、states_explored=0：与实际两个已验证节点不符。
- effective_config.bounds.max_states=1：与 frozen_contract 和完整报告中的 20000 不一致。
- 最终 node.report.complete=false，或清空根 node.report.properties：与本次重验不符，但整体 outcome 相同。
- node.incoming.program_fingerprint 改为 bad-hash：虽然 NodeReport 的指纹检查了，但 incoming 中的结果指纹没有检查。

证据：summary.json 中 bad_attempt_parent、false_counts、false_effective_bounds、false_node_complete、false_root_properties、bad_incoming_result_hash 及对应 JSON。

影响：用户无法把 replay 视为导出搜索轨迹和实验计数的完整一致性验收。当前校验能够确认部分节点的重建和最终性质，保证范围比文档的“任意不一致均显式失败”更窄。

修复要求：统一检查 schema 的身份、引用、边、去重目标和执行配置；重新验证并核对规范性报告字段。由完整记录推导可推导的计数，再与 counts 对照。不适合跨版本直接比较的字段应明确版本/比较规则，不要静默忽略。结构明显损坏时应在昂贵验证前失败。

## F4 · P2：验证预算耗尽时，已生成候选没有保存成 attempt

位置：`src/repair/search.rs:531–532,620–622`。

正常调用 two_cycles 的 B，设置 `--verification-budget 1`。根验证耗尽预算；随后产生一个候选，proposals 已加 1，AttemptFlow::Budget 分支直接退出，没有保存候选。

实际 artifact：`counts.proposals=1`、`attempts=[]`、`verification_calls=1`、stop_reason=verification-budget。这不是人工损坏的输入，而是当前正常运行生成的自相矛盾记录。

证据：verification_one.stdout.json、summary.json 的 verification_one。

修复要求：保留这个已生成但未验证的候选，记录 parent、patch、明确的 budget-blocked 状态和原因；不能伪造 outcome 或 verified 节点。让 proposals 与 attempts 的公开口径一致。若改成在生成前停止，也须保持验证前去重/缓存命中语义，不能通过提前退出掩盖可复用候选。

## 命令与证据组织

全量测试命令：

```sh
cd /Users/kevin/local-repos/ConcIR
INSTA_UPDATE=no CARGO_TARGET_DIR=/private/tmp/concir-audit-repair-loop-target cargo test --offline --all-targets --no-fail-fast
```

两个独立探针脚本读取仓库，输出到脚本同目录；需要上述路径下的最新 concir-backend 二进制：

```sh
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/bbff35b/probes.py
python3 /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/bbff35b/positive_checks.py
```

单独复现 F1/F2：

```sh
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/bbff35b/empty_chain.json
/private/tmp/concir-audit-repair-loop-target/debug/concir-backend replay /Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/bbff35b/forbidden_scope.json
```

当前均错误返回 0；修复后的预期为明确拒绝。干净 valid.json 应继续成功。下一轮任务见 NEXT_CURSOR_PROMPT.md。

文档小修：REPAIR_LOOP_HANDOFF.md 的修前基线应为上一轮独立验证的 208/0；更早的 29ee9ed 为 192/4，不是 208/4。
