---
name: 形式化修正清单方案
overview: 按照 14 条审稿级修正意见,系统性地修正 formal-definitions.tex 和 analysis-repair.tex 中的形式化定义缺陷,使形式对象闭合、层次边界清晰、辅助函数显式化、前提条件完整.
todos:
  - id: fix-stmt-syntax
    content: "#7: 分离 op 与 transfer,把 return 从 Table 1 op 行移除"
    status: completed
  - id: fix-cir-6tuple
    content: "#1: CIR artifact 改为 6-tuple,纳入 goals"
    status: completed
  - id: fix-goal-layers
    content: "#2: business goal 分两层:CIR-level 定义 + CVN-level 解释函数"
    status: completed
  - id: fix-cvn-static-runtime
    content: "#3: CVN 定义拆为静态网 + 运行时状态"
    status: completed
  - id: fix-arc-functions
    content: "#4: 显式定义 w/g/u/Updates 辅助函数"
    status: completed
  - id: fix-condvar-model
    content: "#5: condvar 子模型补全:ra(sid)、rp(cv)、notify_all、初始条件"
    status: completed
  - id: fix-translate-domain
    content: "#6: translate 函数定义域收紧为 CIR_v"
    status: completed
  - id: fix-bounded-val
    content: "#8: 有限值域假设 + termination 前提"
    status: completed
  - id: fix-channel-block
    content: "#9: channel block 降级为启发式分类"
    status: completed
  - id: fix-livelock-starvation
    content: "#10: livelock/starvation 加 fairness 前提或降级"
    status: completed
  - id: fix-diagnostic-witness
    content: "#11: diagnostic 推广到 bug witness"
    status: completed
  - id: fix-bug-selection
    content: "#12: Algorithm 1 加 bug selection policy"
    status: completed
  - id: fix-summary-semantics
    content: "#13: function summary 语义边界澄清"
    status: completed
  - id: fix-anchor-type
    content: "#14: anchor mapping 类型澄清"
    status: completed
isProject: false
---

# 形式化修正清单实施方案

以下按逻辑依赖顺序排列修改项.每项标注对应的修正编号和精确修改位置.

---

## 1. Statement 语法:分离 op 与 transfer (修正 #7)

**文件**: `formal-definitions.tex` line 10-13

**问题**: `return` 同时出现在 op (Table 1 line 59) 和 transfer (Definition 1),语义冲突.

**修改**:
- Definition 1 中将 op 的文法收紧:op 不含 `return`,`return` 只作为 transfer
- 在 Table 1 中把 `return` 从 Control 类的 op 行移除,加一条注释:`return is a transfer, not an operation`
- op 保留 `nop` 作为空操作(用于 branch 的 read + branch 翻译模式)

---

## 2. CIR artifact:纳入 goals 成为 6-tuple (修正 #1)

**文件**: `formal-definitions.tex` line 16-19

**修改**:
- 将 5-tuple 改为 6-tuple:$\mathcal{I} = (R, G, F, \Sigma, e, \Gamma)$
- 新增 $\Gamma$ 为一组 CIR 层目标(见下一条的 CIR-level goal 定义)
- line 117 的文字"goals become part of the CIR artifact"与定义一致

**对 Algorithm 1 的影响**: line 72 `$\mathcal{I} \gets \text{LLM.Generate}(\Psi)$` 的注释 `CIR with goals $\Gamma$` 现在由定义 justified,无需改动.

---

## 3. Business goal 分层:CIR-level 定义 + CVN-level 解释函数 (修正 #2)

**文件**: `formal-definitions.tex` line 99-143

**修改**: 将 Definition 3 拆成两层.

**层 1 (CIR-level goal)**:
```latex
\begin{definition}[CIR business goal]
A CIR business goal is a pair $\gamma^{\mathrm{cir}} = (C_\gamma, V_\gamma)$
where $C_\gamma$ is a set of thread-completion requirements
$\{(f_i, \mathsf{completed})\}$ and resource-availability requirements
$\{(r_j, \mathsf{available})\}$, and $V_\gamma : \mathit{VarName}
\rightharpoonup \mathit{Val}$ specifies required variable values.
\end{definition}
```

