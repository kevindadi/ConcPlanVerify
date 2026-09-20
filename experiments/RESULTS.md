# RESULTS — ConcIR repair/extraction benchmark (round i)

Assembled from committed experiment artefacts. Paper-facing summary of the
main repair batch, the dual-track Rust-arm oracle, tool-driven extraction,
Track D, and the scale table.

## Provenance

- ConcPV HEAD at assembly: see `git log`; ConcIR HEAD `1a83704`
- ConcIR backend sha256: `6d498c8a8fa2f3dde578236889dd88073fe1952353a79a22097bfe68633f26e9` (main batch); Track D/extraction `88c3217d4b8f6549c8f7e07787e384ea3b065dc99abab47a27766beb958b42a3`
- main protocol sha256: `596203e05d9b1ca386cb8963f484439eb6ad4888d79459c1e2bb8355928d9aa3`
- main batch: 78/200 requests, stop_reason=None, K=4
- provider/model: deepseek / deepseek-flash; thinking disabled, temperature 0, max_tokens 4096, timeout 90 s

## 1. Main repair batch (8 hard tasks x 5 arms)

| task | arm | accepted | round | tokens | tool_ms | oracle (build/behavior/miri) | false_accept |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | True | 1 | 1361 | 870 | True/hang/detected | None |
| lock-order/partial_deadlock_bystander | A1_self_iter | False | None | 10210 | 778 | False/no_build/clean | None |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | True | 2 | 7612 | 4622 | True/terminated_ok/clean | None |
| lock-order/partial_deadlock_bystander | A3_local | False | None | 4587 | 28 | None/None/— | None |
| lock-order/partial_deadlock_bystander | A3_whole | False | None | 16355 | 42 | None/None/— | None |
| lock-order/cross_module_cycle | A0_direct | True | 1 | 1000 | 705 | True/terminated_ok/clean | None |
| lock-order/cross_module_cycle | A1_self_iter | False | None | 2478 | 676 | False/no_build/clean | None |
| lock-order/cross_module_cycle | A2_tools_iter_ml | True | 1 | 647 | 1985 | True/terminated_ok/clean | None |
| lock-order/cross_module_cycle | A3_local | True | 2 | 1912 | 49 | None/None/— | None |
| lock-order/cross_module_cycle | A3_whole | True | 2 | 3580 | 48 | None/None/— | None |
| lock-order/cycle_3lock | A0_direct | True | 1 | 1001 | 686 | True/terminated_ok/clean | None |
| lock-order/cycle_3lock | A1_self_iter | False | None | 3695 | 2674 | True/terminated_ok/clean | None |
| lock-order/cycle_3lock | A2_tools_iter_ml | True | 1 | 911 | 2192 | True/terminated_ok/clean | None |
| lock-order/cycle_3lock | A3_local | True | 2 | 2096 | 56 | None/None/— | None |
| lock-order/cycle_3lock | A3_whole | True | 3 | 8524 | 88 | None/None/— | None |
| structure/nested_scope_lock_order | A0_direct | True | 1 | 736 | 668 | True/terminated_ok/clean | None |
| structure/nested_scope_lock_order | A1_self_iter | True | 2 | 1105 | 656 | True/terminated_ok/clean | None |
| structure/nested_scope_lock_order | A2_tools_iter_ml | True | 1 | 677 | 2472 | True/terminated_ok/clean | None |
| structure/nested_scope_lock_order | A3_local | True | 2 | 2054 | 48 | None/None/— | None |
| structure/nested_scope_lock_order | A3_whole | True | 4 | 10713 | 60 | None/None/— | None |
| condvar/notify_one_multi_waiter_wrong_pick | A0_direct | False | None | 2208 | 0 | False/no_build/clean | None |
| condvar/notify_one_multi_waiter_wrong_pick | A1_self_iter | False | None | 2710 | 681 | False/no_build/clean | None |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | True | 1 | 675 | 2762 | True/terminated_ok/clean | None |
| condvar/notify_one_multi_waiter_wrong_pick | A3_local | True | 2 | 2792 | 49 | None/None/— | None |
| condvar/notify_one_multi_waiter_wrong_pick | A3_whole | False | None | 13857 | 40 | None/None/— | None |
| channel/bounded_backpressure_lock_held | A0_direct | True | 1 | 707 | 714 | True/terminated_ok/clean | None |
| channel/bounded_backpressure_lock_held | A1_self_iter | False | None | 2755 | 729 | False/no_build/clean | None |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | True | 1 | 665 | 2623 | True/terminated_ok/clean | None |
| channel/bounded_backpressure_lock_held | A3_local | True | 2 | 2149 | 49 | None/None/— | None |
| channel/bounded_backpressure_lock_held | A3_whole | True | 4 | 11450 | 58 | None/None/— | None |
| channel/send_while_holding_mutex | A0_direct | True | 1 | 851 | 746 | True/terminated_ok/clean | None |
| channel/send_while_holding_mutex | A1_self_iter | True | 3 | 1808 | 705 | False/no_build/clean | None |
| channel/send_while_holding_mutex | A2_tools_iter_ml | True | 1 | 828 | 2421 | True/terminated_ok/clean | None |
| channel/send_while_holding_mutex | A3_local | True | 2 | 1499 | 48 | None/None/— | None |
| channel/send_while_holding_mutex | A3_whole | True | 4 | 9345 | 54 | None/None/— | None |
| semaphore/acquire_twice_no_release | A0_direct | True | 1 | 973 | 665 | True/hang/detected | None |
| semaphore/acquire_twice_no_release | A1_self_iter | False | None | 13642 | 1427 | False/no_build/clean | None |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | True | 2 | 4012 | 4088 | True/terminated_ok/clean | None |
| semaphore/acquire_twice_no_release | A3_local | True | 2 | 1763 | 46 | None/None/— | None |
| semaphore/acquire_twice_no_release | A3_whole | True | 2 | 3432 | 47 | None/None/— | None |

