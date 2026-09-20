# RESULTS — ConcIR repair/extraction benchmark

> **Generated file — do not edit by hand.** Regenerate with:
> ```
> python -m cir_workflow results --out /tmp results --batch experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5 --expert experiments/flash-repair-main-v1/expert-labels/EXPERT_LABELS.json --extraction experiments/extraction-v5 --trackd experiments/detection-v3/TRACKD.json --scale experiments/scale-v2/SCALE.json --output experiments/RESULTS.md
> ```

## Provenance

- batch: `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5`
- expert: `experiments/flash-repair-main-v1/expert-labels/EXPERT_LABELS.json`
- extraction: `experiments/extraction-v5`
- track D: `experiments/detection-v3/TRACKD.json`
- scale: `experiments/scale-v2/SCALE.json`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- protocol[0] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`
- protocol[1] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`
- protocol[2] sha256: `b869a532588c83b2520f4c8df40e4be36c848789e9b7714301b1a22f9159e7f0`

## 1. Main table

| task | arm | accepted | round | tokens | llm_ms | tool_ms | verify_ms | false_accept | build | behavior | miri | conform | expert | extract |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 3/3 | 1 | 1417±99 | 2285±496 | 833±324 | — | 3/3 | True 3/3 | hang 3/3 | detected 2/3 · clean 1/3 | — | yes 3/3 | — 3/3 |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2/3 | 1 | 12574±6516 | 42330±26791 | 902±341 | — | 1/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | — | yes 2/3 · no 1/3 | — 2/2 |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 1/3 | 2 | 7957 | 16521 | 43901 | — | 0/3 | True 2/3 · False 1/3 | terminated_ok 2/3 · no_build 1/3 | clean 3/3 | — | no 3/3 | — 1/1 |
| lock-order/partial_deadlock_bystander | A3_local | 0/3 | — | — | — | — | 25±11 | 0/3 | — | — | — | — | — | — |
| lock-order/partial_deadlock_bystander | A3_whole | 2/3 | 4 | 15255±30 | 389630±760819 | 97±2 | 34±17 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | no 3/3 | — |
| lock-order/partial_deadlock_bystander | A3_tiered | 0/3 | — | — | — | — | 19±7 | 0/3 | — | — | — | — | — | — |
| lock-order/cross_module_cycle | A0_direct | 3/3 | 1 | 898±242 | 1937±88 | 911±433 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A1_self_iter | 3/3 | 1 | 1247±66 | 3333±1910 | 699±29 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 3/3 | 1 | 693 | 1709±1170 | 2188±469 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A3_local | 3/3 | 2 | 1912 | 1228±318 | 55±13 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| lock-order/cross_module_cycle | A3_whole | 3/3 | 2 | 3580 | 3171±2607 | 59±15 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| lock-order/cross_module_cycle | A3_tiered | 3/3 | 2 | 1912 | 1033±360 | 51±12 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| lock-order/cycle_3lock | A0_direct | 3/3 | 1 | 1001 | 1720±215 | 695±25 | — | 3/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | yes 3/3 | — 2/3 · INVALID 1/3 |
| lock-order/cycle_3lock | A1_self_iter | 0/3 | — | — | — | — | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | — | — |
| lock-order/cycle_3lock | A2_tools_iter_ml | 3/3 | 1 | 911 | 1810±1051 | 2190±21 | — | 3/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | yes 3/3 | INVALID 3/3 |
| lock-order/cycle_3lock | A3_local | 3/3 | 2 | 2100±13 | 1211±359 | 67±14 | 23 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| lock-order/cycle_3lock | A3_whole | 2/3 | 3 | 8524 | 5739±1052 | 96±19 | 22±1 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | no 3/3 | — |
| lock-order/cycle_3lock | A3_tiered | 3/3 | 2 | 2096 | 1116±181 | 61±14 | 22±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| structure/nested_scope_lock_order | A0_direct | 3/3 | 1 | 736 | 1407±487 | 717±107 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A1_self_iter | 3/3 | 1 | 1105 | 2167±1067 | 759±44 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 3/3 | 1 | 646 | 1558±859 | 2291±394 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A3_local | 3/3 | 2 | 2054 | 1213±379 | 57±15 | 18 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| structure/nested_scope_lock_order | A3_whole | 2/3 | 4 | 10713 | 8348±1931 | 67±22 | 13±14 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | no 3/3 | — |
| structure/nested_scope_lock_order | A3_tiered | 3/3 | 2 | 2054 | 1077±433 | 51±8 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 3/3 | 1 | 623±213 | 1309±1119 | 241±724 | — | 2/3 | True 3/3 | hang 2/3 · terminated_ok 1/3 | detected 2/3 · clean 1/3 | — | yes 2/3 · no 1/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 3/3 | 1 | 1334±34 | 3100±1394 | 762±127 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 3/3 | 1 | 675 | 6829±16936 | 2575±684 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 3/3 | 2 | 2841±29 | 2142±374 | 63±29 | 19±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | 2/3 | 4 | 13930 | 12192±2559 | 70±2 | 20±24 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | no 3/3 | — |
| condvar/notify_one_multi_waiter_wrong_pick | A3_tiered | 3/3 | 2 | 2851 | 1617±117 | 53±12 | 19±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| channel/bounded_backpressure_lock_held | A0_direct | 3/3 | 1 | 707 | 1486±732 | 744±23 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A1_self_iter | 3/3 | 1 | 1523±730 | 4045±1730 | 773±63 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 3/3 | 1 | 670±15 | 1657±953 | 2742±310 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A3_local | 3/3 | 2 | 2165 | 1435±772 | 53±14 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| channel/bounded_backpressure_lock_held | A3_whole | 3/3 | 4 | 11127±530 | 7991±2005 | 71±27 | 19±5 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| channel/bounded_backpressure_lock_held | A3_tiered | 3/3 | 2 | 2165 | 1032±238 | 49±8 | 18±5 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| channel/send_while_holding_mutex | A0_direct | 3/3 | 1 | 859±22 | 1929±337 | 759±57 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| channel/send_while_holding_mutex | A1_self_iter | 0/3 | — | — | — | — | — | 0/3 | — | — | — | — | — | — |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 3/3 | 1 | 760±27 | 1929±351 | 2526±373 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| channel/send_while_holding_mutex | A3_local | 3/3 | 2 | 1506±38 | 1167±333 | 53±13 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| channel/send_while_holding_mutex | A3_whole | 3/3 | 3 | 5983 | 4272±332 | 58±20 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| channel/send_while_holding_mutex | A3_tiered | 3/3 | 2 | 1519 | 943±210 | 47±5 | 17 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| semaphore/acquire_twice_no_release | A0_direct | 3/3 | 1 | 877±287 | 1492±984 | 452±694 | — | 3/3 | True 3/3 | hang 3/3 | detected 3/3 | — | no 2/3 · yes 1/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A1_self_iter | 3/3 | 1 | 8415±10091 | 26215±30739 | 780±136 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 3/3 | 2 | 4196±419 | 5364±298 | 4057±105 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A3_local | 3/3 | 2 | 1759±11 | 1268±383 | 53±16 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| semaphore/acquire_twice_no_release | A3_whole | 3/3 | 2 | 3432 | 2434±493 | 53±14 | 19±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| semaphore/acquire_twice_no_release | A3_tiered | 3/3 | 2 | 1763 | 927±201 | 47±3 | 17±1 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |

