# Extraction track — termination criterion (J-6)

After the harness fixes (v5 → v6) the extraction track is **frozen**: the
validated count is **0 / 63**, below the `5 / 63` threshold. This is a
**limitation**, not a bug: LLM extraction from Rust source into a codegen-valid
CIR is unreliable, which is exactly the motivation for the model-first workflow.

## What was fixed this round

- **Backend panic → error** (ConcIR): a function name containing `::`
  (`Semaphore::new`) hit `.expect("header registered")` in
  `sem/program.rs`; it now returns `E102`. Regression:
  `tests/backend_errors.rs`.
- **Stronger label prompt** (`prompts/rust_to_cir_extract_v6.md`): labels are the
  skeleton; every label must appear exactly once (emit `nop` rather than drop).
- 14 harness-fixable v5 cells re-run (15 requests); 4 panic cells now produce
  `E102` instead of a panic.

## Result (63 accepted cells, extraction-v5 CELLS.json after v6)

| stage | count |
| --- | --- |
| codegen | 34 |
| conform | 25 |
| explore | 4 |
| **validated (PASS/FAIL)** | **0** |
| contract_invalid (explore, INVALID) | 4 |

## Failure-mode distribution (codegen / conform reasons)

| reason | count |
| --- | --- |
| traces not all conformant (model CIR ≠ code) | 25 |
| missing field `kind` (statement shape) | 9 |
| E931 undefined name (`thread_id`, `read_shared`, `Mutex`) | 8 |
| JSON parse: string `"0"` where i64 expected / map where string expected | 5 |
| E308 wrong resource kind (`read_shared`/`write_shared` on a sync resource) | 2 |
| E201 branch condition is not a comparison | 2 |
| E931 expected expression (`[]`, `!ready`) | 5 |
| E102 unregistered function name | 1 |

The four `explore` cells all return `INVALID`: the extracted CIR uses resource
names the frozen contract does not declare, so the contract cannot be evaluated.

## Interpretation

The model reliably produces *a* CIR, but not one that is simultaneously
(a) codegen-valid, (b) trace-conformant against the real code, and (c)
contract-evaluable. The three failure classes are all model-side; the harness
now classifies each cell with a concrete reason and never panics
(`harness_error = 0`). Per the round rule, no further extraction budget is spent;
the extraction track is reported as a **limitation** in the paper evidence map.
