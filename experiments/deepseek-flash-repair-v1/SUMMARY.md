# deepseek-flash-repair-v1 — summary

Batch: `/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/deepseek-flash-repair-v1/runs/run-20260918T093903-97467-f2db10`

- provider/model: `deepseek` / `deepseek-flash` (base `https://api.deepseek.com`)
- thinking: `{"thinking": {"type": "disabled"}}`; max_tokens=4096; timeout=90.0s
- actual HTTP attempts: **2** / 6 (remaining 4); stop: None

| task | root | calls | round 1 | round 2 | round 3 | status | source | validator | mode |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |
| r1_t2_abba | FAIL | 1 | accepted/replayed |  |  | accepted | llm | tool | external_single_patch |
| r2_t3_cross_module_abba | FAIL | 1 | accepted/replayed |  |  | accepted | llm | tool | external_single_patch |

Accepted on round 1 for both tasks, so the **live rejection-feedback retry was not triggered**; the feedback path is exercised by the offline scripted demos (`offline-demo/`, two rounds each: rejected then accepted).
Rust owns the verdict; the LLM only chose the target and the swap.
