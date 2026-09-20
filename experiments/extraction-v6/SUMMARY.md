# Extraction v6 — SUMMARY

- rerun cells: 14; requests: 15/40
- binary sha256: `b8d441e6eb619a690e07021b01d4166090c0a53292821998a84a9b32b4df2065`
- prompt sha256: `1afeddf031b2fe454477159b33abbe68ddc652fe96a0f0e59c35e7fb769186c1`
- validated: **0** / 63
- stage distribution: `{'codegen': 34, 'conform': 25, 'explore': 4}`

| task | arm | rep | stage | validated | verdict | reason |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 0 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 0 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/cross_module_cycle | A0_direct | 0 | codegen | False | None | E201: branch condition "a_held" is not a comparison |
| lock-order/cross_module_cycle | A1_self_iter | 0 | conform | False | None | traces not all conformant |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 0 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 0 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | explore | False | INVALID | unclassified |
| structure/nested_scope_lock_order | A0_direct | 0 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 0 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 0 | conform | False | None | traces not all conformant |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 0 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 0 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 0 | codegen | False | None | E931: expected expression, found '[]' |
| channel/bounded_backpressure_lock_held | A0_direct | 0 | codegen | False | None | missing field `kind` |
| channel/bounded_backpressure_lock_held | A1_self_iter | 0 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 0 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 0 | codegen | False | None | E931: expected expression, found '!ready' |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 0 | codegen | False | None | missing field `kind` |
| semaphore/acquire_twice_no_release | A0_direct | 0 | codegen | False | None | E931: undefined name 'read_shared' |
| semaphore/acquire_twice_no_release | A1_self_iter | 0 | codegen | False | None | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 0 | codegen | False | None | E931: undefined name 'thread_id' |
| lock-order/partial_deadlock_bystander | A0_direct | 1 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/cross_module_cycle | A0_direct | 1 | conform | False | None | traces not all conformant |
| lock-order/cross_module_cycle | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 1 | codegen | False | None | missing field `kind` |
| lock-order/cycle_3lock | A0_direct | 1 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | explore | False | INVALID |  |
| structure/nested_scope_lock_order | A0_direct | 1 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 1 | conform | False | None | traces not all conformant |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 1 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 1 | codegen | False | None | E931: expected expression, found '[]' |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 1 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 1 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 1 | codegen | False | None | extracted CIR not codegen-able: codegen failed: cannot lower |
| semaphore/acquire_twice_no_release | A0_direct | 1 | codegen | False | None | extracted CIR not codegen-able: codegen failed: cannot lower |
| semaphore/acquire_twice_no_release | A1_self_iter | 1 | codegen | False | None | E308: read_shared requires a Var, found 'main::n' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 1 | codegen | False | None | E931: undefined name 'thread_id' |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | codegen | False | None | extracted CIR not codegen-able: codegen failed: JSON parse e |
| lock-order/cross_module_cycle | A0_direct | 2 | codegen | False | None | E201: branch condition "a_held" is not a comparison |
| lock-order/cross_module_cycle | A1_self_iter | 2 | codegen | False | None | E931: undefined name 'Mutex' |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 2 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 2 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | explore | False | INVALID |  |
| structure/nested_scope_lock_order | A0_direct | 2 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 2 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 2 | conform | False | None | traces not all conformant |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 2 | codegen | False | None | E931: expected expression, found '[]' |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 2 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 2 | codegen | False | None | missing field `kind` |
| channel/bounded_backpressure_lock_held | A0_direct | 2 | codegen | False | None | missing field `kind` |
| channel/bounded_backpressure_lock_held | A1_self_iter | 2 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 2 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 2 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 2 | codegen | False | None | E931: expected expression, found '!ready' |
| semaphore/acquire_twice_no_release | A0_direct | 2 | codegen | False | None | E931: undefined name 'read_shared' |
| semaphore/acquire_twice_no_release | A1_self_iter | 2 | codegen | False | None | E931: undefined name 'thread_id' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 2 | codegen | False | None | E931: undefined name 'thread_id' |