**层 2 (Goal interpretation)**:
```latex
\begin{definition}[Goal interpretation]
Given a CIR goal $\gamma^{\mathrm{cir}}$ and a CVN $\mathcal{N}$
produced by $\mathit{translate}(\mathcal{I})$, the interpretation
$\llbracket \gamma^{\mathrm{cir}} \rrbracket_{\mathcal{N}}$
yields a CVN goal $(\mathit{M}_\gamma, \mathit{V}_\gamma)$ where:
- $(f, \mathsf{completed})$ maps to $M_\gamma(\mathit{cp}(f, \mathit{ret})) \geq 1$
- $(r, \mathsf{available})$ maps to $M_\gamma(\mathit{rp}(r)) \geq I_m(\mathit{rp}(r))$
- variable requirements pass through unchanged
\end{definition}
```

**Figure 4 (goals example)**: 用 CIR-level 语法重写(不暴露 cp/rp),加一段文字说明 cp/rp 是解释函数的输出.

---

## 4. CVN 定义:分离静态网与运行时状态 (修正 #3)

**文件**: `formal-definitions.tex` line 153-156

**修改**: 将 8-tuple 拆成**静态网 + 初始状态**两层.

**静态网** (6-tuple):
$\mathcal{N} = (P, T, A_{\mathrm{in}}, A_{\mathrm{out}}, \mathit{Var}, \mu)$

**运行时状态**:
$S = (M, V)$ where $M: P \to \mathbb{N}_0$ is a marking, $V: \mathit{Var} \to \mathit{Val}$ is a valuation

**初始状态**: $S_0 = (I_m, I_v)$

这样 $V$ 不再既是网的组成部分又是状态的一部分.

---

## 5. 显式定义弧辅助函数 (修正 #4)

**文件**: `formal-definitions.tex`,在 Definition 4 (CVN) 之后,Definition 7 (Enabling) 之前插入

**新增 Definition**:
```latex
\begin{definition}[Arc functions]
For an input arc $a = (p, t) \in A_{\mathrm{in}}$:
$w(a)$ denotes the weight, $g(a)$ denotes the guard ($\textsf{True}$ if absent).
For an output arc $a = (t, p) \in A_{\mathrm{out}}$:
$w(a)$ denotes the weight, $u(a)$ denotes the variable update ($\bot$ if absent).
Shorthand: $w_{\mathrm{in}}(p,t) = w((p,t))$, $w_{\mathrm{out}}(t,p) = w((t,p))$.
$\mathit{guard}(t) = \bigwedge_{a \in A_{\mathrm{in}}(t)} g(a)$.
$\mathit{Updates}(t) = \{u(a) \mid a \in A_{\mathrm{out}}(t), u(a) \neq \bot\}$.
\end{definition}
```

然后 Enabling/Firing 中的 $w(p,t)$、$w_{\mathrm{in}}$、$w_{\mathrm{out}}$、$\mathit{Updates}(t)$ 都有正式定义支撑.

---

## 6. Condvar 子模型补全 (修正 #5)

**文件**: `formal-definitions.tex` line 155, 159, 294

**修改**:
- 在 place 分类段落 (line 159) 中显式加入 **reacquire place** $\mathit{ra}(\mathit{sid})$ 作为 $P_w$ 的子类
- 在 Table 2 后加一段:Condvar 资源生成一个 **signal place** $\mathit{rp}(\mathit{cv})$,初始 0 token
- 补全 `notify_all` 的翻译规则:在 Table 5 中加一行 `notify_all` 条目,描述 $na_w := \mathsf{true}$ 的广播语义
- 明确 $nw_{cv}$ 和 $na_{\mathit{sid}}$ 属于 $\mathit{Var}$,初始值为 $0$ 和 $\mathsf{false}$

---

## 7. Translation 函数的定义域收紧 (修正 #6)

**文件**: `formal-definitions.tex` line 261

**修改**:
- 将 `translate : CIR -> CVN` 改为 `translate : CIR_v -> CVN`,其中 $\textsc{Cir}_v$ 表示通过了静态检查的 CIR artifact
- 加一段文字:Translation is defined only for validated CIR artifacts ($\mathcal{E}_{L1} = \emptyset$). The static checker guarantees that every operation in $F$ belongs to the supported set in Table 1 and that every resource reference is well-typed. This ensures that the translation rules in Table 5 cover all reachable operations.
- 在 Table 5 底部加一行备注:Operations not listed (spawn_async, await, call with body) use structurally identical rules differing only in the transition kind tag.

---

## 8. 有限值域前提 (修正 #8)

**文件**: `formal-definitions.tex` line 185, 319

**修改**:
- 在 Definition 5 (Value domain) 后加一段正式假设:

