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
| lock-order/partial_deadlock_bystander | A1_self_iter | 2/3 | 1 | 12574±6516 | 42330±26791 | 902±341 | — | 2/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | — | yes 3/3 | — 2/2 |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 1/3 | 2 | 7957 | 16521 | 43901 | — | 0/3 | True 2/3 · False 1/3 | terminated_ok 2/3 · no_build 1/3 | clean 3/3 | — | no 3/3 | — 1/1 |
| lock-order/partial_deadlock_bystander | A3_local | 0/3 | — | — | — | — | 25±11 | 0/3 | — | — | — | — | — | — |
| lock-order/partial_deadlock_bystander | A3_whole | 2/3 | 4 | 15255±30 | 389630±760819 | 97±2 | 34±17 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | — | — |
| lock-order/cross_module_cycle | A0_direct | 3/3 | 1 | 898±242 | 1937±88 | 911±433 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A1_self_iter | 3/3 | 1 | 1247±66 | 3333±1910 | 699±29 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 3/3 | 1 | 693 | 1709±1170 | 2188±469 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| lock-order/cross_module_cycle | A3_local | 3/3 | 2 | 1912 | 1228±318 | 55±13 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| lock-order/cross_module_cycle | A3_whole | 3/3 | 2 | 3580 | 3171±2607 | 59±15 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| lock-order/cycle_3lock | A0_direct | 3/3 | 1 | 1001 | 1720±215 | 695±25 | — | 3/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | yes 3/3 | — 2/3 · INVALID 1/3 |
| lock-order/cycle_3lock | A1_self_iter | 0/3 | — | — | — | — | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | — | — |
| lock-order/cycle_3lock | A2_tools_iter_ml | 3/3 | 1 | 911 | 1810±1051 | 2190±21 | — | 3/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | yes 3/3 | — 2/3 · INVALID 1/3 |
| lock-order/cycle_3lock | A3_local | 3/3 | 2 | 2100±13 | 1211±359 | 67±14 | 23 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| lock-order/cycle_3lock | A3_whole | 2/3 | 3 | 8524 | 5739±1052 | 96±19 | 22±1 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | — | — |
| structure/nested_scope_lock_order | A0_direct | 3/3 | 1 | 736 | 1407±487 | 717±107 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A1_self_iter | 3/3 | 1 | 1105 | 2167±1067 | 759±44 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 3/3 | 1 | 646 | 1558±859 | 2291±394 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | no 3/3 | — 3/3 |
| structure/nested_scope_lock_order | A3_local | 3/3 | 2 | 2054 | 1213±379 | 57±15 | 18 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| structure/nested_scope_lock_order | A3_whole | 2/3 | 4 | 10713 | 8348±1931 | 67±22 | 13±14 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 3/3 | 1 | 623±213 | 1309±1119 | 241±724 | — | 3/3 | True 3/3 | hang 2/3 · terminated_ok 1/3 | detected 2/3 · clean 1/3 | — | yes 3/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 3/3 | 1 | 1334±34 | 3100±1394 | 762±127 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 3/3 | 1 | 675 | 6829±16936 | 2575±684 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | 3/3 | 2 | 2841±29 | 2142±374 | 63±29 | 19±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | 2/3 | 4 | 13930 | 12192±2559 | 70±2 | 20±24 | 0/3 | True 2/2 | terminated_ok 2/2 | clean 2/2 | PASS 2/2 | — | — |
| channel/bounded_backpressure_lock_held | A0_direct | 3/3 | 1 | 707 | 1486±732 | 744±23 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A1_self_iter | 3/3 | 1 | 1523±730 | 4045±1730 | 773±63 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 3/3 | 1 | 670±15 | 1657±953 | 2742±310 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| channel/bounded_backpressure_lock_held | A3_local | 3/3 | 2 | 2165 | 1435±772 | 53±14 | 18±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| channel/bounded_backpressure_lock_held | A3_whole | 3/3 | 4 | 11127±530 | 7991±2005 | 71±27 | 19±5 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| channel/send_while_holding_mutex | A0_direct | 3/3 | 1 | 859±22 | 1929±337 | 759±57 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| channel/send_while_holding_mutex | A1_self_iter | 0/3 | — | — | — | — | — | 0/3 | — | — | — | — | — | — |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 3/3 | 1 | 760±27 | 1929±351 | 2526±373 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| channel/send_while_holding_mutex | A3_local | 3/3 | 2 | 1506±38 | 1167±333 | 53±13 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| channel/send_while_holding_mutex | A3_whole | 3/3 | 3 | 5983 | 4272±332 | 58±20 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| semaphore/acquire_twice_no_release | A0_direct | 3/3 | 1 | 877±287 | 1492±984 | 452±694 | — | 3/3 | True 3/3 | hang 3/3 | detected 3/3 | — | unsure 3/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A1_self_iter | 3/3 | 1 | 8415±10091 | 26215±30739 | 780±136 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 3/3 | 2 | 4196±419 | 5364±298 | 4057±105 | — | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | — | unsure 3/3 | — 3/3 |
| semaphore/acquire_twice_no_release | A3_local | 3/3 | 2 | 1759±11 | 1268±383 | 53±16 | 18±3 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |
| semaphore/acquire_twice_no_release | A3_whole | 3/3 | 2 | 3432 | 2434±493 | 53±14 | 19±2 | 0/3 | True 3/3 | terminated_ok 3/3 | clean 3/3 | PASS 3/3 | — | — |

