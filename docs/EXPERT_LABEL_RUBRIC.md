# Expert label rubric (v1)

Used for the Rust-arm oracle's expert track. A label is attached to a **specific
candidate artefact** (sha256), by a named labeler.

## Fields

- `bug_present ∈ {yes, no, unsure}`: is a concurrency defect present in the
  program **as written**? Judge the code, not the intent, and not whether the
  tool chain happened to catch it.
- `defect_lines`: the source lines that carry the defect (empty if none).
- `design_preserved ∈ {yes, no}`: does the program keep the original
  design (same resources / modules / thread structure), as opposed to replacing
  it with a different mechanism?
- `reason`: one sentence.

## Labelers

- `agent-proxy`: an independent static reading of the candidate by the analysis
  agent (not the automatic oracle). For lock-order tasks it reconstructs the
  per-statement lock order and looks for a cycle; for condvar tasks it checks
  the notify/waiter relation; otherwise it returns `unsure`.
- `human`: the project owner. `HUMAN_REVIEW_QUEUE.md` lists the cells that need
  it: **every** expert/automatic disagreement, plus a random sample; the random
  seed is recorded. Human labels are left blank for the owner to fill.

## Agreement

Reported against the automatic oracle's `false_accept` (accepted AND
bug_present). `unsure` labels are excluded from the agreement denominator and
counted separately.
