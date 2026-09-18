请在 /Users/kevin/local-repos/ConcIR 完成下一阶段：修复实验 runner 的可靠性问题，修正 pilot 案例标签/见证，然后实际执行统一预算的 pilot-v2。

先读独立审阅：
/Users/kevin/paper-review/papers/ConcPlanVerify/backend-review/pilot-v1-working-tree/REVIEW.md

同目录有 runner-probe-summary.json、runner_probes.py、data-audit.json、sample-replays.json 和 p3_same_200k.json。基线核心仍为 e8480c4，229 passed / 0 failed；本次问题在新增 scripts/ 和 experiments/，不要重写 Petri、解释器、CIR 或修复内核。

已有事实：228 条原始记录与各自输入/输出一致，11 个抽样 artifact 独立重放通过。8 个共同成功案例的 B/C 验证次数合计 42/36。当前数据没有发现旧 artifact 污染，但 runner 故障探针可以触发假成功；必须修复后才能扩大实验。

保留 pilot-v1 原始数据；保存修正后的代码与新结果到明确版本的输出目录。不要覆盖旧 artifact 或修改旧结果来适配新代码。

第一部分：收敛 runner（R1–R5）

1. 每次真实执行使用独立 attempt 输出目录，并明确绑定本次 artifact、stdout、输入、契约、effective_config 和退出码。旧文件不能成为新运行的成功依据。完整 artifact 缺失、结构不合法、内容与本次身份不符、重放未执行或失败时，不得记为 repaired/already_satisfied。

保留正常非零退出对应的合法 FAIL/UNKNOWN/INVALID/UNSUPPORTED；显式检查状态与退出码映射。input/contract JSON 的原始字节哈希与 AST 规范化哈希不是同一算法，采用明确的规范化/身份确认流程。Popen 异常、非法形状 JSON、输出文件缺失也要形成结构化失败记录，不能中止整个批次或读取旧文件补成结果。

2. 建立 batch/run/attempt/repeat 的清晰身份：repeat 是独立重复测量，attempt 是同一逻辑运行的重试。原始日志可以追加，但生成唯一有效结果索引；汇总、分母、B/C 配对和确定性检查只使用明确选定批次中的有效记录。绑定实际输入、契约、配置、超时、二进制、实验代码和必要环境身份，不仅依赖 case/config 标签。重跑相同 repeat 不应增加样本量；不同配置或不同输入不得配成 B/C 对。

3. Resume 仅复用证据完整且身份匹配的结果。核对 artifact/原始记录存在、内容哈希及重放状态；replay_timeout、artifact 丢失/损坏不能当完成。每阶段 timeout 等执行约束进入身份，配置改变必须生成新运行或明确重新执行。若 search 已完成而 replay 未完成，可以有显式的 replay_pending 恢复流程，不伪造完成。

4. 使用 monotonic 全局 deadline。每个 search/replay 的实际 timeout 为阶段上限与剩余总时间的较小值；启动每个阶段前检查剩余预算。总预算耗尽时保留部分文件、尚未执行清单和待重放状态，清理超时进程组。明确环境采集/构建是否在运行预算外，并单独记录。

5. 可复现身份包含真实 runner、generator、manifest 和统计代码。当前 source_id 的 dirty 哈希来自空 tracked diff，没有覆盖未跟踪的新代码。保存实际代码哈希/源快照，明确纳入哪些源文件、排除哪些输出文件，避免输出改变自身身份。允许后续提交版本，但不依赖假定工作区已经提交。

新增真实 runner 入口的行为回归，至少覆盖：
- 正常运行；非零退出但合法 artifact。
- 同目录历史成功后，新调用失败且未生成文件，不得假成功。
- 仅有 stdout、缺 artifact；损坏/非对象 JSON；进程启动错误；replay 失败/超时。
- 重复执行、只重跑 B、同名 config 实值变化，分母和配对不混合。
- 缺文件/损坏文件的 resume、timeout 改变、replay_timeout 重试。
- 极短总预算内运行一个较长真实子进程，验证阶段 deadline 和进程回收。

