# Pilot v1：独立审阅与下一步

日期：2026-09-18。核心基线 e8480c4，新增 scripts/ 和 experiments/ 尚未提交。本次未修改 ConcIR、生成器、runner 或交付数据。

## 结论

本轮确实完成了 pilot，而非仅交付计划。原始结果支持在当前生成案例上的组合修复能力及有限的诊断收益。但 runner 的失败隔离、续跑和汇总存在可复现错误，案例标签和预算设计也需修正。**下一步应做 runner 收敛和统一预算的 pilot v2，暂不进入 POR/对称约简等性能优化，也不直接把 pilot v1 当论文实验。**

这轮问题位于实验工具和实验口径；不需要重新打开已经验收的 Petri/解释器核心重写。

## 已独立核实

- 核心测试：229 passed / 0 failed。交付 runner selftest：5/5 通过。
- 228 条 results.jsonl 记录与对应原始 artifact、stdout、输入文件哈希、配置、计数、退出码一致，没有重复运行键。
- 抽取 11 个 artifact 重新调用 release CLI replay，11/11 成功，包含大干扰案例、三缺陷 UNKNOWN、权限/保留性质控制等。
- binary SHA256 与交付环境记录一致。
- 主配置 r1 下，共同修复成功的 8 个案例，B/C 验证次数合计 42/36，累计状态 150992/119396。五个二缺陷变体中 A 未修复、B/C 均完成两步修复。
- 记录分类总数与 summary.json 一致：smoke 24；pilot 204。其中 pilot 修复成功 66、原本正确 24、未找到 49、预算截断 17、UNKNOWN 48。重复测量不是独立案例，不能把 204 次运行解释为 204 个问题。

**未在现有 228 条正常运行记录中发现下面 R1 所示污染。** R1–R4 是对 runner 的独立故障/续跑探针，说明后续扩大或恢复实验时会发生错误；不能据此反推本次全部结果无效。

## R1 · P1：旧 artifact 可以把本次失败伪装成修复成功

位置：`scripts/run_pilot.py:250–323`。

同 case/config 名/策略/repeat 的输出目录被复用，运行前没有隔离或清除旧 artifact。分类优先读取现存文件，也不核对 artifact 与本次输入、有效配置、退出码的一致性。

独立探针：先正常执行 p1_same/B，产生 candidate_budget=64 的成功 artifact；再以同一目录、candidate_budget=0 发起新尝试，注入当前 repair 退出码 2、无输出文件的故障。runner 读取旧文件，真实 replay 旧文件成功，最后记录：

```
exit_code=2
config.candidate_budget=0
proposals=1
artifact.effective_config.candidate_budget=64
final_classification=repaired
replay_ok=true
```

另一反例：新运行只有合法形状的 stdout，缺少 --artifact 文件，最终仍可记 repaired、replay_ok=null，没有执行重放。

修正：每次尝试使用独立输出位置，绑定本次生产的 artifact、输入、配置和实际进程结果。成功分类要求完整 artifact 且重放成功；保留合法 FAIL/UNKNOWN 等非零退出，拒绝状态与退出码相矛盾的结果。输入/文件/JSON 错误不能读取历史文件补成成功。

## R2 · P1：重跑与跨配置结果会被混入同一个统计总体

位置：`scripts/run_pilot.py:243–247,493–574`。

run_one 追加 JSONL，但 summarize 对所有历史行直接统计。确定性分组缺少输入、真实配置、二进制和 suite 身份；B/C 配对也只用 case/config 名/repeat，最后一行覆盖同策略旧行。

独立探针中同一个 repeat=1 的两次尝试（第二次配置变化）被计为 records=2、repaired=2，还被列为同一组的“不确定性”。若只重跑 B，配对入口也可能把新配置的 B 与旧配置的 C 拼在一起。

修正：区分实验批次、逻辑运行、重试 attempt 和重复测量。日志可以追加，但统计必须从一个明确的有效运行索引读取；同一逻辑运行只选择有规则的一条最终记录。配对/确定性检查绑定完整输入、配置和执行环境身份，不以人类标签代替身份。

## R3 · P2：续跑接受不完整结果，运行身份遗漏执行约束

位置：`scripts/run_pilot.py:173–188,250–268`。

独立复现：

- 删除成功记录的 artifact，resume 仍返回 reused，保留 replay_ok=true。
- 把上一条结果设为 replay_timeout，resume 仍复用，因为失败排除列表漏了该类别。
- 超时从 5 秒改为 0.00001 秒，运行指纹不变，仍复用旧结果。

修正：使用明确的“完整且已验证”条件；核对所引用的 artifact/原始日志存在且内容哈希一致。运行身份包含实际执行约束，失败/待重放状态不能作为完整结果缓存。需要重用有效 search 输出补做 replay 时，应作为明确状态转换保存。

## R4 · P2：总预算不是硬截止时间

