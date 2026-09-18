# Flash smoke batch — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-arms-smoke-v1/run-20260918T120547-59407-fed5a7`
- protocol sha256: `242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff`
- requests used: 36
- stop reason: None

| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | 1 | 598 | 1712 | 719 |
| lock-order/abba_2lock | A1_self_iter | False | None | 4 | 1371 | 2586 | 959 |
| lock-order/abba_2lock | A2_tools_iter | True | 1 | 1 | 426 | 1475 | 2249 |
| lock-order/abba_2lock | A3_ours_revision | True | 1 | 1 | 2090 | 1984 | 16 |
| lock-order/abba_2lock | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/abba_2lock | A3_tool_repair | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A0_direct | True | 1 | 1 | 524 | 1151 | 693 |
| condvar/lost_wakeup_notify_before_wait | A1_self_iter | False | None | 4 | 1473 | 3034 | 965 |
| condvar/lost_wakeup_notify_before_wait | A2_tools_iter | True | 1 | 1 | 444 | 1137 | 1807 |
| condvar/lost_wakeup_notify_before_wait | A3_ours_revision | False | None | 1 | 2259 | 2098 | 6 |
| condvar/lost_wakeup_notify_before_wait | A3p_ours_patch | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A3_tool_repair | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1 | 818 | 1882 | 693 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4 | 15280 | 51671 | 2450 |
| lock-order/partial_deadlock_bystander | A2_tools_iter | False | None | 4 | 11709 | 12444 | 8604 |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False | None | 4 | 18893 | 9559 | 101 |
| lock-order/partial_deadlock_bystander | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A3_tool_repair | None | None | None | None | None | None |
