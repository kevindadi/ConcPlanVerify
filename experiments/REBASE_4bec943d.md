# REBASE 4bec943d

- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- source: `/Users/kevin/local-repos/ConcIR/target/release/concir-backend`

## v3 accepted A3 CIR `explore` under BIN_MAIN

| task | arm | old verify_pass | new verify_pass | outcome |
| --- | --- | --- | --- | --- |
| lock-order/cross_module_cycle | A3_local | True | True | PASS |
| lock-order/cross_module_cycle | A3_whole | True | True | PASS |
| lock-order/cycle_3lock | A3_local | True | True | PASS |
| lock-order/cycle_3lock | A3_whole | True | True | PASS |
| structure/nested_scope_lock_order | A3_local | True | True | PASS |
| structure/nested_scope_lock_order | A3_whole | True | True | PASS |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | True | PASS |
| channel/bounded_backpressure_lock_held | A3_local | True | True | PASS |
| channel/bounded_backpressure_lock_held | A3_whole | True | True | PASS |
| channel/send_while_holding_mutex | A3_local | True | True | PASS |
| channel/send_while_holding_mutex | A3_whole | True | True | PASS |
| semaphore/acquire_twice_no_release | A3_local | True | True | PASS |
| semaphore/acquire_twice_no_release | A3_whole | True | True | PASS |

Verdict changes: **0** (none)

## conformance-v4 offline smoke under BIN_MAIN

| case | old status | new status |
| --- | --- | --- |
| abba_2lock__fixed | ready | ready |
| bounded_backpressure__fixed | ready | ready |
| cross_module_cycle__fixed | ready | ready |
| lost_wakeup__fixed | ready | ready |
| permit_leak__fixed | ready | ready |
| rendezvous_both_send__fixed | ready | ready |
| rmw-zenoh-998__buggy | review | review |
| scope_bound__correct | ready | ready |
| send_while_holding_mutex__fixed | ready | ready |

Status changes: **0** (none)
