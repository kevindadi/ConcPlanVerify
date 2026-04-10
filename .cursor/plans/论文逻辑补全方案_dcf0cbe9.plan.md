---
name: 论文逻辑补全方案
overview: 以论文为主体,补全形式化定义中的逻辑缺失和表述不精确之处,梳理 CVN 的并发检测能力边界,优化 business goal 概念的论述,并为项目代码列出 TODO.
todos:
  - id: paper-def5-absorb
    content: "formal-definitions.tex: 修正 Definition 5 吸收性描述,区分表达式求值吸收与显式赋值覆盖"
    status: completed
  - id: paper-boolexpr-or
    content: "formal-definitions.tex: BoolExpr 文法补充 Or 变体,并在 Definition 6 中补充 Or 的三值语义"
    status: completed
  - id: paper-channel-table
    content: "formal-definitions.tex: Table 2 Channel(c)->c 改为 Channel->0,或明确定义参数含义"
    status: completed
  - id: paper-arc-def
    content: "formal-definitions.tex: Definition 4 弧注释精确化:guard 仅输入弧,update 仅输出弧"
    status: completed
  - id: paper-done-return
    content: "formal-definitions.tex: Definition 1 transfer done 统一改为 return"
    status: completed
  - id: paper-transition-table
    content: "formal-definitions.tex: Table 3 补充 Sequential、Return;确认 AtomicLoad 保留"
    status: completed
  - id: paper-condvar-notation
    content: "formal-definitions.tex: Table 5 condvar nw++ 改为标准数学记号"
    status: completed
  - id: paper-bug-mechanisms
    content: "analysis-repair.tex: 5 类 bug 补充检测机制说明(可达性/SCC/分类)"
    status: completed
  - id: paper-goal-rationale
    content: "formal-definitions.tex: business goal 补充存在性检查的动机论述和 goal 来源说明"
    status: completed
  - id: paper-prompt-output
    content: "analysis-repair.tex: Table 4 和 Fig 6 的 output contract 与实现统一"
    status: completed
  - id: paper-linear-logic
    content: "formal-definitions.tex: 线性逻辑段落弱化为直觉解释,去掉 tensor 符号"
    status: completed
  - id: code-atomic-load
    content: "项目 TODO: TransitionKind 添加 AtomicLoad,translate_read 对 Atomic 使用 AtomicLoad"
    status: completed
  - id: code-scc-analysis
    content: "项目 TODO: cvn/src/analysis/scc.rs 实现 SCC 分析(livelock/starvation 检测)"
    status: completed
  - id: code-channel-block
    content: "项目 TODO: classify_counterexample 中添加 ChannelBlock 分类逻辑"
    status: completed
  - id: code-business-goals
    content: "项目 TODO: CIR AST 添加 goals 字段,实现 goal reachability check"
    status: completed
  - id: code-diagnostic-tuple
    content: "项目 TODO: BugReport 补全 Lambda(CIR slice)、Gamma_ctx、H 字段"
    status: completed
  - id: code-repair-loop
    content: "项目 TODO: repair_loop 集成 static check 和 goal reachability check"
    status: completed
isProject: false
---

# 论文逻辑补全与形式化修正方案

## 一、CVN 到底能检测哪些并发问题

基于 CVN 的结构(加权 P/T 网 + 全局变量存储 + 三值守卫 + 穷举状态空间搜索),下面从 Petri 网语义角度分析其理论检测能力:

### 1.1 通过可达性直接检测的问题

- **死锁(Deadlock)** -- 非终止状态且无使能变迁.涵盖:双锁死锁、N 锁循环死锁、嵌套锁死锁、信号量耗尽死锁
- **信号丢失(Signal Loss)** -- `CondvarNotifyLost` 变迁在 `nw_cv == 0` 时触发,精确建模了"通知发生在等待之前".由于 `nw_cv` 始终为具体值,此检测无误报无漏报(论文 Proposition 2 已证明)
- **通道阻塞(Channel Block)** -- 当 recv 依赖的 `rp(channel)` token 永远无法被 send 产生时,表现为死锁的特殊形式.分类时检查阻塞变迁是否涉及通道资源库所即可区分
- **原子性违背(Atomicity Violation)** -- 两个线程各自执行 `load -> ... -> store` 时,CVN 会穷举探索 T1.load, T2.load, T1.store, T2.store 这类危险交错.错误的最终值通过 business goal 检测(变量最终值不符合期望).CAS 操作显式建模了成功/失败两条分支,保证了原子 read-modify-write 的正确交错