`accepted`/`false_accept` are k/n over repeats; `round`/`tokens`/`*_ms` are mean±range over accepted cells; oracle columns are per-rep counts. `verify_ms` is the ConcIR `explore`/`conform` time. **A0's `accepted` only means it produced a parseable program** (a single round with no rejection); it is not a correctness claim. `extract` is the validated extraction verdict (`INVALID` = contract could not be evaluated).

### Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | conform_pass_rate | escalation_rate | tokens/correct_accept | by source |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A0_direct | 24 | 24 | 1.00 | 11 | — | — | 548 | {'behavior': 8, 'expert': 9, 'by_construction': 3} |
| A1_self_iter | 24 | 17 | 0.71 | 1 | — | — | 1637 | {'expert': 1} |
| A2_tools_iter_ml | 24 | 22 | 0.92 | 3 | — | — | 869 | {'expert': 3} |
| A3_local | 24 | 21 | 0.88 | 0 | 21/21 = 1.00 | — | 683 | — |
| A3_whole | 24 | 20 | 0.83 | 0 | 20/20 = 1.00 | — | 3627 | — |
| A3_tiered | 24 | 21 | 0.88 | 0 | 21/21 = 1.00 | 3/24 = 0.12 | 684 | — |

`tokens/correct_accept = Σ tokens / (accepted − false_accept)`; `∞` when the denominator is 0. `conform_pass_rate` is over A3 cells with a conform verdict.

