# deepseek-flash-pilot-v1 — results summary

Batch: `/Users/kevin/paper-review/papers/ConcPlanVerify/experiments/deepseek-flash-pilot-v1/runs/run-20260918T090459-72384-02cd06`

- provider/model: `deepseek` / `deepseek-flash` (base `https://api.deepseek.com`)
- thinking: `{"thinking": {"type": "disabled"}}`; max_tokens=4096; per-request timeout=90.0s
- actual HTTP attempts: **3** / 12 (remaining 9)
- batch stop reason: None

| task | calls | check | support | explore | complete | modeling_ok | repair (tool) | replay | status |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| t1_same_order | 1 | valid | supported | pass | True | True | None | None | already_satisfied |
| t2_abba | 1 | valid | supported | fail | True | True | repaired | replayed | repaired |
| t3_cross_module_abba | 1 | valid | supported | fail | True | True | repaired | replayed | repaired |

Separate dimensions: `check`/`support` success is not a property result; `explore` is the property verification; `modeling_ok` is the independent structural fidelity check; `repair` is the **backend tool** repair (not an LLM patch).
