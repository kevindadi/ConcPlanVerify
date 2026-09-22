
## Generation (main) — `flash-gen-main-v1`

Requirements -> program, four arms, 24 tasks, K=4, 3 reps. The contract
is hidden from every model prompt. Rust arms are scored by the bounded
monitor (§1); G3 by the exhaustive model verdict + conform.

Batches (later overrides earlier): `run-20260923T005232`.

| arm | cells | accepted | accept rate | RC | RF_all | RF_acc | defect | awp | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| G0_direct | 72 | 70 | 0.972 | 0.661 | 0.523 | 0.531 | 6 | 0 | 61385 |
| G1_self_iter | 72 | 67 | 0.931 | 0.664 | 0.549 | 0.541 | 3 | 0 | 224344 |
| G2_tools_iter | 72 | 47 | 0.653 | 0.672 | 0.592 | 0.661 | 0 | 0 | 296491 |
| G3_concir | 72 | 31 | 0.431 | 0.67 | 0.67 | 0.67 | 0 | 31 | 566854 |
| G3_codegen | 24 | 20 | 0.833 | 0.776 | 0.672 | 0.765 | 0 | 17 | 106808 |
| **all** | 312 | 235 | 0.753 | 0.677 | 0.581 | 0.602 | 9 | 48 | 1255882 |

Per tier (RF):

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir | G3_codegen |
| --- | --- | --- | --- | --- | --- |
| Simple | 0.558 | 0.517 | 0.517 | 0.511 | 0.773 |
| Medium | 0.661 | 0.623 | 0.674 | 0.72 | 0.656 |
| Complex | 0.361 | 0.524 | 0.577 | 0.91 | 0.586 |

`defect` = accepted and (hang/behavior or monitor/conform violation). `awp` is G3-only (model PASS and conform PASS).
