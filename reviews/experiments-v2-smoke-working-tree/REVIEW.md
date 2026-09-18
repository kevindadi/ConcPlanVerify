# 复核：experiments-v2 第 c 轮（能力族基准 + Flash 冒烟批次）

复核对象：`ConcPlanVerify` HEAD `dac4792`（工作树干净），`ConcIR` HEAD `6105d40`（4 个未提交文件）。
复核方法：读 HANDOFF/FAMILIES/SUMMARY，直接打开冒烟批次的 `result.json`、`candidate.rs`、`revision-1.cir.json`、`calls/*`。

## 结论

- **接受**：审阅项 P1-1/P1-2/P2-2/P2-3/P2-5 的修复；能力族基准骨架与 `build_families.py`（petri==interp，0 预注册失配）；Lockbud 集成与其假阳/假阴的如实记录；Track D v2 的证据落盘。
- **不接受为"实验结果"**：Flash 冒烟批次的 A1、A3（lost_wakeup）结论，以及 HANDOFF 对 lost_wakeup 的解读。三处都是 harness 缺陷或误读，不是方法结果。
- **需要设计层面调整**：当前生成型任务在简单模式上不区分各臂（LLM 本来就写对），比较退化；见 §3。

## 1. 冒烟批次中的 harness 缺陷（P1，必须先修）

| # | 现象 | 证据 | 判定 |
| --- | --- | --- | --- |
| C-1 | **A1 永远不接受**。prompt 要求"无缺陷回复 `NO_ISSUES`"，但 harness 把 `NO_ISSUES` 当作源码写进 `candidate.rs` 去编译，编译失败继续下一轮，直到 K=4 用完 | `lock-order__abba_2lock/A1_self_iter/*/round-{2,3,4}/candidate.rs` 内容均为 `NO_ISSUES`；lost_wakeup 同 | 三个任务 A1=False 全部无效；`flash_smoke.py:79` 只在 prompt 里写了哨兵，无解析分支 |
| C-2 | **A3 lost_wakeup 记为 `tool_error` 后一轮即停**，HANDOFF 却写成"revision did not reach a complete PASS"。实际是 LLM 把 `write_shared.expr` 写成对象 `{"kind":"bool","value":true}`，`check` 返回 exit 2 usage_error（JSON 解析失败），harness 未把该错误作为反馈回送，直接终止 | `A3/run-*/result.json`: `status=tool_error`, `check_status=usage_error`, `versions` 长度 1，http=1 | 该 CIR 语义上是正确的（谓词循环 + 写标志后 notify）。这是 schema 反馈缺口 + 报告误述 |
| C-3 | **HANDOFF 声称 A0/A2 在 lost_wakeup 上"接受了带 bug 的程序，Miri/Lockbud 漏报"**。实际 A2 round-1 的 Rust 是正确的（`while !*ready { wait }`，先置位再 `notify_one`）；A0 同类 | `A2_tools_iter/*/round-1/candidate.rs` | 不存在"漏报"，是 LLM 直接写对了。这一解读若进论文会被审稿人一眼推翻 |
| C-4 | `SUMMARY.md` 表中 A3p/A3_tool_repair 全为 None，但 HANDOFF 叙述有结果（abba：A3p accepted + replayed；tool_repair repaired） | 目录名为 `A3p/`，SUMMARY 用 `A3p_ours_patch` 键；聚合未对齐 | 报告聚合 bug |
| C-5 | A1 partial_deadlock round-4 的候选是代码片段（`let _g1 = m1a.lock()...`），不是完整程序 | `A1_self_iter/*/round-4/candidate.rs` | 输出解析未强制"完整 `fn main`"；应记为 `format_error` 并回送 |

## 2. 其他问题（P2）

- **`same_cv_different_locks` 的 `buggy` 变体 explore 为 PASS**，FAMILIES 注为"correct by construction"。要么改名为 `variant_a/variant_b`，要么删除 `buggy` 标签；标签与判定矛盾会污染 detection 统计（它现在算进"11 buggy FAIL"还是"7 PASS"？）。
- **`.gitignore` 的 `*.txt` 把原始 `stdout.txt/stderr.txt/exit.txt` 全部排除在 git 之外**，HANDOFF 称之为"repository policy"。这与"实验数据全部进 ConcPlanVerify 仓库"的边界约定冲突：哈希可校验完整性，但仓库克隆后证据不在。改为 `!experiments/**/*.txt` 或落盘为 `.log`。
- **A2 的"green"绑定了 Lockbud**，而 Lockbud `DoubleLock` 在正确的 3 锁与 partial_deadlock 程序上稳定假阳 → A2 在这些任务上不可能接受，这惩罚的是工具而非 LLM。应把每个工具的判定分列（`miri_green`/`lockbud_green`/`build_ok`），A2 接受规则在 PROTOCOL 中写成两档：A2-m（build+miri）与 A2-ml（build+miri+lockbud）。
- **ConcIR 工作树漂移**：`rust-toolchain.toml` channel 改成浮动 `nightly`，不可复现；应钉到 `nightly-2026-09-03`（当前 229 通过的版本）。`Cargo.toml`/`ast.rs`/`backend-usage.md` 的未提交改动需分组提交。
- 上轮 prompt 要求的 `cycle_3lock` 变体、`rendezvous` 变体、`nested_scope`、更多 condvar/channel 用例与两个新 real-case 未交付；HANDOFF 已如实声明，此处仅记录。**每族 ≥2 个 buggy 用例**仍未满足（channel 2、atomic-data 1、semaphore 1）。
- 冒烟批次终审 oracle：Rust 臂 `bug_present` 全为 null。没有 oracle，A0/A2 的"accepted"无法转成 false-accept 率，多臂比较的核心指标目前是空的。

