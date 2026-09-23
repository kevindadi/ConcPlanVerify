# gen-code-replay-v2 — v3-code cells on the round-q toolchain

Batch `run-20260923T182939`; cells replayed: 59. 0 LLM requests.

- freeze-6 failures: **19**
  - fixed by build/P-1: **12**
  - fixed by conform/P-2: **5**
  - still failing: **2** {'extra_op': 2}
- freeze-6 accepted cells: 40; regressions: **0**
- accepted cells with monitor `unmapped`: 37/40

| task | before | built | conformant | kind |
| --- | --- | --- | --- | --- |
| atomic-data/bounded_counter_invariant | accept | True | 4/4 | None |
| atomic-data/bounded_counter_invariant | accept | True | 4/4 | None |
| atomic-data/bounded_counter_invariant | accept | True | 4/4 | None |
| atomic-data/counter_overflow_safety | conform | True | 4/4 | None |
| atomic-data/counter_overflow_safety | conform | True | 4/4 | None |
| atomic-data/counter_overflow_safety | conform | True | 4/4 | None |
| channel/bounded_backpressure_lock_held | conform | True | 0/4 | extra_op |
| channel/bounded_backpressure_lock_held | conform | True | 0/4 | extra_op |
| channel/bounded_backpressure_lock_held | conform | True | 4/4 | None |
| channel/rendezvous_both_send | accept | True | 4/4 | None |
| channel/rendezvous_both_send | accept | True | 4/4 | None |
| channel/rendezvous_both_send | accept | True | 4/4 | None |
| channel/send_while_holding_mutex | accept | True | 4/4 | None |
| channel/send_while_holding_mutex | accept | True | 4/4 | None |
| channel/send_while_holding_mutex | accept | True | 4/4 | None |
| condvar/bare_wait_no_predicate | accept | True | 4/4 | None |
| condvar/bare_wait_no_predicate | accept | True | 4/4 | None |
| condvar/bare_wait_no_predicate | accept | True | 4/4 | None |
| condvar/lost_wakeup_notify_before_wait | accept | True | 4/4 | None |
| condvar/lost_wakeup_notify_before_wait | accept | True | 4/4 | None |
| condvar/lost_wakeup_notify_before_wait | accept | True | 4/4 | None |
| lock-order/abba_2lock | accept | True | 4/4 | None |
| lock-order/abba_2lock | accept | True | 4/4 | None |
| lock-order/abba_2lock | accept | True | 4/4 | None |
| lock-order/cross_module_cycle | accept | True | 4/4 | None |
| lock-order/cross_module_cycle | accept | True | 4/4 | None |
| lock-order/cross_module_cycle | accept | True | 4/4 | None |
| lock-order/cycle_3lock | accept | True | 4/4 | None |
| lock-order/cycle_3lock | accept | True | 4/4 | None |
| lock-order/two_independent_cycles | accept | True | 4/4 | None |
| lock-order/two_independent_cycles | accept | True | 4/4 | None |
| lock-order/two_independent_cycles | accept | True | 4/4 | None |
| semaphore/acquire_twice_no_release | build | True | 4/4 | None |
| semaphore/acquire_twice_no_release | build | True | 4/4 | None |
| semaphore/acquire_twice_no_release | build | True | 4/4 | None |
| semaphore/permit_leak | build | True | 4/4 | None |
| semaphore/permit_leak | build | True | 4/4 | None |
| semaphore/permit_leak | build | True | 4/4 | None |
| semaphore/throttle_n_permits | build | True | 4/4 | None |
| semaphore/throttle_n_permits | build | True | 4/4 | None |
| semaphore/throttle_n_permits | build | True | 4/4 | None |
| structure/finite_call_loop | accept | True | 4/4 | None |
| structure/finite_call_loop | accept | True | 4/4 | None |
| structure/finite_call_loop | accept | True | 4/4 | None |
| structure/nested_scope_lock_order | accept | True | 4/4 | None |
| structure/nested_scope_lock_order | accept | True | 4/4 | None |
| structure/nested_scope_lock_order | accept | True | 4/4 | None |
| structure/scope_bound_k_workers | build | True | 4/4 | None |
| structure/scope_bound_k_workers | build | True | 4/4 | None |
| structure/scope_bound_k_workers | build | True | 4/4 | None |
| structure/scope_worker_abba | accept | True | 4/4 | None |
| structure/scope_worker_abba | accept | True | 4/4 | None |
| structure/scope_worker_abba | accept | True | 4/4 | None |
| structure/spawn_join_loop_finite | accept | True | 4/4 | None |
| structure/spawn_join_loop_finite | accept | True | 4/4 | None |
| structure/spawn_join_loop_finite | accept | True | 4/4 | None |
| structure/worker_with_payload | conform | True | 4/4 | None |
| structure/worker_with_payload | accept | True | 4/4 | None |
| structure/worker_with_payload | accept | True | 4/4 | None |
