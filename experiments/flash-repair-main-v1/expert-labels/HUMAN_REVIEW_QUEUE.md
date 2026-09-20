# Human review queue (leave blank for the owner)

All expert/automatic disagreements, all `unsure`, plus a random 4 (seed `20260920`).
Fill `human_label` (`yes`/`no`/`unsure`) and one reason; do not let an agent fill it.

| task | arm | rep | sha256 | agent | auto | human_label | reason |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/cycle_3lock | A0_direct | 0 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 0 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A0_direct | 1 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 1 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A0_direct | 2 | `c7d11e854196` | yes | False | | |
| lock-order/cycle_3lock | A2_tools_iter_ml | 2 | `c7d11e854196` | yes | False | | |
| semaphore/acquire_twice_no_release | A0_direct | 0 | `f42afd77e7a9` | no | True | | |
| semaphore/acquire_twice_no_release | A0_direct | 2 | `f42afd77e7a9` | no | True | | |
| lock-order/partial_deadlock_bystander | A1_self_iter | 1 | `1ff1454288bf` | yes | False | | |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 1 | `d463f5f9f603` | no | False | | |
| lock-order/partial_deadlock_bystander | A3_whole | 2 | `0951c33394bd` | no | False | | |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | 2 | `0501dcb3a49d` | no | False | | |
| channel/bounded_backpressure_lock_held | A3_local | 0 | `6c87085c602a` | no | False | | |
