# model-probe-v2 — PROTOCOL (frozen 2026-09-2xm)

Second/third-model probe on the OpenCode Go endpoint (OpenAI-compatible
`https://opencode.ai/zen/go/v1`, `Authorization: Bearer $OPENCODE_API_KEY`).
Only `/chat/completions` is used (reusing the existing client + identity check);
`/responses` (GPT/Grok/Muse) and `/messages` (MiniMax/Qwen) models are out of
scope.

- `GET /models` stored verbatim in `MODELS.json`.
- Model choice by priority, two different families: `kimi-k2.7-code` (kimi),
  `glm-5.3-flash` (glm). `deepseek-*` excluded (same family as the main model).
- Params aligned with the main protocol: temperature 0, `max_tokens` 4096,
  timeout 90 s. `kimi-k2.7-code` only accepts temperature 1; the client retries
  with 1 and records `temperature` in `requests.jsonl`.
- Identity check: the response `model` must equal the requested model.
- Budget: `LiveBudget` 70 requests + 7200 s wall per model, cost cap $8.
- Arms: `A0_direct`, `A2_tools_iter_ml`, `A3_local`; 1 rep; K=4; same BIN_MAIN,
  frozen contracts, de-leaked inputs. Accepted A0/A2 Rust get behavior + Miri 16.

Known limitation: the task list is `SMOKE_V3_TASKS` (8 tasks); the 2 tasks added
in round l (`abba_2lock`, `bare_wait_no_predicate`) are not included.
