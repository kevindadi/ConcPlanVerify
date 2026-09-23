# RESULTS — ConcIR repair/extraction benchmark

> **Generated file — do not edit by hand.** Regenerate with:
> ```
> python -m cir_workflow results results --batch experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5 --expert experiments/flash-repair-main-v1/expert-labels/EXPERT_LABELS.json --extraction experiments/extraction-v5 --trackd experiments/detection-v3/TRACKD.json --scale experiments/scale-v2/SCALE.json --mutation experiments/conform-mutation-v2/SUMMARY.json --postedit experiments/post-edit-conform-v2/SUMMARY.json --mutation-v1 experiments/conform-mutation-v1/SUMMARY.json --postedit-v1 experiments/post-edit-conform-v1/SUMMARY.json --modelprobe experiments/model-probe-v2 --gen experiments/flash-gen-main-v5-code/run-20260923T211345 --genprobe experiments/gen-model-probe-v3-code --latex experiments/tables --output experiments/RESULTS.md
> ```

## Provenance

- batch: `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5`
- expert: `experiments/flash-repair-main-v1/expert-labels/EXPERT_LABELS.json`
- extraction: `experiments/extraction-v5`
- track D: `experiments/detection-v3/TRACKD.json`
- scale: `experiments/scale-v2/SCALE.json`
- generation batch: `experiments/flash-gen-main-v5-code/run-20260923T211345`
- generation probe: `experiments/gen-model-probe-v3-code`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- binary v2 (conform) sha256: `073129de3a6d378a4e198bf712028c0c950cdf080981fb0073dfb7af5fa7ce5c`
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
| lock-order/abba_2lock | A0_direct | 3/3 | 1 | 808 | 1366±115 | 796±306 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — |
| lock-order/abba_2lock | A1_self_iter | 3/3 | 1 | 1235 | 1786±657 | 694±41 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — |
| lock-order/abba_2lock | A2_tools_iter_ml | 3/3 | 1 | 718 | 1251±413 | 2229±422 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — |
| lock-order/abba_2lock | A3_local | 3/3 | 2 | 1868 | 915±364 | 52±5 | 18±1 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| lock-order/abba_2lock | A3_whole | 3/3 | 2 | 3466 | 2039±361 | 51±6 | 19±5 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | no 3/3 | — |
| lock-order/abba_2lock | A3_tiered | 3/3 | 2 | 1868 | — | — | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| condvar/bare_wait_no_predicate | A0_direct | 3/3 | 1 | 746±28 | 1288±287 | 799±245 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — |
| condvar/bare_wait_no_predicate | A1_self_iter | 3/3 | 1 | 1504±582 | 3520±2887 | 714±67 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | 3/3 | 1 | 647 | 1219±718 | 2239±497 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — |
| condvar/bare_wait_no_predicate | A3_local | 0/3 | — | — | — | — | 18±1 | 0/3 | — | — | — | — | — | — |
| condvar/bare_wait_no_predicate | A3_whole | 3/3 | 3±1 | 8922±3830 | 5750±2626 | 52±7 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | unsure 3/3 | — |
| condvar/bare_wait_no_predicate | A3_tiered | 0/3 | — | — | — | — | 17±1 | 0/3 | — | — | — | — | — | — |

`accepted`/`false_accept` are k/n over repeats; `round`/`tokens`/`*_ms` are mean±range over accepted cells; oracle columns are per-rep counts. `verify_ms` is the ConcIR `explore`/`conform` time. **A0's `accepted` only means it produced a parseable program** (a single round with no rejection); it is not a correctness claim. `extract` is the validated extraction verdict (`INVALID` = contract could not be evaluated).

### Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | conform_pass_rate | escalation_rate | tokens/correct_accept | by source |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A0_direct | 30 | 30 | 1.00 | 11 | — | — | 456 | {'behavior': 8, 'expert': 9, 'by_construction': 3} |
| A1_self_iter | 30 | 23 | 0.77 | 1 | — | — | 1315 | {'expert': 1} |
| A2_tools_iter_ml | 30 | 28 | 0.93 | 3 | — | — | 715 | {'expert': 3} |
| A3_local | 30 | 24 | 0.80 | 0 | 24/24 = 1.00 | — | 675 | — |
| A3_whole | 30 | 26 | 0.87 | 0 | 26/26 = 1.00 | — | 3267 | — |
| A3_tiered | 30 | 24 | 0.80 | 0 | 24/24 = 1.00 | 6/30 = 0.20 | 676 | — |

`tokens/correct_accept = Σ tokens / (accepted − false_accept)`; `∞` when the denominator is 0. `conform_pass_rate` is over A3 cells with a conform verdict.

### A3 decision distribution

