# gen-code-replay-v1 — re-scoring stored LLM Rust (fixed toolchain)

Batch `run-20260923T005232`; 59 G3 code cells; replays the last round Rust of each cell. 0 LLM requests.

- failures at freeze-5: **28**
- fixed to conformant now: **2/28** (7.1% harness gap)
- still failing: **26**
- freeze-5 accepted cells replayed: 31; regressions: **0**
- remaining violation kinds: {'extra_op': 26}
- smoke: conform 0/0

| task | family | before | after conformant | kind |
| --- | --- | --- | --- | --- |
| atomic-data/bounded_counter_invariant | atomic-data | violation | 0/4 | extra_op |
| atomic-data/bounded_counter_invariant | atomic-data | violation | 0/4 | extra_op |
| atomic-data/bounded_counter_invariant | atomic-data | violation | 0/4 | extra_op |
| atomic-data/counter_overflow_safety | atomic-data | violation | 0/4 | extra_op |
| atomic-data/counter_overflow_safety | atomic-data | violation | 0/4 | extra_op |
| atomic-data/counter_overflow_safety | atomic-data | violation | 0/4 | extra_op |
| channel/bounded_backpressure_lock_held | channel | violation | 0/4 | extra_op |
| channel/bounded_backpressure_lock_held | channel | violation | 0/4 | extra_op |
| channel/bounded_backpressure_lock_held | channel | violation | 0/4 | extra_op |
| channel/rendezvous_both_send | channel | PASS | 4/4 | None |
| channel/rendezvous_both_send | channel | PASS | 4/4 | None |
| channel/rendezvous_both_send | channel | PASS | 4/4 | None |
| channel/send_while_holding_mutex | channel | PASS | 4/4 | None |
| channel/send_while_holding_mutex | channel | PASS | 4/4 | None |
| channel/send_while_holding_mutex | channel | PASS | 4/4 | None |
| condvar/bare_wait_no_predicate | condvar | violation | 0/4 | extra_op |
| condvar/bare_wait_no_predicate | condvar | violation | 0/4 | extra_op |
| condvar/bare_wait_no_predicate | condvar | violation | 0/4 | extra_op |
| condvar/lost_wakeup_notify_before_wait | condvar | violation | 4/4 | None |
| condvar/lost_wakeup_notify_before_wait | condvar | violation | 4/4 | None |
| condvar/lost_wakeup_notify_before_wait | condvar | violation | 0/4 | extra_op |
| lock-order/abba_2lock | lock-order | PASS | 4/4 | None |
| lock-order/abba_2lock | lock-order | PASS | 4/4 | None |
| lock-order/abba_2lock | lock-order | PASS | 4/4 | None |
| lock-order/cross_module_cycle | lock-order | PASS | 4/4 | None |
| lock-order/cross_module_cycle | lock-order | PASS | 4/4 | None |
| lock-order/cross_module_cycle | lock-order | PASS | 4/4 | None |
| lock-order/cycle_3lock | lock-order | PASS | 4/4 | None |
| lock-order/cycle_3lock | lock-order | PASS | 4/4 | None |
| lock-order/two_independent_cycles | lock-order | PASS | 4/4 | None |
| lock-order/two_independent_cycles | lock-order | PASS | 4/4 | None |
| lock-order/two_independent_cycles | lock-order | PASS | 4/4 | None |
| semaphore/acquire_twice_no_release | semaphore | violation | 0/4 | extra_op |
| semaphore/acquire_twice_no_release | semaphore | violation | 0/4 | extra_op |
| semaphore/acquire_twice_no_release | semaphore | violation | 0/4 | extra_op |
| semaphore/permit_leak | semaphore | violation | 0/4 | extra_op |
| semaphore/permit_leak | semaphore | violation | 0/4 | extra_op |
| semaphore/permit_leak | semaphore | violation | 0/4 | extra_op |
| semaphore/throttle_n_permits | semaphore | violation | 0/4 | extra_op |
| semaphore/throttle_n_permits | semaphore | violation | 0/4 | extra_op |
| semaphore/throttle_n_permits | semaphore | violation | 0/4 | extra_op |
| structure/finite_call_loop | structure | PASS | 4/4 | None |
| structure/finite_call_loop | structure | PASS | 4/4 | None |
| structure/finite_call_loop | structure | PASS | 4/4 | None |
| structure/nested_scope_lock_order | structure | PASS | 4/4 | None |
| structure/nested_scope_lock_order | structure | PASS | 4/4 | None |
| structure/nested_scope_lock_order | structure | PASS | 4/4 | None |
| structure/scope_bound_k_workers | structure | violation | 0/4 | extra_op |
| structure/scope_bound_k_workers | structure | violation | 0/4 | extra_op |
| structure/scope_bound_k_workers | structure | violation | 0/4 | extra_op |
| structure/scope_worker_abba | structure | PASS | 4/4 | None |
| structure/scope_worker_abba | structure | PASS | 4/4 | None |
| structure/scope_worker_abba | structure | PASS | 4/4 | None |
| structure/spawn_join_loop_finite | structure | PASS | 4/4 | None |
| structure/spawn_join_loop_finite | structure | PASS | 4/4 | None |
| structure/spawn_join_loop_finite | structure | PASS | 4/4 | None |
| structure/worker_with_payload | structure | violation | 0/4 | extra_op |
| structure/worker_with_payload | structure | PASS | 4/4 | None |
| structure/worker_with_payload | structure | PASS | 4/4 | None |
