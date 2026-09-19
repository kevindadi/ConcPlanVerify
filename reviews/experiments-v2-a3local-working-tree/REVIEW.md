# 复核：experiments-v2 第 h 轮（冻结契约强度、A3_local、抽取 v3）

复核对象：`ConcPlanVerify` HEAD `b7df9ad`（干净）；`ConcIR` 无变更（`a65971f`）。
复核方法：读 HANDOFF Round h、重算后的 `CONTRACT_STRENGTH.md`、`NORMALIZATION_REGRESSION.md`；逐格读 v2 批次 `SUMMARY.json` 的 `oracle.model.reason`；在 `extraction-v3c` 的 abba/A0 骨架目录里手动 `cargo build`。

## 结论

- **接受**：G-1 重算——脚本入库、record 带两个契约 sha、v2 partial v3 在冻结契约下被对称 `holds_all` 拒绝，这是契约补强的第一条硬证据；G-2 `A3_local` 主模式、sid 补齐/重命名、E208/E931 反馈增强、规范化回归 3/13 有数字；按"做不完停在当前节"停在 §3 并如实报告。
- **HANDOFF 对抽取失败的归因是错的**（H-1）："remaining gap is model compliance" 不成立——我逐格看了 12 个 reason，至少 4 类是 harness/normalizer 自己的问题。这导致 §4 主批次被一个不该存在的门槛卡住了一轮。
- 停在 §3 是遵守了规则，但规则设计有误：主批次不应被 Rust 臂 oracle 的完善度门控。本轮 prompt 明确解绑。

## H-1 抽取失败逐格归因（P1）

| reason 原文 | 格数 | 实际原因 | 归属 |
| --- | --- | --- | --- |
| `annotated Rust did not build` | 1 | 我在 `extraction-v3c/.../abba/A0_direct/skeleton` 里 `cargo build`：全部错误是 `cannot find module or crate cir_trace`——harness 用 LLM 的 Rust 覆盖了 `src/main.rs`，但没有补 `mod cir_trace;`。LLM 的代码本身可编译 | harness |
| `missing field 'kind' at line N` | 6 | 函数或资源缺 `kind`；HANDOFF 说 normalizer 已扩展函数 `kind`，但 v3b 批次的 6 格仍报此错（v3b 早于该修复或修复未覆盖资源） | normalizer |
| `invalid type: string "count", expected struct ParamDecl` | 1 | `params` 写成字符串数组而非 `{name,type}` | normalizer |
| `invalid type: sequence, expected a string at line 72` | 2 | 某字符串字段被写成数组（无 JSON pointer，无法定位） | normalizer + 反馈缺 pointer |
| `AttributeError: 'str' object has no attribute 'get'` | 1 | Python 崩溃 | harness bug |
| `extract_validated: true` | 3 | 三个 A3 格（模型判定直接来自接受版 CIR） | — |

也就是说 12 格里 0 格是"模型不合规"到无法挽救；**没有一格真正跑到 trace 一致性这一步**。另外 `extracted.cir.json` 里 `spawn.func: "worker1"`（非 FQN）与标注 Rust 里所有线程都用 `t0` 是两处会在下一步暴露的模型侧问题，normalizer 可补 FQN 前缀，tag 规则需要 prompt 或改由工具自动标注（见 H-2）。
要求：每格落盘 `extraction_result.json`，`stage ∈ {parse, normalize, codegen, build, trace, conform, explore}` + JSON pointer + 原始 stderr 路径；HANDOFF 的归因改为按 stage 的分布；`harness_error` 必须为 0。

## H-2 让抽取不再依赖 LLM 改 Rust（P1，结构性）

三轮抽取的失败面都在"LLM 同时产出 CIR 和标注 Rust 且二者一致"。Rust 侧可以完全由工具做：一个 `syn` 驱动的 `concir-instrument` 二进制，对 std-only 单文件 Rust 在每个并发调用点（`.lock()`, `.wait(`, `.wait_while(`, `.notify_one/all(`, `.send(`, `.recv(`, `thread::spawn`, `thread::scope`, `.join()`, 以及用户类型上名为 `acquire/release` 的方法）插入 `cir_trace::ev(<tag>, "L<n>")`，`L<n>` 按源码位置编号，并输出 `labels.json`（`L<n>` → 源码行、操作类型、接收者表达式）。线程 tag 用 `thread_local!` 计数器 + spawn 处传递父 tag 自动生成。LLM 只需把 `labels.json` 映射成 CIR（每条并发语句的 `sid` 必须取自标签集合），Rust 侧零 LLM 参与。
unlock 是隐式 drop，工具只能标注显式 `drop(guard)`；`conform` 需要 `--lenient-unlock`：模型的 `mutex_unlock` 步允许无事件静默推进。规则写进文档，标明这是抽取模式的放宽，codegen 模式不放宽。

## H-3 主批次不应被 Rust 臂 oracle 门控（流程）

Rust 臂的 `oracle.model` 是三列 oracle 之一，缺它时格子记 `inconclusive`，这是既定规则。§4 被 §3 卡住是我在验收线里把"validated > 0"写成前置条件的错。本轮解绑：主批次先跑，抽取与专家标注对盘上候选**事后**补，三者分列。

## H-4 专家标注作为第三个 Rust 臂 oracle（新增）

自动抽取即便修好也会有 `inconclusive`。对主表里每个被接受的 Rust 候选（预计 ≤32 个），按固定 rubric 做人工/代理标注：`bug_present ∈ {yes,no,unsure}`、涉及行号、`design_preserved ∈ {yes,no}`、一句理由；写 `labels.json`（含候选 sha、rubric 版本、标注者）。与自动 oracle 同时存在时报告一致率。这是并发缺陷论文常见做法，只要 rubric 与标注可审计。

## H-5 其他

- `CONTRACT_STRENGTH.md` 六行 `INVALID`（旧 abba 用 `A/B` 资源名）：加一列 `note` 写明"资源名与冻结契约不一致，属早期批次命名漂移"，不要留空的 `rejected`。
- 规范化回归 3/13：其余 10 份的错误码分布（E208/E931/其他）应列出，作为 `A3_local` 有效性的基线。
- `A3_local` 至今没有 live 数据。

## 对实验的判断

方法侧该补的都补了（契约、局部再生成、规范化、一致性层）；现在缺的是**数据**。这一轮要把主批次跑出来，同时把 Rust 臂 oracle 做成"自动抽取 + 专家标注"双轨，把 Track D 与成本表用当前 binary 刷新，并开始汇总一份面向论文的 `RESULTS.md`。任务量可以大，但主批次排第一，其他都不得阻塞它。
