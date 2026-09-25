本轮目标分两段，先 A 后 B，A 完成并交付清单后才开始 B。

A. 仓库重组：`/Users/kevin/local-repos/ConcPlanVerify` 成为唯一的"实验仓库"（prompt、基准、实验数据、审阅记录、协议文档全部在此）；`/Users/kevin/local-repos/ConcIR` 只保留工具代码；`/Users/kevin/paper-review/papers/ConcPlanVerify` 只保留论文 tex 相关文件。
B. 设计并实现一套比原论文更充分的对比实验框架（多臂对比 + 消耗度量 + 消融 + 检测能力对比 + 规模实验），先离线跑通全部 harness，再用 DeepSeek Flash 跑一个冻结的小批次。不补写论文；不换模型。

先读：
/Users/kevin/paper-review/papers/ConcPlanVerify/REPOSITORY_BOUNDARIES.md（将被迁移并改写）
/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/deepseek-flash-repair-v1/LLM_PATCH_REPAIR_HANDOFF.md
/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/offline-integration-v1/CLI_CONTRACT.md
/Users/kevin/local-repos/ConcIR/doc/backend-usage.md
/Users/kevin/local-repos/ConcPlanVerify/README.md 与 python/cir_workflow/{live.py,patch_repair.py,offline_workflow.py,structural.py}
论文 paper.tex 只看 Sec. 4 的模式清单（Table 1），不把论文数字当预期。

════════════════════════════════════════
A. 仓库重组（只移动、改写文档、调整 .gitignore；不改任何工具/工作流代码语义）
════════════════════════════════════════

A1. 目标布局（ConcPlanVerify）

```
ConcPlanVerify/
  python/cir_workflow/            # 现有编排代码（保留）
  prompts/                        # 所有 prompt 资产；python/cir_workflow/prompt_assets 移到此处，代码路径同步改
  benchmarks/
    patterns/P1..P9/              # spec.md（自然语言规格）、buggy.rs、fixed.rs、buggy.cir.json、fixed.cir.json、contract.json
    real-cases/                   # 从 ConcIR/experiments/real-cases-v0 迁入（rmw-zenoh-998、dashmap-369 及其来源记录）
    legacy-cir2cvn/               # 旧 cir2cvn 基准（含 benchmarks/rust 下 20 个 Rust 参考程序）原样迁入
  experiments/                    # 所有批次原始数据：paper 目录下 experiments/* 与 ConcIR/experiments/* 全部迁入，目录名不变
  reviews/                        # paper 目录下 backend-review/* 全部迁入
  scripts/                        # ConcIR/scripts 下 run_pilot.py、pilot_analyze.py、pilot_audit.py、pilot_checks.py 迁入（它们是实验脚本不是工具代码）
  docs/
    REPOSITORY_BOUNDARIES.md      # 迁入并改写为新三仓分工
    prompts/                      # 本文件所在
    MIGRATION_2026-09-18.md       # 迁移清单（见 A3）
```

A2. 三仓分工改写（写入 docs/REPOSITORY_BOUNDARIES.md）

- ConcIR：Rust 工具代码（src/、tests/、doc/、examples/、Cargo.*）。不含实验数据、实验脚本、prompt。`.gitignore` 中 `experiments/` 条目在迁移后删除目录本身。
- ConcPlanVerify：Python 编排 + prompt + 基准 + 实验数据 + 审阅 + 协议文档。
- paper-review/papers/ConcPlanVerify：仅 paper.tex、checklist.tex、refs.bib、neurips_2026.sty、figures/、graphviz.svg 及编译产物（已被 .gitignore）。不再存放 experiments/、backend-review/、REPOSITORY_BOUNDARIES.md。

A3. 迁移规则

