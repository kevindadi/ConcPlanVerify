# Flash repair smoke — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v1/run-20260918T161254-8755-bf0e2a`
- requests used: 30
- stop reason: None

| task | arm | accepted | round | oracle.build | oracle.miri | oracle.model | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | True | False | None | None |
| lock-order/abba_2lock | A1_self_iter | True | 2 | True | False | None | None |
| lock-order/abba_2lock | A2_tools_iter_m | True | 1 | True | False | None | None |
| lock-order/abba_2lock | A2_tools_iter_ml | True | 1 | True | False | None | None |
| lock-order/abba_2lock | A3_ours_revision | True | 2 | None | None | False | False |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | True | False | None | None |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | False | False | None | None |
| lock-order/partial_deadlock_bystander | A2_tools_iter_m | False | None | True | False | None | None |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | False | None | True | False | None | None |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False | None | None | None | True | False |
| condvar/bare_wait_no_predicate | A0_direct | True | 1 | True | False | None | None |
| condvar/bare_wait_no_predicate | A1_self_iter | True | 2 | True | False | None | None |
| condvar/bare_wait_no_predicate | A2_tools_iter_m | True | 1 | True | False | None | None |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | True | 1 | True | False | None | None |
| condvar/bare_wait_no_predicate | A3_ours_revision | False | None | None | None | True | False |