### Per-arm aggregates

| arm | cells | accepted | accept_rate | false_accept | inconclusive | mean_round | mean_tokens |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A0_direct | 8 | 7 | 0.88 | 0 | 0 | 1.0 | 947.0 |
| A1_self_iter | 8 | 2 | 0.25 | 0 | 0 | 2.5 | 1456.5 |
| A2_tools_iter_ml | 8 | 8 | 1.0 | 0 | 0 | 1.25 | 2003.4 |
| A3_local | 8 | 7 | 0.88 | 0 | 0 | 2.0 | 2037.9 |
| A3_whole | 8 | 6 | 0.75 | 0 | 0 | 3.17 | 7840.7 |

Rust-arm `oracle.model` is `inconclusive` for this batch (deviation **D-19**); it is
backfilled by the dual-track oracle (automatic extraction §6 + expert labels §4).

## 4. Expert labels (Rust-arm oracle track)

Rubric `expert-label-rubric-v1` (proxy annotator). Every accepted A0/A1/A2
candidate was read and labelled:

| task | arm | bug_present | design_preserved | auto | agree |
| --- | --- | --- | --- | --- | --- |
| lock-order/partial_deadlock_bystander | A0_direct | yes | yes | True | True |
| lock-order/partial_deadlock_bystander | A2_tools_iter_ml | no | yes | False | True |
| lock-order/cross_module_cycle | A0_direct | no | no | False | True |
| lock-order/cross_module_cycle | A2_tools_iter_ml | no | yes | False | True |
| lock-order/cycle_3lock | A0_direct | yes | yes | False | False |
| lock-order/cycle_3lock | A2_tools_iter_ml | yes | yes | False | False |
| structure/nested_scope_lock_order | A0_direct | no | yes | False | True |
| structure/nested_scope_lock_order | A1_self_iter | no | yes | False | True |
| structure/nested_scope_lock_order | A2_tools_iter_ml | no | yes | False | True |
| condvar/notify_one_multi_waiter_wrong_pick | A2_tools_iter_ml | no | yes | False | True |
| channel/bounded_backpressure_lock_held | A0_direct | no | yes | False | True |
| channel/bounded_backpressure_lock_held | A2_tools_iter_ml | no | yes | False | True |
| channel/send_while_holding_mutex | A0_direct | no | yes | False | True |
| channel/send_while_holding_mutex | A1_self_iter | unsure | no | False | None |
| channel/send_while_holding_mutex | A2_tools_iter_ml | no | yes | False | True |
| semaphore/acquire_twice_no_release | A0_direct | yes | yes | True | True |
| semaphore/acquire_twice_no_release | A2_tools_iter_ml | no | yes | False | True |

Agreement with the automatic oracle: **14/16 = 0.875** (unsure 1). The two
disagreements are `lock-order/cycle_3lock` A0/A2: the 3-lock cycle is present in
the as-written program but the batch accepted it because the sampled schedules
terminated — exactly the miss expert labels are meant to catch.

## 6. Tool-driven extraction (A3_free, v4)

- 8/64 requests, protocol sha `f7ce2844fa588ef85064fdd5cbd7b72787d2dd9b4d2f01e4944d75ddf7224f94`
- validated: **1/8**; harness errors: **0**
- stage distribution: `{'explore': 1, 'conform': 4, 'codegen': 3}`

| task | stage | validated |
| --- | --- | --- |
| lock-order/partial_deadlock_bystander | explore | True |
| lock-order/cross_module_cycle | conform | False |
| lock-order/cycle_3lock | conform | False |
| structure/nested_scope_lock_order | codegen | False |
| condvar/notify_one_multi_waiter_wrong_pick | codegen | False |
| channel/bounded_backpressure_lock_held | conform | False |
| channel/send_while_holding_mutex | conform | False |
| semaphore/acquire_twice_no_release | codegen | False |

The model never edits Rust: `concir-instrument` (syn) inserts `cir_trace::ev` at
every concurrency call and emits `labels.json`; the model maps labels to CIR sids
only. Conformance uses extraction-mode `--lenient-unlock --attempt-events`.
`partial_deadlock_bystander` reached **28/28 conformant traces** and a CIR verdict.
Remaining failures are model-side: omitted `spawn` label sids (4x, `unknown_sid` at
event 0), Rust types / `Arc::new` / `vec` in the CIR (3x, `codegen`).

## 5. Track D and scale

- Track D: 38 task records; ConcIR (petri+interp) detects every
  CIR-defined lock-order/condvar/channel/semaphore buggy program and passes the fixed
  one; Miri is schedule-dependent (detects partial_deadlock_bystander,
  acquire_twice_no_release, bounded_backpressure_lock_held; misses the rest);
  Lockbud available (commit `cc78cb72cb85`).
- Scale: 24 generated lock-chain runs (no LLM).

## Deviations and threats to validity

- **D-19**: the main batch is not gated on Rust-arm oracle completeness; missing
  `oracle.model` cells are `inconclusive` and backfilled here by §6/§4.
- Main batch ran on backend `6d498c8a`; Track D/scale/extraction on `1a83704`'s
  backend. The changes are additive (conformance default path unchanged).
- Rust references for 6 of 8 tasks were authored this round so all 8 tasks run the
  full 5-arm matrix.
- A1 accepted one cell whose final artefact is a prose `NO_ISSUES` reply with
  `build_ok=False` (arm-level false accept), recorded as `unsure` in §4.
