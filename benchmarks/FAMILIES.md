# FAMILIES — capability-family benchmark

Cases are validated with `check`/`support`/`explore` (petri and interp).
`status=ready` means both engines agreed and the files are frozen in
`benchmarks/MANIFEST.json`. Pre-registration mismatches are listed.

| family | case | variants | petri results | source |
| --- | --- | --- | --- | --- |
| lock-order | abba_2lock | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 49, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 39, 'wall_ms': None} | authored; buggy CIR equals the LLM-generated pilot-v1 t2_abba model (sha d2d4958b...) |
| lock-order | cycle_3lock | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 331, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 286, 'wall_ms': None} | authored |
| lock-order | cross_module_cycle | buggy | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 49, 'wall_ms': None} | LLM-generated pilot-v1 t3_cross_module_abba (frozen), declared llm_generated |
| lock-order | partial_deadlock_bystander | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 109, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 149, 'wall_ms': None} | authored |
| condvar | lost_wakeup_notify_before_wait | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 32, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 43, 'wall_ms': None} | authored |
| channel | rendezvous_both_send | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 6, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 9, 'wall_ms': None} | authored |
| semaphore | permit_leak | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 17, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 23, 'wall_ms': None} | authored |
| semaphore | acquire_twice_no_release | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 17, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 35, 'wall_ms': None} | authored |
| semaphore | throttle_n_permits | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 92, 'wall_ms': None} | authored |
| condvar | bare_wait_no_predicate | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 29, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 43, 'wall_ms': None} | authored |
| channel | bounded_backpressure_lock_held | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 12, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 20, 'wall_ms': None} | authored |
| atomic-data | bounded_counter_invariant | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 31, 'wall_ms': None} | authored; exercises bounded Int + safety invariant (a capability with no paper Table 1 coverage) |
| atomic-data | counter_overflow_safety | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 31, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 40, 'wall_ms': None} | authored; bounded Int + safety invariant |
| atomic-data | atomic_lost_update | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 30, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 65, 'wall_ms': None} | authored; atomics + always_reachable |
| structure | scope_bound_k_workers | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 116, 'wall_ms': None} | authored; exercises scope + function bound |
| structure | nested_scope_lock_order | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 51, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 41, 'wall_ms': None} | authored; nested scope |
| structure | scope_worker_abba | buggy, fixed | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 49, 'wall_ms': None}, fixed={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 39, 'wall_ms': None} | authored; scope structure |
| structure | worker_with_payload | correct | correct={'check_valid': True} |  |
| boundary | unbounded_int_unknown | correct | correct={'check_valid': True, 'support': True, 'petri': 'UNKNOWN', 'interp': 'UNKNOWN', 'petri_complete': False, 'interp_complete': False, 'states': 65, 'wall_ms': None} | authored; negative control for the analysis bound |
| lock-order | two_independent_cycles | buggy | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 2211, 'wall_ms': None} | concir regression seed (two independent cycles) |
| condvar | same_cv_different_locks | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 90, 'wall_ms': None} | concir round3 regression seed (precise wait-set: same cv, different locks; correct by construction) |
| condvar | notify_one_multi_waiter_wrong_pick | buggy | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 69, 'wall_ms': None} | concir round2 regression seed (notify_one choice) |
| channel | send_while_holding_mutex | buggy | buggy={'check_valid': True, 'support': True, 'petri': 'FAIL', 'interp': 'FAIL', 'petri_complete': True, 'interp_complete': True, 'states': 5, 'wall_ms': None} | concir regression seed (channel rendezvous mismatch) |
| structure | finite_call_loop | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 6, 'wall_ms': None} | concir round2 regression seed (finite call loop) |
| structure | spawn_join_loop_finite | correct | correct={'check_valid': True, 'support': True, 'petri': 'PASS', 'interp': 'PASS', 'petri_complete': True, 'interp_complete': True, 'states': 10, 'wall_ms': None} | concir round3 regression seed (finite spawn/join loop) |
| boundary | rwlock_unsupported | buggy | buggy={'check_valid': True, 'support': False, 'petri': 'UNSUPPORTED', 'interp': 'UNSUPPORTED', 'petri_complete': False, 'interp_complete': False, 'states': 0, 'wall_ms': None} | ConcIR example; RwLock is unsupported in the backend |
| boundary | async_select_unsupported | buggy | buggy={'check_valid': True, 'support': False, 'petri': 'UNSUPPORTED', 'interp': 'UNSUPPORTED', 'petri_complete': False, 'interp_complete': False, 'states': 0, 'wall_ms': None} | ConcIR example; async/await is unsupported in the backend |
| legacy | P1 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P2 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P3 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P4 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P5 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P6 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P7 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P8 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| legacy | P9 |  |  | paper Table 1 pattern; status=legacy, excluded from v2 experiments |
| real-cases | rmw-zenoh-998 |  |  |  |
| real-cases | dashmap-369 |  |  |  |

## Capabilities covered that the paper's Table 1 did not

- precise condvar wait-set (same cv, different locks; notify choice);
- bounded channel (rendezvous and capacity >= 1);
- counting semaphore (throttle, permit leak);
- bounded Int data domains and `safety` / `var_cmp` invariants;
- `scope` + function `bound`, finite spawn/join/goto loops;
- `always_reachable` (AG EF) at the goal layer;
- `UNSUPPORTED` (RwLock, async/await) and `UNKNOWN` (unbounded Int) boundary controls.
