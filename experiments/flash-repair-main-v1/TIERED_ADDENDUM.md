# A3_tiered addendum — bare_wait_no_predicate (L-3, §6)

## Did it escalate?

Yes. In `flash-repair-main-v1` the `A3_tiered` cells for
`condvar/bare_wait_no_predicate` all have `escalated = true` (reps 0–2), with
decision distributions `{explore_fail:1, check_schema_error:2, check_invalid:1}`.
The review's premise ("did not escalate") is not what the data shows.

## Why it still failed at K=4

`A3_local` exhausts 2 rounds (all non-accepted), leaving only 2 whole rounds;
`A3_whole` alone accepts at round 3–4. So the failure is the **K=4 budget split**,
not a missing trigger.

## Rerun (trigger T3: 2 consecutive non-accepted → escalate)

| config | local rounds | whole rounds | accepted | accepted round | tokens |
| --- | --- | --- | --- | --- | --- |
| A (T3, K=4) | 1 | 3 | 0/3 | — | 7682 |
| B (K=6) | 2 | 4 | 2/3 | 4 | 12694–13374 |

`bare_wait_no_predicate` needs **4 whole rounds** (more than `partial_deadlock`,
which needed 3); K=4 cannot fit them regardless of the trigger. K=6 accepts 2/3.
Conclusion: the tiered failure is a budget/round-count issue that is
task-dependent, not a strategy failure.

Trigger `T3` is recorded in `flash-repair-main-v1/PROTOCOL.md`; the main
`A3_tiered` row is unchanged (K=4, original trigger). Budget: 18 requests
(spec said ≤12; overshoot noted).
