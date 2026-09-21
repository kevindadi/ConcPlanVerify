# 复核：experiments-v2 第 m 轮（操作绑定 conform、第二模型、人工复核合并、freeze-2）

复核对象：`ConcPlanVerify` HEAD `7e52f032`，tag `experiments-v2-freeze-2`；`ConcIR` HEAD `42f7cfa`，tag `concir-freeze-2`。
复核方法：读 HANDOFF Round m、`conform-mutation-v2/{SUMMARY,CONFORM_GAPS}.md`、`post-edit-conform-v2/SUMMARY.md`、`model-probe-v2/{PROTOCOL.md,run-*/SUMMARY.md,budget_*.json}`、`TIERED_ADDENDUM.md`、RESULTS 专家/探针节、`PAPER_EVIDENCE_MAP.md`。

## 结论

- **L-1 解决得干净。** `cir_trace` v2 把事件绑到操作上后，conform 召回从 v1 的 M1 0.545 / M2 0.067 / M4 0.333 变为 **M1 1.0 / M2 0.947 / M4 1.0 / M6 1.0 / M8 0.8**，且失败原因从 `timeout` 变为 `violation`（kind 明确）。剩余盲区 M5（跨无事件语句的移动）与 M7 顺序敏感都如实记录。主张 (d) 现在有可写的数字。
- 人工复核 17 行合并：agent vs human 7/7，human vs auto 9/11；(h) ready。
- tiered 复核结论修正了我的前提：`bare_wait` 上 tiered **确实升级了**，失败是 K=4 装不下 4 轮 whole；K=6 2/3。数据说明"轮数需求任务相关"，结论合理。
- 第二模型探针跑通了两个非 DeepSeek 家族模型并定性复现模式。
- 实验部分**可以收口**。剩余问题是收尾质量（M-1..M-5），不再有方法层面的缺口。

## M-1 探针数据不完整却按完整汇报（P1）

`kimi-k2.7-code` 的 cells 是 A0 5 / A2 6 / A3 7（应各 8），`stop None`、25 请求，却在对照表里写 "5/5、6/6、7/7"。缺失格没有记 `not_run` 也没有解释（目录里有 5 个 kimi run，说明中途重启过多次）。A0/A2 的 tokens 全为 0（未记录）。任务集是 8 个而非 10 个。这一节现在不能进论文；补齐到 10 任务 × 3 臂全格，缺失格必须显式 `not_run` + 原因。`kimi` 强制 temperature 1 需在表注里写明。

## M-2 后编辑实验的"编辑"大半是空操作（P1，证据强度）

`Edit reality` 表：E2 20 格中 15 格 0 行改动，E3 18 格中 11 格 0 行改动。模型对陌生的包装 API 保守到不动代码，于是 `drift 0` 大半是"没编辑"而不是"编辑后仍一致"。论文只能对**真正改了代码**的格（E1 22、E2 5、E3 7）做陈述。两个补救：(a) 汇总把 no-op 剔出分母，单列 `no_edit`；(b) 对 no-op 格做一次"必须产生改动"的重试（prompt 加可验证的最低要求：E2 至少新增一个 `fn` 并被调用，E3 至少改动一处同步相关代码但不得改变行为——**不提 CIR/sid**），再跑 conform。这样 E3 才可能真正触发 unlock 位置移动，检验 conform 在开发者意图下的价值。

## M-3 人工与自动的两处分歧待裁决（P2，需用户）

`acquire_twice/A0 f42afd77e7a9`（auto: behavior=hang）与 `partial_deadlock/A1 a08bd1a020fa`（auto True）人工判 `no`。论文写"人工复核"就必须给这两格一个结论：要么人工改判，要么自动 oracle 的 hang 是假警报（超时阈值？Miri thread_leak？）并修正。需要把两格的原始证据（`candidate.rs`、behavior 的 stdout/stderr 与超时设置、Miri 16 种子状态）整理成一页给用户再判。

## M-4 陈旧文案与提交信息（P3）

- RESULTS 偏差节仍写 "human review queue … left blank for the owner"，与 (h) ready 矛盾。
- 提交 `4971f650` 信息写的是 "Update CELLS.json…"，实际内容是 `HUMAN_REVIEW_QUEUE.md` 与旧 prompt 文件——是用户本地提交时的自动信息，无法改写已推送历史的话，在 HANDOFF 里注明即可。
- `model-probe-v2/SUMMARY.md` 在 run 子目录而非实验根目录，与其他实验不一致。

## M-5 未做项

`concir-instrument` v2（自由 Rust → 包装类型）未实现。突变/后编辑用的是 codegen 输出，所以不在关键路径；但它是"对任意 LLM 生成的 Rust 做后验证"叙事的必要件——没有它，conform v2 只能用于我们自己 codegen 的代码。论文里要么把范围写清（后验证作用于 codegen 产物及其后续编辑），要么补做。建议补做但作为可选。

## 对实验的判断

九项主张 (a)–(j) 中除 (j) 因 M-1 需补齐外全部 ready；(d) 从上一轮的"近乎平凡"变成有召回、有盲区的诚实结果。下一轮是**收口 + 转入写作**：补探针、修后编辑分母、裁决两格分歧、可选补 instrument v2；同时开始论文侧的对齐——现有 `paper.tex` 的评估节还是 9 模式旧设计，方法节没有 `holds_all`/局部再生成/操作绑定 conform，需要一份"论文—证据"差距清单再动笔。
