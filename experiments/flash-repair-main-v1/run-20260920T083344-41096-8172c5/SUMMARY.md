# flash-repair-main-v1 — SUMMARY

- batch: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5`
- protocol sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- requests used: 203 / 300
- stop reason: None

## rep-0

| task | arm | accepted | round | tokens | behavior | false_accept |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1460 | hang | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4859 | None | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 2 | 7957 | terminated_ok | False |
| lock-order/partial_deadlock_bystander | A3_local | False | None | 8236 | None | False |
| lock-order/partial_deadlock_bystander | A3_whole | False | None | 15440 | None | False |
| lock-order/cross_module_cycle | A0_direct | True | 1 | 979 | terminated_ok | False |
| lock-order/cross_module_cycle | A1_self_iter | True | 1 | 1269 | terminated_ok | False |
| lock-order/cross_module_cycle | A2_tools_iter_ml | True | 1 | 693 | terminated_ok | False |
| lock-order/cross_module_cycle | A3_local | True | 2 | 1912 | None | False |
| lock-order/cross_module_cycle | A3_whole | True | 2 | 3580 | None | False |
| lock-order/cycle_3lock | A0_direct | True | 1 | 1001 | terminated_ok | False |
| lock-order/cycle_3lock | A1_self_iter | False | None | 3695 | terminated_ok | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | True | 1 | 911 | terminated_ok | False |
| lock-order/cycle_3lock | A3_local | True | 2 | 2109 | None | False |
| lock-order/cycle_3lock | A3_whole | True | 3 | 8524 | None | False |
| structure/nested_scope_lock_order | A0_direct | True | 1 | 736 | terminated_ok | False |
| structure/nested_scope_lock_order | A1_self_iter | True | 1 | 1105 | terminated_ok | False |
| structure/nested_scope_lock_order | A2_tools_iter_ml | True | 1 | 646 | terminated_ok | False |
| structure/nested_scope_lock_order | A3_local | True | 2 | 2054 | None | False |
| structure/nested_scope_lock_order | A3_whole | True | 4 | 10713 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | True | 1 | 552 | hang | True |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | True | 1 | 1318 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | True | 1 | 675 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | 2 | 2822 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | False | None | 12570 | None | False |
| channel/bounded_backpressure_lock_held | A0_direct | True | 1 | 707 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A1_self_iter | True | 1 | 1253 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | True | 1 | 665 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A3_local | True | 2 | 2165 | None | False |
| channel/bounded_backpressure_lock_held | A3_whole | True | 4 | 10908 | None | False |
| channel/send_while_holding_mutex | A0_direct | True | 1 | 874 | terminated_ok | False |
| channel/send_while_holding_mutex | A1_self_iter | False | None | 407 | None | False |
| channel/send_while_holding_mutex | A2_tools_iter_ml | True | 1 | 749 | terminated_ok | False |
| channel/send_while_holding_mutex | A3_local | True | 2 | 1481 | None | False |
| channel/send_while_holding_mutex | A3_whole | True | 3 | 5983 | None | False |
| semaphore/acquire_twice_no_release | A0_direct | True | 1 | 973 | hang | True |
| semaphore/acquire_twice_no_release | A1_self_iter | True | 1 | 6959 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | True | 2 | 4102 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A3_local | True | 2 | 1752 | None | False |
| semaphore/acquire_twice_no_release | A3_whole | True | 2 | 3432 | None | False |

## rep-1

| task | arm | accepted | round | tokens | behavior | false_accept |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1361 | hang | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | True | 1 | 15832 | terminated_ok | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | False | None | 12528 | no_build | False |
| lock-order/partial_deadlock_bystander | A3_local | False | None | 7305 | None | False |
| lock-order/partial_deadlock_bystander | A3_whole | True | 4 | 15270 | None | False |
| lock-order/cross_module_cycle | A0_direct | True | 1 | 737 | terminated_ok | False |
| lock-order/cross_module_cycle | A1_self_iter | True | 1 | 1203 | terminated_ok | False |
| lock-order/cross_module_cycle | A2_tools_iter_ml | True | 1 | 693 | terminated_ok | False |
| lock-order/cross_module_cycle | A3_local | True | 2 | 1912 | None | False |
| lock-order/cross_module_cycle | A3_whole | True | 2 | 3580 | None | False |
| lock-order/cycle_3lock | A0_direct | True | 1 | 1001 | terminated_ok | False |
| lock-order/cycle_3lock | A1_self_iter | False | None | 3695 | terminated_ok | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | True | 1 | 911 | terminated_ok | False |
| lock-order/cycle_3lock | A3_local | True | 2 | 2096 | None | False |
| lock-order/cycle_3lock | A3_whole | True | 3 | 8524 | None | False |
| structure/nested_scope_lock_order | A0_direct | True | 1 | 736 | terminated_ok | False |
| structure/nested_scope_lock_order | A1_self_iter | True | 1 | 1105 | terminated_ok | False |
| structure/nested_scope_lock_order | A2_tools_iter_ml | True | 1 | 646 | terminated_ok | False |
| structure/nested_scope_lock_order | A3_local | True | 2 | 2054 | None | False |
| structure/nested_scope_lock_order | A3_whole | True | 4 | 10713 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | True | 1 | 552 | hang | True |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | True | 1 | 1331 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | True | 1 | 675 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | 2 | 2851 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | True | 4 | 13930 | None | False |
| channel/bounded_backpressure_lock_held | A0_direct | True | 1 | 707 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A1_self_iter | True | 1 | 1332 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | True | 1 | 665 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A3_local | True | 2 | 2165 | None | False |
| channel/bounded_backpressure_lock_held | A3_whole | True | 4 | 11438 | None | False |
| channel/send_while_holding_mutex | A0_direct | True | 1 | 852 | terminated_ok | False |
| channel/send_while_holding_mutex | A1_self_iter | False | None | 407 | None | False |
| channel/send_while_holding_mutex | A2_tools_iter_ml | True | 1 | 756 | terminated_ok | False |
| channel/send_while_holding_mutex | A3_local | True | 2 | 1519 | None | False |
| channel/send_while_holding_mutex | A3_whole | True | 3 | 5983 | None | False |
| semaphore/acquire_twice_no_release | A0_direct | True | 1 | 686 | hang | True |
| semaphore/acquire_twice_no_release | A1_self_iter | True | 1 | 4098 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | True | 2 | 4033 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A3_local | True | 2 | 1763 | None | False |
| semaphore/acquire_twice_no_release | A3_whole | True | 2 | 3432 | None | False |

## rep-2

| task | arm | accepted | round | tokens | behavior | false_accept |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1429 | hang | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | True | 1 | 9316 | terminated_ok | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | False | None | 15368 | terminated_ok | False |
| lock-order/partial_deadlock_bystander | A3_local | False | None | 7561 | None | False |
| lock-order/partial_deadlock_bystander | A3_whole | True | 4 | 15240 | None | False |
| lock-order/cross_module_cycle | A0_direct | True | 1 | 979 | terminated_ok | False |
| lock-order/cross_module_cycle | A1_self_iter | True | 1 | 1269 | terminated_ok | False |
| lock-order/cross_module_cycle | A2_tools_iter_ml | True | 1 | 693 | terminated_ok | False |
| lock-order/cross_module_cycle | A3_local | True | 2 | 1912 | None | False |
| lock-order/cross_module_cycle | A3_whole | True | 2 | 3580 | None | False |
| lock-order/cycle_3lock | A0_direct | True | 1 | 1001 | terminated_ok | False |
| lock-order/cycle_3lock | A1_self_iter | False | None | 3806 | terminated_ok | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | True | 1 | 911 | terminated_ok | False |
| lock-order/cycle_3lock | A3_local | True | 2 | 2096 | None | False |
| lock-order/cycle_3lock | A3_whole | False | None | 12128 | None | False |
| structure/nested_scope_lock_order | A0_direct | True | 1 | 736 | terminated_ok | False |
| structure/nested_scope_lock_order | A1_self_iter | True | 1 | 1105 | terminated_ok | False |
| structure/nested_scope_lock_order | A2_tools_iter_ml | True | 1 | 646 | terminated_ok | False |
| structure/nested_scope_lock_order | A3_local | True | 2 | 2054 | None | False |
| structure/nested_scope_lock_order | A3_whole | False | None | 10788 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | True | 1 | 765 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | True | 1 | 1352 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | True | 1 | 675 | terminated_ok | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | 2 | 2851 | None | False |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | True | 4 | 13930 | None | False |
| channel/bounded_backpressure_lock_held | A0_direct | True | 1 | 707 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A1_self_iter | True | 1 | 1983 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | True | 1 | 680 | terminated_ok | False |
| channel/bounded_backpressure_lock_held | A3_local | True | 2 | 2165 | None | False |
| channel/bounded_backpressure_lock_held | A3_whole | True | 4 | 11036 | None | False |
| channel/send_while_holding_mutex | A0_direct | True | 1 | 852 | terminated_ok | False |
| channel/send_while_holding_mutex | A1_self_iter | False | None | 407 | None | False |
| channel/send_while_holding_mutex | A2_tools_iter_ml | True | 1 | 776 | terminated_ok | False |
| channel/send_while_holding_mutex | A3_local | True | 2 | 1519 | None | False |
| channel/send_while_holding_mutex | A3_whole | True | 3 | 5983 | None | False |
| semaphore/acquire_twice_no_release | A0_direct | True | 1 | 973 | hang | True |
| semaphore/acquire_twice_no_release | A1_self_iter | True | 1 | 14189 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | True | 2 | 4452 | terminated_ok | False |
| semaphore/acquire_twice_no_release | A3_local | True | 2 | 1763 | None | False |
| semaphore/acquire_twice_no_release | A3_whole | True | 2 | 3432 | None | False |

## Merged (k/n over reps)

| task | arm | accepted | round | tokens | false_accept |
| --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 3/3 | 1.00 | 1417 | 3/3 |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2/3 | 1.00 | 10002 | 0/3 |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 1/3 | 2.00 | 11951 | 0/3 |
| lock-order/partial_deadlock_bystander | A3_local | 0/3 | 0.00 | 7701 | 0/3 |
| lock-order/partial_deadlock_bystander | A3_whole | 2/3 | 4.00 | 15317 | 0/3 |
| lock-order/cross_module_cycle | A0_direct | 3/3 | 1.00 | 898 | 0/3 |
| lock-order/cross_module_cycle | A1_self_iter | 3/3 | 1.00 | 1247 | 0/3 |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 3/3 | 1.00 | 693 | 0/3 |
| lock-order/cross_module_cycle | A3_local | 3/3 | 2.00 | 1912 | 0/3 |
| lock-order/cross_module_cycle | A3_whole | 3/3 | 2.00 | 3580 | 0/3 |
| lock-order/cycle_3lock | A0_direct | 3/3 | 1.00 | 1001 | 0/3 |
| lock-order/cycle_3lock | A1_self_iter | 0/3 | 0.00 | 3732 | 0/3 |
| lock-order/cycle_3lock | A2_tools_iter_ml | 3/3 | 1.00 | 911 | 0/3 |
| lock-order/cycle_3lock | A3_local | 3/3 | 2.00 | 2100 | 0/3 |
| lock-order/cycle_3lock | A3_whole | 2/3 | 3.00 | 9725 | 0/3 |
| structure/nested_scope_lock_order | A0_direct | 3/3 | 1.00 | 736 | 0/3 |
| structure/nested_scope_lock_order | A1_self_iter | 3/3 | 1.00 | 1105 | 0/3 |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 3/3 | 1.00 | 646 | 0/3 |
| structure/nested_scope_lock_order | A3_local | 3/3 | 2.00 | 2054 | 0/3 |
| structure/nested_scope_lock_order | A3_whole | 2/3 | 4.00 | 10738 | 0/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 3/3 | 1.00 | 623 | 2/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 3/3 | 1.00 | 1334 | 0/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 3/3 | 1.00 | 675 | 0/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 3/3 | 2.00 | 2841 | 0/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | 2/3 | 4.00 | 13477 | 0/3 |
| channel/bounded_backpressure_lock_held | A0_direct | 3/3 | 1.00 | 707 | 0/3 |
| channel/bounded_backpressure_lock_held | A1_self_iter | 3/3 | 1.00 | 1523 | 0/3 |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 3/3 | 1.00 | 670 | 0/3 |
| channel/bounded_backpressure_lock_held | A3_local | 3/3 | 2.00 | 2165 | 0/3 |
| channel/bounded_backpressure_lock_held | A3_whole | 3/3 | 4.00 | 11127 | 0/3 |
| channel/send_while_holding_mutex | A0_direct | 3/3 | 1.00 | 859 | 0/3 |
| channel/send_while_holding_mutex | A1_self_iter | 0/3 | 0.00 | 407 | 0/3 |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 3/3 | 1.00 | 760 | 0/3 |
| channel/send_while_holding_mutex | A3_local | 3/3 | 2.00 | 1506 | 0/3 |
| channel/send_while_holding_mutex | A3_whole | 3/3 | 3.00 | 5983 | 0/3 |
| semaphore/acquire_twice_no_release | A0_direct | 3/3 | 1.00 | 877 | 3/3 |
| semaphore/acquire_twice_no_release | A1_self_iter | 3/3 | 1.00 | 8415 | 0/3 |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 3/3 | 2.00 | 4196 | 0/3 |
| semaphore/acquire_twice_no_release | A3_local | 3/3 | 2.00 | 1759 | 0/3 |
| semaphore/acquire_twice_no_release | A3_whole | 3/3 | 2.00 | 3432 | 0/3 |

`false_accept` = accepted AND bug_present, where bug_present follows the §2.1 rule (behavior=hang, model FAIL, expert=yes, or by_construction). `not_run` cells appear in the per-rep tables' JSON.
