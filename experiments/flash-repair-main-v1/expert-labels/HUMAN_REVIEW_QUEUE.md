# Human review queue (leave blank for the owner)

All expert/automatic disagreements plus a random sample (seed `20260920`).
Fill `human_label` (`yes`/`no`/`unsure`) and one reason; do not let an
agent fill it.

| task | arm | rep | sha256 | agent bug_present | auto | human_label | reason |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/cycle_3lock | A0_direct | 0 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | `c7d11e854196` | yes | False | | |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | `1ff1454288bf` | yes | False | | |
| lock-order/cycle_3lock | A0_direct | 1 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A0_direct | 2 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | `c7d11e854196` | yes | False | | |
| lock-order/partial_deadlock_bystander | A0_direct | 2 | `574a7804dc4d` | yes | True | | |
| lock-order/cross_module_cycle | A1_self_iter | 2 | `1fc6e7c5ea43` | no | False | | |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | 2 | `0e3cdb53c009` | unsure | False | | |
| semaphore/acquire_twice_no_release | A1_self_iter | 0 | `8f59f2055f41` | unsure | False | | |