| task | arm | decisions |
| --- | --- | --- |
| lock-order/partial_deadlock_bystander | A3_local | {'explore_fail': 8, 'stalled': 1, 'stalled_local_patch': 3} |
| lock-order/partial_deadlock_bystander | A3_whole | {'accepted': 2, 'check_invalid': 6, 'check_schema_error': 1, 'explore_fail': 3} |
| lock-order/cross_module_cycle | A3_local | {'accepted': 3, 'explore_fail': 3} |
| lock-order/cross_module_cycle | A3_whole | {'accepted': 3, 'explore_fail': 3} |
| lock-order/cycle_3lock | A3_local | {'accepted': 3, 'explore_fail': 3} |
| lock-order/cycle_3lock | A3_whole | {'accepted': 2, 'explore_fail': 6, 'stalled': 1, 'stalled_local_patch': 1} |
| structure/nested_scope_lock_order | A3_local | {'accepted': 3, 'explore_fail': 3} |
| structure/nested_scope_lock_order | A3_whole | {'accepted': 2, 'check_invalid': 6, 'check_schema_error': 1, 'explore_fail': 3} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | {'accepted': 3, 'explore_fail': 3} |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | {'accepted': 2, 'check_invalid': 5, 'check_schema_error': 1, 'explore_fail': 3, 'stalled_local_patch': 1} |
| channel/bounded_backpressure_lock_held | A3_local | {'accepted': 3, 'explore_fail': 3} |
| channel/bounded_backpressure_lock_held | A3_whole | {'accepted': 3, 'check_invalid': 6, 'explore_fail': 3} |
| channel/send_while_holding_mutex | A3_local | {'accepted': 3, 'explore_fail': 3} |
| channel/send_while_holding_mutex | A3_whole | {'accepted': 3, 'check_invalid': 3, 'explore_fail': 3} |
| semaphore/acquire_twice_no_release | A3_local | {'accepted': 3, 'explore_fail': 3} |
| semaphore/acquire_twice_no_release | A3_whole | {'accepted': 3, 'explore_fail': 3} |
| lock-order/abba_2lock | A3_local | {'accepted': 3, 'explore_fail': 3} |
| lock-order/abba_2lock | A3_whole | {'accepted': 3, 'explore_fail': 3} |
| condvar/bare_wait_no_predicate | A3_local | {'check_schema_error': 6, 'explore_fail': 6} |
| condvar/bare_wait_no_predicate | A3_whole | {'accepted': 3, 'check_invalid': 4, 'explore_fail': 3} |

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
| condvar/bare_wait_no_predicate | A0_direct | unsure | yes | False | False | None |
| condvar/bare_wait_no_predicate | A1_self_iter | unsure | yes | False | False | None |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | unsure | yes | False | False | None |
| condvar/bare_wait_no_predicate | A3_whole | unsure | yes | False | False | None |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | yes | yes | False | True | True |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | no | yes | False | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | no | yes | False | False | True |
| lock-order/abba_2lock | A0_direct | no | yes | False | False | True |
| lock-order/abba_2lock | A1_self_iter | no | yes | False | False | True |
| lock-order/abba_2lock | A2_tools_iter_ml | no | yes | False | False | True |
| lock-order/abba_2lock | A3_local | no | yes | False | False | True |
| lock-order/abba_2lock | A3_whole | no | yes | False | False | True |
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

Agreement with the automatic oracle: **42/42 = 1.000** (unsure 4); `design_loss` (accepted ∧ design_preserved=no): **6** {'A0_direct': 2, 'A3_local': 6, 'A3_whole': 6, 'A1_self_iter': 1}.

#### Expert labels (per candidate)