### A3 decision distribution

| task | arm | decisions |
| --- | --- | --- |
| lock-order/partial_deadlock_bystander | A3_local | {'explore_fail': 8, 'stalled_local_patch': 3, 'stalled': 1} |
| lock-order/partial_deadlock_bystander | A3_whole | {'explore_fail': 3, 'check_invalid': 6, 'check_schema_error': 1, 'accepted': 2} |
| lock-order/cross_module_cycle | A3_local | {'explore_fail': 3, 'accepted': 3} |
| lock-order/cross_module_cycle | A3_whole | {'explore_fail': 3, 'accepted': 3} |
| lock-order/cycle_3lock | A3_local | {'explore_fail': 3, 'accepted': 3} |
| lock-order/cycle_3lock | A3_whole | {'explore_fail': 6, 'accepted': 2, 'stalled_local_patch': 1, 'stalled': 1} |
| structure/nested_scope_lock_order | A3_local | {'explore_fail': 3, 'accepted': 3} |
| structure/nested_scope_lock_order | A3_whole | {'explore_fail': 3, 'check_invalid': 6, 'accepted': 2, 'check_schema_error': 1} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | {'explore_fail': 3, 'accepted': 3} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | {'explore_fail': 3, 'check_invalid': 5, 'check_schema_error': 1, 'stalled_local_patch': 1, 'accepted': 2} |
| channel/bounded_backpressure_lock_held | A3_local | {'explore_fail': 3, 'accepted': 3} |
| channel/bounded_backpressure_lock_held | A3_whole | {'explore_fail': 3, 'check_invalid': 6, 'accepted': 3} |
| channel/send_while_holding_mutex | A3_local | {'explore_fail': 3, 'accepted': 3} |
| channel/send_while_holding_mutex | A3_whole | {'explore_fail': 3, 'check_invalid': 3, 'accepted': 3} |
| semaphore/acquire_twice_no_release | A3_local | {'explore_fail': 3, 'accepted': 3} |
| semaphore/acquire_twice_no_release | A3_whole | {'explore_fail': 3, 'accepted': 3} |

### Expert labels

| task | arm | bug_present | design_preserved | design_loss | auto | agree |
| --- | --- | --- | --- | --- | --- | --- |
| channel/bounded_backpressure_lock_held | A0_direct | no | yes | False | False | True |
| channel/bounded_backpressure_lock_held | A1_self_iter | no | yes | False | False | True |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | no | yes | False | False | True |
| channel/bounded_backpressure_lock_held | A3_local | no | no | True | False | True |
| channel/bounded_backpressure_lock_held | A3_whole | no | no | True | False | True |
| channel/send_while_holding_mutex | A0_direct | no | yes | False | False | True |
| channel/send_while_holding_mutex | A2_tools_iter_ml | no | yes | False | False | True |
| channel/send_while_holding_mutex | A3_local | no | no | True | False | True |
| channel/send_while_holding_mutex | A3_whole | no | no | True | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | yes | yes | False | True | True |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | no | yes | False | False | True |
| lock-order/cross_module_cycle | A0_direct | no | no | True | False | True |
| lock-order/cross_module_cycle | A1_self_iter | no | yes | False | False | True |
| lock-order/cross_module_cycle | A2_tools_iter_ml | no | yes | False | False | True |
| lock-order/cross_module_cycle | A3_local | no | yes | False | False | True |
| lock-order/cross_module_cycle | A3_whole | no | yes | False | False | True |
| lock-order/cycle_3lock | A0_direct | yes | yes | False | True | True |
| lock-order/cycle_3lock | A2_tools_iter_ml | yes | yes | False | True | True |
| lock-order/cycle_3lock | A3_local | no | yes | False | False | True |
| lock-order/cycle_3lock | A3_whole | no | yes | False | False | True |
| lock-order/partial_deadlock_bystander | A0_direct | yes | yes | False | True | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | yes | no | True | True | True |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | no | yes | False | False | True |
| lock-order/partial_deadlock_bystander | A3_whole | no | yes | False | False | True |
| semaphore/acquire_twice_no_release | A0_direct | yes | yes | False | True | True |
| semaphore/acquire_twice_no_release | A1_self_iter | no | yes | False | False | True |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | no | yes | False | False | True |
| semaphore/acquire_twice_no_release | A3_local | no | yes | False | False | True |
| semaphore/acquire_twice_no_release | A3_whole | no | yes | False | False | True |
| structure/nested_scope_lock_order | A0_direct | no | yes | False | False | True |
| structure/nested_scope_lock_order | A1_self_iter | no | yes | False | False | True |
| structure/nested_scope_lock_order | A2_tools_iter_ml | no | yes | False | False | True |
| structure/nested_scope_lock_order | A3_local | no | yes | False | False | True |
| structure/nested_scope_lock_order | A3_whole | no | yes | False | False | True |

