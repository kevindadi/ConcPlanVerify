# Conformance smoke

| case | status | holes | lint | traces | conformant | violation | timeout | hang_suspect | coverage |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| abba_2lock__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 9/9 |
| lost_wakeup__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 7/7 |
| permit_leak__fixed | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 5/5 |
| scope_bound__correct | ready | 0 | True | 58 | 58 | 0 | 0 | 0 | 7/7 |
| rendezvous_both_send__fixed | review | 0 | True | 58 | 0 | 58 | 0 | 0 | 0/0 |
| bounded_backpressure__fixed | review | 0 | True | 58 | 49 | 9 | 0 | 0 | 5/5 |
| rmw-zenoh-998__buggy | review | 0 | True | 58 | 24 | 0 | 34 | 0 | 9/9 |

A conformant trace means every observed concurrency step was a step
the verified model could take. It is not a claim that the code is correct.

## worker_with_payload — HOLE fill vs A3_free (live, 2 requests)

| arm | build | conformant | violation | missing | traces | coverage | lint |
| --- | --- | --- | --- | --- | --- | --- | --- |
| skeleton_fill | True | 66 | 0 | 0 | 66 | 5/5 | True |
| A3_free | True | 0 | 0 | 66 | 66 | 0/0 | n/a |

`A3_free` built but wrote no traces (the free program did not call `cir_trace::finish`), so its conformance is 0/66 missing — recorded as is.
