# 复核：experiments-v2 第 l 轮（conform 召回、后编辑、tiered 附录、10 任务、冻结）

复核对象：`ConcPlanVerify` HEAD `57bb74a8`，tag `experiments-v2-freeze-1`（工作树有 1 个未提交改动：`reviews/experiments-v2-contract-strength-working-tree/NEXT_CURSOR_PROMPT.md`，与本轮无关，请自行处理）；`ConcIR` HEAD `f42764f`，tag `concir-freeze-1`。
复核方法：读 HANDOFF Round l、`conform-mutation-v1/{SUMMARY,CONFORM_GAPS}.md`、`post-edit-conform-v1/{SUMMARY.md,CELLS.json}`、`case-partial-deadlock-v1/CASE.md` 附录、`model-probe-v1/SUMMARY.md`、`PAPER_EVIDENCE_MAP.md`、RESULTS 按臂汇总。

## 结论

- §1–§4、§6 交付，127/220 请求；§5 因端点无第二模型停点，处理得当。K-2 关闭得干净：`partial_deadlock` 上 tiered 提前升级 3/3、K=6 3/3，证明此前 0/3 是预算切分。K-3 离线重分类 0 变化，偏差关闭。10 任务主表带来一个**改变结论的发现**：`bare_wait_no_predicate` 需要新增语句，local 0/3、whole 3/3，因此 (b) 应表述为"local 便宜 ~5×，接受率取决于修复是否需要新语句"。这是好的、诚实的修正。
- 但 K-1 的两个实验给出的答案是：**当前 conform 作为后验证几乎没有语义召回**。这是论文核心主张 (d) 的问题，也是本轮最重要的发现。

## L-1 conform 是流级检查，对代码级改动盲（P1，方法）

`CONFORM_GAPS.md` 的归因准确：`cir_trace::ev` 是**与操作解耦的注解**。突变器交换两次 `lock()` 而 `ev` 不动，流不变（M1 0/11 via violation）；删 `drop(guard)` 但 unlock 的 `ev` 在 `drop` 之前发出，流不变（M2 1/15）；`ev` 移到调用前，流不变（M3 0/19）。只有删 `ev`（M6）被 100% 抓住。后编辑 57 格 `original_ev == edited_ev`，drift 0——不是编辑没有偏离模型，而是**只要模型不动注解，conform 就看不见**。E3 "顺手优化锁"17 个构建成功的程序全部 PASS，正是这个盲区的体现。

这不是实现 bug，是设计选择的后果：把事件绑在注解上，等于让"被验证的对象"自己声明自己做了什么。后验证的意义在于捕捉代码与模型的偏离，而偏离恰恰会以"代码改了、注解没改"的形式出现。

**修法是架构性的但范围可控**：事件必须由操作本身发出。`cir_trace` v2 提供包装类型——`cir_trace::Mutex<T>` 的 `lock()` 在获得锁后发 acquire，guard 的 `Drop` 发 release；`Condvar::wait/notify_*`、`Semaphore::acquire/release`、channel `send/recv` 同理；`spawn` 包装器按 **spawn 语句的 sid** 给子线程打 tag（同时消除 M7 的创建顺序假阳）。sid 作为调用点参数（codegen 发出 `m.lock(sid!("main.s3"))`），`concir-instrument` 对自由代码按 (资源, 种类, 顺序) 分配。这样：M1 → 流顺序变 → violation；M2 → release 移到作用域末尾、与后续事件顺序变 → violation（同线程有后续事件时）；M3 → 由构造消除；M7 → 消除。然后重跑突变（0 请求）与后编辑（57 请求），(d) 才有可写的召回数字。

## L-2 M7 "假阳"其实是模型偏离（P3，表述）

交换两个 spawn 语句改变了 `main` 的语句顺序，CIR 中也是两条不同语句；conform 报 violation 在"与所写模型一致"的口径下是正确的，只是语义等价。v2 用 sid 打 tag 后这一项自然消失；论文里不要称其为 false positive，称"顺序敏感、语义等价"。

## L-3 tiered 在 `bare_wait_no_predicate` 上没有升级（P2）

A3_local 0/3、A3_tiered 24/30 与 local 持平，说明 tiered 在该任务上没触发升级。按协议"连续 2 轮 `explore_fail`"应触发；要么 local 的失败是 `check_invalid`/`stalled_local_patch` 之外的类别没被计入触发，要么触发实现有漏。查 3 个 rep 的决策序列，修触发（把任何连续 2 轮非 accepted 计入）或在 PROTOCOL 里写明为何不触发。修后仅重跑该任务 tiered × 3（≤ 12 请求）。

## L-4 §4 新增 4 个 CIR 未进突变/后编辑（P3）

HANDOFF 已声明。L-1 重跑时一并覆盖（23 个程序）。

## L-5 第二模型（用户决定）

端点把 `deepseek-chat`/`deepseek-reasoner` 别名到 `deepseek-flash`，代理无法解决。若你有其他 OpenAI 兼容端点的 key（任一非 DeepSeek 家族模型），泛化探针只需 ~60 请求；没有就在论文 threats 里写单模型。

## L-6 人工复核

`HUMAN_REVIEW_QUEUE.md` 现在 17 行，全部空白。(h) 仍是 partial。

## 对实验的判断

除 (d) 外的主张都已站住并冻结。(d) 当前只能写成"流级一致性 + 强制标注检查"，与论文"后验证 LLM 代码符合 ConcIR"的表述不匹配。建议再做一轮，专门把 `cir_trace` 改成操作绑定事件、重跑突变与后编辑；这是最后一个实验轮，之后开始写作。若你决定不做，(d) 必须按 `CONFORM_GAPS.md` 的口径降级表述，论文主张随之收窄。