```latex
\begin{assumption}[Bounded value domain]
\label{asmp:bounded-val}
For analysis purposes, each base type $\tau$ is interpreted over
a finite set: \textsf{Bool} = $\{true, false\}$,
\textsf{Int} and \textsf{Float} are abstracted to $\{\top\}$
unless a literal appears in the CIR (in which case only the
occurring literals and $\top$ are tracked),
\textsf{String} and \textsf{Enum} are similarly restricted to
occurring literals plus $\top$.
\end{assumption}
```

- line 319 的 termination 声明改为:`Under Assumption~\ref{asmp:bounded-val}, the reachable state space is finite...`

---

## 9. Channel block 降级为启发式分类 (修正 #9)

**文件**: `analysis-repair.tex` line 19

**修改**: 将 channel block 的描述从 formal bug class 降级为 heuristic classification:

> Channel block ($\kappa_{cb}$): a deadlock in which a blocked transition requires a token from a channel resource place. This classification is a **heuristic refinement** of the deadlock predicate: it inspects the resource type of each blocked input arc and reports a channel block when the missing token belongs to a channel place. The heuristic may misclassify a deadlock that coincidentally involves a channel but is caused by lock contention. Formal soundness therefore reduces to the underlying deadlock predicate $\kappa_{dl}$.

---

## 10. Livelock/Starvation 定义收紧或降级 (修正 #10)

**文件**: `analysis-repair.tex` line 20-21

**修改**: 加 fairness 前提,或降级为 heuristic.建议后者(与实现一致):

- Livelock: 加一句 `Under the assumption of fair scheduling (every persistently enabled transition eventually fires), a non-terminal SCC...`
- Starvation: 改为 `Under fair scheduling, starvation manifests as...`
- 加一段 caveat: `Without an explicit fairness assumption, these two categories are best understood as heuristic classifications that flag suspicious non-progress patterns. The formal soundness guarantee of the system (Theorem 5) covers deadlock and signal loss; livelock and starvation detection is sound relative to the fairness assumption.`

---

## 11. Diagnostic 推广到 bug witness (修正 #11)

**文件**: `analysis-repair.tex` line 34-39

**修改**: 将 $\pi_\mu$ 的描述泛化:

> $\pi_\mu$ is a bug witness: for deadlock and signal loss, it is a finite firing sequence ending in the bug state; for livelock and starvation, it is a finite prefix reaching the SCC entry plus a description of the SCC structure.

在 $\Sigma_{\mathit{state}}$ 的描述中加:for SCC-based bugs, $\Sigma_{\mathit{state}}$ summarizes the SCC rather than a single state.

---

## 12. Bug selection policy (修正 #12)

**文件**: `analysis-repair.tex` line 81-83

**修改**: 在 Algorithm 1 的 `GenDiagnostic` 调用前加一行注释或文字段落:

> When multiple bugs are detected ($|\mathcal{B}| > 1$), the system selects the first bug in priority order: deadlock > signal loss > channel block > livelock > starvation. Within the same category, the bug with the shortest witness trace is preferred. This deterministic selection policy ensures reproducibility across repair rounds.

---

## 13. Function summary 语义边界 (修正 #13)

**文件**: `formal-definitions.tex` line 66-71

**修改**: 在 summary 定义后加一段澄清:

> The $\mathit{reads}$ and $\mathit{calls}$ fields serve the static checker (e.g., detecting recursive concurrency or unprotected access through transitive calls) but do not participate in the CVN translation. The $\mathit{has\_concurrency}$ flag is similarly a static annotation. Only $\mathit{writes}$ affects CVN semantics: Phase 3 of the translation generates $\top$-updates for written variables.

---

## 14. Anchor mapping 类型澄清 (修正 #14)

**文件**: `formal-definitions.tex` line 155

**修改**: 将 $\mu: T \rightharpoonup \mathit{SID}^+$ 改为 $\mu: T \to \mathit{SID}$,并加一句:

> Each transition is anchored to exactly one source statement. The mapping is total for transitions generated from CIR statements. Auxiliary transitions (e.g., summary-call transitions) have no anchor and are excluded from witness traces.

如果确实需要多 sid 锚定(比如 condvar 4-transition 组),则保留 $\mathit{SID}^+$ 但加说明:

> For compound operations such as condvar wait, which generates multiple transitions, each transition is anchored to the originating wait statement's sid. The $+$ superscript indicates that future extensions may anchor a transition to a sequence of sids for inter-procedural inlining; in the current system, all anchors are singletons.
