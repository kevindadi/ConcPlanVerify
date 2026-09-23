
## Generation (main) — `flash-gen-main-v2`

Requirements -> verified CIR -> LLM code -> tool post-verification, 24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the CIR exhaustively, the LLM writes the Rust, and conform/monitor post-verify; G3_codegen is the tool-codegen ablation (rep 0).

Batches (later overrides earlier): `run-20260923T182939`.

| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | RF_run | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
`RF_all` counts non-accepted cells as 0; `RF_acc` covers accepted cells; `RF_run` averages cells that carry a monitor value (freeze-5 wording).
| G0_direct | 72 | 70 | 0.972 | 0.661 | 0.472 | 0.531 | 0.523 | 6 | 0 | 61385 |
| G1_self_iter | 72 | 67 | 0.931 | 0.664 | 0.39 | 0.541 | 0.549 | 3 | 0 | 224344 |
| G2_tools_iter | 72 | 47 | 0.653 | 0.672 | 0.376 | 0.661 | 0.592 | 0 | 0 | 296491 |
| G3_concir | 72 | 40 | 0.556 | 0.622 | 0.344 | 0.619 | 0.619 | 0 | 40 | 439513 |
| G3_codegen | 24 | 20 | 0.833 | 0.776 | 0.638 | 0.765 | 0.672 | 0 | 17 | 106808 |
| **all** | 312 | 244 | 0.782 | 0.669 | 0.414 | 0.596 | 0.576 | 9 | 57 | 1128541 |

Per tier (RF):

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir | G3_codegen |
| --- | --- | --- | --- | --- | --- |
| Simple | 0.535 | 0.409 | 0.257 | 0.327 | 0.773 |
| Medium | 0.551 | 0.415 | 0.528 | 0.415 | 0.642 |
| Complex | 0.331 | 0.347 | 0.345 | 0.29 | 0.499 |

`defect` = accepted and (behavior hang or monitor FAIL or conform violation). `awp` is G3-only (model PASS, conform PASS, no monitor FAIL).