| sha256 | task | arm | kind | bug_present | design_preserved | human | cells |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `13d1b761aaca` | channel/bounded_backpressure_lock_held | A0_direct | rust | no | yes | — | 6 |
| `1c28ef235a06` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes | — | 2 |
| `98eed6be2b32` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `6c87085c602a` | channel/bounded_backpressure_lock_held | A3_local | a3-rust | no | no | no | 3 |
| `7f182a8fd554` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | — | 1 |
| `e443cf0e3e70` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | — | 1 |
| `6d7602ab8549` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no | — | 1 |
| `8f09cd79b48c` | channel/send_while_holding_mutex | A0_direct | rust | no | yes | — | 1 |
| `b607a97d0ac7` | channel/send_while_holding_mutex | A0_direct | rust | no | yes | — | 2 |
| `05b2f4c77462` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `1bb2c14dc515` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `ae1e465a6fbb` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `aa0668f905d8` | channel/send_while_holding_mutex | A3_local | a3-rust | no | no | — | 3 |
| `459df3234540` | channel/send_while_holding_mutex | A3_whole | a3-rust | no | no | — | 3 |
| `e53544b9c364` | condvar/bare_wait_no_predicate | A0_direct | rust | unsure | yes | no | 1 |
| `394c5e1e06b6` | condvar/bare_wait_no_predicate | A1_self_iter | rust | unsure | yes | no | 8 |
| `52b496de2a26` | condvar/bare_wait_no_predicate | A3_whole | a3-rust | unsure | yes | no | 2 |
| `90aee29b3ffa` | condvar/bare_wait_no_predicate | A3_whole | a3-rust | unsure | yes | no | 1 |
| `daa46ba4c2ba` | condvar/notify_one_multi_waiter_wrong_pick | A0_direct | rust | yes | yes | — | 2 |
| `0e3cdb53c009` | condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | rust | no | yes | — | 7 |
| `7de135daa5ad` | condvar/notify_one_multi_waiter_wrong_pick | A3_local | a3-rust | no | yes | — | 1 |
| `be85c5f756ec` | condvar/notify_one_multi_waiter_wrong_pick | A3_whole | a3-rust | no | yes | — | 2 |
| `12195b60e3cb` | lock-order/abba_2lock | A0_direct | rust | no | yes | — | 9 |
| `93eaf520459c` | lock-order/abba_2lock | A3_local | a3-rust | no | yes | — | 6 |
| `05fcb391ea81` | lock-order/cross_module_cycle | A0_direct | rust | no | no | — | 2 |
| `9bab819deee2` | lock-order/cross_module_cycle | A0_direct | rust | no | yes | — | 1 |
| `1fc6e7c5ea43` | lock-order/cross_module_cycle | A1_self_iter | rust | no | yes | — | 3 |
| `a7983b7f5fbe` | lock-order/cross_module_cycle | A2_tools_iter_ml | rust | no | yes | — | 3 |
| `e7adea20e72f` | lock-order/cross_module_cycle | A3_local | a3-rust | no | yes | — | 3 |
| `34835028c1d2` | lock-order/cross_module_cycle | A3_whole | a3-rust | no | yes | — | 3 |
| `c7d11e854196` | lock-order/cycle_3lock | A0_direct | rust | yes | yes | yes | 6 |
| `970be001ab1e` | lock-order/cycle_3lock | A3_local | a3-rust | no | yes | — | 5 |
| `d9c02a69c650` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | — | 1 |
| `a4503ffe4461` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | — | 1 |
| `574a7804dc4d` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes | — | 1 |
| `1ff1454288bf` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | yes | yes | yes | 1 |
| `a08bd1a020fa` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | no | no | no | 1 |
| `fbeb211c0ac0` | lock-order/partial_deadlock_bystander | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `f4db34f1644d` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes | — | 1 |
| `0951c33394bd` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes | — | 1 |
| `f42afd77e7a9` | semaphore/acquire_twice_no_release | A0_direct | rust | no | yes | no | 2 |
| `691d644c986c` | semaphore/acquire_twice_no_release | A0_direct | rust | yes | yes | — | 1 |
| `8f59f2055f41` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | — | 1 |
| `c42fa5ece43f` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | no | 1 |
| `b6f7d6651e0e` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes | no | 1 |
| `12cfc7f3617e` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `d463f5f9f603` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `0501dcb3a49d` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes | — | 1 |
| `1901bf15e784` | semaphore/acquire_twice_no_release | A3_local | a3-rust | no | yes | — | 3 |
| `84f0ae3ead6e` | semaphore/acquire_twice_no_release | A3_whole | a3-rust | no | yes | — | 3 |
| `4e6e1cdc7910` | structure/nested_scope_lock_order | A0_direct | rust | no | yes | — | 6 |
| `c9c01200389d` | structure/nested_scope_lock_order | A1_self_iter | rust | no | yes | — | 3 |
| `70525c67bb7c` | structure/nested_scope_lock_order | A3_local | a3-rust | no | yes | — | 3 |
| `77459d011aae` | structure/nested_scope_lock_order | A3_whole | a3-rust | no | yes | — | 2 |

Human review: 11 candidates; agent vs human **7/7** (agent unsure 4); human vs auto **9/11**.

##### Human vs automatic-oracle disagreements

| task | arm | sha | human | auto |
| --- | --- | --- | --- | --- |
| semaphore/acquire_twice_no_release | A0_direct | `f42afd77e7a9` | no | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | `a08bd1a020fa` | no | True |

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

### Conform recall (mutation, v1 vs v2)

