# Translation Rules Reference

> Version 0.1.0 — Last updated 2026-03-16

This document defines the mapping from CIR constructs to CVN elements.

Notation: `cp(f,s)` = control place for function `f`, statement `s`;
`rp(x)` = resource place for resource `x`;
`wp(cv,f,s)` = wait place for condvar `cv` at function `f`, statement `s`.

## 1. Control Flow

| CIR Transfer | CVN Generation |
|-------------|----------------|
| `next(s2)` | Output arc to `cp(f, s2)` |
| `branch(cond, s_t, s_f)` | Two transitions sharing input `cp(f, s_cur)`: true-branch with guard `cond`, false-branch with guard `Not(cond)` |
| `switch(var, map)` | One transition per label with guard `Cmp(Eq, Ref(var), Lit(label))` |
| `return` | Output arc to `cp(f, ret)` |

## 2. Resource Operations

| CIR Op | Input Arcs | Output Arcs | Update |
|--------|-----------|-------------|--------|
| `lock(mtx)` | `cp(f,s) w=1`, `rp(mtx) w=1` | `cp(f,s_next) w=1` | — |
| `drop(mtx)` | `cp(f,s) w=1` | `cp(f,s_next) w=1`, `rp(mtx) w=1` | — |
| `lock(rw)` | `cp(f,s) w=1`, `rp(rw) w=N` | `cp(f,s_next) w=1` | — |
| `read(rw)` | `cp(f,s) w=1`, `rp(rw) w=1` | `cp(f,s_next) w=1` | — |
| `drop(rw)` | `cp(f,s) w=1` | `cp(f,s_next) w=1`, `rp(rw) w=N or 1` | — |
| `acquire(sem)` | `cp(f,s) w=1`, `rp(sem) w=1` | `cp(f,s_next) w=1` | — |
| `release(sem)` | `cp(f,s) w=1` | `cp(f,s_next) w=1`, `rp(sem) w=1` | — |
| `send(ch)` | `cp(f,s) w=1` | `cp(f,s_next) w=1`, `rp(ch) w=1` | — |
| `recv(ch)` | `cp(f,s) w=1`, `rp(ch) w=1` | `cp(f,s_next) w=1` | — |
| `write(v, val)` | `cp(f,s) w=1` | `cp(f,s_next) w=1` | `{v: val_expr}` |
| `read(v) + next` | `cp(f,s) w=1` | `cp(f,s_next) w=1` | — (Sequential) |
| `read(v) + branch` | Merged into branch pair | See branch | guard uses `Ref(v)` |
| `store(a, val)` | `cp(f,s) w=1` | `cp(f,s_next) w=1` | `{a: val_expr}` |
| `cas(a, exp, des)` | Two transitions: succ guard `Cmp(Eq,Ref(a),exp)`, fail guard `Not(...)` | succ: update `{a: des}`, fail: no update | — |

## 3. Condvar Operations

Narrative overview, SignalLoss classification, and `disjunctive_family` semantics:
see [`condvar_modeling.md`](condvar_modeling.md).

### Auxiliary structures

Each Condvar `cv` introduces:
- Resource place `rp(cv)`: initial 0 tokens
- Global variable `nw_cv`: Int, initial 0 (current waiter count)

Each wait call-site `sid` introduces:
- Wait place `wp(sid)`
- Reacquire place `ra(sid)`
- Global variable `na_sid`: Bool, initial false

### wait(cv, mtx)

At sid with successor sid', generates 4 transitions:

```
t_enter  [CondvarWaitEnter]:       cp(f,sid) → wp(sid) + rp(mtx)   update: nw_cv += 1, na_sid ← false
t_wake1  [CondvarWakeByNotify]:    wp(sid) + rp(cv) → ra(sid)      update: nw_cv -= 1
t_wakeA  [CondvarWakeByNotifyAll]: wp(sid) → ra(sid)               guard: na_sid == true
                                                                    update: nw_cv -= 1, na_sid ← false
t_reacq  [CondvarReacquire]:       ra(sid) + rp(mtx) → cp(f,sid')
```

`disjunctive_family`: `t_wake1`, `t_wakeA`, `t_reacq` share `{f}_{sid}:wait_wake`.
`t_enter` has no family.

### notify(cv)

At sid_n with successor sid_n', generates 2 transitions:

```
t_notify [CondvarNotify]:     cp(f,sid_n) → cp(f,sid_n') + rp(cv)  guard: nw_cv > 0
t_lost   [CondvarNotifyLost]: cp(f,sid_n) → cp(f,sid_n')           guard: nw_cv == 0
```

Both share `{f}_{sid_n}:notify`. The lost arm alone is not a bug; SignalLoss is
classified when a waiter remains stuck afterward.

### notify_all(cv)

At sid_n with wait-sites {w1, ..., wk}, successor sid_n', generates 2 transitions:

```
t_notifyAll [CondvarNotifyAll]:     cp(f,sid_n) → cp(f,sid_n')     guard: nw_cv > 0
                                                                    update: na_w1..na_wk ← true
t_allLost   [CondvarNotifyAllLost]: cp(f,sid_n) → cp(f,sid_n')     guard: nw_cv == 0
```

Both share `{f}_{sid_n}:notify_all`.

## 4. Concurrency

| CIR Op | CVN Translation |
|--------|----------------|
| `spawn(f)` | Output arc also to `cp(f, s_first)` (fork) |
| `join(f)` | Input arc also from `cp(f, ret)` (sync) |
| `call(f)` | If f has body: already translated. If summary: Call transition with writes → Unknown |

## 5. FnSummary

```
t_call: cp(caller,s) → cp(caller,s_next), update = {w: Unknown for w in writes}
```
