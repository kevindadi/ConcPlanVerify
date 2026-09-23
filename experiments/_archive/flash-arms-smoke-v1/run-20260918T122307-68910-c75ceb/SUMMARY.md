# Flash smoke batch — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-arms-smoke-v1/run-20260918T122307-68910-c75ceb`
- protocol sha256: `242984312502e5b2586021ffe974232028c3caada8737482738264bc0425c8ff`
- requests used: 42
- stop reason: None

| task | arm | accepted | round | http | tokens | llm_ms | tool_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | 1 | 563 | 2124 | 780 |
| lock-order/abba_2lock | A1_self_iter | False | None | 4 | 1367 | 3245 | 961 |
| lock-order/abba_2lock | A2_tools_iter | False | None | 4 | 7601 | 4185 | 7604 |
| lock-order/abba_2lock | A3_ours_revision | True | 1 | 1 | 2090 | 1856 | 18 |
| lock-order/abba_2lock | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/abba_2lock | A3_tool_repair | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A0_direct | True | 1 | 1 | 524 | 1300 | 709 |
| condvar/lost_wakeup_notify_before_wait | A1_self_iter | False | None | 4 | 1473 | 2637 | 1023 |
| condvar/lost_wakeup_notify_before_wait | A2_tools_iter | False | None | 4 | 7627 | 4044 | 7846 |
| condvar/lost_wakeup_notify_before_wait | A3_ours_revision | False | None | 1 | 2259 | 2101 | 5 |
| condvar/lost_wakeup_notify_before_wait | A3p_ours_patch | None | None | None | None | None | None |
| condvar/lost_wakeup_notify_before_wait | A3_tool_repair | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1 | 1076 | 3110 | 684 |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 4 | 6048 | 18001 | 394 |
| lock-order/partial_deadlock_bystander | A2_tools_iter | False | None | 4 | 11987 | 16766 | 307517 |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False | None | 4 | 18893 | 10576 | 100 |
| lock-order/partial_deadlock_bystander | A3p_ours_patch | None | None | None | None | None | None |
| lock-order/partial_deadlock_bystander | A3_tool_repair | None | None | None | None | None | None |