Agreement with the automatic oracle: **37/37 = 1.000** (unsure 0); `design_loss` (accepted ∧ design_preserved=no): **6** {'A0_direct': 2, 'A3_local': 6, 'A3_whole': 6, 'A1_self_iter': 1}.

#### Expert labels (per candidate)

| sha256 | task | arm | kind | bug_present | design_preserved | cells |
| --- | --- | --- | --- | --- | --- | --- |
| `13d1b761aaca` | channel/bounded_backpressure_lock_held | A0_direct | rust | no | yes | 6 |
| `1c28ef235a06` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes | 2 |
| `98eed6be2b32` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes | 1 |
| `6c87085c602a` | channel/bounded_backpressure_lock_held | A3_local | a3-rust | no | no | 3 |
| `7f182a8fd554` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | 1 |
| `e443cf0e3e70` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | 1 |
| `6d7602ab8549` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | 1 |
| `8f09cd79b48c` | channel/send_while_holding_mutex | A0_direct | rust | no | yes | 1 |
| `b607a97d0ac7` | channel/send_while_holding_mutex | A0_direct | rust | no | yes | 2 |
| `05b2f4c77462` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | 1 |
| `1bb2c14dc515` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | 1 |
| `ae1e465a6fbb` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | 1 |
| `aa0668f905d8` | channel/send_while_holding_mutex | A3_local | a3-rust | no | no | 3 |
| `459df3234540` | channel/send_while_holding_mutex | A3_whole | a3-rust | no | no | 3 |
| `daa46ba4c2ba` | condvar/notify_one_multi_waiter_wrong_pick | A0_direct | rust | yes | yes | 2 |
| `0e3cdb53c009` | condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | rust | no | yes | 7 |
| `7de135daa5ad` | condvar/notify_one_multi_waiter_wrong_pick | A3_local | a3-rust | no | yes | 1 |
| `c6e9a78ee029` | condvar/notify_one_multi_waiter_wrong_pick | A3_local | a3-rust | no | yes | 2 |
| `be85c5f756ec` | condvar/notify_one_multi_waiter_wrong_pick | A3_whole | a3-rust | no | yes | 2 |
| `05fcb391ea81` | lock-order/cross_module_cycle | A0_direct | rust | no | no | 2 |
| `9bab819deee2` | lock-order/cross_module_cycle | A0_direct | rust | no | yes | 1 |
| `1fc6e7c5ea43` | lock-order/cross_module_cycle | A1_self_iter | rust | no | yes | 3 |
| `a7983b7f5fbe` | lock-order/cross_module_cycle | A2_tools_iter_ml | rust | no | yes | 3 |
| `e7adea20e72f` | lock-order/cross_module_cycle | A3_local | a3-rust | no | yes | 3 |
| `34835028c1d2` | lock-order/cross_module_cycle | A3_whole | a3-rust | no | yes | 3 |
| `c7d11e854196` | lock-order/cycle_3lock | A0_direct | rust | yes | yes | 6 |
| `970be001ab1e` | lock-order/cycle_3lock | A3_local | a3-rust | no | yes | 5 |
| `d9c02a69c650` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | 1 |
| `a4503ffe4461` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | 1 |
| `574a7804dc4d` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | 1 |
| `1ff1454288bf` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | yes | yes | 1 |
| `a08bd1a020fa` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | no | no | 1 |
| `fbeb211c0ac0` | lock-order/partial_deadlock_bystander | A2_tools_iter_ml | rust | no | yes | 1 |
| `f4db34f1644d` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes | 1 |
| `0951c33394bd` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes | 1 |
| `f42afd77e7a9` | semaphore/acquire_twice_no_release | A0_direct | rust | no | yes | 2 |
| `691d644c986c` | semaphore/acquire_twice_no_release | A0_direct | rust | yes | yes | 1 |
| `8f59f2055f41` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | 1 |
| `c42fa5ece43f` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | 1 |
| `b6f7d6651e0e` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | 1 |
| `12cfc7f3617e` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | 1 |
| `d463f5f9f603` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | 1 |
| `0501dcb3a49d` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | 1 |
| `1901bf15e784` | semaphore/acquire_twice_no_release | A3_local | a3-rust | no | yes | 3 |
| `84f0ae3ead6e` | semaphore/acquire_twice_no_release | A3_whole | a3-rust | no | yes | 3 |
| `4e6e1cdc7910` | structure/nested_scope_lock_order | A0_direct | rust | no | yes | 6 |
| `c9c01200389d` | structure/nested_scope_lock_order | A1_self_iter | rust | no | yes | 3 |
| `70525c67bb7c` | structure/nested_scope_lock_order | A3_local | a3-rust | no | yes | 3 |
| `77459d011aae` | structure/nested_scope_lock_order | A3_whole | a3-rust | no | yes | 2 |

