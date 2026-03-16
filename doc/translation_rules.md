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

### wait(cv, mtx)

```
t_cv_wait:      cp(f,s) → wp(cv,f,s) + rp(mtx)        [release lock]
t_cv_reacquire: cp(f,s_reacquire) + rp(mtx) → cp(f,s_resume)  [reacquire]
```

### notify(cv)

For each wait-site `(fk, s_wk)`:
```
t_cv_notify_k: cp(g,s) + wp(cv,fk,s_wk) → cp(g,s_next) + cp(fk,s_wk_reacquire)
```

### notify_all(cv)

Chain expansion with 2K transitions for K wait-sites.

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