- 先 `git status` 三仓，列出已跟踪/未跟踪文件；已跟踪的用 `git mv`，未跟踪的直接移动；不覆盖任何已有批次目录，目标存在同名则停止并报告。
- 迁移前后对每个文件计算 sha256，写入 `docs/MIGRATION_2026-09-18.md`（源路径 → 目标路径 → sha256 → 跟踪状态），并校验迁后一致。
- 迁入的 experiments 中所有 handoff/protocol/manifest 内的绝对路径不改写内容（它们是历史记录），但在 MIGRATION 文档里给出"旧路径 → 新路径"映射表。
- 检查 ConcPlanVerify `.gitignore`：实验原始数据（runs/、llm/*.jsonl、artifact json）必须被跟踪，只忽略 `__pycache__`、`.venv`、`.env`、二进制。若单文件 >5 MB 列清单报告，不自行删除。
- 迁移完成后，两个源目录只剩允许保留的内容；paper 目录下若有 `.cursor` 规则引用旧路径则更新。
- 代码中所有指向 `prompt_assets`、`experiments/...`、`scripts/...` 的路径全部改到新位置；Python 56 项测试与 Rust 229 项测试迁后全部通过，作为 A 段验收。
- 不读 `.env`，不动凭据，不提交（commit 由用户决定）；交付时给出建议的 commit 分组。

════════════════════════════════════════
B. 实验框架 v2
════════════════════════════════════════

B0. 设计原则

- 所有臂共享同一组任务输入（自然语言规格 spec.md），同一模型、同一温度、同一轮次上限 K、同一 token 上限；只有"验证/反馈来源"不同。
- 每一臂的"接受"由该臂自己的判定器决定；最终正确性由**独立于所有臂**的终审 oracle 判定，两者分列，用于计算 false-accept。
- 消耗度量统一：HTTP 次数、prompt/completion/total tokens、LLM 墙钟、工具墙钟、迭代轮次、首个正确解出现的轮次。所有度量来自实际记录，不估算。
- 任何臂内的 LLM 迭代都是"LLM 修改整份产物"（Rust 源或 CIR）；我们臂额外保留 `external_single_patch` 作为独立模式单列，不与整份修改混算。
- 现阶段只用 `deepseek` / `deepseek-flash`，规则与前几轮相同（thinking disabled、max_tokens=4096、temperature=0、timeout≤90 s、SDK max_retries=0、最多一次 transient 重试、响应 model 不一致停批、禁止 Pro/回退、key 只从 .env 读且不落盘）。多模型对比留待下一轮，框架必须让模型名成为参数。

B1. 任务集（benchmarks/patterns + real-cases）

- P1–P9：从 `benchmarks/legacy-cir2cvn/benchmarks/rust/` 中挑出对应的 buggy/fixed Rust 参考程序（mutex_deadlock、signal_loss、channel_deadlock、three_way_deadlock、partial_deadlock、dual_condvar、semaphore_throttle、cas_race、fn_summary_prop），复制为 `buggy.rs`/`fixed.rs`；为每个模式写 `spec.md`（描述意图设计，含缺陷所在的设计决策，不写"这里有 bug"）、现行模块化 `buggy.cir.json`/`fixed.cir.json`、`contract.json`（`deadlock_free` + 每任务 `function_completed` preserved；P2 加 `var_eq ready=true`；P5 用 reachability/always_reachable 让缺陷在无死锁时暴露）。
- 额外加入 legacy 里的 scale 用例（deep_lock_chain_4x3、scale_lock_chain_5x3_buggy、scale_lock_chain_6x3、scale_branch_fan_4x2）作为 S 组，只用于 B5 规模实验。
- real-cases：rmw-zenoh-998（可验证）、dashmap-369（RwLock，预期 UNSUPPORTED，如实保留）。
- 每个任务登记 ground-truth：缺陷类型、涉及的资源与语句、预期修复类别。所有文件哈希进 `benchmarks/MANIFEST.json`，冻结后 live 只引用哈希。
- 每个 buggy/fixed Rust 程序必须 `cargo build` 通过，并附一个可判定的**行为测试**（`cargo test`，检查程序在超时内完成且产生预期结果，如 ready 标志置位、所有 worker 完成），用于终审的"行为保持"判定；fixed.rs 必须通过，buggy.rs 允许超时失败。

B2. 检测能力对比（Track D，无 LLM，最先做）

在 buggy.rs / fixed.rs 上分别运行，输出检出/漏报/误报、bug 种类、墙钟：

- lockbud：本机当前未安装（`cargo lockbud` 不存在）。按其 README 安装（需要匹配的 nightly；本机已有多个 nightly），锁定 commit/版本写入 manifest；安装失败如实记录为不可用并停止该臂，不用其他工具替代。
- miri：本机已有（`miri 0.1.0 (a69a63265c 2026-09-03)`）。用 `cargo miri run`，对每个程序做 N 个不同 `-Zmiri-seed` 与 `-Zmiri-preemption-rate` 组合（N 与组合表冻结在协议里），记录是否报 deadlock/data race、首次检出的 seed 数、总墙钟。miri 是动态工具，未检出不等于无 bug，报告里必须这样写。
- ConcIR：在 buggy.cir.json / fixed.cir.json 上 `explore`（petri 与 interp 差分），记录 outcome、complete、states、transitions、墙钟、细化标签（若 §B7 的标签实现了）。注意此臂输入是人写 CIR，不是源码；对比表必须注明输入形态差异，这是"模型层穷举 vs 源码层静态/动态"的能力对比，不是同输入公平对比。
- 输出 `experiments/detection-v1/DETECTION.md/json`。

B3. 生成+迭代对比（Track G，每臂每任务最多 K=4 轮，K 冻结）

臂定义（arm id 固定，写入每条记录）：

- `A0_direct`：LLM 从 spec 直接生成 Rust，单次，不迭代。基线下界。
- `A1_self_iter`：A0 之后，LLM 只凭自己审查（提示"检查并修复并发缺陷，输出完整程序"）迭代到 K 轮或 LLM 自报"无问题"。无工具。
- `A2_tools_iter`：A0 之后，每轮运行 `cargo build` + lockbud + miri（B2 的冻结配置），把工具原始诊断（截断到固定字节上限）反馈给 LLM，迭代到工具全绿或 K 轮。工具全绿即该臂"接受"。
- `A3_ours_revision`：LLM 从 spec 生成模块化 CIR → `check`/`support`/`explore`（冻结 contract）→ 把结构化诊断（现有 feedback prompt）反馈 → LLM 输出整份修订 CIR → 重新验证，直到完整 PASS + preserved 或 K 轮。fidelity 检查照旧分列。
- `A3p_ours_patch`：在 A3 首个静态有效且 FAIL 的 CIR 上，走现有 `repair-context → evaluate-patch → replay` 单补丁闭环（仅对可由 swap 修复的模式，其余记 `not_applicable`）。
- `A3_tool_repair`：同一 FAIL CIR 上的后端确定性策略搜索（strategy c），零 LLM 成本，作为成本参照。

每臂每任务记录：`arm`、`task`、轮次序列（每轮：请求哈希、tokens、LLM 墙钟、工具墙钟、工具原始输出哈希、该臂判定）、`arm_accepted`、`accepted_round`、总消耗。终审见 B4。

我们臂需要一个新的 Python 工作流 `revision_workflow`（或扩展 offline_workflow）：允许冻结前后 LLM 整份修订，模式标签 `repair_mode=llm_revision`；每一版 CIR 都存档并哈希；接受仍由 Rust 完整验证决定；scripted provider 离线测试覆盖"FAIL → 修订 → PASS"、"修订引入静态错误 → 反馈 → 修正"、"K 轮耗尽"。

Rust 臂需要 `rust_arm` 模块：把 LLM 输出写入隔离的临时 cargo 项目（固定 Cargo.toml 模板，只允许 std），`cargo build`/`cargo test`/lockbud/miri 都以子进程、超时、无网络方式运行，原始 stdout/stderr 全部存档；构建失败也是一轮反馈。

B4. 终审 oracle（独立于各臂）

对每臂的**最终产物**：

- Rust 产物：(i) `cargo build`；(ii) B1 的行为测试（超时内完成 + 预期结果）；(iii) lockbud + miri 冻结配置；(iv) 人工/规则对照 ground-truth 判定缺陷是否仍在（记录判定依据）。四项分列，不合成单一分数；"未检出"不写成"正确"。
- CIR 产物：Rust 完整验证（PASS + preserved）+ fidelity；另外把 A3 最终 CIR 用现有 `explore` 输出的 counterexample/blocked facts 与 ground-truth 语句对照。
- 由此计算：`arm_accepted ∧ oracle_bug_present` = false-accept；`arm_accepted ∧ behavior_test_fail` = 行为丢失型修复；`¬arm_accepted ∧ oracle_clean` = 保守拒绝。

B5. 规模与开销实验（无 LLM）

- S 组 + P4 变体：线程数 k=2..5、锁链长度、bounds 变化下 states/transitions/墙钟；标出 UNKNOWN 出现的边界。
- ConcIR 每次调用墙钟与 miri/lockbud 每次墙钟并列。
- 输出 `experiments/scale-v1/`。

B6. 消融（只在 A3 上，离线 scripted 先跑通，live 与 B3 同批）

- `A3_nodiag`：反馈只给 outcome（PASS/FAIL），不给结构化诊断。
- `A3_nopreserved`：contract 去掉 preserved，接受只看 deadlock_free；统计被终审判定为行为丢失的接受数。
- `A3_nofidelity`：不做 fidelity，统计"模型顺手修掉缺陷"被误计为成功的次数。

B7. Rust 小改（可选，若 B2 需要）

给已确认死锁的诊断加**事后**细化标签 `signal_loss` / `channel_block`（只用现有 blocked facts 与 witness，不改 outcome，不在非死锁时发标签），replay 覆盖该字段并兼容旧 artifact。做了就加 P2/P3/P6 回归并更新 doc；没做就在 handoff 写明。

B8. 预算与批次（先冻结 PROTOCOL 再跑）

- 离线：全部 harness（Track D、Rust 臂子进程、revision 工作流、消融开关、终审脚本、度量汇总）用 scripted provider 跑通，测试不联网不读 key；旧测试全部保留通过。
- live 批次 `experiments/flash-arms-v1/`：任务 = P1–P9 + rmw-zenoh-998（10 个）；臂 = A0、A1、A2、A3、A3p、A3_nodiag、A3_nopreserved（A3_nofidelity 是纯统计不需额外请求）。请求上限按 K=4 计算并写入 PROTOCOL，全批 HTTP ≤ 260 次、墙钟 ≤ 4 小时，`budget.json` 全批共享，重启不得重置；按臂分阶段执行，任一阶段停批都交付已得数据。
- 记录与前几轮一致（脱敏 messages、prompt sha256、requested/response model、request id、finish_reason、usage 或 null、耗时、轮次、反馈、工具完整 argv/exit/stdout/stderr 与哈希）。交付前扫描全部证据文件确认不含 key。
- 汇总脚本生成 `SUMMARY.md/json`：每臂×每任务的接受/终审/轮次/tokens/墙钟表；每臂聚合（接受率、false-accept 率、行为丢失率、平均轮次、平均 tokens、平均墙钟）；A3 vs A2 的成本差解释成分（LLM 时间 vs 工具时间）。

B9. 交付后停止

`docs/MIGRATION_2026-09-18.md`、改写的 `docs/REPOSITORY_BOUNDARIES.md`、`benchmarks/MANIFEST.json`、`experiments/{detection-v1,scale-v1,flash-arms-v1}/` 各自 PROTOCOL/SUMMARY/manifest/原始证据、`experiments/flash-arms-v1/EXPERIMENTS_V2_HANDOFF.md`。handoff 报告：三仓变更与提交建议、二进制/工具版本（lockbud commit、miri 版本、rustc）、复现命令、每臂每任务结果、预算使用与停止原因、与预登记的偏差、已知威胁（miri 非穷举、CIR 臂输入形态不同、单模型、样本量）。

结论限定为"单模型（Flash）上多臂对比框架的可行性与首批数据"。不做多模型、不做论文章节、不做新 provider、不改 ConcIR 核心语义（B7 除外）。完成后停止，等待独立审阅。
