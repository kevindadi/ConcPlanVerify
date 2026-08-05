# Goals 写入策略与信任边界

本文规定业务目标(`goals`)在 ConcPlanVerify 各阶段的产出责任、检查语义,以及三类负例(缺失 / 过弱 / 错误引用)对应的防线。

## 1. 语义

一个 `BusinessGoal` 由 `marking`(库所 → 最小 token 数)与 `variables`(全局变量 → 期望值)组成,所有谓词必须在**同一个可达状态**中成立,检查语义为可达性(EF):只要存在一条执行到达满足状态即视为达成。合法的 `marking` 键:

- 资源名(映射到 `rp_{name}`;Channel/Condvar 的 `0` 解释为"无残留");
- `"{fn}.{sid}"`(映射到控制库所 `cp_{fn}_{sid}`,表示"线程执行到某语句");
- 原始库所 id(`cp_`/`rp_`/`wp_`/`ra_` 前缀,面向工具)。

`variables` 的键必须是已声明的 `kind == "var"` 资源(`Var` / `Atomic`)。

## 2. 产出责任(写入策略)

| 场景 | goals 来源 | 强制性 |
| --- | --- | --- |
| NL → CIR 生成(generate / pipeline) | LLM 从需求中提炼 | **应产出**:需求中出现可观察结果(计数值、完成状态、消息送达等)时必须声明对应 goal;纯同步结构需求(如"不得死锁")允许 `goals: []` |
| benchmark fixture | 人工 gold 提供 | 由 manifest 的 `expected.outcome` 定义 |
| repair | 继承输入 CIR 的 goals | 修复**不得删除或弱化** goal(保全约束中列出) |

goals 缺失不改变验证判定(`verified_safe` 仍成立)——这是**信任边界**:验证器只能证明"声明过的性质",无法证明"该声明而未声明的性质"。实验记录中通过`declared_goal_count` 字段暴露该缺口,供 pipeline 层审计。

## 3. 三类负例与防线

| 负例 | benchmark case | 防线 | 判定 |
| --- | --- | --- | --- |
| 不可达 goal | `goal_unreachable` | 状态空间可达性检查 | `goals_unmet`(unmet_goals) |
| 过弱 goal(初始态即满足) | `goal_trivial` | `verify_program` 的初始态平凡性检查:goal 在初始状态成立 → 告警"too weak" | `goals_unmet`(goal_warnings) |
| 错误引用(不存在的库所/变量) | `goal_bad_reference` | `translate_goals`:未知 marking 键、未声明变量均产生告警;全部谓词失效时追加"no usable predicates" | `goals_unmet`(goal_warnings) |
| 缺失 goals | (无 analyzer 判定,见上) | 生成侧策略 + `declared_goal_count` 审计 | `verified_safe`(信任边界) |

三类可检负例均计入 manifest(gold = `goals_unmet`),其 `fixed.json` 携带非平凡、引用正确且可达的 goal,作为误报探针(必须 `verified_safe`)。

## 4. 对修复实验的意义

goal_warnings 与 unmet_goals 都会使状态离开 `verified_safe`,因此codegen 门禁同样拦截弱 goal / 悬空 goal。repair 的保全约束要求"Business goal ... must remain achievable",配合初始态平凡性检查,LLM 无法用"把 goal 改成恒真"来绕过修复。

## 5. Goals 约束的修复难度(拉开 LLM-only 差距)

`goal_constrained_deadlock` 把死锁与业务 goal 绑在一起:

- w3 的 else 臂以 `m2 → m1` 形成跨线程锁序环(缺陷);
- 同一臂是唯一写入 `result = 99` 的路径;
- goal `g_result_special` 要求 `result == 99` 可达。

正确修复只需重排该臂锁序并保留写 99。把所有写规范化为相同值、或删掉 else 臂,可以消掉死锁,但会得到 `goals_unmet` —— 离线探针(在 fixed CIR 上把 99 改成 3)已复现。因此「规范化式乱修」不再被 `verified_safe` 接受,CVN 反馈(指出锁序参与者)比无反馈更有机会一次修对。
