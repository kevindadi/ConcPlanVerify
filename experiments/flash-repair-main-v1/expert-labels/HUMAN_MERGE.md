# Human review merge (17 rows -> 11 candidates)

`HUMAN_REVIEW_QUEUE.md` was filled by the owner (17 rows; `reason` left blank,
merged as `human_reason: null`). The 17 rows collapse to 11 distinct candidate
sha256 (the queue lists one row per (task, arm, rep); candidates repeat across
reps). Same-sha rows are consistent (no conflict).

- agent-proxy vs human: **7/7** agree (4 human-reviewed candidates had agent
  `unsure` and are excluded from the agent comparison; the human judged them).
- human vs automatic oracle: **9/11** agree.
- Human vs auto disagreements:
  - `semaphore/acquire_twice_no_release` A0 `f42afd77e7a9` — human **no**, auto
    **bug**. Auto basis: `oracle.behavior_status = hang` (the accepted A0
    program hangs under the 10 s behavior run) with `build_ok = true` and
    `miri_detected = true`, in all three reps (rep 0/1/2). The owner judged the
    code as defect-free; the disagreement is recorded, not resolved.
  - `lock-order/partial_deadlock_bystander` A1 `a08bd1a020fa` — human **no**,
    auto **bug** (aggregated `false_accept`).
  - (`lock-order/cycle_3lock` A0 `c7d11e854196` — human **yes**, auto clean: the
    expert catch; agrees with the agent-proxy label.)
