
## Generation (main) — `flash-gen-main-v2`

Requirements -> verified CIR -> LLM code -> tool post-verification, 24 tasks, 3 reps. G0/G1/G2 are bounded-monitored; G3_concir verifies the CIR exhaustively, the LLM writes the Rust, and conform/monitor post-verify; G3_codegen is the tool-codegen ablation (rep 0).

Batches (later overrides earlier): `run-20260923T195455`.

| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | RF_run | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
`RF_all` counts non-accepted cells as 0; `RF_acc` covers accepted cells; `RF_run` averages cells that carry a monitor value (freeze-5 wording).
| G0_direct | 72 | 61 | 0.847 | 0.688 | 0.401 | 0.525 | 0.519 | 6 | 0 | 60877 |
| G1_self_iter | 72 | 58 | 0.806 | 0.694 | 0.319 | 0.534 | 0.569 | 3 | 0 | 219979 |
| G2_tools_iter | 72 | 39 | 0.542 | 0.699 | 0.313 | 0.683 | 0.611 | 0 | 0 | 328863 |
| G3_concir | 72 | 54 | 0.75 | 0.657 | 0.493 | 0.657 | 0.657 | 0 | 54 | 335599 |
| G3_codegen | 24 | 20 | 0.833 | 0.776 | 0.638 | 0.765 | 0.672 | 0 | 17 | 106808 |
| **all** | 312 | 232 | 0.744 | 0.694 | 0.401 | 0.61 | 0.594 | 9 | 71 | 1052126 |

Per tier (RF):

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir | G3_codegen |
| --- | --- | --- | --- | --- | --- |
| Simple | 0.392 | 0.266 | 0.137 | 0.499 | 0.773 |
| Medium | 0.479 | 0.344 | 0.456 | 0.642 | 0.642 |
| Complex | 0.331 | 0.347 | 0.345 | 0.337 | 0.499 |

`defect` = accepted and (behavior hang or monitor FAIL or conform violation). `awp` is G3-only (model PASS, conform PASS, no monitor FAIL).
