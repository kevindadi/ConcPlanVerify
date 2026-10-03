# TIERS — generation benchmark complexity tiers

Tiers are computed from `(requirement count, reference threads + resources, reference model states)` where the reference model is the case's `fixed`, `correct` or `buggy` CIR (in that order) and `states` is the petri state count recorded by `explore`.

Thresholds (frozen):

- **Simple**: requirements <= 7 and states <= 45 and shape <= 4.
- **Complex**: states >= 100, or (requirements >= 10 and states >= 40), or shape >= 7.
- **Medium**: everything else.

| task | tier | reqs | [U] | props | preserved | threads | resources | shape | states | reference |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| atomic-data/atomic_lost_update | Medium | 9 | 1 | 2 | 3 | 2 | 1 | 3 | 65 | fixed |
| atomic-data/bounded_counter_invariant | Simple | 7 | 1 | 2 | 2 | 2 | 2 | 4 | 31 | correct |
| atomic-data/counter_overflow_safety | Simple | 7 | 1 | 2 | 2 | 2 | 2 | 4 | 40 | fixed |
| channel/bounded_backpressure_lock_held | Medium | 8 | 2 | 1 | 2 | 2 | 2 | 4 | 20 | fixed |
| channel/rendezvous_both_send | Simple | 7 | 1 | 1 | 2 | 2 | 1 | 3 | 9 | fixed |
| channel/send_while_holding_mutex | Simple | 5 | 1 | 1 | 0 | 2 | 2 | 4 | 9 | fixed |
| condvar/bare_wait_no_predicate | Complex | 10 | 2 | 2 | 3 | 2 | 3 | 5 | 43 | fixed |
| condvar/lost_wakeup_notify_before_wait | Complex | 10 | 2 | 2 | 3 | 2 | 3 | 5 | 43 | fixed |
| condvar/notify_one_multi_waiter_wrong_pick | Complex | 10 | 2 | 1 | 1 | 3 | 4 | 7 | 74 | fixed |
| condvar/same_cv_different_locks | Complex | 10 | 2 | 1 | 0 | 3 | 4 | 7 | 0 | correct |
| lock-order/abba_2lock | Medium | 9 | 1 | 1 | 4 | 2 | 2 | 4 | 39 | fixed |
| lock-order/cross_module_cycle | Medium | 10 | 1 | 1 | 4 | 2 | 2 | 4 | 39 | fixed |
| lock-order/cycle_3lock | Complex | 10 | 1 | 1 | 6 | 3 | 3 | 6 | 286 | fixed |
| lock-order/partial_deadlock_bystander | Complex | 12 | 2 | 3 | 4 | 3 | 5 | 8 | 149 | fixed |
| lock-order/two_independent_cycles | Complex | 12 | 1 | 1 | 8 | 4 | 4 | 8 | 1371 | fixed |
| semaphore/acquire_twice_no_release | Simple | 7 | 1 | 1 | 3 | 2 | 1 | 3 | 35 | fixed |
| semaphore/permit_leak | Simple | 7 | 1 | 1 | 3 | 2 | 1 | 3 | 23 | fixed |
| semaphore/throttle_n_permits | Medium | 7 | 1 | 1 | 3 | 3 | 1 | 4 | 92 | correct |
| structure/finite_call_loop | Simple | 5 | 2 | 1 | 0 | 0 | 0 | 0 | 6 | correct |
| structure/nested_scope_lock_order | Medium | 7 | 3 | 1 | 3 | 3 | 2 | 5 | 41 | fixed |
| structure/scope_bound_k_workers | Complex | 7 | 3 | 1 | 3 | 3 | 1 | 4 | 116 | correct |
| structure/scope_worker_abba | Medium | 9 | 3 | 1 | 4 | 2 | 2 | 4 | 39 | fixed |
| structure/spawn_join_loop_finite | Simple | 6 | 3 | 1 | 0 | 1 | 0 | 1 | 10 | correct |
| structure/worker_with_payload | Medium | 8 | 4 | 1 | 2 | 2 | 2 | 4 | 47 | correct |

## Tier membership

### Simple (8)

- atomic-data/bounded_counter_invariant
- atomic-data/counter_overflow_safety
- channel/rendezvous_both_send
- channel/send_while_holding_mutex
- semaphore/acquire_twice_no_release
- semaphore/permit_leak
- structure/finite_call_loop
- structure/spawn_join_loop_finite

### Medium (8)

- atomic-data/atomic_lost_update
- channel/bounded_backpressure_lock_held
- lock-order/abba_2lock
- lock-order/cross_module_cycle
- semaphore/throttle_n_permits
- structure/nested_scope_lock_order
- structure/scope_worker_abba
- structure/worker_with_payload

### Complex (8)

- condvar/bare_wait_no_predicate
- condvar/lost_wakeup_notify_before_wait
- condvar/notify_one_multi_waiter_wrong_pick
- condvar/same_cv_different_locks
- lock-order/cycle_3lock
- lock-order/partial_deadlock_bystander
- lock-order/two_independent_cycles
- structure/scope_bound_k_workers

