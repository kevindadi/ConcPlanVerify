# 复核：experiments-v2 第 e 轮（去泄漏、规范化、一致性口径）

复核对象：`ConcPlanVerify` HEAD `708c30c`（工作树干净）；`ConcIR` HEAD `987d44e`（干净，toolchain 钉 `nightly-2026-09-04`）。
复核方法：读 HANDOFF Round e；打开 `repair_input/` 实物；跑 Python 全量测试（99 通过）；用当前 release binary 复现 `worker_with_payload` 的双引擎分歧；翻 smoke-d 的 miri `tool_error` 原始 stderr。

## 结论

- **接受**：R-1 去泄漏（`input.rs` 无注释、`program=case_b2`、需求措辞中性、lint 测试）；R-2 全链路（`schema` 子命令、v2 prompt、`normalize.py` 有记录改写、JSON pointer 反馈、停滞→局部修补→`stalled`，且 smoke-d bare_wait v3 原文离线回归 = check valid + explore PASS）；R-4 口径（`(function,sid)` 覆盖率 9/9、7/7、5/5、7/7，每种子一条轨迹）；R-8 卫生（两仓库全部提交，ConcIR 可 checkout 编译）。HANDOFF 对未完成项如实列出，没有粉饰。
- **本轮是半程交付**：上一轮 prompt 的 §四（诊断）、§五.3 多模块、§五.4 live 填洞/`A3_free`、§六 全部 oracle、§七 重跑，都没做。上一轮 prompt 范围过大是我的问题；下一轮只做剩余项，按阻塞顺序排。
- **新发现一个 ConcIR 正确性问题**（N-1），优先级高于其余所有剩余项。

## N-1 petri 与 interp 对空函数体 `call` 判定相反（P1，ConcIR）

`structure/worker_with_payload/correct.cir.json`：`compute` 为 `kind: normal`、`body: []`；`w1/w2` 在持锁状态下 `call main::compute`。
本机复现（release binary `709b74bd…`）：`explore … petri` → `deadlock_free FAIL`（"a reachable state has no enabled step and unfinished threads"）且两个 preserved 目标 FAIL；`explore … interp` → 全部 PASS。
根因推断：空 body 没有 `return` 语句，Petri 翻译没有为"走到函数末尾"生成返回迁移，调用帧永远不弹出；解释器把落出函数末尾当隐式 return。两种语义必须统一，且校验器应对"最后一条语句不是 `return` 也不是无条件 `goto`"给出诊断（error 或 warning，二选一写进 `error_codes.md`）。
影响：论文将依赖"全部基准 petri == interp"作为翻译正确性的证据，这个反例说明 `build_families.py` 的双引擎一致检查还没覆盖到空体/落出末尾这一类；修复后要对全部基准和 `tests/` 里的模型重跑一致性。HANDOFF 把该用例降为 `conformance_only` 是对的，但它应在修复后恢复为正常基准。

## N-2 R-5 根因已由复核方查明（不是 tool_error）

smoke-d `partial_deadlock_bystander/A2_tools_iter_ml/round-3/calls/004-miri/stderr.txt` 末尾：
`error: the main thread terminated without waiting for all remaining threads`（Miri 的线程泄漏检查）。LLM 第 3 轮把旁观者线程改成 detached 无限循环，main 退出时 Miri 判泄漏，exit=1。
不是 `RUSTC_WRAPPER` 残留（`env.json` 只有 MIRIFLAGS/RUST_BACKTRACE/CARGO_*），也不是工具故障。
要求：`rust_arm` 的 Miri 分类加一档 `thread_leak`（匹配该消息），与 `deadlock`/`data_race`/`timeout`/`tool_error` 并列；A2 的 green 规则里 `thread_leak` 算不绿；detection 统计单列。

## N-3 channel 一致性未闭合

HANDOFF：channel codegen 可构建，但"参考模型把 rendezvous 归到与 `ev` 不同的步"，因此不声称 channel conformance。需要定规则：`channel_send` 的 `ev` 在 `send()` 返回后发（= 完成），`channel_recv` 在 `recv()` 返回后发；`conform` 侧把 channel 事件视为"完成步"（与 lock/acquire 同类），rendezvous（cap=0）时 send/recv 两个完成事件的相对顺序不确定，frontier 必须同时接受两种顺序。修好后 `channel/rendezvous_both_send(fixed)`、`send_while_holding_mutex` 的修复版、`rmw-zenoh-998` 都要进 conformance 表。

## N-4 版本记录不一致（P2）

HANDOFF Round e 记 release binary sha `5aac4ac1…`，复核时 `target/release/concir-backend`（2026-09-19 02:17 构建）sha 为 `709b74bd…`。二者至少一个不是 `987d44e` 的产物。下一轮在 HANDOFF 记录 `git rev-parse HEAD` + `cargo build --release` 后的 sha，并在 conformance/detection 结果 JSON 里写 `binary_sha256`，让数据与 binary 绑定。
另：`concir-backend` 无参数时的 usage 文本未列出 `codegen`/`conform`/`schema`。

## 上一轮剩余项（原样带入下一轮）

| 项 | 状态 | 说明 |
| --- | --- | --- |
| R-3 诊断 FQN + `doom_state` + 模板化 hint | 未做 | 是 A3 在 partial_deadlock 上失败的直接原因之一 |
| R-6 多模块 codegen | 未做 | `cross_module_cycle`、real-cases 无法一致性 |
| R-7 live 填洞 + `A3_free` | 未做 | 用例已就绪（有 HOLE），但受 N-1 影响需先修 |
| VI.1 `behavior.rs` ×6 | 未做 | Rust 臂 false-accept 仍不可算 |
| VI.2 Miri many-seeds 进臂 oracle | 未做 | 仍是单种子 8 s；N-2 的分类修复一起做 |
| VI.3 抽取 oracle | 未做 | |
| VII repair-smoke-v2 重跑 | 未做 | 洗净后的输入尚未产生任何 live 数据 |

## 对实验的判断

去泄漏与规范化把两个会让多臂比较作废的缺陷清掉了，这是本轮的实际价值。但仍然没有一条洗净后的 live 数据，Rust 臂仍没有 oracle，所以多臂主实验至今没有可用数字。下一轮的唯一目标是**产出第一批可采信的修复型对照数据**：先修 N-1（否则"petri==interp"这条主张有反例）、N-2、R-3，再补 oracle，然后重跑 3 个任务。多模块 codegen 与抽取 oracle 若时间不够可以延后，其余不能。
