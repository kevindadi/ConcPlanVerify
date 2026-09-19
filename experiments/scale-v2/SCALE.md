# B5 — scale and cost (no LLM)

- ConcIR binary sha256: `88c3217d4b8f6549c8f7e07787e384ea3b065dc99abab47a27766beb958b42a3`

| threads | chain | max_states | outcome | complete | states | transitions | wall_ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | 2 | 5000 | PASS | True | 39 | 56 | 15 |
| 2 | 2 | 20000 | PASS | True | 39 | 56 | 15 |
| 2 | 2 | 200000 | PASS | True | 39 | 56 | 15 |
| 3 | 2 | 5000 | PASS | True | 218 | 437 | 18 |
| 3 | 2 | 20000 | PASS | True | 218 | 437 | 19 |
| 3 | 2 | 200000 | PASS | True | 218 | 437 | 18 |
| 4 | 2 | 5000 | PASS | True | 1267 | 3166 | 40 |
| 4 | 2 | 20000 | PASS | True | 1267 | 3166 | 41 |
| 4 | 2 | 200000 | PASS | True | 1267 | 3166 | 42 |
| 5 | 2 | 5000 | UNKNOWN | False | 5001 | 12650 | 121 |
| 5 | 2 | 20000 | PASS | True | 7780 | 22707 | 205 |
| 5 | 2 | 200000 | PASS | True | 7780 | 22707 | 201 |
| 3 | 3 | 5000 | PASS | True | 320 | 635 | 20 |
| 3 | 3 | 20000 | PASS | True | 320 | 635 | 20 |
| 3 | 3 | 200000 | PASS | True | 320 | 635 | 20 |
| 4 | 3 | 5000 | PASS | True | 1891 | 4606 | 57 |
| 4 | 3 | 20000 | PASS | True | 1891 | 4606 | 54 |
| 4 | 3 | 200000 | PASS | True | 1891 | 4606 | 57 |
| 5 | 3 | 5000 | UNKNOWN | False | 5001 | 11895 | 124 |
| 5 | 3 | 20000 | PASS | True | 11710 | 32877 | 318 |
| 5 | 3 | 200000 | PASS | True | 11710 | 32877 | 318 |
| 6 | 3 | 5000 | UNKNOWN | False | 5001 | 7976 | 99 |
| 6 | 3 | 20000 | UNKNOWN | False | 20001 | 43548 | 445 |
| 6 | 3 | 200000 | PASS | True | 78263 | 243020 | 2471 |

UNKNOWN marks the analysis bound being reached; it is not a safety verdict.