## 3. 实验设计是否足以支撑方法（对用户提醒的回应）

**现状**：生成型任务（给自然语言 spec，让各臂产出代码）在 abba/lost_wakeup 这类教科书模式上，DeepSeek Flash 一轮就写对。于是 A0（只编译）也 accepted，A3 也 accepted，差别只剩 token（A3 多 3 倍）。这种实验对方法是**负面**的，不能作为主实验。

**可以站得住的三条证据链**（其他都砍）：

1. **检出能力（Track D，已有）**：同一批带缺陷的程序，ConcIR 全部完备判定 FAIL、修复版 PASS；Miri 65 种子 0 检出、Lockbud 假阳+假阴。这条已经成立，只需把 `same_cv_different_locks` 标签修好、每族补到 ≥2 个 buggy。
2. **修复型任务替代生成型任务作为多臂主实验**：输入 = 含缺陷的程序（Rust 给 A0/A1/A2；对应 CIR 给 A3）+ 同一份需求。缺陷由构造保证存在，false-accept 可测：A1 说"NO_ISSUES"、A2 工具全绿但程序未变或仍含缺陷，就是一次 false-accept。终审 oracle 用 §4 的一致性层（把各臂输出的 Rust 拉回 CIR 再 explore）+ miri-64 + 行为测试超时，三者分列，不合成。
3. **代码级后验证（用户要求的"后验证过程"）**：这是我们相对 Event-B Agent 的差异点，也是唯一能说"落到代码"的证据；设计见 §4。

**砍掉/降级**：A1 作为独立臂意义有限（Flash 二轮就说 NO_ISSUES），修好 C-1 后只作为 A2 的消融保留；生成型任务只保留 partial_deadlock、cross_module、k-worker 这类 LLM 实际会错的，作为附加表，不作主表。

## 4. 对 Event-B Agent（arXiv 2605.17475, FSE'26）的借鉴与差异

它做的：自然语言需求 → Event-B 模型 → Rodin 模型检查/证明器反馈 → 修复模型与证明，按精化层逐层推进。27 个系统，平均 182 个证明义务；指标 PDR（证明义务放行率 97.86%）、RC（需求覆盖率）、RF（需求满足率）、每系统平均 74 分钟；有组件消融。**它的产出止于模型，没有任何代码、没有模型到代码的一致性证据。**

可借鉴：
- 指标三元组可以直接映射：PDR → 我们的"contract 完备 PASS 率"；RC → fidelity/需求覆盖（每条需求是否在 CIR 中有对应 goal/preserved）；RF → goal 可达 + preserved 保持。再加我们独有的第四个：**代码一致性率（conformance）**。
- 按复杂度分层报告（他们按证明义务数分档；我们按 states/threads/家族分档）。
- 组件消融（他们消融精化与修复；我们消融结构化诊断 `nodiag`、`nopreserved`、以及骨架 vs 自由生成）。

差异点即卖点：我们多走一步——**从验证过的 CIR 生成带 sid 标注的 Rust 骨架，LLM 只填顺序空洞；然后用静态 lint + 运行时轨迹回放证明"代码的每一次观测执行都是模型的一次执行"**。这是 Event-B Agent 做不到的，也是审稿人会问"模型对了代码呢"的直接答案。

## 5. 一致性层（后验证）设计要点

供下一轮 prompt 引用；细节在 `NEXT_CURSOR_PROMPT.md`。

- **codegen**（ConcIR 新子命令 `codegen`）：每个 CIR 并发语句 → 一行 Rust + `cir_trace::ev("sid")`；`spawn/scope/join/call` 生成线程与函数结构；`branch/goto/switch` 生成控制流；`assign_local` 能直译则直译，否则留 `// HOLE(id)`；nobody 函数体留 HOLE。骨架外的每一行带 `// @cir sid` 尾注。
- **hole lint**（Python，`conformance.py`）：填充后的文件与骨架做行级 diff，HOLE 以外不得改动；HOLE 内不得出现 `std::sync`、`std::thread`、`Mutex|Condvar|RwLock|channel|Semaphore|spawn|scope|lock\(|wait\(|notify|send\(|recv\(|atomic|unsafe`；每个 sid 恰出现一次。
- **trace 回放**（ConcIR 新子命令 `conform <program> <trace.jsonl>`）：轨迹事件 `(thread_tag, sid)`；线程按 spawn 结构映射（子线程 tag 由父线程在 spawn 语句处分配，写进骨架）；参考解释器逐事件检查该 sid 在该线程当前位置是否 enabled；不 enabled 即 `violation`，输出前缀。轨迹来源：原生运行 N=50 + miri many-seeds 0..64。
- **结论只说观测到的**：`conformant_traces / total_traces`，以及静态 lint 通过与否；不说"代码正确"，只说"所有观测执行都是已验证模型的执行，且并发操作集合与模型一一对应"。
