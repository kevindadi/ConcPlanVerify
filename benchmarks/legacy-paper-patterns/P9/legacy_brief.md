# P9 legacy brief (fn_summary_prop)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model a producer and a consumer sharing a mutex-protected integer `result`. The producer locks, calls an unmodeled helper `compute`, writes result=1, and unlocks. The consumer locks, calls an unmodeled helper `observe` (which reads `result`), reads result, and unlocks. Provide function summaries for `compute` and `observe`. Declare goals that both threads reach return, and the model must verify safe with the goals reachable.
