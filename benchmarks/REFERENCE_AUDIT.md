# Reference-program audit (J-3)

Two `rust/fixed.rs` references were flagged by Track D. This audit reads the
programs, re-runs Miri (16 seeds) and Lockbud on all fixed references, and
classifies each flag as **(a) tool false positive**, **(b) real reference
defect**, or **(c) classification error**.

Binary for the ConcIR column: `BIN_MAIN 4bec943d…`. Miri seeds: 16. Raw tool
output is under `experiments/detection-v3/rust16/<task>_<side>/calls/`.

## Verdicts

### `lock-order/cycle_3lock` fixed — **(a) Lockbud false positive**

Lockbud reports two `DoubleLock` records on the fixed reference:

```
"first_lock_span":  "src/main.rs:29:13: 29:15",   # let gb = b2.lock()  (s2)
"second_lock_span": "src/main.rs:32:13: 32:15",   # let gb = b2.lock()  (s5)
"explanation": "The first lock is not released when acquiring the second lock"
```

Between the two locks the program explicitly drops the first guard:

```rust
let gb = b2.lock().unwrap(); // s2  (line 29)
drop(gb);                    // s3  (line 30)
drop(ga);                    // s4  (line 31)
let gb = b2.lock().unwrap(); // s5  (line 32)
```

Lockbud's `DoubleLock` does not model `drop(guard)` as a release, so it reads a
legal re-lock as a double lock. The reference acquires mutexes in the global
order `a < b < c` in every worker (w1: a,b; w2: a,b then b,c; w3: a,b,c) — no
cycle. ConcIR `PASS`, Miri `clean 16`. **Lockbud false positive; reference
correct.** (Note the mirror asymmetry: Lockbud is `clean` on the *buggy*
cycle and `detected` on the fixed one — a precision failure, not a reference
defect.)

### `lock-order/partial_deadlock_bystander` fixed — **(c) classification error (fixed)**

Track D previously showed Miri `detected 16` for the fixed reference. The raw
Miri output is:

```
error: the main thread terminated without waiting for all remaining threads
```

i.e. the **thread-leak** marker (the detached bystander loops forever and is
never joined), not a deadlock. It was mis-classified because
`classify_detection` scanned the whole stdout+stderr with a substring match, and
the cargo warning prints the probe path
`.../partial_deadlock_bystander_fixed/probe/target/miri/...`, whose
`partial_deadlock_bystander` component contains the token `deadlock`.

Fix (`python/cir_workflow/rust_arm.py`): detection tokens are now matched with a
left word boundary (`(?<![A-Za-z0-9_])deadlock`, likewise `data[- ]race`), so a
task name embedded in a path cannot count; and the Miri stem `deadlocked` is
matched. Regression: `python/tests/test_rust_arm.py::test_task_name_in_path_is_not_a_detection`.

After the fix the fixed reference is `thread_leak 16`, ConcIR `PASS`, Lockbud
`clean`: **no concurrency defect; the leak is the intentional detached
bystander.**

## All fixed references (Miri 16 seeds, Lockbud)

| task | concir.petri | miri (16) | lockbud |
| --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | PASS | thread_leak 16 | clean |
| lock-order/cross_module_cycle | PASS | clean 16 | clean |
| lock-order/cycle_3lock | PASS | clean 16 | detected |
| structure/nested_scope_lock_order | PASS | clean 16 | clean |
| condvar/notify_one_multi_waiter_wrong_pick | PASS | clean 16 | clean |
| channel/bounded_backpressure_lock_held | PASS | clean 16 | clean |
| channel/send_while_holding_mutex | PASS | clean 16 | clean |
| semaphore/acquire_twice_no_release | PASS | clean 16 | clean |

The only non-clean fixed rows are explained above (Lockbud false positive on
`cycle_3lock`; intentional thread leak on `partial_deadlock_bystander`).

## Impact and recomputation

- Track D (`experiments/detection-v3/TRACKD.json`) was **recomputed** with the
  fixed classifier and 16 seeds; the CIR-only tasks are unchanged.
- The classification fix changes the token set (`deadlocks` no longer matches
  as a bare substring). It can only make detection *stricter*, so it cannot have
  turned a real detection into a miss in the main batch; but A2-ml acceptance
  (`tools_green`) could in principle have been affected by an earlier false
  `detected` from a source comment. The main batch is **not** re-run (budget);
  this is recorded as a deviation in RESULTS/HANDOFF.
- No reference program was changed; no `provenance`/diff is owed.
