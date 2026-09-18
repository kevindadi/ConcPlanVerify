# Flash smoke batch — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-arms-smoke-v1/run-20260918T120310-57899-32c2e1`
- protocol sha256: `242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff`
- requests used: 36
- stop reason: None

| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | 1 | 598 | 1891 | 708 |
| lock-order/abba_2lock | A1_self_iter | False | None | 4 | 1367 | 2458 | 945 |
| lock-order/abba_2lock | A2_tools_iter | True | 1 | 1 | 426 | 1294 | 1934 |
| lock-order/abba_2lock | A3_ours_revision | True | 1 | 1 | 2090 | 2190 | 0 |
| lock-order/abba_2lock | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/abba_2lock | A3_tool_repair | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A0_direct | True | 1 | 1 | 524 | 1178 | 691 |
| condvar/lost_wakeup_notify_before_wait | A1_self_iter | False | None | 4 | 1473 | 3825 | 957 |
| condvar/lost_wakeup_notify_before_wait | A2_tools_iter | True | 1 | 1 | 410 | 983 | 1758 |
| condvar/lost_wakeup_notify_before_wait | A3_ours_revision | False | None | 1 | 2259 | 2173 | 0 |
| condvar/lost_wakeup_notify_before_wait | A3p_ours_patch | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A3_tool_repair | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1 | 1048 | 3379 | 709 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4 | 6413 | 15619 | 2806 |
| lock-order/partial_deadlock_bystander | A2_tools_iter | False | None | 4 | 11178 | 10499 | 8880 |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False | None | 4 | 18893 | 9658 | 0 |
| lock-order/partial_deadlock_bystander | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A3_tool_repair | None | None | None | None | None | None |
