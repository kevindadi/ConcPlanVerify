# Rust-arm bounded oracle (§1 acceptance)

Instrument v2 (wrapper types) -> build -> native 32 runs -> `concir-backend monitor`, resource names auto-mapped by kind/declaration order. Verdicts are **bounded**.

| task | side | built | behavior | hang | RC | RF | monitor FAIL | blocked reqs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/abba_2lock | fixed | True | True | False | 0.89 | 0.89 | [] | {} |
| lock-order/abba_2lock | buggy | True | False | True | 0.89 | 0.00 | [] | {'R1': 'not_observed', 'R2': 'not_observed', 'R3': 'not_observed', 'R6': 'not_observed'} |
| lock-order/cycle_3lock | fixed | True | True | False | 0.90 | 0.90 | [] | {} |
| lock-order/cycle_3lock | buggy | True | False | True | 0.90 | 0.00 | [] | {'R1': 'not_observed', 'R2': 'not_observed', 'R3': 'not_observed', 'R4': 'not_observed', 'R7': 'not_observed'} |
| lock-order/cross_module_cycle | fixed | True | True | False | 0.90 | 0.90 | [] | {} |
| lock-order/cross_module_cycle | buggy | True | False | True | 0.90 | 0.00 | [] | {'R1': 'not_observed', 'R2': 'not_observed', 'R3': 'not_observed', 'R4': 'not_observed', 'R7': 'not_observed'} |
| lock-order/partial_deadlock_bystander | fixed | True | True | False | 0.83 | 0.83 | [] | {} |
| lock-order/partial_deadlock_bystander | buggy | True | False | True | 0.83 | 0.00 | [] | {'R1': 'not_observed', 'R3': 'not_observed', 'R4': 'not_observed', 'R5': 'not_observed', 'R7': 'not_observed', 'R8': 'not_observed'} |
| condvar/bare_wait_no_predicate | fixed | True | True | False | 0.40 | 0.40 | [] | {'R3': 'unsupported', 'R4': 'unsupported', 'R8': 'unsupported', 'R9': 'unsupported'} |
| condvar/bare_wait_no_predicate | buggy | True | False | True | 0.40 | 0.20 | [] | {'R3': 'unsupported', 'R4': 'unsupported', 'R8': 'unsupported', 'R9': 'unsupported'} |
| condvar/notify_one_multi_waiter_wrong_pick | fixed | True | True | False | 0.80 | 0.80 | [] | {} |
| condvar/notify_one_multi_waiter_wrong_pick | buggy | True | False | True | 0.80 | 0.00 | [] | {'R4': 'not_observed', 'R6': 'not_observed'} |
| channel/bounded_backpressure_lock_held | fixed | True | True | False | 0.75 | 0.75 | [] | {} |
| channel/bounded_backpressure_lock_held | buggy | True | False | True | 0.75 | 0.00 | [] | {'R1': 'not_observed'} |
| channel/send_while_holding_mutex | fixed | True | True | False | 0.80 | 0.80 | [] | {} |
| channel/send_while_holding_mutex | buggy | True | False | True | 0.80 | 0.00 | [] | {} |
| semaphore/acquire_twice_no_release | fixed | True | True | False | 0.57 | 0.57 | [] | {'R2': 'unmapped', 'R3': 'unmapped'} |
| semaphore/acquire_twice_no_release | buggy | True | False | True | 0.86 | 0.00 | [] | {'R1': 'not_observed', 'R2': 'not_observed', 'R3': 'not_observed'} |
| structure/nested_scope_lock_order | fixed | True | True | False | 0.57 | 0.57 | [] | {} |
| structure/nested_scope_lock_order | buggy | True | False | True | 0.57 | 0.00 | [] | {'R3': 'not_observed', 'R5': 'not_observed'} |

- fixed references: **8/10** have every non-`[U]` requirement `PASS_bounded`.
- buggy references: **10/10** show a hang or a monitor `FAIL` (here: all hang).

## Why a fixed reference can fall short of 100%

- `condvar/bare_wait_no_predicate`: its requirements about the shared flag are `var_eq` predicates. A free-Rust trace carries no value events, so the monitor reports `unsupported` — an **instrument limitation**, not a program defect (behavior is `DONE ready=true`, all runs complete).
- `semaphore/acquire_twice_no_release`: the reference implements its own semaphore over `Mutex`+`Condvar`, so there is no `Semaphore` runtime resource to align to `main::s`; those clauses are `unmapped` — an **alignment limitation**. Its `function_completed` and `deadlock_free` clauses pass.
- The remaining shortfall is `[U]` terminal-line requirements, which are checked by behavior rather than the contract monitor and so are not counted as `PASS_bounded`.

## Deviation

- Traces here are the 32 native runs; Miri seeds are wired (`rust_oracle.run_miri`) but were not run at 16 seeds for all 20 artifacts this round (runtime). `deadlock_free` is resolved by native behavior.
