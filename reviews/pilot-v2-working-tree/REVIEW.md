# Pilot-v2 交付审阅

日期：2026-09-18。仓库：`/Users/kevin/local-repos/ConcIR`。核心 HEAD：`e8480c4a54840e68d7cf3a62c5075e4fc02e6966`；本轮 scripts/experiments/pilot_tool 为未跟踪交付。审阅没有修改 ConcIR 源码或原始实验数据。

**结论：已交付的四批 pilot-v2 数据通过本轮一致性核验，可作为开发预实验保留；runner 的重启与 replay 恢复尚未达到可用于后续长实验的可靠程度。下一轮只做这些有复现的收尾，不重写 Petri 核心，不引入 LLM/POR，不重跑全部 heavy 实验。**

## 已确认的进展

- 本次重新执行 `INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast`，隔离 target：229 passed / 0 failed / 0 warnings。
- 四批 24 + 12 + 180 + 3 = 219 条，索引键、plan 和 attempts 一致；215 complete，其中 203 repair artifacts，12 explore；另 4 search_timeout。
- 独立核对全部 complete repair 的文件哈希、输入规范化结果、冻结 contract、effective_config、搜索退出码、replay 退出码及分类；另调用交付 auditor 核对计数，无不一致。57 个不同输入文件的字节哈希与规范化哈希通过。代码快照与当前交付代码哈希一致，二进制/helper 哈希一致。
- 重新 replay 9 个 artifact 全通过：覆盖共享资源跨模块、两缺陷、干扰、枚举顺序、权限/保留性质/UNKNOWN/已满足控制，以及 heavy 三缺陷 C 的完整搜索树。
- 重新执行 witness validation：11 个 legal witness 在原冻结 contract 下 PASS；bounds control 在原 contract 下 UNKNOWN、单独 expanded contract 下 PASS；2 个辅助模型 PASS，仍明确不是合法 repair witness；2 个无 witness 控制保持该身份。
- p1/p2 的跨模块共享资源构造现已通过 imports/FQN 真正共享 mutex；相邻交换保持 sid 和 unlock 语句。
- pilot-v1 1906 个已存档文件哈希均未变化。
- 原行为测试在临时复制工作区中 11/11 通过，统计检查通过。但下面的真实生命周期复现说明，这组测试未覆盖声称修复的完整路径。

## 必须修复的发现

### R1 [P1] 重启覆盖历史 attempt，改配置后批次仍混入旧记录

位置：`scripts/run_pilot.py:598–599, 878–921, 926–943`。

`used_attempts` 每次 do_run 都清空，第一项总是 attempt-001；目录仅用 case/config-name/strategy/repeat 命名，并且 mkdir(exist_ok=True)。已有 batch 会先覆盖 environment/code_snapshot/plan，再载入全部旧索引。改变 config 内容虽产生新 run_key，却仍写同一路径；旧键也不会退出当前统计队列。

通过临时复制、精简 manifest 和真实 do_run 路径复现：

1. 首次 A/B/C 正常完成；同 batch 第二次 search 注入 exit=2、不产 artifact。共 6 个 attempt 行只对应 3 个目录；第二次读到了旧 artifact，记录 exit_outcome_mismatch；索引保留旧 repaired，但其目录内 search.exit 已被覆盖为 2，索引仍记 0。旧成功本身曾经有效，但证据已被后续失败覆盖。
2. 同 batch、同 config_name，将 candidate_budget 增加 1：当前 plan 3 条，index/summary 6 条，只剩 3 个 artifact 路径，其中 3 条旧 artifact 哈希失配。

这并不意味着四批已交付数据已经受到污染：本轮逐批检查没有重跑、混配置或哈希失配。它意味着当前 restart/resume 机制无法安全支撑下一批长实验。

修复应优先采用明确、简单的批次不可变规则：同 tag 不带 resume 拒绝；resume 在写任何文件前核验冻结批次身份，配置/输入/代码/二进制不一致要求新 batch；每次真正执行使用独占的新目录，并保留过去的原始证据。目录应包含 run_key，attempt 标识必须跨进程持久唯一。

### R2 [P1] replay_pending 恢复标记 complete，却丢失搜索证据

位置：`scripts/run_pilot.py:707–733`（调用处 959–967）。

