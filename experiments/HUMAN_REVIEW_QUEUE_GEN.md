# HUMAN_REVIEW_QUEUE_GEN — generation cells (owner to fill)

Review each accepted G3 Rust for `bug_present` and per-`Ri` satisfaction.
Rows are deduplicated by rust sha256; coincident tasks share a row.

| task(s) | reps | rust sha256[16] | bug_present | R_i unsatisfied |
| --- | --- | --- | --- | --- |
| atomic-data/bounded_counter_invariant | 0,1,2 | `07afe0fec004d8e2` | | |
| channel/rendezvous_both_send | 0 | `1af51bf899c63112` | | |
| lock-order/cross_module_cycle | 2 | `2482d1db660954d7` | | |
| structure/spawn_join_loop_finite | 2 | `3d51be736fa28770` | | |
| atomic-data/counter_overflow_safety | 0,2 | `42530fc02b4a0df4` | | |
| channel/rendezvous_both_send | 2 | `53261b1cb1398d54` | | |
| channel/bounded_backpressure_lock_held | 0,1 | `56da8bd84ed9573e` | | |
| channel/rendezvous_both_send | 1 | `5cf5bf97db61eded` | | |
