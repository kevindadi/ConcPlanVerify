# Translation Examples

> Version 0.1.0 — Last updated 2026-03-16

## 1. Sequential Chain (Mutex lock/unlock)

**CIR** (fixtures/sequential_chain.json):
```json
worker: s1(lock mtx) → s2(drop mtx) → s3(return)
```

**CVN**:
- Places: `cp_worker_s1`, `cp_worker_s2`, `cp_worker_s3`, `cp_worker_ret`, `rp_mtx`
- `t_worker_s1_lock`: consumes `cp_worker_s1` + `rp_mtx`, produces `cp_worker_s2`
- `t_worker_s2_unlock`: consumes `cp_worker_s2`, produces `cp_worker_s3` + `rp_mtx`
- `t_worker_s3_return`: consumes `cp_worker_s3`, produces `cp_worker_ret`

## 2. Branch (read + branch)

**CIR** (fixtures/branch.json):
```json
main: s5(read count) → branch(count > 0, s6, s7)
```

**CVN**:
- `t_main_s5_branch_true`: guard `Cmp(Gt, Ref("count"), Lit(0))`, output to `cp_main_s6`
- `t_main_s5_branch_false`: guard `Not(Cmp(Gt, Ref("count"), Lit(0)))`, output to `cp_main_s7`

## 3. Switch

**CIR** (fixtures/switch.json):
```json
main: s5(read state) → switch(state, {Init→s6, Running→s7, Done→s8})
```

**CVN**: Three transitions with guards `Cmp(Eq, Ref("state"), Lit(Enum("Init")))` etc.

## 4. Spawn + Join

**CIR** (fixtures/spawn_join.json):
```json
main: s1(spawn worker) → s2(join worker) → s3(return)
```

**CVN**:
- `t_main_s1_spawn`: fork — produces tokens at `cp_main_s2` AND `cp_worker_s_first`
- `t_main_s2_join`: sync — consumes from `cp_main_s2` AND `cp_worker_ret`

## 5. CAS (Atomic compare-and-swap)

**CIR** (fixtures/cas.json):
```json
main: s1(cas flag false true) → branch(...)
```

**CVN**:
- `t_main_s1_branch_true` (CasSuccess): guard `Cmp(Eq, Ref("flag"), Lit(false))`, update `{flag: Lit(true)}`
- `t_main_s1_branch_false` (CasFailure): guard `Not(...)`, no update

## 6. Call Expansion into a Body-less Callee

**CIR** (fixtures/fn_summary.json):
```json
main: s1(call validate) → s2(return)
validate: body-less (nobody), effects: writes=[result]
```

**CVN**:
- `t_main_s1_call` (Call): consumes `cp_main_s1`, produces `cp_validate_s_first` AND `cp_main_s1_callwait` (parked continuation)
- `t_validate_body` (Sequential): consumes `cp_validate_s_first`, produces `cp_validate_ret` with update `{result: Lit(Unknown)}`
- `t_main_s1_call_ret` (Join): consumes `cp_validate_ret` + `cp_main_s1_callwait`, produces `cp_main_s2`
