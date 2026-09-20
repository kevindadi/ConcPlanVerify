# Case: `lock-order/partial_deadlock_bystander` under the `holds_all` contract

Running example candidate. Binary `BIN_MAIN` `4bec943d…` (includes the
`holds_all` preservation hint). Run: `run-20260920T180145`, A3_local and
A3_whole, K=6, reps 0–2, 17 requests.

## Why it is hard

The frozen contract preserves, for **both** workers, a reachable state in which
the worker holds `[main::a, main::b]` simultaneously (`holds_all`), plus
`a-completes` / `b-completes` (`always_reachable`). The buggy program combines
three ingredients:

1. a semaphore cross-handshake (`a` releases `sa`, waits `sb`; `b` releases
   `sb`, waits `sa`);
2. **nested** mutex acquisition (`a` holds `main::a` and then takes `main::b`,
   and symmetrically for `b`);
3. a bystander thread that loops forever, so the system is not globally
   deadlocked — only the two workers stop.

## What the contract blocks

The cheap "fix" is to release the first mutex before taking the second. That
removes the circular wait but destroys the required nested hold: the
`preserved holds_all` properties become unreachable, and `explore` returns:

```
preserved: main::a holds [main::a, main::b] at once -> FAIL
  hint: The design requires a reachable state in which `a` holds all of
        [a, b] simultaneously; no such state exists in this revision. Keep the
        nested acquisition; fix the defect by acquisition order, scope, or
        handshake instead of releasing early.
```

`python -m cir_workflow contract-strength` showed the same on the v2 batch: the
empty "release early" revision is rejected under the frozen contract.

## Outcome

| arm | rep 0 | rep 1 | rep 2 |
| --- | --- | --- | --- |
| A3_local | stalled | stalled | stalled |
| A3_whole | accepted r3 | exhausted | accepted r3 |

A3_whole found a design-preserving fix: **both** workers take `main::a` then
`main::b` (consistent global order) and replace the cross-handshake with a
single shared semaphore (`hs.acquire()` / `hs.release()`), keeping the nested
hold. A3_local (local regeneration) never escaped the "release early" local
patch and stalled in all three repeats.

## A2-ml vs A3

A2-ml's accepted fix (previous round) also releases the first guard before the
second mutex: it builds, Miri is clean, Lockbud is clean, and the automatic
oracle accepts it — but it is **not** design-preserving (`holds_all` is gone).
A3_whole is the only arm that both passes the contract and keeps the nested
acquisition. This is the running example for "tool-green is not design-preserving".