### Extraction oracle

- cells: 63; **validated: 0**; contract_invalid: 4
- stage distribution: `{'codegen': 34, 'conform': 25, 'explore': 4}`
- failure reason distribution: `{'invalid type: string "0"': 5, 'E201: branch condition "a_held" is not a comparison': 2, 'traces not all conformant': 25, 'missing field `kind`': 10, "E931: expected expression, found '[]'": 3, "E931: expected expression, found '!ready'": 2, "E931: undefined name 'read_shared'": 2, "E931: undefined name 'thread_id'": 5, 'invalid type: map': 1, "E308: write_shared requires a Var, found 'main::shared'": 1, "E102: function 'main::Sem::new' is not registered (duplicate or empty name)": 1, "E308: read_shared requires a Var, found 'main::n'": 1, "E931: undefined name 'Mutex'": 1}`

| task | arm | rep | stage | validated | verdict | reason |
| --- | --- | --- | --- | --- | --- | --- |
| channel/bounded_backpressure_lock_held | A0_direct | 0 | codegen | False | — | missing field `kind` |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A0_direct | 2 | codegen | False | — | missing field `kind` |
| channel/bounded_backpressure_lock_held | A1_self_iter | 0 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A1_self_iter | 2 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 0 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 2 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 0 | codegen | False | — | E931: expected expression, found '!ready' |
| channel/send_while_holding_mutex | A0_direct | 1 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 2 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 0 | codegen | False | — | missing field `kind` |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 1 | codegen | False | — | E308: write_shared requires a Var, found 'main::shared' |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 2 | codegen | False | — | E931: expected expression, found '!ready' |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 0 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 2 | codegen | False | — | E931: expected expression, found '[]' |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 0 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 1 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 2 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 0 | codegen | False | — | E931: expected expression, found '[]' |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 1 | codegen | False | — | E931: expected expression, found '[]' |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 2 | codegen | False | — | missing field `kind` |
| lock-order/cross_module_cycle | A0_direct | 0 | codegen | False | — | E201: branch condition "a_held" is not a comparison |
| lock-order/cross_module_cycle | A0_direct | 1 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A0_direct | 2 | codegen | False | — | E201: branch condition "a_held" is not a comparison |
| lock-order/cross_module_cycle | A1_self_iter | 0 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A1_self_iter | 1 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A1_self_iter | 2 | codegen | False | — | E931: undefined name 'Mutex' |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 0 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 1 | codegen | False | — | missing field `kind` |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 2 | conform | False | — | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 0 | conform | False | — | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 1 | conform | False | — | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 2 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | explore | False | INVALID | CIR does not use every required label exactly once |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | explore | False | INVALID | CIR does not use every required label exactly once |
| lock-order/partial_deadlock_bystander | A0_direct | 0 | codegen | False | — | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A0_direct | 1 | codegen | False | — | invalid type: map |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | codegen | False | — | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | codegen | False | — | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | codegen | False | — | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 0 | codegen | False | — | invalid type: string "0" |
| semaphore/acquire_twice_no_release | A0_direct | 0 | codegen | False | — | E931: undefined name 'read_shared' |
| semaphore/acquire_twice_no_release | A0_direct | 1 | codegen | False | — | E102: function 'main::Sem::new' is not registered (duplicate or empty name) |
| semaphore/acquire_twice_no_release | A0_direct | 2 | codegen | False | — | E931: undefined name 'read_shared' |
| semaphore/acquire_twice_no_release | A1_self_iter | 0 | codegen | False | — | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A1_self_iter | 1 | codegen | False | — | E308: read_shared requires a Var, found 'main::n' |
| semaphore/acquire_twice_no_release | A1_self_iter | 2 | codegen | False | — | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 0 | codegen | False | — | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 1 | codegen | False | — | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 2 | codegen | False | — | E931: undefined name 'thread_id' |
| structure/nested_scope_lock_order | A0_direct | 0 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A0_direct | 1 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A0_direct | 2 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 0 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 1 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 2 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 0 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 1 | conform | False | — | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 2 | conform | False | — | traces not all conformant |

