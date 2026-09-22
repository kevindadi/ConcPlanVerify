# flash-gen-main-v2 — protocol (frozen, contracts-v3.1)

Requirements -> verified CIR -> LLM code -> tool post-verification. 24 tasks
across six families, tiers 8/8/8 (`GENERATION_MANIFEST.json` v3.1). The
requirement documents now carry an **Entities** section (role/resource names);
the contract is still hidden, but entity names are part of the requirements.

## Arms (contract hidden in all)

| arm | flow | oracle |
| --- | --- | --- |
| `G0_direct` | one-shot Rust from requirements | bounded monitor |
| `G1_self_iter` | generate -> self-review | bounded monitor |
| `G2_tools_iter` | generate -> build + Miri 16 + Lockbud | bounded monitor |
| `G3_concir` | generate CIR (contract hidden) -> explore vs contract -> revise -> **LLM writes Rust from the verified CIR** -> instrument v2 -> build -> `conform --op-resource` -> `monitor` -> behavior, with feedback | exhaustive (CIR) + bounded (code) |
| `G3_codegen` | ablation: same CIR pipeline, but the **tool** generates the Rust | exhaustive + conform |

K: G3 uses K_cir=4 and K_code=3; others K=4. 3 reps; `G3_codegen` rep 0 only.
DeepSeek Flash, temperature 0.

## Metrics

- `accept_rate`; **RF_all** (unaccepted cells count 0) and **RF_acc** (accepted
  cells only); RC.
- `defect` = accepted and (behavior hang or monitor FAIL or Miri detected or
  conform violation). `accepted_with_proof` = model PASS and conform PASS and
  monitor no FAIL (G3 only).
- cost: tokens/rounds, CIR stage vs code stage separated.
- per tier and overall.

## Budget

DeepSeek Flash <= 1100 requests for reps 0..2; `rep=0` all tiers first, then
`rep=1,2`; exhausted cells are `not_run`.

## Outputs

`run-<ts>/{PROTOCOL.md,budget.json,llm/requests.jsonl,SUMMARY.json,SUMMARY.md}`.
