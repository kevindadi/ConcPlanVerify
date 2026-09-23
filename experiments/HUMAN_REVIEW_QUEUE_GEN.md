# HUMAN_REVIEW_QUEUE_GEN — generation cells (owner to fill)

Review each accepted G3 Rust for `bug_present` and per-`Ri` satisfaction.
Rows are deduplicated by rust sha256; coincident tasks share a row.

| task(s) | reps | rust sha256[16] | bug_present | R_i unsatisfied |
| --- | --- | --- | --- | --- |
| atomic-data/bounded_counter_invariant | 0 | `07afe0fec004d8e2` | | |
| structure/worker_with_payload | 2 | `15fa9f04df5d6fdf` | | |
| semaphore/throttle_n_permits | 1,2 | `18bdc0eb877d58d7` | | |
| channel/rendezvous_both_send | 1 | `1af51bf899c63112` | | |
| atomic-data/counter_overflow_safety | 0 | `42530fc02b4a0df4` | | |
| structure/worker_with_payload | 0 | `4a7adaefa7847430` | | |
| lock-order/cross_module_cycle | 2 | `4ecd2ee317789097` | | |
| channel/rendezvous_both_send | 2 | `53261b1cb1398d54` | | |
