# Expert labels — accepted Rust candidates (main batch)

- rubric: `expert-label-rubric-v1`
- annotator: proxy:opencode (deepseek-v4.1-flash), 2026-09-19
- candidates: 17
- agreement with automatic oracle: 14/16 = 0.875 (unsure excluded: 1)

| task | arm | bug_present | defect lines | design_preserved | auto | agree |
| --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | yes | 44,46,47,59,61,62 | yes | True | True |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | no | — | yes | False | True |
| lock-order/cross_module_cycle | A0_direct | no | — | no | False | True |
| lock-order/cross_module_cycle | A2_tools_iter_ml | no | — | yes | False | True |
| lock-order/cycle_3lock | A0_direct | yes | 12,19,20,27,28 | yes | False | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | yes | 12,19,20,27,28 | yes | False | False |
| structure/nested_scope_lock_order | A0_direct | no | — | yes | False | True |
| structure/nested_scope_lock_order | A1_self_iter | no | — | yes | False | True |
| structure/nested_scope_lock_order | A2_tools_iter_ml | no | — | yes | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | no | — | yes | False | True |
| channel/bounded_backpressure_lock_held | A0_direct | no | — | yes | False | True |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | no | — | yes | False | True |
| channel/send_while_holding_mutex | A0_direct | no | — | yes | False | True |
| channel/send_while_holding_mutex | A1_self_iter | unsure | — | no | False | None |
| channel/send_while_holding_mutex | A2_tools_iter_ml | no | — | yes | False | True |
| semaphore/acquire_twice_no_release | A0_direct | yes | 37,44 | yes | True | True |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | no | — | yes | False | True |

## Reasons

- **lock-order/partial_deadlock_bystander / A0_direct** (`a4503ffe4461`): worker_a holds mtx_a then blocks on sem_b / mtx_b while worker_b holds mtx_b then blocks on sem_a / mtx_a: the partial-deadlock circular wait is intact.
- **lock-order/partial_deadlock_bystander / A2_tools_iter_ml** (`67724b0c3959`): Each worker drops its first guard before acquiring the second mutex, breaking the circular wait; semaphores and both mutexes are retained.
- **lock-order/cross_module_cycle / A0_direct** (`b8a63387fc02`): No deadlock: a single global mutex serialises both tasks, so the flag waits always make progress. But the two-independent-mutex cross-module design is replaced by one global lock+condvar, so the original concurrency design is not preserved.
- **lock-order/cross_module_cycle / A2_tools_iter_ml** (`9bab819deee2`): Both tasks acquire the same mutexes in the same global order (A then B); two-mutex design preserved, cycle removed.
- **lock-order/cycle_3lock / A0_direct** (`c7d11e854196`): Unchanged 3-lock cycle: w1 A->B, w2 B->C, w3 C->A. Deadlock is reachable (report as a false accept).
- **lock-order/cycle_3lock / A2_tools_iter_ml** (`c7d11e854196`): Identical to the buggy reference: A->B, B->C, C->A cycle remains. Accepted by the run by luck of scheduling; expert label yes.
- **structure/nested_scope_lock_order / A0_direct** (`4e6e1cdc7910`): Both inner tasks lock the same mutexes in the same order (A then B); no cycle.
- **structure/nested_scope_lock_order / A1_self_iter** (`c9c01200389d`): Same consistent order (A then B) in both spawned closures.
- **structure/nested_scope_lock_order / A2_tools_iter_ml** (`1293bda034b7`): Same consistent order (A then B); comment states the global ordering rationale.
- **condvar/notify_one_multi_waiter_wrong_pick / A2_tools_iter_ml** (`0e3cdb53c009`): Uses notify_all so both waiters on the shared predicate wake; no waiter is left asleep.
- **channel/bounded_backpressure_lock_held / A0_direct** (`13d1b761aaca`): Mutex is taken and released in a scoped block; neither send nor recv happens with the lock held. No lock-order or channel deadlock.
- **channel/bounded_backpressure_lock_held / A2_tools_iter_ml** (`1c28ef235a06`): Same as A0 but with comments; lock is scoped away from both channel operations.
- **channel/send_while_holding_mutex / A0_direct** (`7872b79e540d`): Sender and receiver release the shared mutex before tx1.send/rx1.recv; the condvar handshake uses a flag so neither blocks with the lock held.
- **channel/send_while_holding_mutex / A1_self_iter** (`1b5ee9f6995a`): final artifact is the model's prose 'NO_ISSUES' reply, not compilable Rust; arm marked accepted with build_ok=False, so this is an arm-level false accept, not a program.
- **channel/send_while_holding_mutex / A2_tools_iter_ml** (`24fb036eb37a`): Both threads release the shared mutex before the rendezvous channel operations; sender waits on rx2 only after releasing the lock.
- **semaphore/acquire_twice_no_release / A0_direct** (`f42afd77e7a9`): w1 calls acq twice before any rel; with one permit the second acq blocks forever and w2's acq also blocks. Same acquire-twice defect.
- **semaphore/acquire_twice_no_release / A2_tools_iter_ml** (`12c050cd7efc`): Semaphore made owner/reentrant: w1's second acq increments its own count and both rels balance; w2 acquires after w1 fully releases.
