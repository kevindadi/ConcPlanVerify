# Contract strength (frozen contracts, G-1)

build_families.py validates 21 buggy FAIL / all fixed PASS under the frozen contracts

| task | version | old outcome | frozen outcome | rejected by frozen contract |
| --- | --- | --- | --- | --- |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/partial_deadlock_bystander | 4 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | PASS | — |
| lock-order/cycle_3lock | 1 | PASS | PASS | — |
| lock-order/abba_2lock | 3 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| condvar/bare_wait_no_predicate | 4 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| lock-order/partial_deadlock_bystander | 3 | PASS | FAIL | preserved: main::a holds [main::a, main::b] at once; preserved: main::b holds [main::a, main::b] at once |
| channel/bounded_backpressure_lock_held | 2 | PASS | PASS | — |
| channel/bounded_backpressure_lock_held | 4 | PASS | PASS | — |
| channel/send_while_holding_mutex | 2 | PASS | PASS | — |
| channel/send_while_holding_mutex | 4 | PASS | PASS | — |
| condvar/notify_one_multi_waiter_wrong_pick | 2 | PASS | PASS | — |
| lock-order/cross_module_cycle | 2 | PASS | PASS | — |
| lock-order/cross_module_cycle | 2 | PASS | PASS | — |
| lock-order/cycle_3lock | 2 | PASS | PASS | — |
| lock-order/cycle_3lock | 3 | PASS | PASS | — |
| semaphore/acquire_twice_no_release | 2 | PASS | PASS | — |
| semaphore/acquire_twice_no_release | 2 | PASS | PASS | — |
| structure/nested_scope_lock_order | 2 | PASS | PASS | — |
| structure/nested_scope_lock_order | 4 | PASS | PASS | — |
