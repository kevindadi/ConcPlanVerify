# gen-expert-labels-v1 — rubric v3 summary

Annotated (sha-dedup, priority G3_concir,G0_direct): **40**; not_run 48; requests 40.

- `bug_present=yes`: **0**; no: 40; unsure: 0
- agent–tool disagreements: 2 (both agent false positives on lock-order reasoning)
- annotated cells with at least one `Ri` unsatisfied per the agent: 11

| arm | task | sha[12] | bug_present | evidence |
| --- | --- | --- | --- | --- |
| G3_concir | atomic-data/bounded_counter_invariant | `07afe0fec004` | no | - |
| G3_concir | atomic-data/counter_overflow_safety | `42530fc02b4a` | no | - |
| G3_concir | channel/bounded_backpressure_lock_held | `56da8bd84ed9` | no | - |
| G3_concir | channel/rendezvous_both_send | `1af51bf899c6` | no | - |
| G3_concir | condvar/bare_wait_no_predicate | `e39c38dd648e` | no | - |
| G3_concir | lock-order/abba_2lock | `f34602b468bf` | no | - |
| G3_concir | lock-order/cross_module_cycle | `f53ea172041b` | no | - |
| G3_concir | lock-order/cycle_3lock | `7f565d54edd3` | no | t1 lines 11-12 (a then b), t2 lines 18-19 (b then c), t3 lines 25-26 (a then c): |
| G3_concir | lock-order/two_independent_cycles | `d26c9bce8bbd` | no | - |
| G3_concir | semaphore/acquire_twice_no_release | `8a51257e72bc` | no | - |
| G3_concir | semaphore/permit_leak | `a630b253afa4` | no | - |
| G3_concir | semaphore/throttle_n_permits | `8e6dd898f1ac` | no | - |
| G3_concir | structure/finite_call_loop | `aa58cb8bf3a1` | no | - |
| G3_concir | structure/nested_scope_lock_order | `f3e71aa4575a` | no | - |
| G3_concir | structure/scope_bound_k_workers | `8907db4ba8c5` | no | - |
| G3_concir | structure/scope_worker_abba | `666fa98f890f` | no | - |
| G3_concir | structure/spawn_join_loop_finite | `afbb3a6ba243` | no | - |
| G3_concir | structure/worker_with_payload | `bbbb9ddcc2c0` | no | - |
| G3_concir | atomic-data/counter_overflow_safety | `e348ba804a45` | no | - |
| G3_concir | channel/rendezvous_both_send | `5cf5bf97db61` | no | - |
| G3_concir | channel/send_while_holding_mutex | `96e76ca395fb` | no | - |
| G3_concir | lock-order/abba_2lock | `7bc67f3a71af` | no | - |
| G3_concir | lock-order/cross_module_cycle | `9c22c0845df2` | no | - |
| G3_concir | lock-order/two_independent_cycles | `90e786f8af13` | no | - |
| G3_concir | semaphore/acquire_twice_no_release | `a58dc7d5a480` | no | - |
| G3_concir | semaphore/permit_leak | `74caa0bbfd89` | no | - |
| G3_concir | structure/nested_scope_lock_order | `a3e87e7f6d02` | no | - |
| G3_concir | structure/worker_with_payload | `a0714e76c4f2` | no | - |
| G3_concir | channel/bounded_backpressure_lock_held | `9b2785d2dabd` | no | - |
| G3_concir | channel/rendezvous_both_send | `53261b1cb139` | no | - |
| G3_concir | lock-order/cross_module_cycle | `2482d1db6609` | no | - |
| G3_concir | lock-order/cycle_3lock | `68216e3ecd00` | no | t3 lines: `let _ga = a3.lock().unwrap(); let _gc = c3.lock().unwrap();` acquire  |
| G3_concir | structure/spawn_join_loop_finite | `3d51be736fa2` | no | - |
| G3_concir | structure/worker_with_payload | `d1158e5d0dd0` | no | - |
| G0_direct | atomic-data/atomic_lost_update | `8cd45a853599` | no | - |
| G0_direct | atomic-data/bounded_counter_invariant | `e2fcca636617` | no | - |
| G0_direct | atomic-data/counter_overflow_safety | `d6d976987104` | no | - |
| G0_direct | channel/bounded_backpressure_lock_held | `9b533aa7773e` | no | - |
| G0_direct | channel/rendezvous_both_send | `ccc7a087ae33` | no | - |
| G0_direct | channel/send_while_holding_mutex | `d2f459a220a0` | no | - |