### 1.2 通过 SCC 分析检测的问题(需要在可达图上做强连通分量分析)

- **活锁(Livelock)** -- 非终止 SCC:某些线程可以无限循环执行变迁,但永远无法到达 return place.例如自旋锁重试循环
- **饥饿(Starvation)** -- SCC 内某线程永远被阻塞,而其他线程持续前进.例如 RwLock 中读者优先导致写者饥饿

### 1.3 通过 business goal 间接检测的问题

- **语义回归** -- 修复后关键行为被删除(变量最终值不对、线程无法到达预期状态)
- **数据竞争后果** -- Atomic 变量的非原子 load-store 序列被交错后,最终值偏离期望
- **功能完整性** -- 某线程未能到达其 return place(任务未完成)

### 1.4 CVN 无法检测的问题

- 弱内存模型相关 bug(需要 fence 操作建模,论文 conclusion 已列为 future work)
- 被 CIR 抽象掉的算术/IO 逻辑中的错误
- 无界状态空间(CVN 要求有界 place 和有限值域)
- Source-to-CIR 的等价性(信任边界)

---

## 二、formal-definitions.tex 修改清单

### 2.1 Definition 5 (Value domain) -- 吸收性描述需精确化

当前文本 (line 186):
> "The element top (unknown) is an absorbing element: once a variable takes the value top, no operation can restore it to a concrete value."

**问题**: 在翻译规则中,`write(x, true)` 产生的输出弧更新 `V[x] <- true` 可以将 Unknown 变量设为具体值.吸收性仅在**表达式求值**层面成立(任何包含 top 的算术或比较运算结果为 top),但不阻止显式赋值覆盖.

**建议修改**: 改为准确的陈述句:

> "The element top is absorbing with respect to expression evaluation. Any arithmetic operation or comparison that receives top as an operand produces top or unknown respectively. A variable-update arc that carries a literal expression replaces the current value unconditionally, including overwriting top with a concrete value. The absorbing property therefore guarantees that derived values cannot spontaneously become concrete, but explicit writes can restore concreteness."

### 2.2 BoolExpr 文法补充 Or

当前文法 (lines 200-207) 缺少 `Or`.实现中 [`cvn/src/model/expr.rs`](cvn/src/model/expr.rs) line 134 有 `Or` 变体并实现了三值 Or 求值(True 短路,双 False 为 False,否则 Unknown).

**建议**: 在 BoolExpr 文法中加入 `Or(BoolExpr, BoolExpr)`.同时在 Definition 6 (Guard evaluation) 中补充一句:
> "The disjunction unknown or true evaluates to true. The disjunction unknown or false evaluates to unknown."

### 2.3 Table 2 Channel 初始 token

当前 (line 169): `Channel(c) -> c`.

**问题**: 参数 c 的含义不明确.如果 c 表示缓冲区容量,标准 Petri 网模型需要**两个**库所(消息库所和容量库所)来正确建模有界缓冲通道.如果仅用一个库所且初始 token 为 c,语义不清.

**建议**: 改为 `Channel -> 0`(初始无消息).当前翻译规则中 send 向 `rp(channel)` 产生 token,recv 从 `rp(channel)` 消耗 token,初始 0 正确建模了"先 send 后 recv"的同步语义.如果未来需要有界缓冲通道,可添加 `rp_cap(channel)` 容量库所,在 future work 中注明.

### 2.4 Table 3 TransitionKind 补全

当前 (line 240) Variable 行列出: `VarRead, VarWrite, AtomicLoad, AtomicStore`.

