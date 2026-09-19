# 复核：experiments-v2 第 g 轮（契约补强、12 格 oracle 补全、channel 一致性）

复核对象：`ConcPlanVerify` HEAD `2d0f30e`（干净）；`ConcIR` HEAD `a65971f`（干净）。
复核方法：读 HANDOFF Round g、`CONTRACT_STRENGTH.{md,json}`、`conformance-v4`；打开 `partial_deadlock_bystander/contract.json`；读 A3 新契约重跑的 `result.json` 与 `calls/*-check/stdout.json`；搜索强度脚本。

## 结论

- **接受**：ConcIR `holds_all` / `mutex_exclusive` / `never_holds_all`；21 个 buggy 用例的 contract 写入设计意图（partial 的 `holds_all(main::a,[a,b])` 与对称项已在）；channel 一致性闭合（fixed/correct 全 100%）；`A3_free` 有数据（66/66 violation）；`behavior_status` 枚举与 "bug fixed" 措辞清除；SUMMARY 展示原始计数；F-2 的三处修复到位并如实报告 0 validated。
- **主结果不能采用**（G-1）：契约强度表用的不是冻结的人工 contract，而是"从每个接受版 CIR 自身名字推导"的契约，且生成脚本未提交、记录里没有契约哈希。
- **A3 连续第三轮因 CIR 书写合规输掉**（G-2）：这已经不是偶发，是整份重发（whole-artifact）模式的结构性问题，需要改主模式。

## G-1 契约强度表：推导契约循环、不可复现（P1）

`CONTRACT_STRENGTH.md` 自述："strengthened contract derived from each CIR's own names: … `holds_all` for any function that locks ≥2 mutexes"。问题：
1. 契约从**被评对象**推导，是循环论证——v2 partial 接受版被拒的三条里有 `preserved: main::c holds [a,b]`，c 是旁观者，从来不该同时持两锁；它被列为"设计意图"只是因为 LLM 的输出里 c 顺序拿了 a、b。
2. 12 个 PASS 同样不可信：推导契约对"没有任何函数拿 ≥2 锁"的接受版不会生成任何 `holds_all`，等于没加强。
3. `CONTRACT_STRENGTH.json` 的 record 只有 `run/task/accepted_version/cir/old_status/new_outcome/rejected`，没有所用契约及其 sha；仓库里搜不到生成该表的脚本（`rg CONTRACT_STRENGTH` 只命中 md/json）。
要求：用每个任务**冻结的** `benchmarks/families/<task>/contract.json`（本轮已含设计意图）重算全部 14 个接受版；脚本进 `python/cir_workflow/contract_strength.py` 并有 CLI 入口；record 加 `contract_path`、`contract_sha256`、`binary_sha256`；表格分两列：旧契约 outcome / 冻结新契约 outcome。预期 v2 partial v3 仍 FAIL（人工契约有对称 `holds_all`），其余数字以重算为准。

## G-2 A3 第三次因书写合规失败（P1）

新契约下 A3 重跑 partial：`explore_fail → sid_invalid → check_invalid → check_invalid`。第 4 轮的 check 诊断：`E208 init value does not match base type Int` ×3（`done_a/done_b/output`）、`E931 undefined name 'i'` ×2（旁观者里用了未声明的局部）。三轮 smoke 下来，A3 的失败轮次里 **>70% 是 schema/类型/未声明名**，不是并发推理。
根因是 whole-artifact 重发：每轮 LLM 重写整份 JSON（资源声明、所有函数），改一个函数的同时把别处写坏。
要求（改主模式，不是再补 normalizer）：
1. `A3_local` 成为主臂：反馈里明确列出 `related_functions`；LLM 只回 `{"functions": {"main::a": [body...], ...}, "new_resources": [...]}`；harness 合并回上一版（未提及的函数与资源原样保留），再 check/explore。整份重发降为消融臂 `A3_whole`。停滞时的"局部修补"已有实现，直接升级为主路径。
2. `normalize.py` 允许 sid 规范化：缺失 sid → 按序补 `s<n>`；不合规 sid（`b6`/`y1`）→ 确定性重命名并**同时重写** `goto/branch/switch` 的目标；映射表写进 `normalizations[]`。此前"不改 sid"的限制取消——重命名连带目标改写是纯语法操作，可追溯性由映射表保证。
3. 反馈里 E208 类错误附上"声明的 base 类型 + 实际 init 值 + 一个合法例子"；E931 附上"该函数已声明的 locals/params 列表"。
4. 用 smoke-d、v2、本轮重跑里全部 `check_invalid`/`sid_invalid` 的原文做离线回归：经新 normalizer 后有多少能直接 valid（报告数字）。

## G-3 抽取 oracle 0/23：原因可修（P1）

原因分布：缺 sid 6、非 `{cir,rust}` 对象 4、畸形 1。
- 缺 sid → 上面的 sid 补齐即可解决。
- "非对象"：把 Rust 源码塞进 JSON 字符串要求 LLM 转义换行与引号，Flash 经常写坏。改协议：**两个围栏块**（```json 放 CIR、```rust 放标注源码），harness 分别抓取。
- 12 个候选仍在盘上，修好后 ≤24 请求补跑；目标 `extract_validated` >0，否则给出每格首个 violation 的事件与 frontier。

## G-4 channel "attempt-events" 的证据强度（P2，记录即可）

本轮把 channel 的 `ev` 改成**尝试**事件（调用前）以闭合一致性。这与 lock/acquire/wait 的**完成**事件不一致，且对 rendezvous 而言，尝试事件只证明"到达了 send 语句"，不证明配对时机正确——任何走到 send 的程序都 conformant。可接受，但必须写进 `doc/backend-usage.md` 与论文 threats；长期方案是每个阻塞操作发 `try`+`done` 两个事件。本轮不改。

## G-5 `A3_free` 66/66 violation 的可解释性（P2）

自由生成 + 自标注失败，无法区分"代码错"还是"标注错"。作为"为什么要骨架"的消融足够，但要在 HANDOFF 写明这个歧义；可选：对该程序跑一次抽取 oracle，若抽取验证通过且模型 PASS，则说明是标注错，反之代码错。

## G-6 12 格 Rust 数据：`no_output` 10

v2 候选早于终态要求，`behavior_status` 全是 `no_output`，`oracle.model` 全 unverified。也就是说，**Rust 臂至今只有 1 格（A0 partial hang）有结论**。必须跑一次带终态要求、新契约、`A3_local` 的 live 批次才有可比数字。

## 其他

- HANDOFF 说 "21 buggy contracts"，上一轮说 19；MANIFEST 以哪个为准写清。
- `build_families.py` 在新契约下 buggy FAIL / fixed PASS 已通过（HANDOFF 声明），但 CONTRACT_STRENGTH 未引用这一事实作为契约有效性的前提，补一句。
- ConcIR 未记录本轮 `cargo test` 数量；补。

## 对实验的判断

契约补强的机制到位了，但用来证明它有效的那张表用错了契约。重算是零成本的（离线）。A3 的问题现在很清楚：不是验证反馈没用，是让一个小模型每轮重写整份 JSON 不现实；`A3_local` 把每轮的书写面缩到一两个函数体，是方法层面的合理设计（论文里 Tier 2 "局部再生成"本来就是这么写的），不是给 Flash 开小灶。做完 G-1..G-3 后，跑一次 8 个 hard 任务的 v3 批次，得到第一张可进论文的多臂表。
