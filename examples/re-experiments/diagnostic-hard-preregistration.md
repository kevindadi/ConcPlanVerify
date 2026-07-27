# Hard Diagnostic-Feedback Ablation: Frozen Protocol

This protocol was fixed before any model request for the hard rerun. No task
will be removed or replaced after outcomes are observed. The earlier three-case
pilot remains in `diagnostic-ablation.json` and is not pooled with this rerun.

## Tasks

| Task | Intended difficulty | Required repair class |
| --- | --- | --- |
| `dense_lock_graph` | Four concurrent workers, five locks, three acquisitions per worker, and one nonlocal inversion | Find and reorder the inverted worker while retaining all lock/drop operations |
| `partial_semaphore_goals` | Coupled zero-count semaphores, opposing mutex order, and two return goals | Move the rendezvous before lock acquisition and impose one lock order |
| `staged_lock_cycles` | Three sequentially gated, independent three-worker lock cycles; the verifier can expose distinct stage-local counterexamples | Repair all three fault sites, possibly over multiple verifier iterations |
| `mixed_three_stage` | Sequential channel/mutex blocking, three-lock cycle, and semaphore/mutex partial deadlock with goals | Repair three different mechanisms without deleting modeled behavior |

The deterministic reference CIR for each task is used only to establish that a
strictly preserving safe repair exists. It is never included in a model prompt.

## Conditions

- `self_repair` (reported as **Unguided**): the model is told that the CIR needs
  a concurrency-protocol repair, but receives no problem class, location,
  trace, state, SID slice, resource localization, or hint.
- `coarse` (reported as **LLM-only**): the model receives the current primary
  problem class and a generic class-level description, so it knows what failed
  but not where; no SID, resource/function localization, witness, state summary,
  or CIR slice is supplied.
- `structured`: the complete current verifier payload rendered by the released
  feedback formatter, including witness, involved resources/functions, CIR
  slice, preservation constraints, and repair hint where available.

Every cell uses DeepSeek V4 Flash (provider API model ID `deepseek-v4-pro`),
temperature 0, high reasoning, thinking enabled, the same initial CIR, a
16,384-token output limit, a maximum of five model repair rounds, and one run.
The output limit accommodates the largest 14 KB CIR without truncating the
required complete-JSON response.
Provider/transport failures are logged and retried without consuming a model
round, up to eight failures per cell.

## Outcomes

The primary endpoint is strict acceptance within five rounds:

$$
\mathrm{success} = (\mathrm{status}=\texttt{verified\_safe}) \land
\mathrm{Preserve}(CIR_0,CIR_r).
$$

`Preserve` requires exact equality of resources, protection entries, business
goals, function names/kinds, per-function SID sets, and per-function operation
multisets. Reordering and transfer rewiring are permitted. The runner records
raw verifier-safe candidates, strict successes, accepted round, all remaining
bug kinds, invalid candidates, preservation deviations, and token usage.

After the run, we strengthened the check to require exact equality of `entry`
and `fn_summaries` as well. All 12 accepted candidates also pass this stronger
check, so no reported outcome changes.

The four tasks and all three conditions will be reported regardless of whether
the result favors structured diagnostics.
