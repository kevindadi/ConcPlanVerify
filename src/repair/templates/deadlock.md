## Repair Strategy: Deadlock — Uniform Lock Ordering

A deadlock occurs when two or more threads each hold a lock and wait for a lock held by the other, forming a circular dependency. The model checker found a reachable state where no thread can make progress.

### Fix Rules

1. Choose a single global lock ordering (e.g. alphabetical by resource name).
2. Every function must acquire locks in that order. If a function currently acquires lock B before lock A, swap them.
3. Do NOT change the operations performed between lock and drop — only reorder the lock/drop pairs.
4. Every lock must still have a matching drop in the same function.

### Example: Buggy ConcIR (deadlock)

Two threads acquire two mutexes in opposite order:

```json
{
  "name": "w1", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx_a", "lock"], "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "mtx_b", "lock"], "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx_b", "drop"], "transfer": ["next", "s4"]},
    {"sid": "s4", "op": ["res_op", "mtx_a", "drop"], "transfer": ["next", "s5"]},
    {"sid": "s5", "op": "return",                     "transfer": "return"}
  ]
}
```

```json
{
  "name": "w2", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx_b", "lock"], "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "mtx_a", "lock"], "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx_a", "drop"], "transfer": ["next", "s4"]},
    {"sid": "s4", "op": ["res_op", "mtx_b", "drop"], "transfer": ["next", "s5"]},
    {"sid": "s5", "op": "return",                     "transfer": "return"}
  ]
}
```

### Fixed ConcIR

Make `w2` acquire locks in the same order as `w1` (mtx_a before mtx_b):

```json
{
  "name": "w2", "kind": "closure",
  "body": [
    {"sid": "s1", "op": ["res_op", "mtx_a", "lock"], "transfer": ["next", "s2"]},
    {"sid": "s2", "op": ["res_op", "mtx_b", "lock"], "transfer": ["next", "s3"]},
    {"sid": "s3", "op": ["res_op", "mtx_b", "drop"], "transfer": ["next", "s4"]},
    {"sid": "s4", "op": ["res_op", "mtx_a", "drop"], "transfer": ["next", "s5"]},
    {"sid": "s5", "op": "return",                     "transfer": "return"}
  ]
}
```

The fix only changed `w2` to acquire `mtx_a` then `mtx_b`, matching `w1`'s order. No other changes.