### Track D — tasks with a Rust reference

| task | side | concir.petri | concir.interp | miri | lockbud |
| --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | buggy | FAIL | FAIL | clean 16 | detected |
| lock-order/abba_2lock | fixed | PASS | PASS | clean 16 | clean |
| lock-order/cycle_3lock | buggy | FAIL | FAIL | clean 16 | clean |
| lock-order/cycle_3lock | fixed | PASS | PASS | clean 16 | detected |
| lock-order/cross_module_cycle | buggy | FAIL | FAIL | clean 16 | detected |
| lock-order/cross_module_cycle | fixed | PASS | PASS | clean 16 | clean |
| lock-order/partial_deadlock_bystander | buggy | FAIL | FAIL | timeout 16 | clean |
| lock-order/partial_deadlock_bystander | fixed | PASS | PASS | thread_leak 16 | clean |
| semaphore/acquire_twice_no_release | buggy | FAIL | FAIL | detected 16 | clean |
| semaphore/acquire_twice_no_release | fixed | PASS | PASS | clean 16 | clean |
| condvar/bare_wait_no_predicate | buggy | FAIL | FAIL | clean 16 | clean |
| condvar/bare_wait_no_predicate | fixed | PASS | PASS | clean 16 | clean |
| channel/bounded_backpressure_lock_held | buggy | FAIL | FAIL | detected 16 | clean |
| channel/bounded_backpressure_lock_held | fixed | PASS | PASS | clean 16 | clean |
| structure/nested_scope_lock_order | buggy | FAIL | FAIL | clean 16 | detected |
| structure/nested_scope_lock_order | fixed | PASS | PASS | clean 16 | clean |
| condvar/notify_one_multi_waiter_wrong_pick | buggy | FAIL | FAIL | detected 16 | clean |
| condvar/notify_one_multi_waiter_wrong_pick | fixed | PASS | PASS | clean 16 | clean |
| channel/send_while_holding_mutex | buggy | FAIL | FAIL | detected 16 | clean |
| channel/send_while_holding_mutex | fixed | PASS | PASS | clean 16 | clean |

### Track D (CIR-only)

