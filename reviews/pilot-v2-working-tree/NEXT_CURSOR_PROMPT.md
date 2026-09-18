你继续负责 /Users/kevin/local-repos/ConcIR。上一轮 pilot-v2 已被独立审阅：229 核心测试通过，219 条交付记录一致，203 个 complete repair artifact 的证据核对通过，9 个 artifact 新鲜 replay 通过（含 heavy 三缺陷 C）。跨模块共享资源、合法相邻交换 witness、统一 bounds 的修正有效。保留这些成果，不重新设计核心。

本轮是有明确终点的 runner 生命周期收尾。目标：重复启动、中断恢复、配置改变时，实验的原始证据和统计分母仍然正确。完成后即可推进真实案例语料；本轮不做 LLM/POR/新修复操作，不扩充同构合成案例，也不重跑全部 heavy。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/pilot-v2-working-tree/REVIEW.md
以及同目录 evidence/reproductions.json、reproduce.py、behavior.log。
文档中的行号以 reviewed-inputs 快照为准。不要把“已有 11 项 behavior 全过”当作下面路径已覆盖。

一、修复不可变 batch 和持久 attempt 身份（P1）

已复现：同 batch 二次启动，attempt 又从 001 开始；search.exit 被新失败覆盖，index 仍保留旧 repaired。同名 config 改 candidate_budget 后，plan=3、index/summary=6，6 条记录共用 3 个 artifact 路径，3 个旧哈希失配。

采用最简单清晰的生命周期规则：
- 已有 batch tag，不带 --resume 时拒绝复用，且在写 environment、plan、snapshot、index 等任何文件前拒绝。
- --resume 必须先核验冻结的 batch/cohort 身份。输入、完整配置、manifest、搜索/replay timeout、engine、核心二进制和实际实验代码等影响结果的内容不一致，不得向旧 batch 混写；提示使用新 batch。total-budget 属于恢复资源额度还是实验身份，明确选一种规则并记录，不要静默改变。
- 真正重试应分配跨启动唯一的 attempt 标识；raw 路径包含 run_key，目录独占创建，禁止 exist_ok=True 接纳旧 attempt。既支持已有 attempts，也考虑上次崩溃留下的目录；不覆盖历史文件。
- 旧 batch 的实验代码快照、plan 和原始 evidence 保持冻结；恢复事件单独记录。选择哪个 attempt 进入有效索引必须明确，且只能选择身份一致、证据可验证的记录。
- resume 复用不等于新的独立 repeat。当前 cohort 每个逻辑 run 只有一个有效记录；不把历史配置或重试计入新样本数。

二、修复 replay_pending 恢复的完整证据（P1）

已复现：真实 replay 成功后记录为 complete/repaired，但没有 artifact_path、exit_code、search_wall_ms、verification_calls、states_explored、patch_len、root_outcome、wall_ms。

要求：
- 恢复记录继承原搜索记录并重新验证 artifact 的 hash、输入、冻结 contract、effective_config、搜索退出码/结果绑定。
- 保留 artifact 路径/hash、全部原搜索计数、修复链信息、搜索耗时；明确原 search 来源和本次 replay attempt，记录 replay 命令/日志/退出码/耗时。
- 恢复只运行 replay，不偷偷再做 search；不能把缺失原搜索成本记成零，也不能把 replay 时间伪装成 search 时间。
- complete 必须满足同一套完整证据条件；恢复成功后能够再次 resume，并被 summarize 和 auditor 接受。
- 缺失/篡改 artifact、失败/超时 replay、缺少必需证据，不得标 complete。
- 对 complete repair 缺少 hash、计数或 evidence 的情况，audit/summarize 必须明确报错。不要用“if hash exists”或 `value or 0` 掩盖缺失。

三、外部输出目录和统计入口的小修复

- `python3 scripts/run_pilot.py --out <仓库外临时目录> behavior` 目前因 code_identity 的 relative_to(REPO) 崩溃。使该受支持命令正常工作；身份方案必须支持所需的外部夹具路径，避免 basename 冲突。
- 确定性分组使用完整 cohort/config/input/timeout/engine 身份，去掉 repeat 后比较不同 repeat；B/C 配对再排除 strategy。不同配置不能因名字相同被当成重复运行，不同代码/二进制身份不得混合。
- 统计加载验证 index 与冻结 plan 一致、run key 唯一、repeat 不重复；不静默覆盖同组重复项。UNKNOWN、timeout、no_acceptable、already_satisfied 与 repaired 保持分开。
- explore 的 evidence_status=complete 与后端 report.complete 是不同概念；表格分别保留，尤其 p3_same/100k 是 FAIL 但探索不完整。

四、补真正的生命周期回归

使用临时目录和最小真实案例；调用真实 do_run/CLI、classify、summarize、audit 路径，故障仅在 subprocess/文件/时间边界注入。不要用测试替代函数重写待测算法。
至少覆盖：
1. 已有 batch 同 tag、不带 resume：写入前拒绝，旧数据哈希不变。
2. 中断后同 cohort --resume：新 attempt 不覆盖旧日志/孤立目录，已完整结果正确复用，无重复分母。
3. 同 tag 改 candidate_budget/输入/timeout/code 或 binary 身份：拒绝混写；新 tag 可以正常运行。
4. 旧成功后当前 search exit=2 不产 artifact：失败或明确复用旧证据，不得把旧 artifact 与新 stdout/exit 绑定。
5. 搜索完成但 replay 未执行→新进程恢复：只补 replay，字段齐全，auditor 和 summarize 通过；第三次启动可复用，不再次搜索。
6. 恢复路径 replay timeout/failure/缺文件/hash 不符：状态正确，不被标 complete。
7. 同名不同配置、重复 attempt、仅 B 缺 C：统计分组、配对和分母正确。
8. 仓库外 --out 的 behavior 完整退出 0。

五、最小运行验收与交付

- 不修改 experiments/pilot-v1/ 和已冻结的 pilot-v2 四批目录；本轮产物放 experiments/pilot-v2-lifecycle/ 或等价新目录。
- 修好后执行 behavior、统计检查及新增生命周期测试；用新 batch 跑现有 24 条 smoke，再用一两个小案例演示正常/中断/replay 恢复。无需重跑 180 条 pilot 或 150 秒 heavy。
- 跑必要的 offline Rust 检查，保持核心语义与修复行为不变。
- 交付 LIFECYCLE_HANDOFF.md：每个发现的修复位置、命令、测试结果、新 batch 身份、有效记录与 attempt 数、旧数据未改动的哈希证据，明确未完成项。
- 修正文档口径：主 pilot 是 16 个案例、180 次运行；B/C 的 30 个共同成功对含重复/配置，不是 30 个独立程序；heavy B28/C20 是单案例证据。FAIL/incomplete 不等于完整探索。

这些验收通过就结束本轮，不再自动扩展框架。下一轮实质研究工作是来源可追溯的真实并发案例、源码→CIR 映射和适用范围审查；当前数据还不能替代这部分实验。
