# flash-gen-main-v1 — protocol (frozen)

Requirements -> program generation benchmark. Target: 24 tasks across
lock-order / condvar / channel / semaphore / atomic-data / structure, tiers
8 Simple / 8 Medium / 8 Complex (`benchmarks/GENERATION_MANIFEST.json`).

## Arms (contract hidden in all four)

| arm | input | flow | oracle |
| --- | --- | --- | --- |
| `G0_direct` | `REQUIREMENTS.md` | one-shot Rust | bounded monitor |
| `G1_self_iter` | `REQUIREMENTS.md` | generate -> self-review rounds | bounded monitor |
| `G2_tools_iter` | `REQUIREMENTS.md` | generate -> build + Miri 16 + Lockbud feedback | bounded monitor |
| `G3_concir` | `REQUIREMENTS.md` | generate CIR (contract hidden) -> normalize/check -> explore vs `contract.json` -> whole-artifact revision -> codegen -> build -> conform -> behavior | exhaustive model verdict + conform |

K = 4 rounds, 3 reps, DeepSeek Flash, temperature 0. The verification contract
is **never** included in a model prompt; only explore/revision feedback (which
names failed properties) is fed back in G3's later rounds.

## Metrics

- **RC** = decidable requirements / total; **RF** = satisfied requirements / total
  (`PASS` for G3, `PASS_bounded` for the Rust arms).
- **defect** = accepted and (behavior hang or monitor FAIL) — expert track and
  Miri-16 on the accepted artifact are recorded where run; the model arm's
  `accepted-with-proof` = model PASS and conform PASS.
- cost: tokens, rounds, `llm_ms` / `tool_ms` / `verify_ms`.
- aggregated per tier (Simple/Medium/Complex) and overall.

## Budgets

DeepSeek Flash <= 1100 requests for reps 0..2; `rep=0` across all tiers first,
then `rep=1,2`; exhausted cells are `not_run`.

## Outputs

`run-<timestamp>/{PROTOCOL.md, budget.json, llm/requests.jsonl, SUMMARY.json,
SUMMARY.md}`; each cell stores its artifact (candidate rs / codegen project)
and oracle records.
