# flash-repair-smoke-v3 — SUMMARY

- batch: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v3/run-20260919T065313-94177-958361`
- protocol sha256: `596203e05d9b1ca386cb8963f484439eb6ad4888d79459c1e2bb8355928d9aa3`
- binary sha256: `6d498c8a8fa2f3dde578236889dd88073fe1952353a79a22097bfe68633f26e9`
- requests used: 78 / 200
- stop reason: None

## Main table

| task | arm | accepted | round | tokens | llm_ms | tool_ms | oracle.build | oracle.behavior | oracle.miri | oracle.model | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1361 | 2272 | 870 | True | hang | detected 16 | inconclusive | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 10210 | 29212 | 778 | False | no_build | False | inconclusive | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 2 | 7612 | 14915 | 4622 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/partial_deadlock_bystander | A3_local | False | None | 4587 | 2791 | 28 | — | — | — | FAIL | False |
| lock-order/partial_deadlock_bystander | A3_whole | False | None | 16355 | 10861 | 42 | — | — | — | FAIL | False |
| lock-order/cross_module_cycle | A0_direct | True | 1 | 1000 | 1850 | 705 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/cross_module_cycle | A1_self_iter | False | None | 2478 | 4945 | 676 | False | no_build | False | inconclusive | False |
| lock-order/cross_module_cycle | A2_tools_iter_ml | True | 1 | 647 | 1280 | 1985 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/cross_module_cycle | A3_local | True | 2 | 1912 | 745 | 49 | — | — | — | PASS | False |
| lock-order/cross_module_cycle | A3_whole | True | 2 | 3580 | 1947 | 48 | — | — | — | PASS | False |
| lock-order/cycle_3lock | A0_direct | True | 1 | 1001 | 1537 | 686 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/cycle_3lock | A1_self_iter | False | None | 3695 | 4929 | 2674 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | True | 1 | 911 | 1567 | 2192 | True | terminated_ok | clean 16 | inconclusive | False |
| lock-order/cycle_3lock | A3_local | True | 2 | 2096 | 905 | 56 | — | — | — | PASS | False |
| lock-order/cycle_3lock | A3_whole | True | 3 | 8524 | 4756 | 88 | — | — | — | PASS | False |
| structure/nested_scope_lock_order | A0_direct | True | 1 | 736 | 1261 | 668 | True | terminated_ok | clean 16 | inconclusive | False |
| structure/nested_scope_lock_order | A1_self_iter | True | 2 | 1105 | 1355 | 656 | True | terminated_ok | clean 16 | inconclusive | False |
| structure/nested_scope_lock_order | A2_tools_iter_ml | True | 1 | 677 | 1004 | 2472 | True | terminated_ok | clean 16 | inconclusive | False |
| structure/nested_scope_lock_order | A3_local | True | 2 | 2054 | 1178 | 48 | — | — | — | PASS | False |
| structure/nested_scope_lock_order | A3_whole | True | 4 | 10713 | 6699 | 60 | — | — | — | PASS | False |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | False | None | 2208 | 2517 | 0 | False | no_build | False | inconclusive | False |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | False | None | 2710 | 6172 | 681 | False | no_build | False | inconclusive | False |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | True | 1 | 675 | 1205 | 2762 | True | terminated_ok | clean 16 | inconclusive | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | 2 | 2792 | 1817 | 49 | — | — | — | PASS | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | False | None | 13857 | 10480 | 40 | — | — | — | FAIL | False |
| channel/bounded_backpressure_lock_held | A0_direct | True | 1 | 707 | 1325 | 714 | True | terminated_ok | clean 16 | inconclusive | False |
| channel/bounded_backpressure_lock_held | A1_self_iter | False | None | 2755 | 5939 | 729 | False | no_build | False | inconclusive | False |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | True | 1 | 665 | 1152 | 2623 | True | terminated_ok | clean 16 | inconclusive | False |
| channel/bounded_backpressure_lock_held | A3_local | True | 2 | 2149 | 938 | 49 | — | — | — | PASS | False |
| channel/bounded_backpressure_lock_held | A3_whole | True | 4 | 11450 | 7785 | 58 | — | — | — | PASS | False |
| channel/send_while_holding_mutex | A0_direct | True | 1 | 851 | 1889 | 746 | True | terminated_ok | clean 16 | inconclusive | False |
| channel/send_while_holding_mutex | A1_self_iter | True | 3 | 1808 | 3974 | 705 | False | no_build | False | inconclusive | False |
| channel/send_while_holding_mutex | A2_tools_iter_ml | True | 1 | 828 | 1864 | 2421 | True | terminated_ok | clean 16 | inconclusive | False |
| channel/send_while_holding_mutex | A3_local | True | 2 | 1499 | 868 | 48 | — | — | — | PASS | False |
| channel/send_while_holding_mutex | A3_whole | True | 4 | 9345 | 6405 | 54 | — | — | — | PASS | False |
| semaphore/acquire_twice_no_release | A0_direct | True | 1 | 973 | 1473 | 665 | True | hang | detected 16 | inconclusive | True |
| semaphore/acquire_twice_no_release | A1_self_iter | False | None | 13642 | 42729 | 1427 | False | no_build | False | inconclusive | False |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | True | 2 | 4012 | 4439 | 4088 | True | terminated_ok | clean 16 | inconclusive | False |
| semaphore/acquire_twice_no_release | A3_local | True | 2 | 1763 | 689 | 46 | — | — | — | PASS | False |
| semaphore/acquire_twice_no_release | A3_whole | True | 2 | 3432 | 2107 | 47 | — | — | — | PASS | False |

## Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | inconclusive | mean_round | mean_tokens |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A0_direct | 8 | 7 | 0.88 | 2 | 8 | 1.00 | 1105 |
| A1_self_iter | 8 | 2 | 0.25 | 0 | 8 | 2.50 | 4800 |
| A2_tools_iter_ml | 8 | 8 | 1.00 | 0 | 8 | 1.25 | 2003 |
| A3_local | 8 | 7 | 0.88 | 0 | 0 | 2.00 | 2356 |
| A3_whole | 8 | 6 | 0.75 | 0 | 0 | 3.17 | 9657 |

## A3 decision distribution

| task | arm | decisions |
| --- | --- | --- |
| channel/bounded_backpressure_lock_held | A3_local | {'explore_fail': 1, 'accepted': 1} |
| channel/bounded_backpressure_lock_held | A3_whole | {'explore_fail': 1, 'check_invalid': 2, 'accepted': 1} |
| channel/send_while_holding_mutex | A3_local | {'explore_fail': 1, 'accepted': 1} |
| channel/send_while_holding_mutex | A3_whole | {'explore_fail': 1, 'check_invalid': 2, 'accepted': 1} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | {'explore_fail': 1, 'accepted': 1} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | {'explore_fail': 1, 'check_invalid': 1, 'check_schema_error': 2} |
| lock-order/cross_module_cycle | A3_local | {'explore_fail': 1, 'accepted': 1} |
| lock-order/cross_module_cycle | A3_whole | {'explore_fail': 1, 'accepted': 1} |
| lock-order/cycle_3lock | A3_local | {'explore_fail': 1, 'accepted': 1} |
| lock-order/cycle_3lock | A3_whole | {'explore_fail': 2, 'accepted': 1} |
| lock-order/partial_deadlock_bystander | A3_local | {'explore_fail': 1, 'stalled_local_patch': 1, 'stalled': 1} |
| lock-order/partial_deadlock_bystander | A3_whole | {'explore_fail': 1, 'check_invalid': 2, 'check_schema_error': 1} |
| semaphore/acquire_twice_no_release | A3_local | {'explore_fail': 1, 'accepted': 1} |
| semaphore/acquire_twice_no_release | A3_whole | {'explore_fail': 1, 'accepted': 1} |
| structure/nested_scope_lock_order | A3_local | {'explore_fail': 1, 'accepted': 1} |
| structure/nested_scope_lock_order | A3_whole | {'explore_fail': 1, 'check_invalid': 2, 'accepted': 1} |

`oracle.behavior`: terminated_ok / terminated_wrong_state / hang / no_output / no_build. `oracle.model` for Rust arms is `inconclusive` in this batch (deviation D-19: the batch is not gated on Rust-arm oracle completeness; extraction/expert labels are filled in later).