这些测试应执行 classify/run_one/do_run/summarize 的真实调用路径，故障注入放在 subprocess 或文件边界；不能只证明后端会拒绝坏 JSON，或手写比较两个哈希不相等。

第二部分：修正实验输入与见证（R6）

1. p1_cross 目前只含 main，与 p1_same 除 program 名外完全相同。真正构造跨模块争用：两侧参与线程位于不同模块并访问同一组锁，通过合法 requires/provides/FQN 绑定；验证契约引用正确。为生成参数增加结构性断言，确认 cross/name-order/module-order/interference 参数确实改变预期维度。跨模块独立子系统与同一死锁跨模块是不同覆盖，明确命名。

2. 合法修复见证必须从原程序通过允许的相邻 mutex_lock 交换产生，保留稳定 sid 及所有 unlock，不重写整段函数。保存实际补丁链、父子身份和权限检查证据，对原始冻结 contract 独立验证。可以复用现有 Rust 库做小型见证生成/验收工具，不必修改核心语义。

3. 原预算下 UNKNOWN 的见证如需扩界，另外保存 bounded_witness 和 expanded_analysis 结果，标明两个不同的 contract/config。不能将所有 witness 默认改成 1000000 状态，再称为对同一冻结契约的验证。权限禁止/不可满足 preserved 控制的语义修正版只能作为辅助模型，不作为合法修复见证。

4. 分开记录：构造缺陷数、理论编辑下界、允许编辑空间可修复性、已验证修复上界/最小值。无可接受修复的控制案例不写 minimum patch length=1。未知或未证明的字段使用 null/unknown，给出论证来源。

第三部分：统一预算 pilot-v2，先定位瓶颈

本轮不要实现 POR、对称约简、新候选算法、LLM 或新原语，也不增加大量同模板样本。沿用修正后的小案例集。

独立对照已发现：p3_same 仅把 max_states 从 20000 改为 200000，即可完整验证为 FAIL，103825 状态，约 5.26 秒。该结果只涉及根验证，尚未证明完整三缺陷搜索能在时间预算内完成。先排除“给三缺陷更小预算”造成的混杂，再决定优化优先级。

具体运行顺序：
- 先通过 runner 行为回归及原 8×3 smoke。
- 对 1/2/3 缺陷及一个可达干扰变体，做根验证预算矩阵：同一组 max_states（例如 20000、100000、200000），其他 bounds、二进制、进程超时保持一致。全部配置预先写 manifest，保留所有 UNKNOWN/timeout。
- 选择预算矩阵中可完整分析的同预算配置，运行 A/B/C；同时保留一组紧搜索预算。比较 B/C 时所有验证和搜索约束完全一致。
- 对主要配置做 3 次重复，检查非时间输出确定性并报告耗时分布。输出包含完整计划、已执行/未执行项、验证和重放成本，时间与缓存规则明确。

初始可保持每进程 30 秒、主搜索批次总计 10 分钟；见证检查和根预算矩阵作为单独的有界批次，提前写入配置并如实记录。预算用尽就报告未完成项，不临时放宽某个策略、不把超时/未执行项排除后只算成功率。

第四部分：交付

生成 PILOT_V2_HANDOFF.md 及脚本自动生成的表格，避免手抄耗时数字（v1 的 wall_ms 表与原始 JSONL 已有不一致）。至少包含：
- R1–R6 的修正位置、行为测试和真实结果。
- 不可变批次标识、输入/代码/二进制身份、运行命令、原始数据与有效索引。
- B/C 在同输入同预算下的配对效果，成功集合差异、原本正确、失败/UNKNOWN/预算/timeout 分母。
- 根验证预算敏感性与完整修复搜索的分开结果；据实判断验证还是候选搜索限制规模。
- witness 的原契约结果、扩界辅助结果和编辑空间限制。
- pilot-v1 的数据更正说明，原始数据保持不变；pilot-v2 仍为生成式开发预实验，不宣称 held-out 泛化、统计显著性或正式论文实验完成。

运行适当的 runner/生成器检查及 INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast，保住已验收核心。交付后由 Codex 检查证据链和口径，再决定进入真实来源案例集建设或另开性能优化任务。
