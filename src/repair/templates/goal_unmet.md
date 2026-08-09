## Repair Strategy — Unreachable Business Goal

The translated ConcIR is free of concurrency bugs (no deadlock, no blocked
channel operation, no lost condvar signal), but the declared business
goal was never witnessed in the state-space exploration. This means the
program, while *safe*, has dropped the functional behavior the user
asked for.

### Root cause taxonomy

Typical causes, in order of frequency:

1. A thread that produces a goal-relevant token was removed during an
   earlier repair pass (e.g. a `notify`, a `channel_send`, or the
   terminating `return` of a consumer).
2. A guard condition is too strict and the goal-reaching branch is
   never taken (three-valued evaluation retains the branch as *unknown*
   in the CVN, but the producer never fires).
3. A spawn/join pair was rewritten such that the worker which reaches
   the goal place is never started.
4. A protection or resource declaration was tightened in a way that
   removes the lock/unlock pair guarding the goal transition.

### What the repair must do

* Restore the producer transition(s) for the unmet predicate. For
  `M(rp_X) >= k` this means re-introducing enough successful
  acquire/release cycles so that `X` accumulates `k` tokens. For a
  control-place reachability goal, it means ensuring the thread does
  reach that statement on some interleaving.
* Do **not** weaken or remove any preservation constraint listed above.
* Do **not** add spurious concurrency (fresh locks, extra spawns)
  unless the missing behavior genuinely requires it.

### Output

Produce a complete revised ConcIR JSON. Keep all existing resources,
protection entries, functions, and the declared goal set unchanged.
