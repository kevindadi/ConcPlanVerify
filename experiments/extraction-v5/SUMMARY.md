# Extraction v5 — SUMMARY (from CELLS.json)

- batch: `experiments/extraction-v5`
- binary sha256: `4bec943dd486473caab24b17233b536e31641fbbb1e3d8d505d499b3113fb3f2`
- prompt sha256: `34d5c22964bb0af395a8671c96f48126a1dafb6c5a6da6b8ba790a9952a0c8c4`
- requests: 98/120
- **validated: 0** / 63 (definition: stage=explore AND contract evaluation succeeded AND verdict in {PASS,FAIL})
- contract_invalid (explore, verdict INVALID): 4
- harness errors: 0
- stage distribution: `{'codegen': 34, 'conform': 25, 'explore': 4}`
- failure reason distribution: `{'invalid type: string "0"': 5, 'E201: branch condition "a_held" is not a comparison': 2, 'traces not all conformant': 25, 'missing field `kind`': 10, "E931: expected expression, found '[]'": 3, "E931: expected expression, found '!ready'": 2, "E931: undefined name 'read_shared'": 2, "E931: undefined name 'thread_id'": 5, 'invalid type: map': 1, "E308: write_shared requires a Var, found 'main::shared'": 1, "E102: function 'main::Sem::new' is not registered (duplicate or empty name)": 1, "E308: read_shared requires a Var, found 'main::n'": 1, "E931: undefined name 'Mutex'": 1}`

| task | arm | rep | stage | validated | verdict | reason |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | 0 | codegen | False | None | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | 0 | codegen | False | None | invalid type: string "0" |
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
| lock-order/partial_deadlock_bystander | A0_direct | 1 | codegen | False | None | invalid type: map |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | codegen | False | None | invalid type: string "0" |
| lock-order/cross_module_cycle | A0_direct | 1 | conform | False | None | traces not all conformant |
| lock-order/cross_module_cycle | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 1 | codegen | False | None | missing field `kind` |
| lock-order/cycle_3lock | A0_direct | 1 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | explore | False | INVALID | CIR does not use every required label exactly once |
| structure/nested_scope_lock_order | A0_direct | 1 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| structure/nested_scope_lock_order | A2_tools_iter_ml | 1 | conform | False | None | traces not all conformant |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 1 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | 1 | codegen | False | None | missing field `kind` |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | 1 | codegen | False | None | E931: expected expression, found '[]' |
| channel/bounded_backpressure_lock_held | A0_direct | 1 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A1_self_iter | 1 | conform | False | None | traces not all conformant |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | 1 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A0_direct | 1 | conform | False | None | traces not all conformant |
| channel/send_while_holding_mutex | A2_tools_iter_ml | 1 | codegen | False | None | E308: write_shared requires a Var, found 'main::shared' |
| semaphore/acquire_twice_no_release | A0_direct | 1 | codegen | False | None | E102: function 'main::Sem::new' is not registered (duplicate or empty name) |
| semaphore/acquire_twice_no_release | A1_self_iter | 1 | codegen | False | None | E308: read_shared requires a Var, found 'main::n' |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 1 | codegen | False | None | E931: undefined name 'thread_id' |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | codegen | False | None | invalid type: string "0" |
| lock-order/partial_deadlock_bystander | A1_self_iter | 2 | codegen | False | None | invalid type: string "0" |
| lock-order/cross_module_cycle | A0_direct | 2 | codegen | False | None | E201: branch condition "a_held" is not a comparison |
| lock-order/cross_module_cycle | A1_self_iter | 2 | codegen | False | None | E931: undefined name 'Mutex' |
| lock-order/cross_module_cycle | A2_tools_iter_ml | 2 | conform | False | None | traces not all conformant |
| lock-order/cycle_3lock | A0_direct | 2 | explore | False | INVALID | unclassified |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | explore | False | INVALID | CIR does not use every required label exactly once |
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
