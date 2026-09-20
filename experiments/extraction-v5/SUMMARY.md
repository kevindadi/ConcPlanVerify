# Extraction v5 — SUMMARY

- batch: `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- prompt sha256: `34d5c22964bb0af395a8671c96f48126a1dafb6c5a6da6b8ba790a9952a0c8c4`
- requests: 98/120
- validated: **2** / 63
- harness errors: 0
- stage distribution: `{'labels': 10, 'codegen': 29, 'conform': 22, 'explore': 2}`
- note: validation replay is offline; native schedules are random so the validated count is not perfectly reproducible.

| task | arm | rep | stage | validated | verdict | behavior | miri |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 0 | labels | False | None | hang | detected |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 0 | codegen | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A0_direct | 0 | codegen | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A1_self_iter | 0 | labels | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 0 | conform | False | None | terminated_ok | clean |
| lock-order/cycle_3lock | A0_direct | 0 | conform | False | None | terminated_ok | clean |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | explore | True | INVALID | terminated_ok | clean |
| structure/nested_scope_lock_order | A0_direct | 0 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A1_self_iter | 0 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 0 | conform | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 0 | codegen | False | None | hang | detected |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 0 | codegen | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 0 | codegen | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A0_direct | 0 | codegen | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A1_self_iter | 0 | labels | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 0 | conform | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A0_direct | 0 | codegen | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 0 | codegen | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A0_direct | 0 | codegen | False | None | hang | detected |
| semaphore/acquire_twice_no_release | A1_self_iter | 0 | codegen | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 0 | codegen | False | None | terminated_ok | clean |
| lock-order/partial_deadlock_bystander | A0_direct | 1 | codegen | False | None | hang | detected |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | codegen | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A0_direct | 1 | conform | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A1_self_iter | 1 | conform | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 1 | codegen | False | None | terminated_ok | clean |
| lock-order/cycle_3lock | A0_direct | 1 | labels | False | None | terminated_ok | clean |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | labels | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A0_direct | 1 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A1_self_iter | 1 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 1 | conform | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | labels | False | None | hang | detected |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 1 | codegen | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 1 | codegen | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | conform | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | conform | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 1 | conform | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A0_direct | 1 | conform | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 1 | labels | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A0_direct | 1 | labels | False | None | hang | detected |
| semaphore/acquire_twice_no_release | A1_self_iter | 1 | codegen | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 1 | codegen | False | None | terminated_ok | clean |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | codegen | False | None | hang | clean |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | labels | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A0_direct | 2 | codegen | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A1_self_iter | 2 | codegen | False | None | terminated_ok | clean |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 2 | conform | False | None | terminated_ok | clean |
| lock-order/cycle_3lock | A0_direct | 2 | explore | True | INVALID | terminated_ok | clean |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | labels | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A0_direct | 2 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A1_self_iter | 2 | conform | False | None | terminated_ok | clean |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 2 | conform | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 2 | codegen | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 2 | codegen | False | None | terminated_ok | clean |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 2 | codegen | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A0_direct | 2 | codegen | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A1_self_iter | 2 | conform | False | None | terminated_ok | clean |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 2 | conform | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A0_direct | 2 | conform | False | None | terminated_ok | clean |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 2 | codegen | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A0_direct | 2 | codegen | False | None | hang | detected |
| semaphore/acquire_twice_no_release | A1_self_iter | 2 | codegen | False | None | terminated_ok | clean |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 2 | codegen | False | None | terminated_ok | clean |
