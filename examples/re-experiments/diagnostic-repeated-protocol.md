# Repeated Diagnostic-Feedback Experiment: Fixed Protocol

This protocol is fixed before any model request in the repeated experiment.
The earlier pilot and four-task hard ablation remain separate and are not
pooled with these results.

## Artifact

The experiment uses one newly constructed CIR, `interference_call_tree`, which
does not occur in the original nine-pattern benchmark. It contains five
processing regions, a two-level staged `spawn`/`join` tree, external call
summaries with nested callee relations, 20 synchronization resources, 24
modeled functions, and more than 150 statements. Four regions are safe
distractors.
The only concurrency defect is a three-worker lock cycle in one region. The
defective and reference CIRs have identical resources, functions, statement
IDs, calls, goals, and per-function operation multisets; the reference differs
only in the ordering of two lock/drop operations.

The reference CIR is used only to prove before the experiment that a strictly
preserving safe repair exists. It is never supplied to the model.

## Conditions

- **Unguided:** the model is told only that the CIR requires a concurrency
  repair. It receives no problem class or localization.
- **LLM-only:** the model is told that a deadlock exists and receives a generic
  deadlock repair description, but no function/resource/SID localization,
  witness, state summary, or CIR slice.
- **Structured:** the model receives the complete localized CVN diagnostic,
  including the witness and CIR slice.

All conditions use DeepSeek V4 Flash, temperature 0, the identical initial CIR,
the same preservation instruction, and at most three repair rounds. Each
condition is run in 10 separate requests, for 30 trials total. A later round
receives feedback for its own preceding candidate, according to the same
condition.

## Outcomes

The primary endpoint is the number of strict successes out of 10 within three
rounds:

$$
\mathrm{success} = (\mathrm{status}=\texttt{verified\_safe}) \land
\mathrm{Preserve}(CIR_0,CIR_r).
$$

`Preserve` requires exact equality of the program entry, resources, protection
relations, external-function summaries, business goals, function names/kinds,
per-function SID sets, and per-function operation multisets. Reordering and
control-flow rewiring are allowed. Secondary outcomes are first-round success,
accepted-round distribution, verifier-safe but behavior-dropping candidates,
invalid candidates, and token use. All 30 evaluable trials will be reported.
