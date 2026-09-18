# Track D — detection capability (no LLM)

- ConcIR binary sha256: `fe3d22c5a52f84b5a46d7a4665328056cec9b132f33eec9ea52c3c1158b7d12d`
- Miri combos: [{'seed': 0, 'preemption_rate': 0.01}, {'seed': 1, 'preemption_rate': 0.05}, {'seed': 2, 'preemption_rate': 0.1}, {'seed': 3, 'preemption_rate': 0.2}, {'seed': 4, 'preemption_rate': 0.5}]
- Lockbud: {'available': True, 'path': '/Users/kevin/local-repos/ConcPlanVerify/tools/lockbud/target/release/lockbud', 'commit': 'cc78cb72cb85cb80e596717339cca95ef50c8fe0', 'binary_sha256': 'a6cc62c0eb02f8df9905642dd6a3717f31baf7d3731b9e56680a309409f20c56', 'toolchain': 'nightly-2026-02-07'}

## Per task

| task | CIR buggy | CIR fixed | Miri buggy | seeds | Lockbud buggy | Miri fixed | tool errors | notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | FAIL | PASS | False | 6 | True | False | 0 | CIR detects |
| lock-order/cycle_3lock | FAIL | PASS | False | 6 | False | False | 0 | CIR detects |
| lock-order/cross_module_cycle | FAIL | None | None | 0 | None | None | 0 | CIR detects |
| lock-order/partial_deadlock_bystander | FAIL | PASS | None | 0 | None | None | 0 | CIR detects |
| condvar/lost_wakeup_notify_before_wait | FAIL | PASS | None | 0 | None | None | 0 | CIR detects |
| channel/rendezvous_both_send | FAIL | PASS | None | 0 | None | None | 0 | CIR detects |
| semaphore/permit_leak | FAIL | PASS | None | 0 | None | None | 0 | CIR detects |
| semaphore/throttle_n_permits | None | None | None | 0 | None | None | 0 |  |
| atomic-data/bounded_counter_invariant | None | None | None | 0 | None | None | 0 |  |
| structure/scope_bound_k_workers | None | None | None | 0 | None | None | 0 |  |
| boundary/unbounded_int_unknown | None | None | None | 0 | None | None | 0 |  |
| lock-order/two_independent_cycles | FAIL | None | None | 0 | None | None | 0 | CIR detects |
| condvar/same_cv_different_locks | PASS | None | None | 0 | None | None | 0 |  |
| condvar/notify_one_multi_waiter_wrong_pick | FAIL | None | None | 0 | None | None | 0 | CIR detects |
| channel/send_while_holding_mutex | FAIL | None | None | 0 | None | None | 0 | CIR detects |
| structure/finite_call_loop | None | None | None | 0 | None | None | 0 |  |
| structure/spawn_join_loop_finite | None | None | None | 0 | None | None | 0 |  |
| boundary/rwlock_unsupported | UNSUPPORTED | None | None | 0 | None | None | 0 |  |
| boundary/async_select_unsupported | UNSUPPORTED | None | None | 0 | None | None | 0 |  |
| P1 | — | — | — | — | — | — | — | skipped_legacy |
| P2 | — | — | — | — | — | — | — | skipped_legacy |
| P3 | — | — | — | — | — | — | — | skipped_legacy |
| P4 | — | — | — | — | — | — | — | skipped_legacy |
| P5 | — | — | — | — | — | — | — | skipped_legacy |
| P6 | — | — | — | — | — | — | — | skipped_legacy |
| P7 | — | — | — | — | — | — | — | skipped_legacy |
| P8 | — | — | — | — | — | — | — | skipped_legacy |
| P9 | — | — | — | — | — | — | — | skipped_legacy |
| real-cases/rmw-zenoh-998 | FAIL | PASS | None | 0 | None | None | 0 | CIR detects |
| real-cases/dashmap-369 | UNSUPPORTED | None | None | 0 | None | None | 0 |  |

## Caveats

- Miri is dynamic and schedule-dependent: a miss is not proof of absence, and is not evidence of safety.
- The deadlock fixtures are lock-order bugs that only manifest on some interleavings; each Miri seed runs one schedule, so a miss here is expected rather than surprising.
- The ConcIR arm consumes human-written CIR, not Rust source; the comparison is capability-level, not same-input.
- Lockbud is a static RUSTC_WRAPPER pinned to nightly-2026-02-07; its reports are 'possibly' over-approximations and cover double-lock / conflicting-lock-order / condvar misuse only.
