# Flash repair smoke — SUMMARY

- batch dir: `/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d`
- requests used: 40
- stop reason: None

| task | arm | accepted | round | oracle.build | oracle.behavior | oracle.miri | oracle.model | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | A0_direct | True | 1 | True | no_output | clean 16 | unverified | False |
| lock-order/abba_2lock | A1_self_iter | True | 2 | True | no_output | clean 16 | unverified | False |
| lock-order/abba_2lock | A2_tools_iter_m | True | 1 | True | no_output | clean 16 | unverified | False |
| lock-order/abba_2lock | A2_tools_iter_ml | True | 1 | True | no_output | clean 16 | unverified | False |
| lock-order/abba_2lock | A3_ours_revision | True | 2 | None | None | None | validated:PASS | False |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | True | hang | timeout 16 | unverified | True |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | False | no_build | False | inconclusive | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_m | True | 4 | True | no_output | clean 16 | unverified | False |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 4 | True | no_output | clean 16 | unverified | False |
| lock-order/partial_deadlock_bystander | A3_ours_revision | True | 3 | None | None | None | validated:PASS | False |
| condvar/bare_wait_no_predicate | A0_direct | True | 1 | True | no_output | clean 16 | unverified | False |
| condvar/bare_wait_no_predicate | A1_self_iter | True | 2 | True | no_output | clean 16 | unverified | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_m | True | 1 | True | no_output | clean 16 | unverified | False |
| condvar/bare_wait_no_predicate | A2_tools_iter_ml | True | 1 | True | no_output | clean 16 | unverified | False |
| condvar/bare_wait_no_predicate | A3_ours_revision | True | 4 | None | None | None | validated:PASS | False |

## oracle.model reasons

- lock-order/abba_2lock / A0_direct: annotated Rust did not build
- lock-order/abba_2lock / A1_self_iter: AttributeError: 'str' object has no attribute 'get'
- lock-order/abba_2lock / A2_tools_iter_m: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/lock-order__abba_2lock/A2_tools_iter_m/extracted.cir.json': missing field `kind` at line 53 column 9
- lock-order/abba_2lock / A2_tools_iter_ml: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/lock-order__abba_2lock/A2_tools_iter_ml/extracted.cir.json': missing field `kind` at line 54 column 9
- lock-order/partial_deadlock_bystander / A0_direct: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/lock-order__partial_deadlock_bystander/A0_direct/extracted.cir.json': invalid type: sequence, expected a string at line 72 column 13
- lock-order/partial_deadlock_bystander / A2_tools_iter_m: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/lock-order__partial_deadlock_bystander/A2_tools_iter_m/extracted.cir.json': invalid type: string "count", expected struct ParamDecl at line 12 column 19
- lock-order/partial_deadlock_bystander / A2_tools_iter_ml: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/lock-order__partial_deadlock_bystander/A2_tools_iter_ml/extracted.cir.json': missing field `kind` at line 78 column 9
- condvar/bare_wait_no_predicate / A0_direct: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3c/run-20260919T060244-87150-773a1c/condvar__bare_wait_no_predicate/A0_direct/extracted.cir.json': invalid type: sequence, expected a string at line 69 column 13
- condvar/bare_wait_no_predicate / A1_self_iter: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/condvar__bare_wait_no_predicate/A1_self_iter/extracted.cir.json': missing field `kind` at line 61 column 9
- condvar/bare_wait_no_predicate / A2_tools_iter_m: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/condvar__bare_wait_no_predicate/A2_tools_iter_m/extracted.cir.json': missing field `kind` at line 61 column 9
- condvar/bare_wait_no_predicate / A2_tools_iter_ml: extracted CIR not codegen-able: codegen failed: JSON parse error in '/Users/kevin/local-repos/ConcPlanVerify/experiments/flash-repair-smoke-v2/run-20260919T042328-54287-47223d/extraction-v3b/run-20260919T060118-84844-ff0127/condvar__bare_wait_no_predicate/A2_tools_iter_ml/extracted.cir.json': missing field `kind` at line 40 column 9

`behavior_status`: terminated_ok / terminated_wrong_state / hang / no_output / no_build. `oracle.model` is not truncated (see reasons section above).
