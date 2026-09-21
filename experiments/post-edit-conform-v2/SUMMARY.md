# post-edit-conform-v2 — SUMMARY

- binary sha256: `073129de3a6d378a4e198bf712028c0c950cdf080981fb0073dfb7af5fa7ce5c`
- `no_edit` = 0 changed lines; excluded from the conform/drift denominator.

| edit | cells | no_edit | real edits | conform PASS (real) | drift-only (real) | retried | retry real | retry conform PASS | retry drift-only |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| E1 | 22 | 0 | 22 | 22 | 0 | 0 | 0 | 0 | 0 |
| E2 | 21 | 15 | 6 | 5 | 0 | 15 | 15 | 14 | 0 |
| E3 | 21 | 11 | 10 | 7 | 0 | 11 | 3 | 10 | 0 |

## Forced-edit retry (E2/E3 no-op cells)

E2 retry must add and call a `fn`; E3 retry must change sync-related code while keeping output identical.

### E2 (15 retried)

| task | arm | changed_lines | sync_calls_moved | conform | reason |
| --- | --- | --- | --- | --- | --- |
| lock-order/cycle_3lock | A3_local | 31 | 0 | PASS | None |
| lock-order/cycle_3lock | A3_whole | 33 | 0 | PASS | None |
| structure/nested_scope_lock_order | A3_local | 44 | 0 | PASS | None |
| structure/nested_scope_lock_order | A3_whole | 44 | 0 | None | None |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 28 | 0 | PASS | None |
| condvar/notify_one_multi_waiter_wrong_pick | A3_tiered | 26 | 0 | PASS | None |
| channel/bounded_backpressure_lock_held | A3_local | 32 | 0 | PASS | None |
| channel/bounded_backpressure_lock_held | A3_whole | 32 | 0 | PASS | None |
| channel/send_while_holding_mutex | A3_whole | 30 | 0 | PASS | None |
| lock-order/abba_2lock | A3_local | 33 | 0 | PASS | None |
| lock-order/abba_2lock | A3_whole | 33 | 0 | PASS | None |
| condvar/bare_wait_no_predicate | A3_whole | 31 | 0 | PASS | None |
| lock-order/partial_deadlock_bystander | A3_whole | 12 | 0 | PASS | None |
| channel/bounded_backpressure_lock_held | A3_whole | 30 | 0 | PASS | None |
| lock-order/partial_deadlock_bystander | A3_whole | 15 | 0 | PASS | None |

### E3 (11 retried)

| task | arm | changed_lines | sync_calls_moved | conform | reason |
| --- | --- | --- | --- | --- | --- |
| lock-order/cross_module_cycle | A3_local | 0 | 0 | PASS | None |
| lock-order/cycle_3lock | A3_local | 0 | 0 | PASS | None |
| lock-order/cycle_3lock | A3_whole | 9 | 0 | PASS | None |
| structure/nested_scope_lock_order | A3_local | 0 | 0 | PASS | None |
| structure/nested_scope_lock_order | A3_whole | 0 | 0 | PASS | None |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 343 | 5 | None | None |
| channel/send_while_holding_mutex | A3_whole | 0 | 0 | PASS | None |
| semaphore/acquire_twice_no_release | A3_local | 0 | 0 | PASS | None |
| semaphore/acquire_twice_no_release | A3_whole | 0 | 0 | PASS | None |
| channel/bounded_backpressure_lock_held | A3_whole | 0 | 0 | PASS | None |
| lock-order/partial_deadlock_bystander | A3_whole | 10 | 0 | PASS | None |