| task | side | concir.petri | concir.interp |
| --- | --- | --- | --- |
| condvar/lost_wakeup_notify_before_wait | buggy | FAIL | FAIL |
| condvar/lost_wakeup_notify_before_wait | fixed | PASS | PASS |
| channel/rendezvous_both_send | buggy | FAIL | FAIL |
| channel/rendezvous_both_send | fixed | PASS | PASS |
| semaphore/permit_leak | buggy | FAIL | FAIL |
| semaphore/permit_leak | fixed | PASS | PASS |
| semaphore/throttle_n_permits | buggy | None | None |
| semaphore/throttle_n_permits | fixed | None | None |
| atomic-data/bounded_counter_invariant | buggy | None | None |
| atomic-data/bounded_counter_invariant | fixed | None | None |
| atomic-data/counter_overflow_safety | buggy | FAIL | FAIL |
| atomic-data/counter_overflow_safety | fixed | PASS | PASS |
| atomic-data/atomic_lost_update | buggy | FAIL | FAIL |
| atomic-data/atomic_lost_update | fixed | PASS | PASS |
| structure/scope_bound_k_workers | buggy | None | None |
| structure/scope_bound_k_workers | fixed | None | None |
| structure/scope_worker_abba | buggy | FAIL | FAIL |
| structure/scope_worker_abba | fixed | PASS | PASS |
| structure/worker_with_payload | buggy | None | None |
| structure/worker_with_payload | fixed | None | None |
| boundary/unbounded_int_unknown | buggy | None | None |
| boundary/unbounded_int_unknown | fixed | None | None |
| lock-order/two_independent_cycles | buggy | FAIL | FAIL |
| lock-order/two_independent_cycles | fixed | None | None |
| condvar/same_cv_different_locks | buggy | None | None |
| condvar/same_cv_different_locks | fixed | None | None |
| structure/finite_call_loop | buggy | None | None |
| structure/finite_call_loop | fixed | None | None |
| structure/spawn_join_loop_finite | buggy | None | None |
| structure/spawn_join_loop_finite | fixed | None | None |
| boundary/rwlock_unsupported | buggy | UNSUPPORTED | UNSUPPORTED |
| boundary/rwlock_unsupported | fixed | None | None |
| boundary/async_select_unsupported | buggy | UNSUPPORTED | UNSUPPORTED |
| boundary/async_select_unsupported | fixed | None | None |
| P1 | buggy | None | None |
| P1 | fixed | None | None |
| P2 | buggy | None | None |
| P2 | fixed | None | None |
| P3 | buggy | None | None |
| P3 | fixed | None | None |
| P4 | buggy | None | None |
| P4 | fixed | None | None |
| P5 | buggy | None | None |
| P5 | fixed | None | None |
| P6 | buggy | None | None |
| P6 | fixed | None | None |
| P7 | buggy | None | None |
| P7 | fixed | None | None |
| P8 | buggy | None | None |
| P8 | fixed | None | None |
| P9 | buggy | None | None |
| P9 | fixed | None | None |
| real-cases/rmw-zenoh-998 | buggy | FAIL | FAIL |
| real-cases/rmw-zenoh-998 | fixed | PASS | PASS |
| real-cases/dashmap-369 | buggy | UNSUPPORTED | UNSUPPORTED |
| real-cases/dashmap-369 | fixed | None | None |

Lockbud: available=True commit=`None` (miri seeds=16).

### Scale

| threads | chain_len | max_states | outcome | complete | states | wall_ms |
| --- | --- | --- | --- | --- | --- | --- |
| 2 | 2 | 5000 | PASS | True | 39 | 15 |
| 2 | 2 | 20000 | PASS | True | 39 | 15 |
| 2 | 2 | 200000 | PASS | True | 39 | 15 |
| 3 | 2 | 5000 | PASS | True | 218 | 18 |
| 3 | 2 | 20000 | PASS | True | 218 | 19 |
| 3 | 2 | 200000 | PASS | True | 218 | 18 |
| 4 | 2 | 5000 | PASS | True | 1267 | 40 |
| 4 | 2 | 20000 | PASS | True | 1267 | 41 |
| 4 | 2 | 200000 | PASS | True | 1267 | 42 |
| 5 | 2 | 5000 | UNKNOWN | False | 5001 | 121 |
| 5 | 2 | 20000 | PASS | True | 7780 | 205 |
| 5 | 2 | 200000 | PASS | True | 7780 | 201 |
| 3 | 3 | 5000 | PASS | True | 320 | 20 |
| 3 | 3 | 20000 | PASS | True | 320 | 20 |
| 3 | 3 | 200000 | PASS | True | 320 | 20 |
| 4 | 3 | 5000 | PASS | True | 1891 | 57 |
| 4 | 3 | 20000 | PASS | True | 1891 | 54 |
| 4 | 3 | 200000 | PASS | True | 1891 | 57 |
| 5 | 3 | 5000 | UNKNOWN | False | 5001 | 124 |
| 5 | 3 | 20000 | PASS | True | 11710 | 318 |
| 5 | 3 | 200000 | PASS | True | 11710 | 318 |
| 6 | 3 | 5000 | UNKNOWN | False | 5001 | 99 |
| 6 | 3 | 20000 | UNKNOWN | False | 20001 | 445 |
| 6 | 3 | 200000 | PASS | True | 78263 | 2471 |

## Deviations and threats to validity

- D-19: the main batch is not gated on Rust-arm oracle completeness; missing `oracle.model` cells are `inconclusive` and backfilled by extraction/expert labels.
- Expert labels are LLM-assisted (agent-proxy); the human review queue (`HUMAN_REVIEW_QUEUE.md`) is left blank for the owner.
- All offline recomputation uses a single BIN_MAIN; any result on another binary sha is listed here.
