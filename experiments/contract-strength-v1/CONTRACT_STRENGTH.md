# Contract strength (F-1)

A3 accepted CIRs re-checked offline against a *strengthened* contract derived
from each CIR's own names: deadlock_free + every scope member's completion +
`holds_all` for any function that locks >=2 mutexes + `var_eq ready` when a
`ready` Var exists. `new_outcome` is the verdict; `rejected` lists the failing
preserved properties.

Totals over 14 accepted versions: PASS 12, FAIL 2, INVALID 0.

| batch/task | accepted ver | old | new | rejected |
| --- | --- | --- | --- | --- |
| smoke-c/d:run-20260918T115401-56019-0eaab7 | 1 | accepted | PASS | — |
| smoke-c/d:run-20260918T120310-57899-32c2e1 | 1 | accepted | PASS | — |
| smoke-c/d:run-20260918T120547-59407-fed5a7 | 1 | accepted | PASS | — |
| smoke-c/d:run-20260918T122307-68910-c75ceb | 1 | accepted | PASS | — |
| smoke-c/d:run-20260918T123238-73821-fdfe0d | 1 | accepted | PASS | — |
| smoke-c/d:run-20260918T123238-73821-fdfe0d | 4 | accepted | FAIL | preserved: main::bystander completes |
| smoke-c/d:offline | 1 | accepted | PASS | — |
| smoke-c/d:offline | 1 | accepted | PASS | — |
| v1:run-20260918T140756-98699-bf59df | 3 | accepted | PASS | — |
| v1:run-20260918T150907-4893-65a616 | 2 | accepted | PASS | — |
| v1:run-20260918T161254-8755-bf0e2a | 2 | accepted | PASS | — |
| v2:run-20260919T042328-54287-47223d | 4 | accepted | PASS | — |
| v2:run-20260919T042328-54287-47223d | 2 | accepted | PASS | — |
| v2:run-20260919T042328-54287-47223d | 3 | accepted | FAIL | preserved: main::a holds ['main::a', 'main::b']; preserved: main::b holds ['main::b', 'main::a']; preserved: main::c holds ['main::a', 'main::b'] |
