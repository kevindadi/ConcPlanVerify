# Flash smoke batch — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-arms-smoke-v1/run-20260918T115401-56019-0eaab7`
- protocol sha256: `242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff`
- requests used: 33
- stop reason: None

| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | 1 | 588 | 1517 | 1164 |
| lock-order/abba_2lock | A1_self_iter | False | None | 4 | 1367 | 2684 | 976 |
| lock-order/abba_2lock | A2_tools_iter | True | 1 | 1 | 426 | 910 | 1972 |
| lock-order/abba_2lock | A3_ours_revision | True | 1 | 1 | 2090 | 0 | 0 |
| lock-order/abba_2lock | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/abba_2lock | A3_tool_repair | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A0_direct | True | 1 | 1 | 524 | 1109 | 689 |
| condvar/lost_wakeup_notify_before_wait | A1_self_iter | False | None | 4 | 1473 | 3007 | 970 |
| condvar/lost_wakeup_notify_before_wait | A2_tools_iter | True | 1 | 1 | 410 | 1207 | 1820 |
| condvar/lost_wakeup_notify_before_wait | A3_ours_revision | False | None | 1 | 2259 | 0 | 0 |
| condvar/lost_wakeup_notify_before_wait | A3p_ours_patch | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A3_tool_repair | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1 | 1276 | 3487 | 723 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4 | 4807 | 11928 | 1630 |
| lock-order/partial_deadlock_bystander | A2_tools_iter | True | 1 | 1 | 954 | 2570 | 300742 |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False | None | 4 | 16058 | 0 | 0 |
| lock-order/partial_deadlock_bystander | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A3_tool_repair | None | None | None | None | None | None |
