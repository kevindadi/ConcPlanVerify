# Human review queue (leave blank for the owner)

All expert/automatic disagreements, all `unsure`, plus a random 4 (seed `20260920`).
Fill `human_label` (`yes`/`no`/`unsure`) and one reason; do not let an agent fill it.

| task                                   | arm              | rep | sha256         | agent  | auto  | human_label | reason |
| -------------------------------------- | ---------------- | --- | -------------- | ------ | ----- | ----------- | ------ |
| lock-order/cycle_3lock                 | A0_direct        | 0   | `c7d11e854196` | yes    | False | yes         |        |
| lock-order/cycle_3lock                 | A2_tools_iter_ml | 0   | `c7d11e854196` | yes    | False | yes         |        |
| lock-order/cycle_3lock                 | A0_direct        | 1   | `c7d11e854196` | yes    | False | yes         |        |
| lock-order/cycle_3lock                 | A2_tools_iter_ml | 1   | `c7d11e854196` | yes    | False | yes         |        |
| lock-order/cycle_3lock                 | A0_direct        | 2   | `c7d11e854196` | yes    | False | yes         |        |
| lock-order/cycle_3lock                 | A2_tools_iter_ml | 2   | `c7d11e854196` | yes    | False | yes         |        |
| semaphore/acquire_twice_no_release     | A0_direct        | 0   | `f42afd77e7a9` | no     | True  | no          |        |
| semaphore/acquire_twice_no_release     | A0_direct        | 2   | `f42afd77e7a9` | no     | True  | no          |        |
| lock-order/partial_deadlock_bystander  | A1_self_iter     | 1   | `1ff1454288bf` | yes    | False | yes         |        |
| condvar/bare_wait_no_predicate         | A0_direct        | 0   | `e53544b9c364` | unsure | False | no          |        |
| condvar/bare_wait_no_predicate         | A1_self_iter     | 0   | `394c5e1e06b6` | unsure | False | no          |        |
| condvar/bare_wait_no_predicate         | A3_whole         | 0   | `52b496de2a26` | unsure | False | no          |        |
| condvar/bare_wait_no_predicate         | A3_whole         | 2   | `90aee29b3ffa` | unsure | False | no          |        |
| semaphore/acquire_twice_no_release     | A1_self_iter     | 1   | `c42fa5ece43f` | no     | False | no          |        |
| lock-order/partial_deadlock_bystander  | A1_self_iter     | 2   | `a08bd1a020fa` | no     | False | no          |        |
| semaphore/acquire_twice_no_release     | A1_self_iter     | 2   | `b6f7d6651e0e` | no     | False | no          |        |
| channel/bounded_backpressure_lock_held | A3_local         | 0   | `6c87085c602a` | no     | False | no          |        |
