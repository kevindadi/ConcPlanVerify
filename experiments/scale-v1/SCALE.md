# B5 — scale and cost (no LLM)

- ConcIR binary sha256: `73b5dd64012e5c53c4fb0736a553c8b6d37d6bb99b55766c573136ec9c364794`

| threads | chain | max_states | outcome | complete | states | transitions | wall_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | 2 | 5000 | PASS | True | 39 | 56 | 3 |
| 2 | 2 | 20000 | PASS | True | 39 | 56 | 3 |
| 2 | 2 | 200000 | PASS | True | 39 | 56 | 3 |
| 3 | 2 | 5000 | PASS | True | 218 | 437 | 6 |
| 3 | 2 | 20000 | PASS | True | 218 | 437 | 6 |
| 3 | 2 | 200000 | PASS | True | 218 | 437 | 6 |
| 4 | 2 | 5000 | PASS | True | 1267 | 3166 | 29 |
| 4 | 2 | 20000 | PASS | True | 1267 | 3166 | 29 |
| 4 | 2 | 200000 | PASS | True | 1267 | 3166 | 29 |
| 5 | 2 | 5000 | UNKNOWN | False | 5001 | 12650 | 117 |
| 5 | 2 | 20000 | PASS | True | 7780 | 22707 | 206 |
| 5 | 2 | 200000 | PASS | True | 7780 | 22707 | 218 |
| 3 | 3 | 5000 | PASS | True | 320 | 635 | 8 |
| 3 | 3 | 20000 | PASS | True | 320 | 635 | 8 |
| 3 | 3 | 200000 | PASS | True | 320 | 635 | 8 |
| 4 | 3 | 5000 | PASS | True | 1891 | 4606 | 45 |
| 4 | 3 | 20000 | PASS | True | 1891 | 4606 | 44 |
| 4 | 3 | 200000 | PASS | True | 1891 | 4606 | 44 |
| 5 | 3 | 5000 | UNKNOWN | False | 5001 | 11895 | 119 |
| 5 | 3 | 20000 | PASS | True | 11710 | 32877 | 327 |
| 5 | 3 | 200000 | PASS | True | 11710 | 32877 | 326 |
| 6 | 3 | 5000 | UNKNOWN | False | 5001 | 7976 | 92 |
| 6 | 3 | 20000 | UNKNOWN | False | 20001 | 43548 | 478 |
| 6 | 3 | 200000 | PASS | True | 78263 | 243020 | 2638 |

UNKNOWN marks the analysis bound being reached; it is not a safety verdict.
