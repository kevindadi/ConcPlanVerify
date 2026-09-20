# REBASE 03d343e5

- binary sha256: `03d343e558854d65934ee0a2ea00d522d4dca102966d8c499fb4367d37b583aa`
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
