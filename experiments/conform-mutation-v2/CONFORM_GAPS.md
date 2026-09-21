# conform blind spots (v2, operation-bound events)

BIN_V2 `073129de3a6d378a4e198bf712028c0c950cdf080981fb0073dfb7af5fa7ce5c`. v2 emits events from the operation (helpers in `cir_trace`), and
conform v2 checks (tag, sid, op, resource). Compared with v1
(`conform-mutation-v1/CONFORM_GAPS.md`):

| op | v1 recall | v2 recall | v2 failure reason |
| --- | --- | --- | --- |
| M1 swap adjacent locks | 0.545 (all timeout) | **1.0** | violation (order) |
| M2 delete an unlock | 0.067 (timeout) | **0.947** | violation (missing) |
| M3 move ev | 0.0 | n/a by construction | — |
| M4 notify_all <-> notify_one | 0.333 (timeout) | **1.0** | violation (order: op mismatch) |
| M5 move a channel op one statement | — | 0.0 | blind spot |
| M6 delete a sync call | 1.0 | 1.0 | violation (missing) |
| M7 swap two spawn statements (control) | 0.105 fp | 2/23 | order-sensitive (see below) |
| M8 lock a different resource | — | **0.8** | violation (resource) |

## Remaining blind spots

- **M5 (move a channel op one statement earlier)** — 0/6. The statement swapped
  across is `pc = N;` (a state-machine assignment, no event), so the event
  stream is unchanged. Moving a channel op *across a lock* (the v1 operator)
  is not present in the codegen skeletons because the channel op and the lock
  are in different `pc` states; the honest statement is that v2 binds channel
  events to the channel call, but a reorder that does not change the event
  stream is still invisible.
- **M7 (swap two spawn statements)** — 21/23 PASS. For `scope` programs the
  children are created from one `scope` statement (event unchanged) → PASS; for
  `spawn` programs swapping two spawn statements reorders two spawn events,
  which is a genuine model-order deviation (2/23). This is **order-sensitivity,
  not a false positive** (per review L-2).
- **M8** — 8/10; the 2 misses are cases where the two resources are the same
  kind and the swapped lock's sid still matches a model step with that resource
  on another path.
