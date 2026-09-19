# Contract strength (frozen contracts, G-1)

build_families.py validates 21 buggy FAIL / all fixed PASS under the frozen contracts

| task | version | old outcome | frozen outcome | rejected by frozen contract |
| --- | --- | --- | --- | --- |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/abba_2lock | 1 | PASS | INVALID | — |
| lock-order/partial_deadlock_bystander | 4 | PASS | INVALID | — |
| lock-order/abba_2lock | 3 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| condvar/bare_wait_no_predicate | 4 | PASS | PASS | — |
| lock-order/abba_2lock | 2 | PASS | PASS | — |
| lock-order/partial_deadlock_bystander | 3 | PASS | FAIL | preserved: main::a holds [main::a, main::b] at once; preserved: main::b holds [main::a, main::b] at once |
| lock-order/abba_2lock | 1 | PASS | PASS | — |
| lock-order/cycle_3lock | 1 | PASS | PASS | — |
