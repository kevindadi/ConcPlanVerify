## Repair Strategy: Livelock — Add Termination Guarantee

A livelock occurs when threads keep executing transitions but never reach their return places. Typical causes include spin-loops that retry indefinitely without making progress (e.g., CAS retry loops without backoff, or busy-wait loops polling a condition that never changes).

### Fix Rules

1. Every loop in the CIR must have a reachable exit path that leads to `return`.
2. For CAS retry loops: ensure the failure branch eventually leads to either a successful CAS or a different exit path, not an infinite retry.
3. For busy-wait loops: add a bounded retry count or replace busy-wait with a proper condvar wait.
4. If a thread spins on a condition, ensure some other thread will eventually change that condition.

### Example Pattern

A CAS loop that retries indefinitely on failure:

```json
{"sid": "s1", "op": ["res_op", "flag", "cas", "false", "true"],
 "transfer": ["branch", "flag == false", "s2", "s1"]}
```

Fixed: add a retry limit or alternative exit:

```json
{"sid": "s1", "op": ["res_op", "counter", "load"],
 "transfer": ["branch", "counter < 3", "s2", "s4"]},
{"sid": "s2", "op": ["res_op", "flag", "cas", "false", "true"],
 "transfer": ["branch", "flag == false", "s3", "s5"]},
{"sid": "s3", "op": ["res_op", "counter", "store", "0"],
 "transfer": ["next", "s4"]},
{"sid": "s4", "op": "return", "transfer": "return"},
{"sid": "s5", "op": ["res_op", "counter", "store", "1"],
 "transfer": ["next", "s1"]}
```

The key principle: every cycle in the control flow graph must have a condition that eventually becomes true and breaks the cycle.
