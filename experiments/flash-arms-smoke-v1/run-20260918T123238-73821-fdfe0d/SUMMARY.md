# Flash smoke batch — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-arms-smoke-v1/run-20260918T123238-73821-fdfe0d`
- protocol sha256: `242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff`
- requests used: 36
- stop reason: None

| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | 1 | 598 | 1637 | 682 |
| lock-order/abba_2lock | A1_self_iter | False | None | 4 | 1367 | 2579 | 911 |
| lock-order/abba_2lock | A2_tools_iter | True | 1 | 1 | 426 | 1130 | 2552 |
| lock-order/abba_2lock | A3_ours_revision | True | 1 | 1 | 2090 | 2231 | 15 |
| lock-order/abba_2lock | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/abba_2lock | A3_tool_repair | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A0_direct | True | 1 | 1 | 524 | 1061 | 677 |
| condvar/lost_wakeup_notify_before_wait | A1_self_iter | False | None | 4 | 1473 | 3258 | 928 |
| condvar/lost_wakeup_notify_before_wait | A2_tools_iter | True | 1 | 1 | 410 | 1101 | 2565 |
| condvar/lost_wakeup_notify_before_wait | A3_ours_revision | False | None | 1 | 2259 | 2390 | 5 |
| condvar/lost_wakeup_notify_before_wait | A3p_ours_patch | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A3_tool_repair | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1 | 808 | 1893 | 722 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4 | 5454 | 11421 | 2161 |
| lock-order/partial_deadlock_bystander | A2_tools_iter | False | None | 4 | 16680 | 29325 | 9856 |
| lock-order/partial_deadlock_bystander | A3_ours_revision | True | 4 | 4 | 18893 | 10157 | 105 |
| lock-order/partial_deadlock_bystander | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A3_tool_repair | None | None | None | None | None | None |
