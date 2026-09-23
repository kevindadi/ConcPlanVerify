
## Generation (main) — `flash-gen-main-v2`

Requirements -> verified CIR -> LLM code -> tool post-verification, 24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the CIR exhaustively, the LLM writes the Rust, and conform/monitor post-verify; G3_codegen is the tool-codegen ablation (rep 0).

Batches (later overrides earlier): `run-20260923T211345`.

| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | RF_run | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
`RF_all` counts non-accepted cells as 0; `RF_acc` covers accepted cells; `RF_run` averages cells that carry a monitor value (freeze-5 wording).
| G0_direct | 72 | 68 | 0.944 | 0.676 | 0.448 | 0.512 | 0.504 | 6 | 0 | 62708 |
| G1_self_iter | 72 | 62 | 0.861 | 0.699 | 0.396 | 0.559 | 0.545 | 3 | 0 | 284591 |
| G2_tools_iter | 72 | 47 | 0.653 | 0.701 | 0.379 | 0.681 | 0.581 | 0 | 0 | 323908 |
| G3_concir | 72 | 58 | 0.806 | 0.692 | 0.555 | 0.689 | 0.689 | 0 | 58 | 314729 |
| G3_codegen | 24 | 20 | 0.833 | 0.776 | 0.638 | 0.765 | 0.672 | 0 | 17 | 106808 |
| **all** | 312 | 255 | 0.817 | 0.699 | 0.459 | 0.618 | 0.586 | 9 | 75 | 1092744 |

Per tier (RF):

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir | G3_codegen |
| --- | --- | --- | --- | --- | --- |
| Simple | 0.462 | 0.457 | 0.326 | 0.668 | 0.773 |
| Medium | 0.551 | 0.384 | 0.465 | 0.637 | 0.642 |
| Complex | 0.331 | 0.347 | 0.345 | 0.361 | 0.499 |

`defect` = accepted and (behavior hang or monitor FAIL or conform violation). `awp` is G3-only (model PASS, conform PASS, no monitor FAIL).
