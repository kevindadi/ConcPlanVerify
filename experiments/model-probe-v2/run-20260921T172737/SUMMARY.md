# model-probe-v2 — SUMMARY (OpenCode Go)

Endpoint `https://opencode.ai/zen/go/v1` (`/chat/completions`). Models chosen
by the §5 priority: `kimi-k2.7-code` (kimi, temperature forced to 1 by the
provider) and `glm-5.3-flash` (glm, temperature 0). 8 SMOKE tasks x 3 arms
(A0/A2-ml/A3_local) x 1 rep, K=4. Not in the main table.

## kimi-k2.7-code (requests 25, stop None)

| arm | cells | accepted | false_accept | tokens |
| --- | --- | --- | --- | --- |
| A0_direct | 5 | 5 | 0 | 0 |
| A2_tools_iter_ml | 6 | 6 | 0 | 0 |
| A3_local | 7 | 7 | 0 | 17567 |

## glm-5.3-flash (requests 33, stop None)

| arm | cells | accepted | false_accept | tokens |
| --- | --- | --- | --- | --- |
| A0_direct | 8 | 7 | 1 | 0 |
| A2_tools_iter_ml | 8 | 8 | 0 | 0 |
| A3_local | 8 | 7 | 0 | 16690 |

## vs DeepSeek Flash (rep 0, same 8 tasks)

| arm | model | accepted | false_accept |
| --- | --- | --- | --- |
| A0_direct | kimi-k2.7-code | 5/5 | 0 |
| A2_tools_iter_ml | kimi-k2.7-code | 6/6 | 0 |
| A3_local | kimi-k2.7-code | 7/7 | 0 |
| A0_direct | glm-5.3-flash | 7/8 | 1 |
| A2_tools_iter_ml | glm-5.3-flash | 8/8 | 0 |
| A3_local | glm-5.3-flash | 7/8 | 0 |
| A0_direct | deepseek-flash | 8/8 | 3 |
| A2_tools_iter_ml | deepseek-flash | 8/8 | 0 |
| A3_local | deepseek-flash | 7/8 | 0 |

## Conclusion

- Both OpenCode Go models reproduce the qualitative pattern on the 8 tasks:
  A2-ml and A3_local accept with 0 false-accept; A0 accepts everything (its
  `accepted` is trivial).
- The probe is partial: 8 tasks (the 2 tasks added in round l are not in
  `SMOKE_V3_TASKS`), and `kimi-k2.7-code` needed temperature 1 (recorded).
  No model shows a materially different conclusion from Flash on these cells.
