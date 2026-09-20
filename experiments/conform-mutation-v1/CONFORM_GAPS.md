# conform blind spots (mutation experiment)

Strict codegen-mode `conform` replays the **observed `cir_trace` event stream**
against the model; it does not inspect the code's actual synchronisation state.
It therefore has full recall for mutations that change the event stream and low
recall for mutations that keep the event stream identical. The mutation
experiment quantifies exactly that boundary.

## Recall by operator

| op | mutants | conform FAIL | recall | failure reason |
| --- | --- | --- | --- | --- |
| M1 swap adjacent locks | 11 | 6 | 0.545 | all `timeout` (deadlock), none `violation` |
| M2 delete a `drop(guard)` | 15 | 1 | 0.067 | `timeout` |
| M3 move `ev` before its call | 19 | 0 | 0.0 | — |
| M4 `notify_all` ↔ `notify_one` | 3 | 1 | 0.333 | `timeout` |
| M6 delete an `ev` | 19 | 19 | 1.0 | all `violation` |
| M7 swap spawn order (control) | 19 | 2 | 0.105 (false positive) | `violation` |

## Blind spots (conform does not catch; `equivalent`-excluded)

- **M3 (move `ev` before its call)** — 0/19. Moving an event to a different
  program point does not change the event *stream* (same sids, same per-thread
  order), so conform accepts. This is an annotation-to-code attachment error that
  conform cannot see; the instrumenter's label check is what guards it.
- **M2 (delete a `drop(guard)`)** — 1/15. The generated code emits the unlock
  `ev` *before* `drop`, so deleting the `drop` leaves the unlock event in the
  stream; the held region is extended but the event stream is unchanged. Only a
  resulting hang (1 case) is caught.
- **M1 (swap adjacent locks)** — 6/11, all via `timeout`. The two lock `ev`s are
  not swapped with the statements, so the event stream is unchanged; conform
  catches M1 only when the swap creates a deadlock. A pure order change that
  stays deadlock-free is invisible.
- **M4** — 1/3, via `timeout`; the event *kind* changes only if the model's
  `condvar_notify`/`condvar_notify_all` sids differ, and here the mutation hit a
  notify whose sid did not change.

## False positive

- **M7 (swap two spawn statements)** — 2/19 `violation`. Swapping the spawn
  pushes changes which child thread is created first; the model maps child tags
  by creation order (`child_tags`), while the generated closures carry fixed
  tags, so the tag↔thread assignment flips and conform reports a violation on a
  program that is semantically equivalent. This is a **tag-order sensitivity**
  (false positive) of the current conformance口径.

## Implication

`conform` is a **stream-level** check: recall 1.0 on event-stream mutations (M6),
0 on code-level mutations that preserve the stream (M3). This is the honest
characterisation for the paper — conform complements, not replaces, behavior
and Miri, and the instrumenter's mandatory-label check covers the M3 class.
No conform rule was changed this round.
