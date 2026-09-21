# model-probe-v2 — SUMMARY (OpenCode Go, 10 tasks)

`https://opencode.ai/zen/go/v1` `/chat/completions`. Models: `kimi-k2.7-code` (kimi; **temperature forced to 1** by the provider) and `glm-5.3-flash` (glm; temperature 0). 10 MAIN tasks x 3 arms x 1 rep, K=4, BIN_MAIN. Not in the main table.

- run: `run-20260921T195024`; reused cells from earlier runs; `not_run` cells carry a reason.

## kimi-k2.7-code

requests 3; cells 30 (not_run 3)

| arm | cells | not_run | accepted | false_accept | tokens |
| --- | --- | --- | --- | --- | --- |
| A0_direct | 10 | 1 | 9 | 1 | 9802 |
| A2_tools_iter_ml | 10 | 1 | 8 | 0 | 16798 |
| A3_local | 10 | 1 | 9 | 0 | 25302 |

agent-proxy labels (accepted A0/A2, by sha): 6 candidates; bug_present yes 0, unsure 2.
A3_local accepted CIR -> a3-to-rust-v2 conform: {'FAIL': 1, 'PASS': 8}

## glm-5.3-flash

requests 0; cells 30 (not_run 0)

| arm | cells | not_run | accepted | false_accept | tokens |
| --- | --- | --- | --- | --- | --- |
| A0_direct | 10 | 0 | 9 | 1 | 1593 |
| A2_tools_iter_ml | 10 | 0 | 10 | 0 | 1386 |
| A3_local | 10 | 0 | 8 | 0 | 33884 |

agent-proxy labels (accepted A0/A2, by sha): 4 candidates; bug_present yes 0, unsure 2.
A3_local accepted CIR -> a3-to-rust-v2 conform: {'PASS': 8}

## vs DeepSeek Flash (rep 0)

| arm | model | accepted/10 | false_accept |
| --- | --- | --- | --- |
| A0_direct | kimi-k2.7-code | 9/10 | 1 |
| A2_tools_iter_ml | kimi-k2.7-code | 8/10 | 0 |
| A3_local | kimi-k2.7-code | 9/10 | 0 |
| A0_direct | glm-5.3-flash | 9/10 | 1 |
| A2_tools_iter_ml | glm-5.3-flash | 10/10 | 0 |
| A3_local | glm-5.3-flash | 8/10 | 0 |
| A0_direct | deepseek-flash | 10/10 | 3 |
| A2_tools_iter_ml | deepseek-flash | 10/10 | 0 |
| A3_local | deepseek-flash | 8/10 | 0 |

## Conclusion

- Both models reproduce the qualitative pattern: A2-ml and A3_local accept with few/no false-accepts; A0's `accepted` is trivial.
- kimi temperature 1 (provider constraint); 3 kimi cells `not_run` (APIConnectionError) — recorded, not hidden.

