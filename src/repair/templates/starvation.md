## Repair Strategy: Starvation — Ensure Fair Progress

Starvation occurs when one thread is perpetually blocked while other threads continue to make progress. This commonly happens with reader-writer locks (readers starve writers) or unfair lock acquisition patterns where one thread always loses the race.

### Fix Rules

1. If using RwLock and a writer is starved, consider splitting the critical section so the writer can acquire the lock between reader batches.
2. If one thread repeatedly loses a lock race, restructure the locking so each thread has a bounded wait. For example, use a turn-taking variable or acquire locks in a sequence that guarantees each thread gets a turn.
3. Ensure that every thread eventually reaches its return place — no thread should be indefinitely blocked by others' progress.
4. If a thread holds a lock for an extended sequence of operations, shorten the critical section by moving non-critical operations outside the lock.

### Example Pattern

A writer is starved because two readers continuously hold the read lock:

```json
{"sid": "s1", "op": ["res_op", "rw", "read_lock"], "transfer": ["next", "s2"]},
{"sid": "s2", "op": "nop",                          "transfer": ["next", "s3"]},
{"sid": "s3", "op": ["res_op", "rw", "drop"],       "transfer": ["next", "s1"]}
```

Fixed: the reader releases the lock and does not immediately re-acquire, giving the writer a chance:

```json
{"sid": "s1", "op": ["res_op", "rw", "read_lock"], "transfer": ["next", "s2"]},
{"sid": "s2", "op": "nop",                          "transfer": ["next", "s3"]},
{"sid": "s3", "op": ["res_op", "rw", "drop"],       "transfer": ["next", "s4"]},
{"sid": "s4", "op": "nop",                          "transfer": ["next", "s5"]},
{"sid": "s5", "op": "return",                        "transfer": "return"}
```

The key principle: every thread must have a guaranteed path to its return place that does not depend on winning an unbounded race.
