# 复核：experiments-v2 第 i 轮（主批次 v3、双轨 oracle、instrument、Track D/scale 刷新、RESULTS）

复核对象：`ConcPlanVerify` HEAD `41032fb`（干净）；`ConcIR` HEAD `1a83704`（干净）。
复核方法：读 `experiments/RESULTS.md`、`flash-repair-smoke-v3/run-…958361/SUMMARY.md`、`expert-labels/CANDIDATES.json` 摘要；打开 A0/A1 若干格的 `candidate.rs`；对照 RESULTS 与 SUMMARY 的数字。

## 结论

- **这是第一张可进论文的表**，且信号方向正确：`A3_local` 7/8 接受、均 2 轮、2356 tokens、全部在含 `holds_all` 的冻结契约下模型 PASS；`A3_whole` 6/8、3.17 轮、9657 tokens，`check_invalid` 在 local 模式下基本消失——局部再生成的消融成立。A0 两格 `hang` 的可测 false-accept；专家标注抓到 `cycle_3lock` A0/A2 两格"三锁环仍在但 16 种子都没撞上"——Miri 16 种子 + Lockbud 全绿却接受了带环程序，这是本项目最想要的那类证据。`concir-instrument` 走通端到端（1/8 validated，harness_error 0）。Track D、scale、RESULTS 都有了。
- **但表里有三处会改变数字的缺陷**（I-1..I-3），以及 RESULTS 与 SUMMARY 数字不一致（I-4）。修完需要重跑主批次；既然要重跑，就直接跑 3 次重复，让主表带方差。

## I-1 A0/A1 把"模型宣称无缺陷"记成 `no_build`（P1，改变主表）

- `notify_one/A0_direct/round-1/candidate.rs` 内容是 `NO_ISSUES`：在修复型任务里，模型对**由构造保证有缺陷**的输入宣称无缺陷，这是对 buggy 输入的接受，应记 `decision=claims_no_issue`、`accepted=True(input)`、`bug_present=True`（构造保证）、`false_accept=True`。现在记成 `no_build/accepted=False`，把 A0 的一次失败洗成了"没产出"。
- `cross_module_cycle/A1_self_iter/round-{2,3,4}/candidate.rs` 是散文："Looking at this program, both threads acquire locks in the same order… " ——模型在用自然语言说 NO_ISSUES，harness 按 `format_error` 处理并把三轮全部烧掉。A1 的 0.25 接受率大半是这个原因，不是模型不接受。
- `send_while_holding_mutex/A1` 在 r3 因 `NO_ISSUES` 被接受，但接受的候选 `build_ok=False`：`NO_ISSUES` 接受规则缺"被接受的候选必须已构建成功"这一条件。
要求：A0/A1 的回复解析改为三分法——完整程序 / 明确无缺陷宣称（sentinel 或散文均归此类，用一个宽松但记录原文的分类器，并在 PROTOCOL 写明判定词表）/ 其他（一次格式重试后记 `format_error`）。宣称无缺陷时：A0 = 接受 buggy 输入；A1 = 接受**最近一次构建成功**的候选，若没有则 `claims_no_issue_unbuilt`（不接受）。

## I-2 `false_accept` 没有并入专家标注（P1，改变汇总）

上轮规则：`bug_present = behavior=hang ∨ model=FAIL(validated) ∨ expert=yes`。SUMMARY 的 `cycle_3lock` A0/A2 两格 `false_accept=False`，而 `CANDIDATES.json` 标 `bug_present=yes`。按规则 A0 应为 3/7、A2-ml 为 1/8。SUMMARY 需加 `oracle.expert` 列并按规则重算；分歧格单列。

## I-3 A1 接受但未构建（P1）

见 I-1 第三条；这一格在 §4 标成 `unsure`，实际应是"未接受"。

## I-4 RESULTS.md 是手工拼的，与 SUMMARY 不一致（P1）

RESULTS §1 全部 `false_accept=None`、按臂 A0 `false_accept=0`、`mean_tokens` 947/1456；SUMMARY 同批次 A0 `false_accept=2`、`mean_tokens` 1105/4800。RESULTS 自述 "Assembled from committed artefacts"，没有生成命令。上一轮要求的 `python -m cir_workflow results` 未实现。RESULTS 必须由 JSON 渲染，否则不能进论文。§5 Track D 与 scale 也是散文而非表。

## I-5 HANDOFF 没有 Round i

RESULTS.md 不能替代 HANDOFF：HANDOFF 记过程（对照表、commit、偏差、停点），RESULTS 记结果。补 Round i。

## I-6 抽取 v4 的剩余失败可修（P2）

- 4× `conform unknown_sid at event 0`：模型漏掉 `spawn`/`scope` 标签对应的语句。`labels.json` 是**必填清单**：CIR 中每个标签必须恰出现一次作为 sid，缺失 → 在 codegen 前就回送"缺失标签列表"重试一次（计预算）。
- 3× `codegen`：CIR 里出现 Rust 类型 / `Arc::new` / `vec`。prompt 明示"只用 schema 的类型名"，normalizer 把 `Arc<Mutex<T>>`→`Mutex`、`Vec<_>`→拒绝回送。
- 1/8 validated 的 partial/A0：RESULTS 没写模型 verdict（应为 FAIL，与 `hang` 一致）；写上，这是双轨 oracle 第一次交叉验证成功。

## I-7 A3 在 partial_deadlock 上的停滞（P2，方法侧）

`A3_local`: `explore_fail → stalled_local_patch → stalled`。含 `holds_all` 的契约把"释放再获取"的空洞修复挡掉后，Flash 两轮给出同一候选。看 `preserved holds_all` 失败时的反馈：只有 "required behaviour … no longer reachable" 一类文字，没有说"设计要求某状态下 a 同时持有 a、b，你的修订里没有任何这样的状态；请保持嵌套获取，改用统一顺序或调整握手"。ConcIR 为 `holds_all` 失败加模板化 hint；同时这条是主表里唯一 A3 双臂都失败的任务，值得单独作为案例分析（含 K=6 探针一次）。

## I-8 binary 不统一（P2）

主批次 `6d498c8a`（`a65971f`），Track D/抽取/scale `88c3217d`（`1a83704`）。变更是加性的，但论文只能写一个 sha。重跑主批次时用最终 binary，并把 contract-strength、conformance-v4 的离线部分用同一 binary 重算。

## I-9 其他

- 专家标注者是 Cursor 代理（"proxy annotator"），rubric v1；论文中须如实写"LLM 辅助的专家标注 + 抽样人工复核"。建议用户本人对 ≥6 格（含两格分歧）做人工复核并记 `labeler: human`。
- 6 个任务的 Rust 参考程序是本轮新写的（RESULTS 已声明）；它们的 `repair_input/` 去泄漏 lint 是否通过，HANDOFF 要写。
- `A2_tools_iter_ml` 8/8 接受、Lockbud 全 clean——包括 `cycle_3lock`（Lockbud 在 detection-v2 里对同类程序假阴）。Track D v3 表里应能直接对上这一格。

## 对实验的判断

数据的方向已经清楚：验证驱动的局部再生成用更少 token 给出带证明的修复，工具驱动的基线会以全绿接受带环程序。下一轮的任务是让这张表**站得住**：修 I-1/I-2/I-3 后以 3 次重复重跑主批次（均值 ± 极差），统一 binary，RESULTS 全部由命令生成，抽取 v4 与专家标注覆盖全部接受格，`partial_deadlock` 单独做案例。之后就可以开始写论文的实验节。
