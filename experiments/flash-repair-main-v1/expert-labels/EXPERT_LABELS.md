# Expert labels v2 (per candidate)

- rubric `expert-label-rubric-v2`; candidates 54; unsure 4 (0.074); agreement 108/117
- design_loss by arm: {'A0_direct': 1, 'A3_local': 2, 'A3_whole': 4, 'A1_self_iter': 1}

| sha256 | task | arm | kind | bug_present | design_preserved | unsure_reason | evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `d9c02a69c650` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes |  | lock sequences: ma->mb; mb->ma; lock; cycle among ['ma', 'mb'] |
| `fbeb211c0ac0` | lock-order/partial_deadlock_bystander | A2_tools_iter_ml | rust | no | yes |  | lock sequences: ma->mb; ma->mb; no cycle |
| `05fcb391ea81` | lock-order/cross_module_cycle | A0_direct | rust | no | no |  | lock sequences: lock; lock; no cycle |
| `1fc6e7c5ea43` | lock-order/cross_module_cycle | A1_self_iter | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `a7983b7f5fbe` | lock-order/cross_module_cycle | A2_tools_iter_ml | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `e7adea20e72f` | lock-order/cross_module_cycle | A3_local | a3-rust | no | yes |  | lock sequences: r_other__b->r_main__a; r_other__b->r_main__a; no cycle |
| `34835028c1d2` | lock-order/cross_module_cycle | A3_whole | a3-rust | no | yes |  | lock sequences: r_main__a->r_other__b; r_main__a->r_other__b; no cycle |
| `c7d11e854196` | lock-order/cycle_3lock | A0_direct | rust | yes | yes |  | lock sequences: a->b; b->c; c->a; cycle among ['a', 'b', 'c'] |
| `970be001ab1e` | lock-order/cycle_3lock | A3_local | a3-rust | no | yes |  | lock sequences: r_main__a->r_main__b; r_main__b->r_main__c; r_main__a->r_main__c |
| `4e6e1cdc7910` | structure/nested_scope_lock_order | A0_direct | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `c9c01200389d` | structure/nested_scope_lock_order | A1_self_iter | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `70525c67bb7c` | structure/nested_scope_lock_order | A3_local | a3-rust | no | yes |  | lock sequences: r_main__a->r_main__b; r_main__a->r_main__b; no cycle |
| `77459d011aae` | structure/nested_scope_lock_order | A3_whole | a3-rust | no | yes |  | lock sequences: r_main__done; r_main__a->r_main__b; r_main__a->r_main__b; no cyc |
| `daa46ba4c2ba` | condvar/notify_one_multi_waiter_wrong_pick | A0_direct | rust | yes | yes |  | waiters=2; notify_one=yes; notify_all=no |
| `0e3cdb53c009` | condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | rust | no | yes |  | waiters=2; notify_one=no; notify_all=yes |
| `7de135daa5ad` | condvar/notify_one_multi_waiter_wrong_pick | A3_local | a3-rust | no | yes |  | waiters=2; notify_one=no; notify_all=yes |
| `13d1b761aaca` | channel/bounded_backpressure_lock_held | A0_direct | rust | no | yes |  | channel ops with a lock held: none |
| `1c28ef235a06` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes |  | channel ops with a lock held: none |
| `6c87085c602a` | channel/bounded_backpressure_lock_held | A3_local | a3-rust | no | no |  | channel ops with a lock held: none |
| `7f182a8fd554` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no |  | channel ops with a lock held: none |
| `8f09cd79b48c` | channel/send_while_holding_mutex | A0_direct | rust | no | yes |  | channel ops with a lock held: none |
| `05b2f4c77462` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes |  | channel ops with a lock held: none |
| `aa0668f905d8` | channel/send_while_holding_mutex | A3_local | a3-rust | no | no |  | channel ops with a lock held: none |
| `459df3234540` | channel/send_while_holding_mutex | A3_whole | a3-rust | no | no |  | channel ops with a lock held: none |
| `f42afd77e7a9` | semaphore/acquire_twice_no_release | A0_direct | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `8f59f2055f41` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `12cfc7f3617e` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `1901bf15e784` | semaphore/acquire_twice_no_release | A3_local | a3-rust | no | yes |  | acq=0,rel=0; acq=0,rel=0 |
| `84f0ae3ead6e` | semaphore/acquire_twice_no_release | A3_whole | a3-rust | no | yes |  | acq=0,rel=0; acq=0,rel=0 |
| `12195b60e3cb` | lock-order/abba_2lock | A0_direct | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `93eaf520459c` | lock-order/abba_2lock | A3_local | a3-rust | no | yes |  | lock sequences: r_main__a->r_main__b; r_main__a->r_main__b; no cycle |
| `e53544b9c364` | condvar/bare_wait_no_predicate | A0_direct | rust | unsure | yes | ambiguous_spec | waiters=1; notify_one=no; notify_all=yes |
| `394c5e1e06b6` | condvar/bare_wait_no_predicate | A1_self_iter | rust | unsure | yes | ambiguous_spec | waiters=1; notify_one=no; notify_all=yes |
| `52b496de2a26` | condvar/bare_wait_no_predicate | A3_whole | a3-rust | unsure | yes | ambiguous_spec | waiters=1; notify_one=yes; notify_all=no |
| `a4503ffe4461` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes |  | lock sequences: ma->mb; mb->ma; cycle among ['ma', 'mb'] |
| `1ff1454288bf` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | yes | yes |  | lock sequences: ma->mb; mb->ma; cycle among ['ma', 'mb'] |
| `f4db34f1644d` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes |  | lock sequences: r_main__a->r_main__b->r_main__done_a; r_main__a->r_main__b->r_ma |
| `9bab819deee2` | lock-order/cross_module_cycle | A0_direct | rust | no | yes |  | lock sequences: a->b; a->b; no cycle |
| `be85c5f756ec` | condvar/notify_one_multi_waiter_wrong_pick | A3_whole | a3-rust | no | yes |  | waiters=2; notify_one=no; notify_all=yes |
| `e443cf0e3e70` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no |  | channel ops with a lock held: none |
| `b607a97d0ac7` | channel/send_while_holding_mutex | A0_direct | rust | no | yes |  | channel ops with a lock held: none |
| `1bb2c14dc515` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes |  | channel ops with a lock held: none |
| `691d644c986c` | semaphore/acquire_twice_no_release | A0_direct | rust | yes | yes |  | acq=2,rel=1 |
| `c42fa5ece43f` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `d463f5f9f603` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `574a7804dc4d` | lock-order/partial_deadlock_bystander | A0_direct | rust | yes | yes |  | lock sequences: ma->mb; mb->ma; cycle among ['ma', 'mb'] |
| `a08bd1a020fa` | lock-order/partial_deadlock_bystander | A1_self_iter | rust | no | no |  | lock sequences: ma->mb; mb->ma; no cycle |
| `0951c33394bd` | lock-order/partial_deadlock_bystander | A3_whole | a3-rust | no | yes |  | lock sequences: r_main__a->r_main__b->r_main__done_a; r_main__a->r_main__b->r_ma |
| `98eed6be2b32` | channel/bounded_backpressure_lock_held | A2_tools_iter_ml | rust | no | yes |  | channel ops with a lock held: none |
| `6d7602ab8549` | channel/bounded_backpressure_lock_held | A3_whole | a3-rust | no | no |  | channel ops with a lock held: none |
| `ae1e465a6fbb` | channel/send_while_holding_mutex | A2_tools_iter_ml | rust | no | yes |  | channel ops with a lock held: none |
| `b6f7d6651e0e` | semaphore/acquire_twice_no_release | A1_self_iter | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `0501dcb3a49d` | semaphore/acquire_twice_no_release | A2_tools_iter_ml | rust | no | yes |  | acq=2,rel=2; acq=1,rel=1 |
| `90aee29b3ffa` | condvar/bare_wait_no_predicate | A3_whole | a3-rust | unsure | yes | ambiguous_spec | waiters=1; notify_one=yes; notify_all=no |