**问题**: 实现中 `load`(原子加载)使用 `VarRead`,没有 `AtomicLoad`.但论文列出了 `AtomicLoad`,这是正确的设计意图——`AtomicLoad` 与 `VarRead` 在语义上都是读取变量,但在**分类诊断**上需要区分:`VarRead` 发生在持锁保护的 Var 上,`AtomicLoad` 发生在无锁保护的 Atomic 上,后者参与原子性违背检测.

**建议**: 论文保留 `AtomicLoad`.**项目 TODO**: 在 `TransitionKind` 中添加 `AtomicLoad` 变体,`translate_read` 中当资源类型为 `Atomic` 时使用 `AtomicLoad` 而非 `VarRead`.

补充缺少的 kind:
- 加入 `Sequential`(非同步的顺序步骤)和 `Return`(函数返回)

### 2.5 Definition 4 (CVN) 弧注释精确化

当前 (line 150): "A_in and A_out are sets of weighted arcs potentially annotated with guards g or updates u".

**问题**: 实际上 guard 仅在输入弧,update 仅在输出弧.这是合理的设计(guard 控制使能,update 在发射后生效),但定义应如实描述.

**建议修改**:
> "A_in is a set of weighted input arcs from P to T, each carrying an optional guard g. A_out is a set of weighted output arcs from T to P, each carrying an optional variable update u."

### 2.6 Definition 1 transfer 类型 done -> return

当前 (line 12): transfer 包含 `done`.CIR 实现和论文其他位置都使用 `return`.

**建议**: 统一改为 `return`.

### 2.7 Table 5 翻译规则中 condvar 的 nw++ 记号

当前 (line 288): `nw_{cv}{+\!+}` 和 `na_{sid} <- false`.

**建议**: 改为标准数学表达式 `nw_{cv} := nw_{cv} + 1`,避免自定义操作符.

---

## 三、analysis-repair.tex 修改清单

### 3.1 Bug 类别保留 5 类,补充检测机制说明

论文声称的 5 类 (lines 16-22) 从 CVN 理论能力上都是可检测的(见上文第一节分析).建议保留,但补充每类的检测机制说明:

- **Deadlock**: 非终止状态,使能变迁集为空.通过穷举可达性搜索直接发现.
- **Signal loss**: `CondvarNotifyLost` 变迁触发.由翻译规则中的 `nw_cv == 0` 守卫精确编码.
- **Livelock**: 可达图中的非终止强连通分量,SCC 内不包含 return place.需要在可达图上执行 Tarjan 或 Kosaraju 算法.
- **Starvation**: SCC 内某线程的控制库所在所有 SCC 内状态中始终不变.与 livelock 共用 SCC 基础设施.
- **Channel block**: 死锁的特殊形式,阻塞变迁的输入弧涉及通道资源库所.分类时检查 `PlaceKind::Resource` 的 `resource_type` 是否为 `Channel`.

**项目 TODO**:
- 实现 SCC 分析模块 `cvn/src/analysis/scc.rs`(使用 petgraph 的 `tarjan_scc` 或 `kosaraju_scc`)
- 在 `AnalysisResult` 中添加 `livelocks` 和 `starvations` 字段
- 在 `src/repair/mod.rs` 的 `classify_counterexample` 中添加 `ChannelBlock` 分类逻辑

### 3.2 Diagnostic tuple 保留 7 字段,项目补全

论文的 Definition 4 (lines 34-39) D = (kappa, pi_mu, Sigma_state, Sigma_wait, Lambda, Gamma_ctx, H) 是正确的目标设计.

**项目 TODO**:
- `BugReport` 添加 `cir_slice: Vec<Statement>` 字段(Lambda)
- `BugReport` 添加结构化的 `wait_ownership: WaitOwnershipSummary` 字段(Sigma_wait)
- `BugReport` 添加 `preservation_constraints: Vec<String>` 字段(Gamma_ctx),从 CIR 的 resources/protection 动态生成
- `BugReport` 添加 `repair_hint: String` 字段(H),将 `suggestion_for()` 结果纳入