位置：`scripts/run_pilot.py:411–420`；run_one 的 search/replay 都取得完整 per-process timeout。

总预算只在两次逻辑运行之间检查。实际子进程未收到剩余时间，search 完成后也不检查总预算再启动 replay。

用 0.05 秒总预算和真实 sleep(0.25) 子进程替代昂贵 repair，子进程仍完整运行。本次探针总耗时约 0.377 秒（含环境采集），其中 0.25 秒是截止后仍执行的子进程。真实最坏情况可继续执行一次完整 search 和 replay 超时。

修正：用 monotonic deadline，把 min(阶段超时,剩余总时间) 传给每个进程，并在 replay 前检查。保留 not_executed、search_done/replay_pending、external_timeout 等状态及部分文件。Popen 异常也应形成结果而非中断整个批次。

## R5 · P2：新实验代码未进入可复现的源码身份

位置：`scripts/run_pilot.py:80–101`。

交付时 runner、generator 都是 untracked，git diff HEAD 为空。全部结果的 source_id 为 `e8480c4...+dirty:e3b0c44298fc`，dirty 后缀是空内容哈希。修改任何未跟踪 runner/generator 内容仍得到这个标识。binary SHA 能标识核心二进制，却不能标识实验调度、生成或统计逻辑。

修正：记录 runner、generator、manifest、分析脚本的内容哈希/源快照；在版本控制或固定源码 manifest 中包含实际实验代码，排除输出目录，避免生成输出反过来改变运行身份。

## R6 · P2：部分案例维度与“修复见证”标签不对应实际模型

位置：`experiments/pilot-v1/generate_cases.py:257–306,322–324,197–212,389–421`。

- `cross=True` 只写到 params，不改变模型构造。p1_cross 的 modules 只有 main，去掉 program 名称后与 p1_same 完全一致，却标为跨模块单循环。这不能算跨模块覆盖。
- witness 重写整个锁函数，修改了 mutex_unlock 资源及稳定 sid 对应的锁资源。当前合法修复空间只有相邻 lock 语句交换，因此这是一个语义修正版模型，不是已经验证可应用的合法修复补丁链。
- 所有 witness 契约（包括原本 200000 的普通案例）都改成了 1000000 状态，还更改了契约名。扩界验证可以另作辅助证据，但不能写成对同一冻结 contract 的直接验证。
- 权限禁止和 preserved 不可满足的控制案例仍带“minimum patch length is 1”的通用文字。应区分缺陷结构下界、允许编辑空间内可修复性、完整契约下的最小修复；不能把构造缺陷数当作不可修复控制的最优修复数。

修正：检验生成参数与模型结构一致。合法修复见证从原程序按权限和哈希应用实际补丁产生，保留原始契约/预算；若额外扩界或绕过权限，仅标注辅助语义模型，并单独保存其契约和验证结果。

## 实验设计：先排除预算混杂，再决定优化

一、二缺陷的 max_states=200000，三缺陷则预先降低到 20000。因此三缺陷全 UNKNOWN 能说明当前冻结预算不足，但不能直接证明三缺陷已经达到实现的性能极限，更不足以据此优先投入 POR。

独立对照：保持 p3_same 程序、性质和其余边界不变，只把 max_states 改为 200000。release CLI 的根验证得到 **完整 FAIL，103825 状态，约 5.26 秒**。这是根验证探针，不是完整三缺陷修复，也不应与交付的 20000 状态结果混算。结果见 p3_same_200k.json。

下一轮在相同 max_states 与相同外部时间约束下比较规模，并以预算矩阵分别改变验证上限和搜索上限。先定位哪一项截断、实耗时多少，再决定是否值得做优化。受控扩界必须产生新的实验配置和契约文件，不能覆盖旧结果。

## 报告数字与证据

本次从原始 r1 数据重算，共同成功案例的 B/C wall_ms 合计为 4482/3523；Handoff 写为 4539/3544。验证次数和状态合计一致，耗时表应由统计脚本生成以避免旧数字残留。本次抽查只能确认保存的历史耗时与汇总关系，不把独立复跑时间当作原运行耗时。

核心测试日志见 full-tests.txt；runner selftest 见 selftest.txt。实际故障注入见 runner_probes.py、runner-probe-summary.json；原数据核对见 data_audit.py、data-audit.json；独立重放见 sample-replays.json。reviewed-inputs/ 保存审阅时的实验代码、manifest、结果及交付文档快照，delivery-sha256.json 保存整份交付的文件指纹。

## 下一步

先修复实验 runner 的证据绑定、身份和统计，再修正数据标签/见证，另建 pilot-v2 输出统一预算的预跑。保留 pilot-v1 原始结果，给出更正说明。下一轮不增加 LLM/POR/新原语，也不要求增加大量同模板变体。完整执行 prompt 见 NEXT_CURSOR_PROMPT.md。
