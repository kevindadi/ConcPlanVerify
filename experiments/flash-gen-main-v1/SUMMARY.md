# flash-gen-main-v1 — generation (main)

Requirements -> program, four arms, 24 tasks, K=4, 3 reps. Rust arms are
scored by the bounded monitor (§1); G3 by the exhaustive model verdict +
conform. The contract is hidden from every model prompt.

Merged batches (later overrides earlier): `run-20260922T001631`, `run-20260922T015338`.

## Per arm

| arm | cells | accepted | accept rate | RC | RF | defect | accepted-with-proof | tokens |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| G0_direct | 72 | 68 | 0.944 | 0.65 | 0.422 | 6 | 0 | 58145 |
| G1_self_iter | 72 | 67 | 0.931 | 0.656 | 0.46 | 1 | 0 | 202663 |
| G2_tools_iter | 72 | 46 | 0.639 | 0.639 | 0.468 | 0 | 0 | 293973 |
| G3_concir | 72 | 45 | 0.625 | 0.776 | 0.504 | 2 | 20 | 539360 |
| **all** | 288 | 226 | 0.785 | 0.686 | 0.466 | 9 | 20 | 1094141 |

## Per tier × arm (RF)

| tier | G0_direct | G1_self_iter | G2_tools_iter | G3_concir |
| --- | --- | --- | --- | --- |
| Simple | 0.396 | 0.39 | 0.421 | 0.666 |
| Medium | 0.517 | 0.53 | 0.53 | 0.305 |
| Complex | 0.369 | 0.464 | 0.46 | 0.542 |

`defect` = accepted and (hang/behavior or monitor/conform violation).
`accepted-with-proof` is G3-only (model PASS and conform PASS).