| op | v1 recall | v2 recall | v2 failure reason |
| --- | --- | --- | --- |
| M1 | 0.545 | 1.0 | {'violation': 11, 'timeout': 2} |
| M2 | 0.067 | 0.947 | {'violation': 16, 'timeout': 2} |
| M3 | 0.0 | — | — |
| M4 | 0.333 | 1.0 | {'violation': 5} |
| M5 | — | 0.0 | {} |
| M6 | 1.0 | 1.0 | {'violation': 22, 'timeout': 1} |
| M7 | 0.105 | 0.087 | {'violation': 2} |
| M8 | — | 0.8 | {'violation': 8} |

### Post-edit drift (v1 vs v2)

| edit | v1 conform PASS | v1 drift-only-conform | v2 conform PASS | v2 drift-only-conform |
| --- | --- | --- | --- | --- |
| E1 | 19/19 | 0 | 22/22 | 0 |
| E2 | 14/19 | 0 | 5/6 | 0 |
| E3 | 17/19 | 0 | 7/10 | 0 |

### Model probe (OpenCode Go)

| model | arm | cells | accepted | false_accept |
| --- | --- | --- | --- | --- |
| kimi-k2.7-code | A0_direct | 10 | 9 | 1 |
| kimi-k2.7-code | A2_tools_iter_ml | 10 | 8 | 0 |
| kimi-k2.7-code | A3_local | 10 | 9 | 0 |
| glm-5.3-flash | A0_direct | 10 | 9 | 1 |
| glm-5.3-flash | A2_tools_iter_ml | 10 | 10 | 0 |
| glm-5.3-flash | A3_local | 10 | 8 | 0 |

Not part of the main table; 8 SMOKE tasks, 1 rep, K=4.

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
- Expert labels are LLM-assisted (agent-proxy) with the owner's human review merged (17 rows / 11 candidates); 2 human/auto disagreements await the owner (`HUMAN_DISAGREEMENT_EVIDENCE.md`).
- All offline recomputation uses BIN_MAIN (main table) or BIN_V2 (conform/a3-to-rust/mutation/post-edit); both shas are listed above.

## Generation (main) — composite (see Batches)

Requirements -> verified CIR -> LLM code -> tool post-verification, 24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the CIR exhaustively, the LLM writes the Rust, and conform/monitor post-verify; G3_codegen is the tool-codegen ablation (rep 0).

Batches (per-cell `source_run`; earlier runs overridden by later):
- `flash-gen-main-v2/run-20260923T005232` — G0_direct: 54, G1_self_iter: 54, G2_tools_iter: 54, G3_codegen: 24
- `flash-gen-main-v5-code/run-20260923T211345` — G0_direct: 18, G1_self_iter: 18, G2_tools_iter: 18, G3_concir: 72

| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | RF_run | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
`RF_all` counts non-accepted cells as 0; `RF_acc` covers accepted cells; `RF_run` averages cells that carry a monitor value (freeze-5 wording).
| G0_direct | 72 | 68 | 0.944 | 0.676 | 0.448 | 0.512 | 0.504 | 6 | 0 | 62708 |
| G1_self_iter | 72 | 62 | 0.861 | 0.699 | 0.396 | 0.559 | 0.545 | 3 | 0 | 284591 |
| G2_tools_iter | 72 | 47 | 0.653 | 0.701 | 0.379 | 0.681 | 0.581 | 0 | 0 | 323908 |
| G3_concir | 72 | 58 | 0.806 | 0.692 | 0.555 | 0.689 | 0.689 | 0 | 58 | 314729 |
| G3_codegen | 24 | 20 | 0.833 | 0.776 | 0.638 | 0.765 | 0.672 | 0 | 17 | 106808 |
| **all** | 312 | 255 | 0.817 | 0.699 | 0.459 | 0.618 | 0.586 | 9 | 75 | 1092744 |

Per tier (RF):

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir | G3_codegen |
| --- | --- | --- | --- | --- | --- |
| Simple | 0.462 | 0.457 | 0.326 | 0.668 | 0.773 |
| Medium | 0.551 | 0.384 | 0.465 | 0.637 | 0.642 |
| Complex | 0.331 | 0.347 | 0.345 | 0.361 | 0.499 |

`defect` = accepted and (behavior hang or monitor FAIL or conform violation). `awp` is G3-only (model PASS, conform PASS, no monitor FAIL).

## Generation with a frontier model

Model `kimi-k3` (OpenCode Go, chat/completions), 1 rep, K=4, temperature None.

| arm | cells | not_run | accepted | accept rate | RC | RF_all | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| G0_direct | 24 | 0 | 23 | 0.958 | 0.63 | 0.446 | 0 | 0 | 56895 |
| G3_concir | 24 | 0 | 17 | 0.708 | 0.676 | 0.479 | 0 | 17 | 110583 |
| **all** | 48 | 0 | 40 | 0.833 | 0.65 | 0.462 | 0 | 17 | 167478 |
