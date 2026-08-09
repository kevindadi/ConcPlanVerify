## Repair Strategy: Behavioral Dead Transition

The CVN reachability analysis proved that the reported transition
**never fires on any feasible interleaving**. The ConcIR statement anchored
to that transition is effectively unreachable, which almost always
indicates a logical defect upstream rather than a concurrency race.

Typical causes to inspect, in priority order:

1. **Unsatisfiable `branch` guard.**
   A preceding `branch` statement tests a predicate whose false (or
   true) arm leads here but whose condition is statically contradicted
   by earlier `write`s, constants, or `CasSuccess`/`CasFailure` pairs.
   Either relax the predicate or correct the earlier update.

2. **Missing producer of a required resource.**
   The transition consumes from a resource (channel, condvar-notify,
   semaphore permit, …) that no reachable code path ever produces.
   Add the missing `send`, `notify`, `release`, or initial marking.

3. **Dead continuation.**
   An earlier statement transfers with `return` or `branch` in a way
   that permanently bypasses this sid. Re-route the control flow so
   the dead statement has a reachable predecessor.

4. **Ordering bug in spawn/join.**
   A worker function is scheduled such that its `spawn` site is never
   reached (e.g., placed after an early `return`), making every
   statement inside it dead. Move the `spawn` onto a live path.

Do **not** respond by deleting the dead statement unless the preserved
business goals remain reachable without it — the point of the check is
that the author's intended behaviour is currently unrealisable.

### ConcIR edit checklist

- Keep all declared resources, protection entries, functions, and
  `goals` unchanged in identity; only the control flow / guard
  expressions may be edited.
- Preserve every `sid` that is not part of the dead path. New sids are
  allowed for restored predecessors.
- After repair, re-run the analyser: a valid fix eliminates the
  `DeadTransition` report *and* keeps every previously-reachable goal
  reachable.
