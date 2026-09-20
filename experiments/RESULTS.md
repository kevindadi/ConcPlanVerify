# RESULTS — ConcIR repair/extraction benchmark

> **Generated file — do not edit by hand.** Regenerate with:
> ```
> python -m cir_workflow results --out /tmp results --batch experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5 --trackd experiments/detection-v3/DETECTION.json --scale experiments/scale-v2/SCALE.json --output experiments/RESULTS.md
> ```

## Provenance

- batch: `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5`
- track D: `experiments/detection-v3/DETECTION.json`
- scale: `experiments/scale-v2/SCALE.json`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- protocol[0] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`
- protocol[1] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`
- protocol[2] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`

## 1. Main table

| task | arm | accepted | round | tokens | false_accept | build | behavior | miri | expert | extract |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 3/3 | 1 | 1417±99 | 3/3 | True | hang | clean | — | — |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2/3 | 1 | 12574±6516 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 1/3 | 2 | 7957 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/partial_deadlock_bystander | A3_local | 0/3 | — | — | 0/3 | None | — | — | — | — |
| lock-order/partial_deadlock_bystander | A3_whole | 2/3 | 4 | 15255±30 | 0/3 | None | — | — | — | — |
| lock-order/cross_module_cycle | A0_direct | 3/3 | 1 | 898±242 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cross_module_cycle | A1_self_iter | 3/3 | 1 | 1247±66 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 3/3 | 1 | 693 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cross_module_cycle | A3_local | 3/3 | 2 | 1912 | 0/3 | None | — | — | — | — |
| lock-order/cross_module_cycle | A3_whole | 3/3 | 2 | 3580 | 0/3 | None | — | — | — | — |
| lock-order/cycle_3lock | A0_direct | 3/3 | 1 | 1001 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cycle_3lock | A1_self_iter | 0/3 | — | — | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cycle_3lock | A2_tools_iter_ml | 3/3 | 1 | 911 | 0/3 | True | terminated_ok | clean | — | — |
| lock-order/cycle_3lock | A3_local | 3/3 | 2 | 2100±13 | 0/3 | None | — | — | — | — |
| lock-order/cycle_3lock | A3_whole | 2/3 | 3 | 8524 | 0/3 | None | — | — | — | — |
| structure/nested_scope_lock_order | A0_direct | 3/3 | 1 | 736 | 0/3 | True | terminated_ok | clean | — | — |
| structure/nested_scope_lock_order | A1_self_iter | 3/3 | 1 | 1105 | 0/3 | True | terminated_ok | clean | — | — |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 3/3 | 1 | 646 | 0/3 | True | terminated_ok | clean | — | — |
| structure/nested_scope_lock_order | A3_local | 3/3 | 2 | 2054 | 0/3 | None | — | — | — | — |
| structure/nested_scope_lock_order | A3_whole | 2/3 | 4 | 10713 | 0/3 | None | — | — | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 3/3 | 1 | 623±213 | 2/3 | True | hang | clean | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 3/3 | 1 | 1334±34 | 0/3 | True | terminated_ok | clean | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 3/3 | 1 | 675 | 0/3 | True | terminated_ok | clean | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 3/3 | 2 | 2841±29 | 0/3 | None | — | — | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | 2/3 | 4 | 13930 | 0/3 | None | — | — | — | — |
| channel/bounded_backpressure_lock_held | A0_direct | 3/3 | 1 | 707 | 0/3 | True | terminated_ok | clean | — | — |
| channel/bounded_backpressure_lock_held | A1_self_iter | 3/3 | 1 | 1523±730 | 0/3 | True | terminated_ok | clean | — | — |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 3/3 | 1 | 670±15 | 0/3 | True | terminated_ok | clean | — | — |
| channel/bounded_backpressure_lock_held | A3_local | 3/3 | 2 | 2165 | 0/3 | None | — | — | — | — |
| channel/bounded_backpressure_lock_held | A3_whole | 3/3 | 4 | 11127±530 | 0/3 | None | — | — | — | — |
| channel/send_while_holding_mutex | A0_direct | 3/3 | 1 | 859±22 | 0/3 | True | terminated_ok | clean | — | — |
| channel/send_while_holding_mutex | A1_self_iter | 0/3 | — | — | 0/3 | None | — | — | — | — |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 3/3 | 1 | 760±27 | 0/3 | True | terminated_ok | clean | — | — |
| channel/send_while_holding_mutex | A3_local | 3/3 | 2 | 1506±38 | 0/3 | None | — | — | — | — |
| channel/send_while_holding_mutex | A3_whole | 3/3 | 3 | 5983 | 0/3 | None | — | — | — | — |
| semaphore/acquire_twice_no_release | A0_direct | 3/3 | 1 | 877±287 | 3/3 | True | hang | detected | — | — |
| semaphore/acquire_twice_no_release | A1_self_iter | 3/3 | 1 | 8415±10091 | 0/3 | True | terminated_ok | clean | — | — |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 3/3 | 2 | 4196±419 | 0/3 | True | terminated_ok | clean | — | — |
| semaphore/acquire_twice_no_release | A3_local | 3/3 | 2 | 1759±11 | 0/3 | None | — | — | — | — |
| semaphore/acquire_twice_no_release | A3_whole | 3/3 | 2 | 3432 | 0/3 | None | — | — | — | — |

`accepted`/`false_accept` are k/n over repeats; `round`/`tokens` are mean±range over accepted cells. `expert` is the manual/proxy label, `extract` the validated extraction verdict.

### Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | by source |
| --- | --- | --- | --- | --- | --- |
| A0_direct | 24 | 24 | 1.00 | 8 | {'behavior': 8, 'by_construction': 3} |
| A1_self_iter | 24 | 17 | 0.71 | 0 | — |
| A2_tools_iter_ml | 24 | 22 | 0.92 | 0 | — |
| A3_local | 24 | 21 | 0.88 | 0 | — |
| A3_whole | 24 | 20 | 0.83 | 0 | — |

### A3 decision distribution

| task | decisions |
| --- | --- |

### Expert labels

| task | arm | bug_present | design_preserved | auto | agree |
| --- | --- | --- | --- | --- | --- |

Agreement with the automatic oracle: **n/a** (unsure 0).

### Extraction oracle

- cells: 0; validated: 0; stage distribution: `{}`

| task|arm | stage | validated | verdict |
| --- | --- | --- | --- |

### Track D (detection capability)

| task | side | concir.petri | concir.interp | miri | lockbud |
| --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | buggy | FAIL | FAIL | None | None |
| lock-order/abba_2lock | fixed | PASS | PASS | None | None |
| lock-order/cycle_3lock | buggy | FAIL | FAIL | None | None |
| lock-order/cycle_3lock | fixed | PASS | PASS | None | None |
| lock-order/cross_module_cycle | buggy | FAIL | FAIL | None | None |
| lock-order/cross_module_cycle | fixed | PASS | PASS | None | None |
| lock-order/partial_deadlock_bystander | buggy | FAIL | FAIL | None | None |
| lock-order/partial_deadlock_bystander | fixed | PASS | PASS | None | None |
| condvar/lost_wakeup_notify_before_wait | buggy | FAIL | FAIL | None | None |
| condvar/lost_wakeup_notify_before_wait | fixed | PASS | PASS | None | None |
| channel/rendezvous_both_send | buggy | FAIL | FAIL | None | None |
| channel/rendezvous_both_send | fixed | PASS | PASS | None | None |
| semaphore/permit_leak | buggy | FAIL | FAIL | None | None |
| semaphore/permit_leak | fixed | PASS | PASS | None | None |
| semaphore/acquire_twice_no_release | buggy | FAIL | FAIL | None | None |
| semaphore/acquire_twice_no_release | fixed | PASS | PASS | None | None |
| semaphore/throttle_n_permits | buggy | None | None | None | None |
| semaphore/throttle_n_permits | fixed | None | None | None | None |
| condvar/bare_wait_no_predicate | buggy | FAIL | FAIL | None | None |
| condvar/bare_wait_no_predicate | fixed | PASS | PASS | None | None |
| channel/bounded_backpressure_lock_held | buggy | FAIL | FAIL | None | None |
| channel/bounded_backpressure_lock_held | fixed | PASS | PASS | None | None |
| atomic-data/bounded_counter_invariant | buggy | None | None | None | None |
| atomic-data/bounded_counter_invariant | fixed | None | None | None | None |
| atomic-data/counter_overflow_safety | buggy | FAIL | FAIL | None | None |
| atomic-data/counter_overflow_safety | fixed | PASS | PASS | None | None |
| atomic-data/atomic_lost_update | buggy | FAIL | FAIL | None | None |
| atomic-data/atomic_lost_update | fixed | PASS | PASS | None | None |
| structure/scope_bound_k_workers | buggy | None | None | None | None |
| structure/scope_bound_k_workers | fixed | None | None | None | None |
| structure/nested_scope_lock_order | buggy | FAIL | FAIL | None | None |
| structure/nested_scope_lock_order | fixed | PASS | PASS | None | None |
| structure/scope_worker_abba | buggy | FAIL | FAIL | None | None |
| structure/scope_worker_abba | fixed | PASS | PASS | None | None |
| structure/worker_with_payload | buggy | None | None | None | None |
| structure/worker_with_payload | fixed | None | None | None | None |
| boundary/unbounded_int_unknown | buggy | None | None | None | None |
| boundary/unbounded_int_unknown | fixed | None | None | None | None |
| lock-order/two_independent_cycles | buggy | FAIL | FAIL | None | None |
| lock-order/two_independent_cycles | fixed | None | None | None | None |
| condvar/same_cv_different_locks | buggy | None | None | None | None |
| condvar/same_cv_different_locks | fixed | None | None | None | None |
| condvar/notify_one_multi_waiter_wrong_pick | buggy | FAIL | FAIL | None | None |
| condvar/notify_one_multi_waiter_wrong_pick | fixed | PASS | PASS | None | None |
| channel/send_while_holding_mutex | buggy | FAIL | FAIL | None | None |
| channel/send_while_holding_mutex | fixed | PASS | PASS | None | None |
| structure/finite_call_loop | buggy | None | None | None | None |
| structure/finite_call_loop | fixed | None | None | None | None |
| structure/spawn_join_loop_finite | buggy | None | None | None | None |
| structure/spawn_join_loop_finite | fixed | None | None | None | None |
| boundary/rwlock_unsupported | buggy | UNSUPPORTED | UNSUPPORTED | None | None |
| boundary/rwlock_unsupported | fixed | None | None | None | None |
| boundary/async_select_unsupported | buggy | UNSUPPORTED | UNSUPPORTED | None | None |
| boundary/async_select_unsupported | fixed | None | None | None | None |
| P1 | buggy | None | None | None | None |
| P1 | fixed | None | None | None | None |
| P2 | buggy | None | None | None | None |
| P2 | fixed | None | None | None | None |
| P3 | buggy | None | None | None | None |
| P3 | fixed | None | None | None | None |
| P4 | buggy | None | None | None | None |
| P4 | fixed | None | None | None | None |
| P5 | buggy | None | None | None | None |
| P5 | fixed | None | None | None | None |
| P6 | buggy | None | None | None | None |
| P6 | fixed | None | None | None | None |
| P7 | buggy | None | None | None | None |
| P7 | fixed | None | None | None | None |
| P8 | buggy | None | None | None | None |
| P8 | fixed | None | None | None | None |
| P9 | buggy | None | None | None | None |
| P9 | fixed | None | None | None | None |
| real-cases/rmw-zenoh-998 | buggy | FAIL | FAIL | None | None |
| real-cases/rmw-zenoh-998 | fixed | PASS | PASS | None | None |
| real-cases/dashmap-369 | buggy | UNSUPPORTED | UNSUPPORTED | None | None |
| real-cases/dashmap-369 | fixed | None | None | None | None |

Lockbud: available=True commit=`cc78cb72cb85`.

### Scale

| threads | chain_len | max_states | outcome | complete | states | wall_ms |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | 2 | 5000 | PASS | True | 39 | 15 |
| 2 | 2 | 20000 | PASS | True | 39 | 15 |
| 2 | 2 | 200000 | PASS | True | 39 | 15 |
| 3 | 2 | 5000 | PASS | True | 218 | 18 |
| 3 | 2 | 20000 | PASS | True | 218 | 19 |
| 3 | 2 | 200000 | PASS | True | 218 | 18 |
| 4 | 2 | 5000 | PASS | True | 1267 | 40 |
| 4 | 2 | 20000 | PASS | True | 1267 | 41 |
| 4 | 2 | 200000 | PASS | True | 1267 | 42 |
| 5 | 2 | 5000 | UNKNOWN | False | 5001 | 121 |
| 5 | 2 | 20000 | PASS | True | 7780 | 205 |
| 5 | 2 | 200000 | PASS | True | 7780 | 201 |
| 3 | 3 | 5000 | PASS | True | 320 | 20 |
| 3 | 3 | 20000 | PASS | True | 320 | 20 |
| 3 | 3 | 200000 | PASS | True | 320 | 20 |
| 4 | 3 | 5000 | PASS | True | 1891 | 57 |
| 4 | 3 | 20000 | PASS | True | 1891 | 54 |
| 4 | 3 | 200000 | PASS | True | 1891 | 57 |
| 5 | 3 | 5000 | UNKNOWN | False | 5001 | 124 |
| 5 | 3 | 20000 | PASS | True | 11710 | 318 |
| 5 | 3 | 200000 | PASS | True | 11710 | 318 |
| 6 | 3 | 5000 | UNKNOWN | False | 5001 | 99 |
| 6 | 3 | 20000 | UNKNOWN | False | 20001 | 445 |
| 6 | 3 | 200000 | PASS | True | 78263 | 2471 |

## Deviations and threats to validity

- D-19: the main batch is not gated on Rust-arm oracle completeness; missing `oracle.model` cells are `inconclusive` and backfilled by extraction/expert labels.
- Expert labels are LLM-assisted (agent-proxy); a random sample plus every disagreement is queued for human review (`expert-labels/HUMAN_REVIEW_QUEUE.md`).
- All offline recomputation uses a single BIN_MAIN; any result on another binary sha is listed here.
