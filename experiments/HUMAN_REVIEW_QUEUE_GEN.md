# HUMAN_REVIEW_QUEUE_GEN — generation cells (owner to fill)

Review each accepted Rust for `bug_present` and per-`Ri` satisfaction. Rows are
sha-deduplicated; the two agent-vs-tool disagreements are first. Leave verdicts blank.

| arm | task | rep | rust sha256[12] | agent bug_present | bug_present | R_i unsatisfied |
| --- | --- | --- | --- | --- | --- | --- |
| G3_concir | lock-order/cycle_3lock | 0 | `7f565d54edd3` | yes | | |
| G3_concir | lock-order/cycle_3lock | 2 | `68216e3ecd00` | yes | | |
| G3_concir | atomic-data/bounded_counter_invariant | 0 | `07afe0fec004` | no | | |
| G3_concir | atomic-data/counter_overflow_safety | 0 | `42530fc02b4a` | no | | |
| G3_concir | channel/bounded_backpressure_lock_held | 0 | `56da8bd84ed9` | no | | |
| G3_concir | channel/rendezvous_both_send | 0 | `1af51bf899c6` | no | | |
| G3_concir | condvar/bare_wait_no_predicate | 0 | `e39c38dd648e` | no | | |
| G3_concir | lock-order/abba_2lock | 0 | `f34602b468bf` | no | | |
| G0_direct | atomic-data/atomic_lost_update | 0 | `8cd45a853599` | no | | |
| G0_direct | atomic-data/bounded_counter_invariant | 0 | `e2fcca636617` | no | | |
| G0_direct | atomic-data/counter_overflow_safety | 0 | `d6d976987104` | no | | |
| G0_direct | channel/bounded_backpressure_lock_held | 0 | `9b533aa7773e` | no | | |