### 3.3 Algorithm 1 保留,项目补全

Algorithm 1 的三层循环是正确的设计.当前实现缺少 static check 集成和 goal reachability check.

**项目 TODO**:
- 在 `repair_loop` 中集成 CIR static check(调用 `cir::validate`)
- 实现 goal reachability check(利用已有的 `exists_path` 函数)

### 3.4 Prompt template (Table 4) 与 Fig 6 的输出约定统一

Table 4 最后一行 "Output contract" 说 "return only the revised CIR fragment or statement list".但 [`src/repair/render.rs`](src/repair/render.rs) line 47 要求 "输出修复后的完整 CIR JSON".

**建议**: 论文中保留 "revised fragment" 为理想目标(减少 token 消耗),在实现说明或脚注中注明当前原型要求完整 JSON 以简化解析.或者修改论文为 "return the revised CIR artifact" 以匹配实现.

---

## 四、Business Goal 概念分析与建议

### 4.1 当前设计的合理性

用户的核心诉求是:修复并发 bug 后不能破坏原有功能.Business goal 作为**语义回归防护**是正确的思路.Definition 3 定义的 `(M_gamma, V_gamma)` 既简洁又有足够表达力:

- **Marking 约束**:`cp(worker, ret): 1` 确保 worker 线程能完成
- **变量约束**:`ready: true` 确保关键状态被正确设置
- **资源约束**:`rp(m0): 1` 确保锁最终被释放

### 4.2 改进建议

当前 Definition 3 使用 **存在性检查**(至少一个可达状态满足 goal).这对于检测"修复是否删除了必要行为"是充分的.但可以考虑在论文中补充说明:

1. **为什么存在性而非普遍性**:如果要求所有终止状态都满足 goal(普遍性),会过于严格——并发程序天然有多种合法终止状态.存在性检查回答的问题是"程序是否仍然有能力达成目标",这正是防止语义回归所需要的.

2. **Goal 的来源**:补充一段说明 goal 可以由用户在需求规格中显式指定,也可以由 LLM 在生成 CIR 时自动派生.这使得 goal 既是验收条件也是 LLM 的 "contract".

3. **与线性逻辑的联系**:当前 lines 114-115 的线性逻辑段落可以保留,但应弱化为直觉解释而非方法论声明.建议改为:
   > "Intuitively, this check can be understood through the resource-consumption perspective: each CVN transition consumes and produces tokens, and the business goal asks whether the target token configuration is derivable from the initial configuration through some sequence of firings."

   删除 tensor 符号,用自然语言陈述即可.

### 4.3 项目 TODO

- 在 `cir/src/ast.rs` 的 `Program` 中添加 `goals: Vec<BusinessGoal>` 字段
- 定义 `BusinessGoal` 结构体(marking 约束 + 变量约束)
- 在 `cvn/src/analysis/` 中实现 goal reachability check(遍历 `reachability_graph` 中的状态)
- 在 `src/repair/llm.rs` 的 `repair_loop` 中集成 goal check

---

## 五、properties.tex 中的对应调整

如果上述修改实施,properties.tex 需要同步调整:

- Theorem 4 (Soundness) 的"bug configuration"应明确涵盖 5 类 bug
- 补充 SCC-based 检测的 soundness 论述(livelock/starvation 的 SCC 判定条件是 sound 的,因为可达图是精确的)
- Business goal reachability 的 soundness 也自然成立(在同一个穷举可达图上的线性扫描)

---

## 六、总结:修改优先级

1. **高优先级(逻辑正确性)**:Definition 5 吸收性、Definition 4 弧注释、BoolExpr 文法补 Or、Table 2 Channel、transfer done->return
2. **中优先级(完整性)**:Table 3 补全 TransitionKind、bug 检测机制说明、prompt 输出约定统一
3. **论文概念强化**:business goal 存在性检查的论述、线性逻辑段落弱化
4. **项目 TODO**:AtomicLoad、SCC 分析、ChannelBlock 分类、business goal 数据结构和检查、diagnostic tuple 补全
