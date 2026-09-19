# Flash repair smoke — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d`
- requests used: 40
- stop reason: None

| task | arm | accepted | round | oracle.build | oracle.behavior | oracle.miri | oracle.model | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/abba_2lock | A1_self_iter | True | 2 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/abba_2lock | A2_tools_iter_m | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/abba_2lock | A2_tools_iter_ml | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/abba_2lock | A3_ours_revision | True | 2 | None | None | None | validated:PASS | False |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | True | hang | False | unverified:extracted CIR not codegen-able: codegen failed: J | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | False | no_build | False | inconclusive | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_m | True | 4 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 4 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| lock-order/partial_deadlock_bystander | A3_ours_revision | True | 3 | None | None | None | validated:PASS | False |
| condvar/bare_wait_no_predicate | A0_direct | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| condvar/bare_wait_no_predicate | A1_self_iter | True | 2 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_m | True | 1 | True | terminated | False | unverified:extracted CIR not codegen-able: codegen failed: J | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | True | 1 | True | terminated | False | unverified:reply not a {cir,rust} object | False |
| condvar/bare_wait_no_predicate | A3_ours_revision | True | 4 | None | None | None | validated:PASS | False |

## v1 → v2 comparison

| task | arm | v1 accept/round/tokens | v2 accept/round/tokens | change | reason |
| --- | --- | --- | --- | --- | --- |
| condvar/bare_wait_no_predicate | A0_direct | True/1/1029 | True/1/706 | same | de-leak |
| condvar/bare_wait_no_predicate | A1_self_iter | True/2/1552 | True/2/1054 | same | de-leak |
| condvar/bare_wait_no_predicate | A2_tools_iter_m | True/1/920 | True/1/616 | same | de-leak |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | True/1/916 | True/1/616 | same | de-leak |
| condvar/bare_wait_no_predicate | A3_ours_revision | False/None/10138 | True/4/10689 | changed | de-leak + schema/normalize + diagnostics |
| lock-order/abba_2lock | A0_direct | True/1/975 | True/1/765 | same | de-leak |
| lock-order/abba_2lock | A1_self_iter | True/2/1593 | True/2/1149 | same | de-leak |
| lock-order/abba_2lock | A2_tools_iter_m | True/1/861 | True/1/675 | same | de-leak |
| lock-order/abba_2lock | A2_tools_iter_ml | True/1/941 | True/1/675 | same | de-leak |
| lock-order/abba_2lock | A3_ours_revision | True/2/3157 | True/2/3281 | same | de-leak + schema/normalize + diagnostics |
| lock-order/partial_deadlock_bystander | A0_direct | True/1/1956 | True/1/1337 | same | de-leak |
| lock-order/partial_deadlock_bystander | A1_self_iter | False/None/10700 | False/None/16443 | same | de-leak |
| lock-order/partial_deadlock_bystander | A2_tools_iter_m | False/None/13117 | True/4/12467 | changed | de-leak |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | False/None/13156 | True/4/15184 | changed | de-leak |
| lock-order/partial_deadlock_bystander | A3_ours_revision | False/None/14752 | True/3/9689 | changed | de-leak + schema/normalize + diagnostics |
