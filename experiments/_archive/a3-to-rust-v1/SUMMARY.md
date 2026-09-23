# a3-to-rust-v1 — SUMMARY

- binary sha256: `03d343e558854d65934ee0a2ea00d522d4dca102966d8c499fb4367d37b583aa`
- accepted A3 cells: 74; unique CIRs: 23
- requests: 0/110

| cir sha | task | arm | holes | conform | traces | behavior | miri |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `b6670dfe6687` | lock-order/cross_module_cycle | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `ee219afd4c9a` | lock-order/cross_module_cycle | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `2973ea709269` | lock-order/cycle_3lock | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `012722eebc94` | lock-order/cycle_3lock | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `d8d9b2cb5b6d` | structure/nested_scope_lock_order | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `d09f72501a3f` | structure/nested_scope_lock_order | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `25af7b24bd57` | condvar/notify_one_multi_waiter_wrong_pick | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `555d3c7f7255` | condvar/notify_one_multi_waiter_wrong_pick | A3_tiered | 0 | PASS | 36 | terminated_ok | 16 |
| `6dee7d7f06cf` | channel/bounded_backpressure_lock_held | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `7dfedd6c4074` | channel/bounded_backpressure_lock_held | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `941a9d3838d9` | channel/send_while_holding_mutex | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `24fd4d02b94e` | channel/send_while_holding_mutex | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `fe3204a68c36` | semaphore/acquire_twice_no_release | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `b0a4ff3addef` | semaphore/acquire_twice_no_release | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `cd445420f22d` | lock-order/abba_2lock | A3_local | 0 | PASS | 36 | terminated_ok | 16 |
| `50f0bcebf6ac` | lock-order/abba_2lock | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `b44af96dba20` | condvar/bare_wait_no_predicate | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `65484edc58b6` | lock-order/partial_deadlock_bystander | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `bf12945c0b41` | condvar/notify_one_multi_waiter_wrong_pick | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `5b12a619f0bb` | channel/bounded_backpressure_lock_held | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `8942393e8cc5` | lock-order/partial_deadlock_bystander | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `cba39799b5c3` | channel/bounded_backpressure_lock_held | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
| `0e8e6594b4f0` | condvar/bare_wait_no_predicate | A3_whole | 0 | PASS | 36 | terminated_ok | 16 |
