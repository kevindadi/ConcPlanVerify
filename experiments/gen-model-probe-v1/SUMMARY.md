# gen-model-probe-v1 — generation with a frontier model

Model `kimi-k3` (OpenCode Go, chat/completions), 1 rep, K=4, temperature 0.0. Arms G0/G2/G3 on all 24 tasks; same oracle as the main batch (bounded monitor for Rust, exhaustive model + conform for G3).

| arm | cells | not_run | accepted | accept rate | RC | RF | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| G0_direct | 24 | 0 | 22 | 0.917 | 0.651 | 0.499 | 0 | 0 | 48968 |
| G2_tools_iter | 24 | 0 | 15 | 0.625 | 0.58 | 0.434 | 0 | 0 | 27763 |
| G3_concir | 24 | 0 | 17 | 0.708 | 0.776 | 0.684 | 0 | 10 | 133420 |
| **all** | 72 | 0 | 54 | 0.75 | 0.678 | 0.549 | 0 | 10 | 210151 |

Temperature is forced to 1 by the provider for models that reject 0; recorded per request in `llm/requests.jsonl`.
