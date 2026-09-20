# flash-repair-smoke-v3 — A0/A1 reply reclassification (§1.3)

- batch: `run-20260919T065313-94177-958361` (offline, no requests)
- classifier: three-way (`program` / `claims_no_issue` / `other`);
  see `python/cir_workflow/arms.py:classify_reply` and the word list
  in `flash-repair-main-v1/PROTOCOL.md`.

| task | arm | round | reply.kind | old | new | new accepted |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | program | continue | continue | False |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | other | format_error | format_error | False |
| lock-order/partial_deadlock_bystander | A1_self_iter | 3 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| lock-order/partial_deadlock_bystander | A1_self_iter | 4 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| lock-order/cross_module_cycle | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| lock-order/cross_module_cycle | A1_self_iter | 1 | program | continue | continue | False |
| lock-order/cross_module_cycle | A1_self_iter | 2 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| lock-order/cross_module_cycle | A1_self_iter | 3 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| lock-order/cross_module_cycle | A1_self_iter | 4 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| lock-order/cycle_3lock | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| lock-order/cycle_3lock | A1_self_iter | 1 | program | continue | continue | False |
| lock-order/cycle_3lock | A1_self_iter | 2 | program | continue | continue | False |
| lock-order/cycle_3lock | A1_self_iter | 3 | program | continue | continue | False |
| lock-order/cycle_3lock | A1_self_iter | 4 | program | continue | continue | False |
| structure/nested_scope_lock_order | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| structure/nested_scope_lock_order | A1_self_iter | 1 | program | continue | continue | False |
| structure/nested_scope_lock_order | A1_self_iter | 2 | claims_no_issue | self_no_issues | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 2 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 3 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 4 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 1 | program | continue | continue | False |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 2 | other | format_error | format_error | False |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 3 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 4 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | program | continue | continue | False |
| channel/bounded_backpressure_lock_held | A1_self_iter | 2 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| channel/bounded_backpressure_lock_held | A1_self_iter | 3 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| channel/bounded_backpressure_lock_held | A1_self_iter | 4 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| channel/send_while_holding_mutex | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| channel/send_while_holding_mutex | A1_self_iter | 1 | program | continue | continue | False |
| channel/send_while_holding_mutex | A1_self_iter | 2 | claims_no_issue | format_error | claims_no_issue | True (r1) |
| channel/send_while_holding_mutex | A1_self_iter | 3 | claims_no_issue | self_no_issues | claims_no_issue | True (r1) |
| semaphore/acquire_twice_no_release | A0_direct | 1 | program | build_ok | build_ok | True (r1) |
| semaphore/acquire_twice_no_release | A1_self_iter | 1 | other | format_error | format_error | False |
| semaphore/acquire_twice_no_release | A1_self_iter | 2 | program | continue | continue | False |
| semaphore/acquire_twice_no_release | A1_self_iter | 3 | program | continue | continue | False |
| semaphore/acquire_twice_no_release | A1_self_iter | 4 | other | format_error | format_error | False |

## Per-cell outcome change

| task | arm | old accepted | new accepted | new accepted round |
| --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | True | 1 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | True | 1 |
| lock-order/cross_module_cycle | A0_direct | True | True | 1 |
| lock-order/cross_module_cycle | A1_self_iter | False | True | 1 |
| lock-order/cycle_3lock | A0_direct | True | True | 1 |
| lock-order/cycle_3lock | A1_self_iter | False | False | None |
| structure/nested_scope_lock_order | A0_direct | True | True | 1 |
| structure/nested_scope_lock_order | A1_self_iter | True | True | 1 |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | False | True | 1 |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | False | True | 1 |
| channel/bounded_backpressure_lock_held | A0_direct | True | True | 1 |
| channel/bounded_backpressure_lock_held | A1_self_iter | False | True | 1 |
| channel/send_while_holding_mutex | A0_direct | True | True | 1 |
| channel/send_while_holding_mutex | A1_self_iter | True | True | 1 |
| semaphore/acquire_twice_no_release | A0_direct | True | True | 1 |
| semaphore/acquire_twice_no_release | A1_self_iter | False | False | None |

## Notes

- `notify_one/A0`: `NO_ISSUES` becomes `claims_no_issue` (accept the buggy
  input, `false_accept=True`, `bug_present` by construction).
- `cross_module_cycle/A1`: rounds 2–4 are fence-free prose ('same order')
  → `claims_no_issue`; round 1 built, so the new rule accepts the round-1
  candidate instead of burning the remaining rounds.
- `send_while/A1`: round 3 `NO_ISSUES`; round 1 built, so it is accepted on
  the round-1 candidate (the old rule accepted the r3 non-built state).
- `format_error` rounds are what the new one-retry-per-reply rule targets.
  Limitation: the new harness issues the retry *within* the same round, so for
  arms with an `other` round before a claim (e.g. `notify_one/A1` r2) the
  replayed round indices after that point are indicative, not exact.
