# TASKS — deepseek-flash-pilot-v1 (frozen)

Three modelling tasks, fixed before any model call. `TASKS.json` is the machine
form; contracts are in `contracts/`. All contracts are `deadlock_free` plus
preserved reachable completion, with standard bounds and lock reordering allowed.

| id | requirement (summary) | contract | structural check | expected modelling |
| --- | --- | --- | --- | --- |
| `t1_same_order` | two tasks `main::t1`,`main::t2`, same two mutexes `a`,`b`, one consistent order `a`→`b` | `contracts/t1_same_order.json` | `same_order_pair` | no inversion; ≥2 functions share the pair; locks balanced |
| `t2_abba` | `t1` locks `a`→`b`, `t2` locks `b`→`a` (existing ABBA), reproduced faithfully | `contracts/t2_abba.json` | `abba_inversion` | an inversion is present; locks balanced |
| `t3_cross_module_abba` | modules `main`/`other`; `main::t1` locks `main::a`→`other::b`, `other::t2` locks `other::b`→`main::a`, cross-module `requires.resources` | `contracts/t3_cross_module_abba.json` | `cross_module_abba` | cross-module inversion + cross-module resource refs; locks balanced |

Naming conventions, the exact requirements text, and the per-task structural
checks are fixed in `TASKS.json`; the full text is what was sent to the model.
Expected tool outcomes are recorded separately in the handoff: task 1 is expected
`PASS`/already-satisfied; tasks 2 and 3 are expected `FAIL` with the ABBA
reproduced faithfully. The structural check judges fidelity independently of the
tool verdict: a model that removes the inversion is a `modeling_mismatch`, not a
success.
