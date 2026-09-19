# Flash repair smoke — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d`
- requests used: 40
- stop reason: None

| task | arm | accepted | round | oracle.build | oracle.behavior | oracle.miri | oracle.model | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| lock-order/abba_2lock | A1_self_iter | True | 2 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| lock-order/abba_2lock | A2_tools_iter_m | True | 1 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| lock-order/abba_2lock | A2_tools_iter_ml | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/abba_2lock | A3_ours_revision | True | 2 | None | None | None | validated:PASS | False |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | True | hang | False | unverified:extracted CIR has invalid sids | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | False | no_build | False | inconclusive | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_m | True | 4 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 4 | True | terminated | False | unverified:extracted CIR has invalid sids | False |
| lock-order/partial_deadlock_bystander | A3_ours_revision | True | 3 | None | None | None | validated:PASS | False |
| condvar/bare_wait_no_predicate | A0_direct | True | 1 | True | terminated | False | unverified:extracted CIR has invalid sids | False |
| condvar/bare_wait_no_predicate | A1_self_iter | True | 2 | True | terminated | False | unverified:extracted CIR has invalid sids | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_m | True | 1 | True | terminated | False | unverified:extracted CIR has invalid sids | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | True | 1 | True | terminated | False | unverified:extracted CIR has invalid sids | False |
| condvar/bare_wait_no_predicate | A3_ours_revision | True | 4 | None | None | None | validated:PASS | False |
