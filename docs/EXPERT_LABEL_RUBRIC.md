# Expert label rubric (v2)

A label is attached to a **specific candidate artefact** (sha256); several cells
may share one artefact (deduplicated), recorded in `cells:
[(task, arm, rep) …]`.

## Fields

- `bug_present ∈ {yes, no, unsure}` — is a concurrency defect present in the
  program **as written**? Judge the code, not the intent, and not whether a tool
  happened to catch it.
- `evidence` — the **reasoning**: for each thread the acquisition sequence of
  locks / semaphores / channels / condvars, and whether a cycle or a lost wakeup
  exists. Required for every label.
- `design_preserved ∈ {yes, no}` — does the program keep the original design
  (resources, modules, thread structure) judged against `requirements.txt` and
  the frozen contract's `preserved` list? For lock-order tasks this means the
  nested `holds_all` acquisition is retained.
- `unsure_reason ∈ {needs_execution, unfamiliar_api, ambiguous_spec}` — required
  when `bug_present = unsure`. Target: `unsure ≤ 15%`.

## Labelers

- `agent-proxy` — an independent static reading by the analysis agent (not the
  automatic oracle): per-thread lock-order cycle detection (guard-drop and
  brace-scope aware), the condvar notify/waiter relation, lock-held-across-
  channel detection, and semaphore acquire/release balance.
- `human` — the project owner. `HUMAN_REVIEW_QUEUE.md` lists every
  expert/automatic disagreement, every `unsure`, and a random sample (seed
  recorded); the human columns are left blank.

## Metrics

- **agreement** against the automatic oracle's `false_accept`, at cell level.
- **design_loss** = accepted cells whose candidate has `design_preserved = no`,
  counted by arm.
