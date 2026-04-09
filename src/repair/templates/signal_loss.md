## Repair Strategy: Signal Loss — While-Loop Guard on Wait

A signal loss occurs when a `notify_one` or `notify_all` fires before any thread has entered `wait`, causing the notification to be lost. The waiter then blocks forever because no future notification will arrive.

### Fix Rules

1. Before calling `wait`, read the predicate variable (e.g. `ready`) and branch on it.
2. If the predicate is already true, skip the wait entirely.
3. If the predicate is false, call `wait`, then loop back to re-check the predicate.
4. Make sure `notify` happens AFTER writing the predicate to true, not before.
5. The waiter must hold the mutex when reading the predicate and when calling wait.

### Example: Buggy CIR (signal loss)

The waiter enters `wait` unconditionally — if the notifier runs first, the signal is lost:

```json
{
  "name": "waiter", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx", "lock"],       "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "cv", "wait", "mtx"],  "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx", "lock"],        "transfer": ["next", "s4"]},
    {"sid": "s4", "op": ["res_op", "mtx", "drop"],        "transfer": ["next", "s5"]},
    {"sid": "s5", "op": "return",                          "transfer": "return"}
  ]
}
```

### Fixed CIR

Add a while-loop: read the predicate, branch — if true skip wait, if false wait then loop back:

```json
{
  "name": "waiter", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx", "lock"],        "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "ready", "read"],       "transfer": ["branch", "ready == true", "s5", "s3"]},
    {"sid": "s3", "op": ["res_op", "cv", "wait", "mtx"],   "transfer": ["next", "s4"]},
    {"sid": "s4", "op": ["res_op", "mtx", "lock"],         "transfer": ["next", "s2"]},
    {"sid": "s5", "op": ["res_op", "mtx", "drop"],         "transfer": ["next", "s6"]},
    {"sid": "s6", "op": "return",                           "transfer": "return"}
  ]
}
```

The fix adds a `read(ready)` + `branch` before the wait. If `ready` is already true (notify already happened), skip to drop. If false, wait then loop back to re-check. This is the standard while-loop condvar pattern.