`accepted`/`false_accept` are k/n over repeats; `round`/`tokens`/`*_ms` are mean±range over accepted cells; oracle columns are per-rep counts. `verify_ms` is the ConcIR `explore`/`conform` time. **A0's `accepted` only means it produced a parseable program** (a single round with no rejection); it is not a correctness claim. `extract` is the validated extraction verdict (`INVALID` = contract could not be evaluated).

### Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | conform_pass_rate | tokens/correct_accept | by source |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A0_direct | 24 | 24 | 1.00 | 12 | — | 593 | {'expert': 9, 'behavior': 8, 'by_construction': 3} |
| A1_self_iter | 24 | 17 | 0.71 | 2 | — | 1747 | {'expert': 2} |
| A2_tools_iter_ml | 24 | 22 | 0.92 | 3 | — | 869 | {'expert': 3} |
| A3_local | 24 | 21 | 0.88 | 0 | 21/21 = 1.00 | 683 | — |
| A3_whole | 24 | 20 | 0.83 | 0 | 20/20 = 1.00 | 3627 | — |

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
| channel/bounded_backpressure_lock_held | A0_direct | unsure | yes | False | False | None |
| channel/bounded_backpressure_lock_held | A1_self_iter | unsure | yes | False | False | None |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | unsure | yes | False | False | None |
| channel/send_while_holding_mutex | A0_direct | unsure | yes | False | False | None |
| channel/send_while_holding_mutex | A2_tools_iter_ml | unsure | yes | False | False | None |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | yes | yes | False | True | True |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | unsure | yes | False | False | None |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | unsure | yes | False | False | None |
| lock-order/cross_module_cycle | A0_direct | unsure | no | True | False | None |
| lock-order/cross_module_cycle | A1_self_iter | no | yes | False | False | True |
| lock-order/cross_module_cycle | A2_tools_iter_ml | no | yes | False | False | True |
| lock-order/cycle_3lock | A0_direct | yes | yes | False | True | True |
| lock-order/cycle_3lock | A2_tools_iter_ml | yes | yes | False | True | True |
| lock-order/partial_deadlock_bystander | A0_direct | yes | yes | False | True | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | yes | yes | False | True | True |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | no | yes | False | False | True |
| semaphore/acquire_twice_no_release | A0_direct | unsure | yes | False | True | None |
| semaphore/acquire_twice_no_release | A1_self_iter | unsure | yes | False | False | None |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | unsure | yes | False | False | None |
| structure/nested_scope_lock_order | A0_direct | no | yes | False | False | True |
| structure/nested_scope_lock_order | A1_self_iter | no | yes | False | False | True |
| structure/nested_scope_lock_order | A2_tools_iter_ml | no | yes | False | False | True |

Agreement with the automatic oracle: **11/11 = 1.000** (unsure 11); `design_loss` (accepted ∧ design_preserved=no): **1**.

### Extraction oracle

- cells: 63; **validated: 0**; contract_invalid: 2
- stage distribution: `{'labels': 10, 'codegen': 29, 'conform': 22, 'explore': 2}`
- failure reason distribution: `{'CIR does not use every required label exactly once': 10, 'panic at src/sem/program.rs:333': 4, 'E201: branch condition "a_held" is not a comparison': 2, 'traces not all conformant': 22, 'missing field `kind`': 9, "E931: expected expression, found '[]'": 3, "E931: expected expression, found '!ready'": 2, "E931: undefined name 'read_shared'": 2, "E931: undefined name 'thread_id'": 5, "E308: read_shared requires a Var, found 'main::n'": 1, "E931: undefined name 'Mutex'": 1}`

| task | arm | rep | stage | validated | verdict | reason |
| --- | --- | --- | --- | --- | --- | --- |
| channel/bounded_backpressure_lock_held | A0_direct | 0 | codegen | False | — | missing field `kind` |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A0_direct | 2 | codegen | False | — | missing field `kind` |
| channel/bounded_backpressure_lock_held | A1_self_iter | 0 | labels | False | — | CIR does not use every required label exactly once |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A1_self_iter | 2 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 0 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 1 | conform | False | — | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 2 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 0 | codegen | False | — | E931: expected expression, found '!ready' |
| channel/send_while_holding_mutex | A0_direct | 1 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 2 | conform | False | — | traces not all conformant |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 0 | codegen | False | — | missing field `kind` |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 1 | labels | False | — | CIR does not use every required label exactly once |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 2 | codegen | False | — | E931: expected expression, found '!ready' |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 0 | codegen | False | — | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | labels | False | — | CIR does not use every required label exactly once |
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
| lock-order/cross_module_cycle | A1_self_iter | 0 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/cross_module_cycle | A1_self_iter | 1 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A1_self_iter | 2 | codegen | False | — | E931: undefined name 'Mutex' |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 0 | conform | False | — | traces not all conformant |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 1 | codegen | False | — | missing field `kind` |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 2 | conform | False | — | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 0 | conform | False | — | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 1 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/cycle_3lock | A0_direct | 2 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/partial_deadlock_bystander | A0_direct | 0 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/partial_deadlock_bystander | A0_direct | 1 | codegen | False | — | panic at src/sem/program.rs:333 |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | codegen | False | — | panic at src/sem/program.rs:333 |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | codegen | False | — | panic at src/sem/program.rs:333 |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | labels | False | — | CIR does not use every required label exactly once |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 0 | codegen | False | — | panic at src/sem/program.rs:333 |
| semaphore/acquire_twice_no_release | A0_direct | 0 | codegen | False | — | E931: undefined name 'read_shared' |
| semaphore/acquire_twice_no_release | A0_direct | 1 | labels | False | — | CIR does not use every required label exactly once |
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
