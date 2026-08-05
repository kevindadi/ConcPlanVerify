# Condvar Modeling and Dead-Transition Family Detection

This document explains how CIR condition variables map onto CVN, why "false" dead transitions can appear, and how `disjunctive_family` elevates OR-variants to first-class citizens.

## 1. CVN Is Not a Colored-Token CPN

The current CVN (see [`cvn/README.md`](../cvn/README.md), [`cvn_spec.md`](cvn_spec.md)) is a **weighted P/T net with global-variable guards**:

- Places carry uncolored tokens;
- Transition behavior is determined by arc weights, `BoolExpr` guards, and `VarUpdate`;
- There are **no** color-matched waiter tokens.

Therefore, "woken by notify" and "woken by notify_all" for `wait` must be split into two mutually exclusive transitions (or an equivalent guarded fork). They cannot be expressed as a single color-matched wake transition. A true CPN (with colored waiters) could merge the two; see "Future Directions" at the end — this repository does not implement that in the current round.

## 2. Translation Overview (Unchanged)

Details are in [`translation_rules.md`](translation_rules.md) §3; implementation is in [`src/translator/condvar.rs`](../src/translator/condvar.rs).

### Auxiliary Structures

| Structure | Meaning |
|------|------|
| `rp(cv)` | Condvar resource place; 1 token is deposited on successful `notify` |
| `nw_cv` | Current waiter count (Int) |
| `wp(fn,sid)` | Wait place for this wait call site |
| `ra(fn,sid)` | Intermediate place after wake, before reacquiring the lock |
| `na_fn_sid` | Boolean flag for `notify_all` targeting this wait site |

### `wait(cv, mtx)` → 4 Transitions

```
t_enter     : cp → wp + rp(mtx)     nw++, na←false
t_wake1     : wp + rp(cv) → ra      nw--          (notify_one path)
t_wakeA     : wp → ra               guard na      (notify_all path)
t_reacquire : ra + rp(mtx) → cp'
```

`t_enter` is **not** placed in a family (only permanent death of this transition alone is meaningful). `t_wake1` / `t_wakeA` / `t_reacquire` share the family `{fn}_{sid}:wait_wake`.

### `notify` / `notify_all` → 2 Transitions Each

```
t_notify[_all]      : guard nw > 0   …successful delivery / set na
t_notify[_all]_lost : guard nw == 0  …notifier proceeds (lost-wakeup entry)
```

They share `{fn}_{sid}:notify` and `{fn}_{sid}:notify_all` respectively.

## 3. How SignalLoss Is Detected

`notify_*_lost` is **not itself a defect**: in Rust, `notify` with no waiters is legal. The defect is that a **waiter then remains stuck in `wp` and can never be woken again**, which appears in the state space as a deadlock / block involving wait places. `repair::analyze` classifies such counterexamples as `BugKind::SignalLoss`.

## 4. Why `disjunctive_family` Is Needed

Multiple transitions compiled from the same CIR statement form a **disjunctive family**: at most one branch fires per execution. If "single transition never fires" were reported as `DeadTransition`, then:

- A program that correctly uses `notify_all` would get a false positive because `cv_wake1` never fires;
- A program that only uses `notify` would get a false positive on `cv_wakeA`.

Field semantics ([`cvn::model::Transition::disjunctive_family`](../cvn/src/model/transition.rs)):

- Transitions with the same `Some(id)` form one family;
- **If any member of the family fires in the reachability graph ⇒ the whole family is alive**;
- Only when no member fires does `find_dead_transitions` report **one** counterexample (representative = lexicographically smallest transition id).

The translator calls `set_disjunctive_family` after creating transitions; detection is done inside [`cvn::analysis::find_dead_transitions`](../cvn/src/analysis/search.rs). Transition-id suffix heuristics are **no longer** used.

`BranchTrue` / `BranchFalse` and `CasSuccess` / `CasFailure` are **not** automatically placed in a family: in this project, "one arm forever false" can be a real defect (e.g. the `dead_transition` case).

## 5. Future Directions (True CPN, Not Implemented)

If wait places carried waiter colors (or an equivalent identity), a single wake transition could match by color and simplify the `notify_all` broadcast model. That would require extending the CVN core beyond the current "P/T + guards" design, so it is planned separately.