_replay_phase 只补 replay 结果和 artifact hash，没有继承搜索记录，没有 artifact_path，也没有恢复搜索计数、退出码、patch/root 信息和 wall_ms。真实 do_run --resume 路径中，用真实成功搜索记录构造“搜索已完成、replay 待完成”的合法中间状态，然后执行真实 replay，结果为 complete/repaired/replay_ok=true，但缺失 artifact_path、exit_code、search_wall_ms、verification_calls、states_explored、patch_len、root_outcome、wall_ms。

这条结果随后不能再次正常 resume，无法通过完整证据核验，并且统计中的 `or 0` 会把缺失的验证/状态计数当作零。

应继承并重新验证原 search evidence，记录 search 来源和本次 replay attempt；原 artifact、退出码、配置/输入、计数、修复链、搜索耗时必须保留。只有完整证据通过才能标 complete。统计/audit 对 complete 记录缺少必需字段应报错，不能默认为零。

### R3 [P2] 文档支持的外部 --out 导致 behavior 命令崩溃

位置：`scripts/run_pilot.py:139–148`，behavior 内部 tiny_manifest 位于 out 下。

直接执行 `python3 scripts/run_pilot.py --out /private/tmp/concir-pilot-v2-audit behavior`，在 code_identity 对临时 manifest/generator 路径执行 relative_to(REPO) 时 ValueError。此前的若干子检查已执行，但整个回归命令 exit=1。将未改动脚本和必要输入复制到临时工作区、使用默认 out 后，11/11 才完整通过。

应支持仓库外 manifest/output 的稳定身份，或明确约束 manifest 并把行为测试夹具放在合法位置；--out 指向临时目录本身不应导致崩溃。不要把不同文件仅按 basename 折叠成同一身份。

### 与 R1/R2 一起收尾的统计规则

`pilot_analyze.py:88–99` 的确定性分组仍然只使用 case/config_name/strategy。不同配置/输入/timeout 若进入索引会被误当作 repeat。分组应使用完整实验身份（排除 repeat，pairing 再排除 strategy），只在同一身份内比较不同 repeat；索引加载应检查重复 key、重复 repeat、与冻结 plan 的对应关系及 complete 字段完整性。这里无需另建统计框架。

## 结果的正确解释

- pilot 主批是 **16 个案例**：14 个案例 main×3 repeats+tight×1，2 个三缺陷案例 main×1+tight×1，三种策略，共 180。Handoff 中“15 cases”的分母说明需更正。
- 180 条中 repaired 72、already_satisfied 12、no_acceptable_candidate 54、budget_exhausted 26、analysis_unknown 12、search_timeout 4。不能把控制组失败解释为错误修复。
- 主批 B/C 60 对，其中 30 对双方 repaired；这些包含同一模型的不同 repeat/config，不是 30 个独立程序。共同成功对验证调用合计 B150/C132，states B480602/C385814，搜索 wall_ms B14213/C11318；C 验证调用 30/30 不更多，15/30 更少。
- heavy 三缺陷 p3_same：A no_acceptable_candidate，B/C 均 repaired，合法 chain 长度均为 3。B28/C20 次验证，states 2298516/1664282，交付搜索时间 99.630/72.053 秒。单案例单次测量，只能支持这里诊断排序减少验证工作，不能外推普遍速度提升。
- 根验证 p3_same：20k 为 UNKNOWN/incomplete；100k 为 FAIL/**incomplete**（已发现反例）；200k 为 FAIL/complete，103825 states。务必区分 finding FAIL 与完整探索。上轮的 20k budget 是重要混杂因素。
- 数据不支持“三缺陷无法修复”，也不足以断言瓶颈只来自状态空间而与搜索无关。总成本同时取决于验证次数及每次验证成本；先保留这个观测，真实案例上再测。

## 下一步

先完成有限范围的 runner 生命周期修复及真实路径回归，再用新 batch 做 smoke 和极小恢复验收。旧四批数据冻结保留；不要求全部 pilot/heavy 重跑。后续实质工作是小规模、来源可追溯的真实并发案例及源码→CIR 映射与适用范围说明，而不是继续堆同构合成模型。下一轮完整任务见 `NEXT_CURSOR_PROMPT.md`。

证据：`evidence/reproductions.json`、`reproduce.py`（临时副本上的真实 do_run/replay）、`delivery-audit.json`、`fresh-replay.json`、`core-tests.txt`、`witness_results.json`、`v1-preservation.json`。复现脚本只写临时路径；重新执行应使用新临时根或清理上次临时夹具，不能对原始交付批次运行。
