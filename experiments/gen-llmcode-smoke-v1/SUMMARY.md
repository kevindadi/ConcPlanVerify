# gen-llmcode-smoke-v1 — LLM code from verified CIR (§1.3)

LLM generates Rust from the 45 accepted CIRs of `flash-gen-main-v1`; instrument v2 -> build -> `conform --op-resource` -> `monitor` -> behavior, with conform/monitor feedback (K_code=3).

- cells: 45 (not_run 0), requests 107
- build rate: 45/45
- conform PASS rate: **15/45**
- accepted (build ∧ conform ∧ monitor ∧ behavior): 15/45
- accepted-with-proof: 15/45
- of the 23 CIRs the tool codegen could not build: 8/23 now pass conform

| task | accepted | rounds | conform_ok | monitor | codegen_err_before | decisions |
| --- | --- | --- | --- | --- | --- | --- |
| atomic-data/bounded_counter_invariant | False | 3 | False | None | True | {'post_verify_fail': 1, 'build_failed': 2} |
| atomic-data/counter_overflow_safety | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| channel/bounded_backpressure_lock_held | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| channel/send_while_holding_mutex | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/bare_wait_no_predicate | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/lost_wakeup_notify_before_wait | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/same_cv_different_locks | False | 3 | False | ok | False | {'build_failed': 2, 'post_verify_fail': 1} |
| lock-order/abba_2lock | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| lock-order/cycle_3lock | True | 1 | True | ok | True | {'accepted': 1} |
| lock-order/two_independent_cycles | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| semaphore/acquire_twice_no_release | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| semaphore/permit_leak | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| structure/finite_call_loop | True | 1 | True | ok | False | {'accepted': 1} |
| structure/nested_scope_lock_order | True | 1 | True | ok | True | {'accepted': 1} |
| structure/spawn_join_loop_finite | True | 1 | True | ok | True | {'accepted': 1} |
| structure/worker_with_payload | True | 2 | True | ok | False | {'post_verify_fail': 1, 'accepted': 1} |
| atomic-data/bounded_counter_invariant | False | 3 | False | ok | True | {'build_failed': 2, 'post_verify_fail': 1} |
| atomic-data/counter_overflow_safety | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| channel/bounded_backpressure_lock_held | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| channel/send_while_holding_mutex | False | 3 | False | ok | False | {'build_failed': 1, 'post_verify_fail': 2} |
| condvar/bare_wait_no_predicate | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/lost_wakeup_notify_before_wait | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/same_cv_different_locks | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| lock-order/abba_2lock | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| lock-order/cycle_3lock | True | 1 | True | ok | False | {'accepted': 1} |
| lock-order/two_independent_cycles | True | 1 | True | ok | True | {'accepted': 1} |
| semaphore/acquire_twice_no_release | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| semaphore/permit_leak | False | 3 | False | ok | True | {'build_failed': 1, 'post_verify_fail': 2} |
| structure/finite_call_loop | True | 1 | True | ok | False | {'accepted': 1} |
| structure/spawn_join_loop_finite | True | 1 | True | ok | True | {'accepted': 1} |
| structure/worker_with_payload | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| atomic-data/bounded_counter_invariant | True | 1 | True | ok | True | {'accepted': 1} |
| atomic-data/counter_overflow_safety | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| channel/bounded_backpressure_lock_held | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| channel/send_while_holding_mutex | False | 3 | False | ok | False | {'build_failed': 1, 'post_verify_fail': 2} |
| condvar/bare_wait_no_predicate | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| condvar/lost_wakeup_notify_before_wait | False | 3 | False | ok | False | {'post_verify_fail': 3} |
| lock-order/abba_2lock | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| lock-order/cycle_3lock | True | 1 | True | ok | False | {'accepted': 1} |
| lock-order/two_independent_cycles | True | 1 | True | ok | True | {'accepted': 1} |
| semaphore/acquire_twice_no_release | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| semaphore/permit_leak | False | 3 | False | ok | True | {'post_verify_fail': 3} |
| structure/finite_call_loop | True | 1 | True | ok | False | {'accepted': 1} |
| structure/spawn_join_loop_finite | True | 1 | True | ok | True | {'accepted': 1} |
| structure/worker_with_payload | True | 2 | True | ok | False | {'post_verify_fail': 1, 'accepted': 1} |
