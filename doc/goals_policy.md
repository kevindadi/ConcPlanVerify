# Goals Write Policy and Trust Boundary

This document defines the production responsibilities for business goals (`goals`) at each stage of ConcPlanVerify, the checking semantics, and the defensive lines corresponding to three classes of negative cases (missing / too weak / bad reference).

## 1. Semantics

A `BusinessGoal` consists of `marking` (place → minimum token count) and `variables` (global variable → expected value). All predicates must hold in **the same reachable state**. The checking semantics is reachability (EF): the goal is met if there exists an execution that reaches a satisfying state. Legal `marking` keys:

- Resource names (mapped to `rp_{name}`; for Channel/Condvar, `0` means "no residue");
- `"{fn}.{sid}"` (mapped to control place `cp_{fn}_{sid}`, meaning "thread has reached a given statement");
- Raw place ids (prefixes `cp_`/`rp_`/`wp_`/`ra_`, for tooling).

Keys in `variables` must be declared resources with `kind == "var"` (`Var` / `Atomic`).

## 2. Production Responsibility (Write Policy)

| Scenario | Goals source | Mandatoriness |
| --- | --- | --- |
| NL → CIR generation (generate / pipeline) | LLM extracts from requirements | **Should produce**: when the requirements include observable outcomes (counts, completion status, message delivery, etc.), the corresponding goal must be declared; pure synchronization requirements (e.g. "must not deadlock") may use `goals: []` |
| Benchmark fixture | Human-provided gold | Defined by the manifest's `expected.outcome` |
| Repair | Inherit goals from the input CIR | Repair **must not delete or weaken** goals (listed in preservation constraints) |

Missing goals do not change the verification verdict (`verified_safe` still holds) — this is the **trust boundary**: the verifier can only prove "declared properties," not "properties that should have been declared but were not." Experiment records expose this gap via the `declared_goal_count` field for pipeline-level auditing.

## 3. Three Classes of Negative Cases and Defenses

| Negative case | Benchmark case | Defense | Verdict |
| --- | --- | --- | --- |
| Unreachable goal | `goal_unreachable` | State-space reachability check | `goals_unmet` (unmet_goals) |
| Too-weak goal (satisfied in the initial state) | `goal_trivial` | Initial-state triviality check in `verify_program`: goal holds in the initial state → warn "too weak" | `goals_unmet` (goal_warnings) |
| Bad reference (nonexistent place/variable) | `goal_bad_reference` | `translate_goals`: unknown marking keys and undeclared variables produce warnings; when all predicates are unusable, append "no usable predicates" | `goals_unmet` (goal_warnings) |
| Missing goals | (no analyzer verdict; see above) | Generation-side policy + `declared_goal_count` audit | `verified_safe` (trust boundary) |

All three checkable negative cases are counted in the manifest (gold = `goals_unmet`). Their `fixed.json` carries a nontrivial, correctly referenced, and reachable goal as a false-positive probe (must be `verified_safe`).

## 4. Implications for Repair Experiments

Both `goal_warnings` and `unmet_goals` take the state out of `verified_safe`, so the codegen gate equally blocks weak goals / dangling goals. Repair preservation constraints require "Business goal ... must remain achievable"; together with the initial-state triviality check, the LLM cannot bypass repair by "changing the goal to a tautology."

## 5. Goal-Constrained Repair Difficulty (Widening the LLM-Only Gap)

`goal_constrained_deadlock` binds deadlock to a business goal:

- w3's else arm forms a cross-thread lock-order cycle via `m2 → m1` (the defect);
- The same arm is the only path that writes `result = 99`;
- Goal `g_result_special` requires that `result == 99` be reachable.

A correct fix only needs to reorder locks on that arm while keeping the write of 99. Normalizing all writes to the same value, or deleting the else arm, can eliminate the deadlock but yields `goals_unmet` — an offline probe (changing 99 to 3 on the fixed CIR) has reproduced this. Thus "normalize-style blind fixes" are no longer accepted as `verified_safe`.

**Experiment results (2026-08-05)**: `goal_constrained_deadlock` and its dense twin both succeeded in 1 round with `result=99` preserved under DeepSeek v4 Pro's three feedback modes, and under dense × Flash llm_only (`results/goal_constrained_repair_ab.json`, `results/goal_constrained_flash.json`). Layered conclusions:

1. **Oracle layer is effective**: normalizing away 99 → `goals_unmet`; the acceptance gate holds;
2. **Current strong models did not spontaneously fall into the trap**: Pro/Flash still preserve the distinctive write without CVN feedback; repair success rate did not diverge;
3. **Use**: as a regression probe (preventing future prompt/model regressions to "delete the arm to clear deadlock"), and as a contrast case when swapping to weaker models or adversarial rewrites.
