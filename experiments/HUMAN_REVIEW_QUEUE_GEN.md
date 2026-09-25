# HUMAN_REVIEW_QUEUE_GEN — generation cells (owner confirmed)

Rows are sha-deduplicated; the two agent-vs-tool disagreements are first.

Confirmation recorded 2026-09-25: the owner explicitly stated in the review
conversation that all pending human reviews were completed and authorized
correcting these records. Existing per-row verdicts and reasons are retained;
only their pending/confirmed status is reconciled. This is an owner confirmation,
not a new review performed by the coding assistant. It covers these 12 listed
rows and does not establish a structural-equivalence audit of all generated
programs or additional unrecorded review fields.

| arm | task | rep | rust sha256[12] | agent bug_present | bug_present | R_i unsatisfied |
| --- | --- | --- | --- | --- | --- | --- |
| G3_concir | lock-order/cycle_3lock | 0 | `7f565d54edd3` | yes | no | — (owner 2026-09-24: agent false positive; lock order a<b<c is acyclic, t3 takes a then c) |
| G3_concir | lock-order/cycle_3lock | 2 | `68216e3ecd00` | yes | no | — (owner 2026-09-24: agent false positive; same ordering as rep0 with explicit drops) |
| G3_concir | atomic-data/bounded_counter_invariant | 0 | `07afe0fec004` | no | no | R7 (prints `DONE done=2`, the counter, instead of `DONE done=1`) — owner confirmed (confirmation recorded 2026-09-25; proposal dated 2026-09-24) |
| G3_concir | atomic-data/counter_overflow_safety | 0 | `42530fc02b4a` | no | no | — (guarded increment `if tmp < 1`, ends at 1, prints `DONE done=1`) — owner confirmed (confirmation recorded 2026-09-25) |
| G3_concir | channel/bounded_backpressure_lock_held | 0 | `56da8bd84ed9` | no | no | R2 [U] (no shared lock `m` implemented at all; R5 holds vacuously) — owner confirmed (confirmation recorded 2026-09-25) |
| G3_concir | channel/rendezvous_both_send | 0 | `1af51bf899c6` | no | no | — (`sync_channel(0)`, one send / one recv) — owner confirmed (confirmation recorded 2026-09-25) |
| G3_concir | condvar/bare_wait_no_predicate | 0 | `e39c38dd648e` | no | no | — (predicate loop `while !ready { wait }`, notify under lock, prints `DONE ready=true`) — owner confirmed (confirmation recorded 2026-09-25) |
| G3_concir | lock-order/abba_2lock | 0 | `f34602b468bf` | no | no | — (both workers lock a then b) — owner confirmed (confirmation recorded 2026-09-25) |
| G0_direct | atomic-data/atomic_lost_update | 0 | `8cd45a853599` | no | no | R9 (prints `DONE done=2`, the counter, instead of `DONE done=1`) — owner confirmed (confirmation recorded 2026-09-25) |
| G0_direct | atomic-data/bounded_counter_invariant | 0 | `e2fcca636617` | no | no | R7 (prints `DONE done=2`); extra `Mutex` around `c` but consistent order m→c, no deadlock — owner confirmed (confirmation recorded 2026-09-25) |
| G0_direct | atomic-data/counter_overflow_safety | 0 | `d6d976987104` | no | no | — (guarded increment, ends at 1, prints `DONE done=1`) — owner confirmed (confirmation recorded 2026-09-25) |
| G0_direct | channel/bounded_backpressure_lock_held | 0 | `9b533aa7773e` | no | no | — (hand-rolled 1-slot channel with two condvars; `m` taken only in short scopes, never while waiting on the channel) — owner confirmed (confirmation recorded 2026-09-25) |
