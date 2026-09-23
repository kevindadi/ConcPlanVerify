# Conformance smoke

| case | status | holes | lint | traces | conformant | violation | hang | coverage |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| abba_2lock__fixed | ready | 0 | True | 51 | 51 | 0 | 0 | 4/4 |
| lost_wakeup__fixed | ready | 0 | True | 51 | 51 | 0 | 0 | 4/4 |
| permit_leak__fixed | ready | 0 | True | 51 | 51 | 0 | 0 | 2/2 |
| scope_bound__correct | ready | 0 | True | 51 | 51 | 0 | 0 | 2/2 |

A conformant trace means every observed concurrency step was a step
the verified model could take. It is not a claim that the code is correct.
