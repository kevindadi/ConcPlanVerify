# Conformance smoke

| case | status | holes | lint | traces | conformant | violation | timeout | hang_suspect | coverage |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| abba_2lock__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 9/9 |
| lost_wakeup__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 7/7 |
| permit_leak__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 5/5 |
| scope_bound__correct | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 7/7 |
| cross_module_cycle__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 9/9 |
| rendezvous_both_send__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 3/3 |
| bounded_backpressure__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 5/5 |
| send_while_holding_mutex__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 3/3 |
| rmw-zenoh-998__buggy | review | 0 | True | 58 | 44 | 0 | 14 | 0 | 9/9 |

A conformant trace means every observed concurrency step was a step
the verified model could take. It is not a claim that the code is correct.
